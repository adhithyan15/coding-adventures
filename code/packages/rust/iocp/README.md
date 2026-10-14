# iocp (Rust)

Thin Rust wrapper over Windows I/O Completion Ports.

## What is this?

This crate exposes the completion-port primitives needed for TCP-first Windows
backends: port creation, handle association, completion waiting, and posted
user-space wakeups.

## Development

```bash
cargo test -p iocp
```


## Bounded spacing and transport reconciliation

Failed completions preserve their nonnull OVERLAPPED identity before timeout classification. None alone requests INFINITE; finite waits saturate to DWORD_MAX-1. Windows stop latency remains dependent on the configured finite poll timeout.

Validation is source-attributed: Windows applicable packages and standalone
checks are recorded separately from Unix-only/hosted and whole-PR gates.
Final-source Miri is **DEFERRED/PENDING**, never passed.
