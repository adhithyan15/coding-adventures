# smart-home-controller-runtime

Central durable ownership for the normalized smart-home runtime, its automation
engine, and their shared `smart-home-runtime-store` envelope.

The controller restores both runtimes together and exposes their existing
`Arc<Mutex<_>>` adapter handles. Its transaction API serializes callers,
mutates cloned candidates, saves runtime state, automation definitions,
automation idempotency state, and audit history as one durable revision, then
publishes both candidates. Callback, encoding, storage, and compare-and-swap
failures leave the shared state and controller revision unchanged.

HTTP adapters that already hold the shared mutexes can use
`runtime_persistence_adapter()` and `automation_persistence_adapter()` without
recursively locking the supplied runtime values. Native workers can use
`transaction()`, `evaluate_automations()`, or `tick()`.

```bash
bash BUILD
```
