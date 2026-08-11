# smart-home-controller-runtime

Central durable ownership for the normalized smart-home runtime, its automation
engine, and their shared `smart-home-runtime-store` envelope.

The controller restores both runtimes together and exposes their existing
`Arc<Mutex<_>>` adapter handles. Its transaction API serializes callers,
mutates cloned candidates, saves runtime state, automation definitions,
automation idempotency state, and audit history as one durable revision, then
publishes both candidates. Callback, encoding, storage, and compare-and-swap
failures leave the shared state and controller revision unchanged.

Callers that prepare work against a previously observed durable snapshot can
use `transaction_at_revision()`. The controller checks that revision under the
same locks used for commit and rejects stale work before invoking its mutation
callback. This lets recoverable workflows such as pairing preserve their
compare-and-swap boundary while sharing the central runtime owner.

HTTP adapters that already hold the shared mutexes can use
`runtime_persistence_adapter()` and `automation_persistence_adapter()` without
recursively locking the supplied runtime values. Native workers can use
`transaction()`, `evaluate_automations()`, or `tick()`.

```bash
bash BUILD
```
