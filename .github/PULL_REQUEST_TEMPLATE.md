## Summary

Brief description of what this PR changes and why.

## Type

- [ ] Feature
- [ ] Bug fix
- [ ] Refactor
- [ ] Documentation
- [ ] Test
- [ ] New deterministic function
- [ ] New pack
- [ ] Infrastructure / CI

## Tier

- [ ] NanoClaw (edge)
- [ ] RuFlo (brain / routing)
- [ ] RuVector (knowledge)
- [ ] Cross-tier
- [ ] CLI / REPL
- [ ] Pack system (nstn-packs)
- [ ] Infrastructure / CI

---

## Core Checklist

- [ ] `cargo fmt --check` passes
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` — zero warnings
- [ ] `cargo test --workspace` — all tests pass
- [ ] New public functions have doc comments explaining WHY
- [ ] New public functions have unit tests
- [ ] No new `unsafe` code
- [ ] Protobuf contracts unchanged (or changes are backwards-compatible)
- [ ] CHANGELOG.md updated

---

## Deterministic Function Checklist

_(Complete this section if adding or modifying a deterministic function. Skip otherwise.)_

- [ ] Function is pure — same input always produces same output
- [ ] No LLM calls, no network calls, no side effects
- [ ] Added to the correct module in `crates/common/src/deterministic/`
- [ ] Pattern matching added in `try_deterministic_resolution()` (if it should intercept queries)
- [ ] Unit tests cover: happy path, edge cases, and error cases
- [ ] `#[must_use]` attribute applied
- [ ] Added to module table in README.md
- [ ] Swift equivalent added to `NanoClawKit` if it's a parity function

---

## Pack Submission Checklist

_(Complete this section if this PR adds or updates an nstn-pack. Skip otherwise.)_

- [ ] `pack.toml` is present and valid (name, version, author, description, function entries)
- [ ] `rules.toml` is present if using the TOML rule evaluator
- [ ] All Rust functions in `src/lib.rs` are pure (no side effects, no `unsafe`)
- [ ] All Rust functions have unit tests
- [ ] Pack name is kebab-case and does not conflict with existing packs
- [ ] Activation trigger patterns do not shadow core routing tiers
- [ ] Pack is listed in `packs/registry.toml`
- [ ] README or `pack.toml` description explains what the pack does and example queries it handles
- [ ] `cargo test -p <pack-name>` passes

---

## Testing

How was this tested? Which test cases cover the change?

```bash
# Paste any relevant test commands here
cargo test -p nstn-common -- deterministic
```

## Related Issues

Closes #
