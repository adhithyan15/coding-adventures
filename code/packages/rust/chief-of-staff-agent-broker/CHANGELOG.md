# Changelog

## Unreleased

### Changed

- **Windows key adoption** (D18S P2.8b-1). The broker no longer refuses to
  run on Windows (exit 69): it adopts exactly the handles named in
  `CHIEF_OF_STAFF_HANDLES`, one per slot in slot order, refusing (66) a
  count, repeat or value that is not exact before any key is read, then
  reads each through `read_owner_only_secret_from`. New native test
  (`tests/broker_windows.rs`, windows-2025): the real broker binary,
  confined by the applier in its own container, comes up from key handles,
  and refuses a missing handle (66) or a short key (67).
- `tests/launched.rs` launches a `BrokerProgram`, the launcher's new
  program type (D18S P2.7b).
- The process entry point moved from the binary into the library as
  `process::run_inherited` (D18S P2.6d-2c). Every binary that runs a broker,
  the production one and each test fixture, is now one call to it, instead
  of each carrying its own copy of the descriptor checks. The library is
  `deny(unsafe_code)`, with `process` its single exception, which a unit
  test enforces. The test also requires the `deny` to stay in `lib.rs`, and
  it recognises every way of lowering the lint, whatever the spacing:
  `allow`, `warn` or `expect`, alone or in a list. `run_inherited` documents that it may only be the whole
  of `main` in a process the broker launcher exec'd.

### Added

- `tests/launched.rs` (P2.6d-3):
  - the launched broker runs with `NoNewPrivs: 1` and `Seccomp: 2`;
  - a key directory, or another secret directory, open to group or others
    refuses the launch before anything is opened;
  - the scratch area is now owner-only, under Cargo's temporary directory.
- `serve_process`: the binary's whole serve loop, as a library function, so
  a test binary can run exactly what production runs (P2.6d-2b). `main.rs`
  keeps only the unsafe descriptor adoption.
- `tests/launched.rs` (Linux): the real binary, launched through
  `chief-of-staff-broker-launcher`, verified by digest and exec'd by
  descriptor, publishes end to end through the relay. Launch catches a key
  file whose public half is not the definition's, and refuses a key file
  open to others before spawning.
- The agent broker (D18S P2.6d-1):
  - `LoadedKeys`: slot-table validation against the binding, then key
    loading with the old authority's rules;
  - `ChannelBroker`: Receive, Publish and Acknowledge, served with the
    agent's keys through `ChannelCallbacks`;
  - `FramedCallbacks`: those callbacks over stdin and stdout.
- The `chief-of-staff-agent-broker` binary:
  - core dumps suppressed first;
  - keys read from inherited descriptors, with a tripwire on the
    descriptor count;
  - `Ready` reports public halves;
  - requests served one at a time;
  - it exits non-zero, silently, on anything an honest supervisor never
    sends.
- Before encrypting, the broker checks every reserved header, and refuses
  to encrypt twice under one sequence (nonce reuse) within its lifetime.
- Delivery receipts are per broker, capped at 4096.
- Security review round 1:
  - a refused commit is followed by `AbandonAppend`;
  - the descriptor tripwire scans up to the open-file limit, not just the
    next descriptor;
  - the README says plainly that the daemon, which supplies definitions,
    is not defended against.
