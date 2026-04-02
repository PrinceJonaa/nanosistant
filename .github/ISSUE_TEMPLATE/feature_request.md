---
name: Feature Request
about: Propose a new feature or enhancement
title: "[FEAT] "
labels: enhancement
assignees: PrinceJonaa
---

## Problem

What problem does this solve, or what opportunity does it create?

## Proposed Solution

How should it work? Be as specific as possible.

## Which tier would this affect?

- [ ] NanoClaw (edge)
- [ ] RuFlo (brain / routing)
- [ ] RuVector (knowledge)
- [ ] Cross-tier
- [ ] CLI / REPL
- [ ] Pack system (nstn-packs)
- [ ] iOS client (NanoClawKit)

## Is this a new deterministic function?

- [ ] Yes — it's a pure function (same input → same output, zero tokens)
- [ ] No — LLM inference is required
- [ ] Partially — some sub-steps are deterministic

If yes, which module would it belong to?

- [ ] logic
- [ ] graph
- [ ] information
- [ ] probability
- [ ] music
- [ ] finance
- [ ] data
- [ ] time
- [ ] text
- [ ] code
- [ ] geo
- [ ] physics
- [ ] health
- [ ] social
- [ ] new module needed

## Is this a pack proposal?

- [ ] Yes — this should ship as an nstn-pack (domain-specific, optional install)
- [ ] No — this belongs in the core

## Does this affect the routing pipeline?

- [ ] Yes — changes confidence ladder behavior
- [ ] Yes — adds a new routing tier
- [ ] No

## Design Considerations

- Does this add a new agent domain?
- Does this require new protobuf messages?
- Does this interact with ruflo MCP?
- Could it break existing deterministic behavior?

## Alternatives Considered

What other approaches did you consider?

## Additional Context

Links, prior art, related issues, or anything else relevant.
