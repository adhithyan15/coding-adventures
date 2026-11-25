## 0.3.1

- Security review of the fuzz engine. MEDIUM: a `Handle` reused across executions could be touched by an
  `async void` continuation or a spawned task during the next run. Now a fresh handle per execution, retired
  in a `finally` (`Edge` ignored, `Tick`/`Reserve` throw a retired sentinel); a counting
  `SynchronizationContext` makes a still-suspended `async void` target the crash `async-void-pending` (and a
  faulted one `async-void-exception`, never terminating the host).
- Budget sentinels carry (campaign, epoch); one rethrown in a later run is `stale-budget-sentinel`.
- `FuzzOptions.ToString` omits the seed; `CrashRecord.Frames`, `FuzzReport.Crashes` and `Outcome.Frames` are
  `ReadOnlyCollection`s. Docs: stack capture is O(depth) per crash and per minimisation candidate.
- Version 0.3.1.
