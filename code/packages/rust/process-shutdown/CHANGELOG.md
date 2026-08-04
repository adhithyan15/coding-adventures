# Changelog

## 0.1.0 — 2026-08-03

- Add an exclusive process-global cooperative shutdown listener.
- Map Unix SIGINT/SIGTERM and Windows console termination events to a portable
  two-variant event.
- Keep native callbacks signal-safe by dispatching user code on a named worker
  thread with bounded polling latency.
- Restore previous Unix handlers or remove the Windows callback on teardown.
- Reject competing listener installation and expose stable, payload-blind
  failure types.
