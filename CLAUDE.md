# CLAUDE.md — Nanosistant (NSTN-1)

## Project

Nanosistant is a general-purpose personal AI system. Three-tier Rust architecture:
NanoClaw (edge) → RuFlo (brain) → RuVector (knowledge). Protobuf contracts at tier boundaries.

## Stack

- Language: Rust (all tiers)
- Async: tokio
- HTTP: axum
- Serialization: serde + serde_json (internal), prost + tonic (cross-tier gRPC)
- LLM: Anthropic Claude API (primary), OpenAI-compatible (secondary via provider trait)
- Vector DB: Qdrant (self-hosted)
- Foundation: claw-code Rust crates (forked)

## Rules

1. Tiers never import each other's internals. Protobuf is the boundary.
2. Orchestrator is deterministic code, not an LLM.
3. Each domain agent is a ConversationRuntime with scoped prompt + tools.
4. Handoffs between agents use typed Rust structs → protobuf. No freeform text handoffs.
5. MCP is the tool transport layer, not the orchestration layer.
6. Closed-form operations (math, lookups, templates) run as code. Zero tokens.
7. Tool executions go through the hook system (pre/post) from claw-code.
8. Do not build infrastructure speculatively. Build when pain demands it.
9. Do not scope this as a single-domain project. It handles ANY task domain.

## Verification

cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

## Working Agreement

- Small, reviewable changes
- Every public function has tests
- Comments explain WHY
- Zero clippy warnings
- Minimize dependency tree
- When in doubt, keep it simple
