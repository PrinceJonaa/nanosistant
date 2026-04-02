# Nanosistant Documentation

**Nanosistant (NSTN-1)** is a self-hosted, sovereignty-first personal AI system built on a three-tier architecture: NanoClaw (edge) → RuFlo (brain) → RuVector (knowledge).

> For the project overview and build instructions, see the [root README](../README.md).

---

## Architecture

Design documentation for the system as a whole.

| Document | Description |
|---|---|
| [Architecture Overview](./architecture/overview.md) | Three-tier architecture, mermaid diagrams, system invariants |
| [Routing Pipeline](./architecture/routing.md) | Seven-tier confidence ladder deep dive (Tier 0 → Tier 6) |
| [Memory System](./architecture/memory.md) | L0–L3 four-tier memory, dreaming loop, patch proposals |
| [Typed IR](./architecture/typed-ir.md) | Typed intermediate representations — the LLM proposal / Rust execution boundary |
| [Security](./architecture/security.md) | Permission modes, tool sandboxing, handoff validation, distortion flags |
| [Deployment](./architecture/deployment.md) | Production deployment patterns, scaling, monitoring |

---

## Crate Reference

API documentation for the 13 workspace crates.

| Document | Crate | Description |
|---|---|---|
| [Crate Reference](./crates/README.md) | all | Table of all 13 crates with LoC, key types, and dependency graph |
| [nstn-common](./crates/nstn-common.md) | `nstn-common` | 30+ deterministic functions, confidence-ladder router, event system, typed IR, proto types |
| [nstn-ruflo](./crates/nstn-ruflo.md) | `nstn-ruflo` | Orchestrator, multi-tier memory, dreaming loop, watchdog, MCP bridge, gRPC server |
| [nstn-ruvector](./crates/nstn-ruvector.md) | `nstn-ruvector` | Vector store, embeddings, document ingestion pipeline, MCP server |
| [nstn-nanoclaw](./crates/nstn-nanoclaw.md) | `nstn-nanoclaw` | Edge runtime, gRPC client, local executor, offline queue |

---

## Developer Guides

Step-by-step guides for common tasks.

| Document | Description |
|---|---|
| [Quickstart](./guides/quickstart.md) | Prerequisites, build, tests, first run, Docker |
| [Adding a Domain Agent](./guides/adding-a-domain.md) | Create TOML config, system prompt, keywords, knowledge ingestion, test routing |
| [Model Providers](./guides/model-providers.md) | Anthropic, Azure OpenAI, OpenAI, xAI, Ollama, auto-detection logic |

---

## Diagrams

Architecture and flow diagrams are embedded in the architecture documents above using Mermaid. Standalone diagram files (where they exist) are in [`./diagrams/`](./diagrams/).

---

## Quick Navigation

**I want to…**

- **Build and run the project** → [Quickstart](./guides/quickstart.md)
- **Add a new domain** → [Adding a Domain Agent](./guides/adding-a-domain.md)
- **Connect a different AI model** → [Model Providers](./guides/model-providers.md)
- **Understand the routing system** → [Routing Pipeline](./architecture/routing.md) · [nstn-common router module](./crates/nstn-common.md#module-router)
- **Understand the memory system** → [Memory System](./architecture/memory.md) · [nstn-ruflo memory module](./crates/nstn-ruflo.md#module-memory)
- **Use deterministic functions** → [nstn-common deterministic module](./crates/nstn-common.md#module-deterministic)
- **Set up knowledge retrieval** → [nstn-ruvector](./crates/nstn-ruvector.md) · [Adding a Domain — Step 4](./guides/adding-a-domain.md#step-4-ingest-domain-knowledge-optional)
- **Run the edge tier** → [nstn-nanoclaw](./crates/nstn-nanoclaw.md)
- **Run the gRPC server** → [nstn-ruflo — grpc_server](./crates/nstn-ruflo.md#module-grpc_server)
- **Understand the dreaming loop** → [nstn-ruflo — dreamer](./crates/nstn-ruflo.md#module-dreamer)
- **See all 13 crates at a glance** → [Crate Reference](./crates/README.md)

---

## Version

**Documentation version:** 0.5.0  
**Last updated:** April 2026
