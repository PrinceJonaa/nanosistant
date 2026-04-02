# Model Providers

Nanosistant supports five model provider families. The active provider is selected automatically from environment variables; you can override it by setting specific variables or by passing a model name explicitly.

> **See also:** [Quickstart](./quickstart.md) · [nstn-ruflo — model_router](../crates/nstn-ruflo.md#module-model_router) · [Crate Reference — nstn-api](../crates/README.md)

---

## Provider Overview

| Provider | Env variable | Model family | Default base URL |
|---|---|---|---|
| **Anthropic** | `ANTHROPIC_API_KEY` | `claude-*` | `https://api.anthropic.com` |
| **Azure OpenAI** | `AZURE_OPENAI_API_KEY` + `AZURE_OPENAI_BASE_URL` | `gpt-*`, `o1`, `o3`, `o4` | Azure deployment URL |
| **OpenAI** | `OPENAI_API_KEY` | `gpt-*`, `o1`, `o3`, `o4` | `https://api.openai.com/v1` |
| **xAI (Grok)** | `XAI_API_KEY` | `grok-*` | `https://api.x.ai/v1` |
| **Local / Ollama** | `OPENAI_BASE_URL` (no API key) | any | `http://localhost:11434/v1` |

---

## Anthropic

Anthropic is the primary and recommended provider. The `nstn-api` crate uses Anthropic's native Messages API with SSE streaming.

```bash
export ANTHROPIC_API_KEY=sk-ant-api03-...
./target/debug/nanosistant
```

**Override the base URL** (e.g. for a proxy or enterprise endpoint):

```bash
export ANTHROPIC_BASE_URL=https://your-proxy.example.com
export ANTHROPIC_API_KEY=sk-ant-...
./target/debug/nanosistant
```

### Supported Models

| Alias | Canonical model ID | Token limit |
|---|---|---|
| `opus` | `claude-opus-4-6` | 32 000 |
| `sonnet` (default) | `claude-sonnet-4-6` | 16 000 |
| `haiku` | `claude-haiku-4-5-20251213` | 8 000 |
| `claude-sonnet-4-20250514` | `claude-sonnet-4-20250514` | 16 000 |
| `claude-opus-4-20250514` | `claude-opus-4-20250514` | 32 000 |
| `claude-haiku-4-20250514` | `claude-haiku-4-20250514` | 8 000 |

Use the short aliases in `config/settings.toml` or when passing `--model` on the CLI:

```bash
./target/debug/nanosistant --model opus
```

### Agent Configuration

In `config/agents/*.toml`, set the `model` field:

```toml
[agent]
model = "claude-sonnet-4-20250514"
```

This is treated as a **floor** by the `model_router` — the router may select a more capable tier for complex queries but never downgrades below it.

---

## Azure OpenAI

Azure OpenAI uses the OpenAI-compatible API format but requires an Azure deployment URL and uses the `api-key` header instead of `Authorization: Bearer`.

```bash
export AZURE_OPENAI_API_KEY=your-azure-key
export AZURE_OPENAI_BASE_URL=https://your-resource.openai.azure.com/openai/deployments/your-deployment-name
./target/debug/nanosistant --model gpt-4o
```

The URL format is:
```
{AZURE_OPENAI_BASE_URL}/chat/completions?api-version=2025-04-01-preview
```

The API version is fixed at `2025-04-01-preview` in `nstn-api`.

### Supported models

Any model deployed to your Azure OpenAI resource. The provider is auto-detected for model names matching the `gpt-*`, `o1`, `o3`, `o4` patterns when `AZURE_OPENAI_API_KEY` is set.

```bash
./target/debug/nanosistant --model gpt-4o
./target/debug/nanosistant --model o3-mini
```

---

## OpenAI

Standard OpenAI API with Bearer token authentication.

```bash
export OPENAI_API_KEY=sk-...
./target/debug/nanosistant --model gpt-4o
```

**Override the base URL:**

```bash
export OPENAI_API_KEY=sk-...
export OPENAI_BASE_URL=https://api.openai.com/v1   # default; change for proxies
./target/debug/nanosistant --model gpt-4o
```

### Supported models

Any model available on the OpenAI API (GPT-4o, GPT-4o mini, o1, o3, o4, etc.). The provider is auto-detected for `gpt-*`, `o1`, `o3`, `o4` model names when `OPENAI_API_KEY` is set and `AZURE_OPENAI_API_KEY` is not.

---

## xAI (Grok)

xAI models (Grok) use an OpenAI-compatible API.

```bash
export XAI_API_KEY=xai-...
./target/debug/nanosistant --model grok-3
```

**Override the base URL:**

```bash
export XAI_API_KEY=xai-...
export XAI_BASE_URL=https://api.x.ai/v1   # default
./target/debug/nanosistant --model grok
```

### Model aliases

| Alias | Canonical model ID | Token limit |
|---|---|---|
| `grok` | `grok-3` | 64 000 |
| `grok-3` | `grok-3` | 64 000 |
| `grok-mini` | `grok-3-mini` | 64 000 |
| `grok-3-mini` | `grok-3-mini` | 64 000 |
| `grok-2` | `grok-2` | 64 000 |

---

## Local via Ollama

Run any Ollama model with no API key by pointing `OPENAI_BASE_URL` at the local Ollama server.

```bash
# Start Ollama first
ollama serve
ollama pull llama3

# Point NSTN at it
export OPENAI_BASE_URL=http://localhost:11434/v1
./target/debug/nanosistant --model llama3
```

No `OPENAI_API_KEY` is needed; Ollama ignores the Authorization header.

Any model supported by Ollama works: `llama3`, `mistral`, `codellama`, `phi3`, etc.

**Note on capabilities:** Local models may not support all features (structured tool calls, streaming, etc.). The deterministic routing tier (Tier 0 through Tier 4) works without any model — only the LLM agent turns are affected.

---

## Auto-Detection Logic

When no explicit `--model` flag is passed, the provider is selected from environment variables in this priority order:

```
detect_provider_kind(model: &str) → ProviderKind
```

1. **Model name in the registry** — If the model name (or alias) is in the `MODEL_REGISTRY` lookup table in `nstn-api/src/providers/mod.rs`, the registered provider is used unconditionally.
   - `claude-*` → Anthropic
   - `grok-*` → xAI

2. **Azure model shape + key** — If the model name looks like a GPT/o-series model (`gpt-*`, `o1`, `o3`, `o4`, `davinci`, `turbo`) **and** `AZURE_OPENAI_API_KEY` is set → Azure OpenAI.

3. **OpenAI key present** — Same model shape but no Azure key → OpenAI.

4. **`ANTHROPIC_API_KEY` present** → Anthropic (fallback for unknown model names).

5. **`OPENAI_API_KEY` present** → OpenAI.

6. **`XAI_API_KEY` present** → xAI.

7. **`AZURE_OPENAI_API_KEY` present** → Azure OpenAI.

8. **Default** → Anthropic (attempts OAuth login flow if no key found).

In practice, set exactly one API key environment variable and the correct provider is selected automatically.

---

## Environment Variable Summary

| Variable | Provider | Required? | Notes |
|---|---|---|---|
| `ANTHROPIC_API_KEY` | Anthropic | Yes (for Anthropic) | `sk-ant-api03-...` |
| `ANTHROPIC_BASE_URL` | Anthropic | No | Override for proxies |
| `OPENAI_API_KEY` | OpenAI | Yes (for OpenAI) | `sk-...` |
| `OPENAI_BASE_URL` | OpenAI / Ollama | No | Override or local server |
| `AZURE_OPENAI_API_KEY` | Azure OpenAI | Yes (for Azure) | |
| `AZURE_OPENAI_BASE_URL` | Azure OpenAI | Yes (for Azure) | Full deployment URL |
| `XAI_API_KEY` | xAI | Yes (for xAI) | `xai-...` |
| `XAI_BASE_URL` | xAI | No | Override for proxies |
| `RUST_LOG` | All | No | `info`, `debug`, `trace` |

---

## Model Tier Selection (`model_router`)

The `model_router` module in `nstn-ruflo` can upgrade the model tier for complex queries, independent of the provider. The agent TOML `model` field acts as a **floor**:

| Condition | Selected tier |
|---|---|
| `router_confidence ≥ 0.95` and `complexity < 0.30` | Fast (haiku / grok-mini) |
| `router_confidence ≥ 0.70` and `complexity < 0.65` | Balanced (sonnet / grok-3) |
| `complexity ≥ 0.65` or `domain == "framework"` | Powerful (opus) |
| (default) | Balanced |

If the agent's configured model is already `claude-opus-4-20250514`, the router never downgrades to sonnet — it can only stay at opus or upgrade (which has no effect since opus is the ceiling).

This is a deterministic decision — no LLM is called to select the model.
