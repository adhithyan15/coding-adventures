# Changelog

## Unreleased

- Fix a start-up race in `test_runner_exits_after_termination_signal`. It
  sent SIGTERM a fixed 0.1s after spawning the runner, so on a loaded CI
  runner the signal could arrive before `add_signal_handler` had run, and the
  default action killed the process (`-15`). Both cases now handshake and wait
  for the reply first; the runner installs its handlers before it reads any
  input. Under saturated CPUs the old test failed 15/15 runs and the new one
  0/15.
- Target kernel API v2 exactly and reject legacy v1 stage metadata before the
  runner starts its protocol loop.
- Consume live host-mediated storage watches as bounded asynchronous streams,
  cancel abandoned iterators, and release every capability stream at run end.

## 0.1.0 — 2026-10-01

- Add the bounded Python FM02 plugin runner and idiomatic stage/context API.
- Pass the reusable cross-process runner corpus for lifecycle, wire values,
  mediated capabilities, stream shapes, cancellation, typed errors, malformed
  peers, identity mismatches, and resource limits.
- Bound ingress and work queues, preflight allocations, redact internal errors,
  enforce strict parameter shapes, wake streams on cancellation, and provide
  deadline-bounded signal shutdown with authored init/dispose hooks.
