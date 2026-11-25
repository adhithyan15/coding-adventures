## [0.01] - Security review of the fuzz engine (VER06q), round 1

### Fixed

- The honest `eval { local $SIG{ALRM} = sub { die }; alarm 2; ...; alarm 0 }` idiom was a false
  `alarm-tampered` (a timer at 0 with time to spare), so four such runs aborted a campaign. Only a
  replaced handler, or a run that overran its deadline while the timer was cancelled or re-armed, is
  tampering now. Documented: a target that uses `alarm` itself suspends the engine's wall-clock bound
  while its own alarm is active (the step budget and the post-run elapsed check still apply).
- The whole-second `CORE::alarm` fallback rounds the timer up but tamper detection used the fractional
  span, so every execution looked tampered on a platform without `Time::HiRes::alarm`. The comparisons
  now use the rounded span; `Executor::FORCE_INTEGER_ALARM` is the test seam, with tests.
- A re-armed timer could fire between leaving the eval and clearing the live flag and die outside an
  eval: the flag is now cleared inside a retried eval.

### Tests

- The counters-versus-budgets cross-check is now tested: `Handle::_test_clear_exhausted` (a test-only,
  documented-unsafe seam) clears a live handle's sticky marker after a breach, and the verdict must still
  be a timeout or an oom. Verified to fail when the `// $handle_over->($st)` part is removed.
- The leak-abort tests fail as clean assertions (not by dying with "No plan") when `MAX_LEAKED` is raised.
