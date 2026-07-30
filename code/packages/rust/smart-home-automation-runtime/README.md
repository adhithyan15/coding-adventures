# smart-home-automation-runtime

Deterministic schedule and event automation execution over
`smart-home-runtime`.

The engine owns typed definitions, conditions, scene expansion, stable
idempotency keys, dry-run plans, and a durable audit journal. It delegates
every device mutation to the existing runtime command tool so capability
authorization and command audit remain on the canonical D23 path.

Definitions and the engine snapshot serialize into the existing
`smart-home-runtime-store` envelope. That lets the production local controller
restore definitions, consumed trigger occurrences, and audit records together
with normalized home state.

```bash
bash BUILD
```
