# Changelog

## Unreleased

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
