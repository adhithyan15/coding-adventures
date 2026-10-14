# Changelog

## Unreleased

- Fix a race in `test_runner_exits_after_termination_signal`. It sent
  SIGTERM after a fixed 0.1 s, about as long as the interpreter takes to
  start. On a loaded CI runner the signal arrived before the runner's
  handler was installed and killed it with -15 (X-7). The test now sends a
  handshake and waits for the reply, which proves the handler is in place,
  before it signals.
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
