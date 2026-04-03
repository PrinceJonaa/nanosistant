## Description

Brief description of changes. Link to related issue.

Closes #

## Type

- [ ] Bug fix
- [ ] Feature
- [ ] Refactor
- [ ] Documentation
- [ ] Test
- [ ] Pack submission
- [ ] Deterministic function

## Component

- [ ] NanoClaw (Edge)
- [ ] RuFlo (Brain)
- [ ] RuVector (Knowledge)
- [ ] Pack System
- [ ] CLI / REPL
- [ ] API Client
- [ ] iOS Client
- [ ] Documentation
- [ ] CI / Infrastructure

## Verification

- [ ] `cargo fmt --check` passes
- [ ] `cargo clippy --workspace --all-targets` — zero warnings
- [ ] `cargo test --workspace` — all tests pass
- [ ] New public functions have doc comments
- [ ] New public functions have unit tests

### If adding a deterministic function:

- [ ] Function is pure (no side effects, no I/O, no LLM calls)
- [ ] `#[must_use]` added to all pure return functions
- [ ] Placed in correct det/ module (universal/ or domain/)
- [ ] Pattern added to `try_deterministic_resolution()` if it handles natural language
- [ ] Swift parity considered (should NanoClawKit get this too?)

### If submitting a pack:

- [ ] `pack.toml` with all required fields
- [ ] `rules.toml` and/or `functions.rs` included
- [ ] Every function has at least one test
- [ ] `README.md` explains what the pack does
- [ ] Test coverage noted in pack.toml

## Testing

How was this tested? Which test cases cover the change?
