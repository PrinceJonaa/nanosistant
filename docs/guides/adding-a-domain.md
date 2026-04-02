# Adding a Domain Agent

This guide walks through adding a new domain agent to Nanosistant. A domain agent is a specialised AI assistant scoped to a particular knowledge area (music, finance, fitness, legal, etc.). The routing pipeline directs messages to the correct domain agent automatically.

> **See also:** [Quickstart](./quickstart.md) · [nstn-ruflo — agent_config](../crates/nstn-ruflo.md#module-agent_config) · [nstn-common — router](../crates/nstn-common.md#module-router) · [nstn-ruvector](../crates/nstn-ruvector.md)

---

## Overview

Adding a domain agent requires four artifacts:

1. **Agent config TOML** — identity, model, permission mode, and routing keywords.
2. **System prompt** — the domain expertise file loaded as context.
3. **Routing keywords** — what the agent responds to.
4. **(Optional) Knowledge documents** — domain-specific documents ingested into the vector store.

The routing pipeline picks up the new agent automatically at startup via `load_agent_configs()`.

---

## Step 1: Create the Agent Config TOML

Create a new file at `config/agents/{domain}.toml`. Use one of the existing agents as a template.

**Example:** `config/agents/fitness.toml`

```toml
[agent]
name = "fitness"
description = "Personal training, nutrition, and wellness specialist."
model = "claude-sonnet-4-20250514"
permission_mode = "read_only"

[agent.triggers]
keywords = [
    "workout",
    "training",
    "exercise",
    "nutrition",
    "calories",
    "protein",
    "cardio",
    "strength",
    "macros",
    "cut",
    "bulk",
    "recovery",
    "sleep",
    "heart rate",
    "VO2 max",
    "rep",
    "set",
    "periodization",
]
priority = 10

[agent.prompt]
identity_file = "config/prompts/identity.md"
domain_file = "config/prompts/fitness.md"

[agent.knowledge]
domain_filter = "fitness"
auto_retrieve = true

[agent.tools]
include = ["read_file"]
deterministic = [
    "word_count",
    "percentage_change",
    "days_until",
]
```

### Config Fields Reference

| Field | Type | Description |
|---|---|---|
| `name` | `String` | Unique domain identifier. Must match the file name (e.g. `"fitness"` → `fitness.toml`). |
| `description` | `String` | Human-readable description shown in `--agents` output. |
| `model` | `String` | Model ID. See [Model Providers](./model-providers.md). Acts as a floor — the model router may upgrade it for complex queries. |
| `permission_mode` | `String` | `"read_only"` \| `"workspace_write"` \| `"danger_full_access"`. Start with `"read_only"` and escalate only as needed. |
| `triggers.keywords` | `Vec<String>` | Words and phrases that trigger routing to this agent. |
| `triggers.priority` | `u32` | Higher priority wins when two domains tie. `general` uses 0; domain agents typically use 10–20. |
| `prompt.identity_file` | `String` | Shared identity document (usually `config/prompts/identity.md`). |
| `prompt.domain_file` | `String` | Domain-specific system prompt. |
| `knowledge.domain_filter` | `String` | Filter for vector store queries — only chunks tagged with this domain are retrieved. |
| `knowledge.auto_retrieve` | `bool` | Whether to automatically run a knowledge query before each agent turn. |
| `tools.include` | `Vec<String>` | Tool names enabled for this agent (e.g. `"bash"`, `"read_file"`, `"write_file"`). |
| `tools.deterministic` | `Vec<String>` | Deterministic function names from `nstn-common` exposed as tools to this agent. |

---

## Step 2: Create the System Prompt

Create a domain-specific prompt file at the path specified in `prompt.domain_file`.

**Example:** `config/prompts/fitness.md`

```markdown
# Fitness Agent

You are an expert personal trainer and nutritionist with deep knowledge of:
- Resistance training (programming, periodization, technique cues)
- Cardiovascular conditioning (zone training, VO2 max, lactate threshold)
- Sports nutrition (macronutrient strategy, meal timing, supplementation)
- Recovery (sleep, HRV, deload weeks, mobility)
- Goal-setting frameworks (SMART goals, progress tracking, body composition)

## Principles

- Evidence-based recommendations only. Cite research when claiming specific effects.
- Always ask about the user's current fitness level, goals, and any injuries before programming.
- Never provide medical diagnoses. Refer to healthcare professionals for injury assessment.
- Prefer progressive overload and sustainable habits over quick-fix approaches.

## Available Tools

You have access to these deterministic functions:
- `percentage_change(from, to)` — for calculating progress (body weight, strength PRs, etc.)
- `days_until(date)` — for event-based periodization (competition, holiday, etc.)

Use them instead of approximating arithmetic.
```

**Best practices for domain prompts:**

- State clearly what the agent specialises in and what it defers on.
- List the deterministic tools available so the model uses them instead of approximating math.
- Set hard boundaries (e.g. "never provide medical diagnoses").
- Keep the prompt focused — the shared `identity.md` already covers the system's general behaviour.

---

## Step 3: Add Routing Keywords

Keywords are the primary signal for the confidence-ladder router. The `router_from_trigger_configs()` function in `nstn-common` converts them into:

- **Tier 1 (Aho-Corasick):** literal keyword matches — O(n + z) per query.
- **Tier 2 (Regex):** morphological variants (e.g. `"workout"` → `r"\bworkou\w*\b"` catches `workouts`, `working out`).
- **Tier 3 (Weighted keywords):** token-level scoring.
- **Tier 4 (Fuzzy):** edit-distance typo recovery for keywords ≥ 4 characters.

### Keyword Selection Guidelines

| Guideline | Example |
|---|---|
| **Multi-word phrases are high-precision anchors** — a single match is sufficient. Assign weight 0.95. | `"heart rate zone"`, `"macro split"`, `"rep max"` |
| **Longer single words are more specific** — prefer 6+ characters. Weight 0.85. | `"nutrition"`, `"periodization"`, `"recovery"` |
| **Short words need context** — they are lower-weight (0.70) and can collide with other domains. | `"cut"`, `"set"`, `"rep"` |
| **Avoid generic words** that appear in natural conversation unrelated to your domain. | Avoid `"plan"`, `"goal"`, `"today"` |
| **Use domain jargon** — specificity prevents cross-domain false positives. | `"VO2 max"`, `"deload"`, `"periodization"` |

Higher `priority` breaks ties when two domains score equally. Increment priority for agents that should dominate their niche.

---

## Step 4: Ingest Domain Knowledge (Optional)

If your agent needs to answer questions from a body of documents (research papers, internal wikis, playbooks, etc.), ingest them into the vector store.

### Add an ingestion source

Edit `config/ingestion.toml`:

```toml
[[source]]
path = "knowledge/fitness"
domain = "fitness"
doc_type = "reference"
extensions = ["md", "txt"]
recursive = true
```

Place your documents in `knowledge/fitness/`:

```
knowledge/fitness/
    programming-foundations.md
    nutrition-guide.md
    periodization-models.md
```

Documents should be structured markdown with `## ` (H2) section headers — the ingestion pipeline splits on these headers to create individual knowledge chunks.

**Example document structure:**

```markdown
# Programming Foundations

## Progressive Overload

Progressive overload is the fundamental principle of strength training...

## Periodization Models

Linear periodization applies a consistent progression each session...

## Deload Strategy

A deload week reduces volume and/or intensity by 40–60%...
```

### Run ingestion

```bash
cargo run -p nstn-ruvector --bin ingest -- --config config/ingestion.toml
```

Or programmatically:

```rust
use nstn_ruvector::{IngestionPipeline, IngestionSource, VectorStore};

let mut store = VectorStore::in_memory();
let mut pipeline = IngestionPipeline::new();
pipeline.add_source(IngestionSource {
    path: "knowledge/fitness".into(),
    domain: "fitness".into(),
    doc_type: "reference".into(),
    extensions: vec!["md".into()],
    recursive: true,
});
let result = pipeline.run(&mut store);
println!("{} chunks ingested", result.chunks_ingested);
```

When `knowledge.auto_retrieve = true` in the agent config, the runtime automatically runs a `KnowledgeQuery` before each agent turn and injects the top-matching chunks as context.

---

## Step 5: Test Routing

After adding the config and rebuilding, test that messages route to your new agent:

```bash
cargo build --workspace

# Expect routing to "fitness"
./target/debug/nanosistant --print "how should I structure my workout split"

# Expect routing to "fitness"  
./target/debug/nanosistant --print "how many calories should I eat on a bulk"

# Expect NOT to route to "fitness" (should go to general)
./target/debug/nanosistant --print "what's the weather today"
```

### Inspect routing decisions

Run with `RUST_LOG=debug` to see the confidence ladder in action:

```bash
RUST_LOG=nstn_ruflo=debug ./target/debug/nanosistant --print "nutrition plan for bulking"
```

You will see output like:

```
DEBUG nstn_ruflo::orchestrator: confidence-ladder routed
      domain="fitness" confidence=0.847 tier=1
```

- `tier=1` — resolved by the Aho-Corasick automaton (fastest, most precise).
- `tier=2` — resolved by regex morphology.
- `tier=3` — resolved by weighted keyword scoring.
- `tier=4` — resolved by fuzzy edit-distance.
- `tier=6` — resolved by ruflo MCP (Q-learning / MoE / semantic).
- No tier (`Ambiguous`) — fell through to the LLM escape hatch.

### Test the routing unit-level

Add a test to `crates/ruflo/tests/` to verify routing:

```rust
#[test]
fn routes_fitness_message_to_fitness_agent() {
    let mut orch = build_orchestrator_with_fitness();
    let result = orch.route("s1", "help me structure a workout split", "");
    match result {
        RouteResult::AgentRoute { domain, .. } => assert_eq!(domain, "fitness"),
        other => panic!("expected AgentRoute, got {other:?}"),
    }
}
```

---

## Existing Agents for Reference

| Agent | Config | Keywords (sample) |
|---|---|---|
| `general` | `config/agents/general.toml` | (none — fallback) |
| `music` | `config/agents/music.toml` | verse, hook, beat, BPM, 808, vocal chain, DAW |
| `investment` | `config/agents/investment.toml` | stock, earnings, trade, 13F, institutional |
| `development` | `config/agents/development.toml` | code, rust, bug, deploy, refactor, compile |
| `framework` | `config/agents/framework.toml` | distortion lattice, false prophet, archetype |
