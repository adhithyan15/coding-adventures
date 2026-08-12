# Changelog

## Unreleased

- Add provider-neutral Level 1 tool turns that carry an exact catalog and prior
  call/result transcript while returning either final text or one structured
  tool call without granting the skill runtime execution authority.
- Bind authenticated channel UUIDs and bounded model settings to an independently
  verified Level 1 package, rejecting missing, extra, or wrong-direction names.
- Construct Level 1 runtimes directly from authenticated, policy-matched sealed
  SKILL packages.

## 0.1.0 - 2026-08-03

- Add a provider-neutral Level 1 `SKILL.md` turn executor.
- Add receive, LLM completion, publish, then acknowledge channel coordination.
- Preserve replayability when model calls or output publication fail.
