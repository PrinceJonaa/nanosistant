---
name: Bug Report
about: Report a bug in Nanosistant
title: "[BUG] "
labels: bug
assignees: PrinceJonaa
---

## Description

A clear, concise description of the bug.

## Steps to Reproduce

- [ ] Step 1: …
- [ ] Step 2: …
- [ ] Step 3: …

## Expected Behavior

What should happen.

## Actual Behavior

What actually happens. Include any error messages verbatim.

## Which tier failed?

- [ ] NanoClaw (edge) — local resolution, offline queue, iOS client
- [ ] RuFlo (brain) — routing, orchestrator, budget, watchdog
- [ ] RuVector (knowledge) — vector search, ingestion, RAG
- [ ] Cross-tier
- [ ] Not sure

## Which routing tier failed? (if applicable)

- [ ] Tier 0 — Deterministic function
- [ ] Tier 1 — Aho-Corasick automaton
- [ ] Tier 2 — Regex bank
- [ ] Tier 3 — Weighted keywords
- [ ] Tier 4 — Fuzzy / Levenshtein
- [ ] Tier 6 — ruflo MCP
- [ ] Tier 7 — LLM classifier
- [ ] Not routing-related

## Was the path deterministic or LLM?

- [ ] Deterministic (zero tokens — pure function output)
- [ ] LLM (tokens spent on classification or response)
- [ ] ruflo MCP
- [ ] Unknown

## Crate / Module

Which crate or module is affected? (e.g. `nstn-common`, `nstn-ruflo`, `nstn-ruvector`, `nstn-nanoclaw`, deterministic module name)

## Environment

- **OS:**
- **Rust version:** (`rustc --version`)
- **Nanosistant version:** (`nanosistant --version`)
- **ruflo available:** yes / no
- **Qdrant available:** yes / no

## Logs

```
paste logs here — use RUST_LOG=debug for verbose output
```

## Additional Context

Any other context, config snippets, or related issues.
