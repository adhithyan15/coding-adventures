## 0.3.2

- Security re-review of the fuzz engine. A fault in an `async void` target that lands on a pool thread (even
  while the target blocks) is now the crash `async-void-exception`: the context records any posted callback
  that throws, from any thread, while the run is live. A post alone is no longer read as a fault, so an
  `async Task` target that yields and completes is a pass. A bounded grace (10 ms, only when work is pending
  or in flight) precedes `async-void-pending`.
- `Handle.Tick`/`Reserve` run wholly under the handle's lock (no retired-check/update TOCTOU), and the executor
  reads the exhaustion state after `Retire()`.
- Docs: late callbacks, target-global state, `ExecutionContext` flow and the serial epoch increment.
- Version 0.3.2.
