# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| 0.1.x   | Yes       |

## Reporting a Vulnerability

This is a private repository. If you discover a security vulnerability:

1. **Do NOT open a public issue.**
2. Email: jonathan.bonner@stu.bmcc.cuny.edu
3. Include:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if any)

You will receive a response within 48 hours.

## Security Design

### Sovereignty Model
- All data stays on user-controlled infrastructure
- No telemetry or data exfiltration
- LLM API keys are user-held, never stored by the system
- No vendor lock-in — models and infrastructure are swappable

### Permission Model
- `ReadOnly` — NanoClaw edge tier (can't modify files)
- `WorkspaceWrite` — domain agents (can modify project files)
- `DangerFullAccess` — admin only (full system access)
- Permissions are enforced per-agent via TOML config

### Code Security
- Zero `unsafe` Rust code (workspace-level lint)
- All dependencies auditable via `cargo audit`
- Protobuf contracts enforce typed boundaries between tiers
- Handoff validation prevents specification-gap attacks (MAST research)

### ruflo Isolation
- ruflo runs as a sandboxed child process
- Communication is via stdio JSON-RPC only — no shared memory
- ruflo has no direct access to the Rust orchestrator's state
- Bridge timeout prevents hung processes (30s default)
