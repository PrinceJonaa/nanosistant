# NSTN Hub

> The deterministic function registry for [Nanosistant](https://github.com/PrinceJonaa/nanosistant).

Install verified, tested function packs. Pure math and logic — no LLM calls, no tokens burned, no hallucinations. Every function has a test proving it works.

## Quick install

```bash
nanosistant install music-theory-core
```

## Browse packs

Visit the hub at: https://princejonaa.github.io/nstn-hub/

## Contributing a pack

1. Fork [nanosistant](https://github.com/PrinceJonaa/nanosistant)
2. Create `packs/your-domain/your-pack-name/pack.toml`
3. Add `rules.toml` (TOML rules) and/or `functions.rs` (Rust)
4. Add tests in `tests/`
5. Open a PR

See the [Contribute](https://princejonaa.github.io/nstn-hub/#contribute) page for full format docs.

## License

MIT
