# Changelog

## Unreleased

### Fixed

- `the_deadline_does_not_charge_the_broker_for_the_daemon_s_time` lets
  its slow broker answer 1.5 s late against the 300 ms deadline, where it
  used to be 400 ms. The 100 ms margin lost the race on macOS CI, where the
  relay's timed wait can wake late.

### Added

- **The broker on macOS** (D18S P2.7b).
  - `BrokerProgram` is what `launch` runs. On Linux it holds the verified
    broker. On macOS it holds the verified broker and the verified Seatbelt
    shim, and `new` refuses a shim that is the broker. Elsewhere no value
    of it can exist.
  - On macOS, `launch` confines the broker through
    `MacosConfinement::apply_inheriting`, under `broker_plan()` for
    `OsFamily::Macos`. It takes the same steps, in the same order, as on
    Linux.
  - **Breaking:** `launch` takes `&BrokerProgram` instead of
    `&VerifiedExecutable`.
- The broker runs confined (D18S P2.6d-3): `launch` starts it through
  `chief-of-staff-linux-sandbox` under `broker_plan()`, a plan with no
  capabilities, instead of spawn-isolation's `isolate_and_exec`.
- Before every launch, each key file's directory, and each directory given
  with `BrokerKeyFiles::with_secret_directories`, must be owner-only.
  `BrokerKeyFiles::secret_directories` lists them.
- `LaunchError::SecretDirectory` and `LaunchError::Confinement`.
- Review round 1: `BrokerKeyFiles::check_secret_directories`, which skips
  a directory that does not exist yet; the daemon calls it at startup too.
- `BrokerRelay::stop` takes `&mut self` and waits, bounded, for the relay
  thread. If the thread is still running it keeps the handle, so the
  supervisor can ask again (P2.6d-2b).
- `LaunchedBroker::discard`: kill and reap a broker that will not be used.
- `start_relay` kills and reaps the broker if its relay thread cannot start,
  rather than leaving it running unrelayed.
- The broker launcher (D18S P2.6d-2a):
  - `BrokerKeyFiles` maps each binding to its key slots;
  - `abandon_pending_on_write_channels`;
  - `launch`: verified exec with the keys on 3..3+n, then Bootstrap;
  - `check_ready`: the public keys checked against the definitions;
  - `start_relay` and `start_relay_over`: one relay thread per broker,
    serving `CallbackServer`, with a deadline that excludes callback time;
  - `PinnedBindingResolver`, `ResponseSink`, `RelayEnd`.
- Off Linux, `VerifiedExecutable` is an empty type, so no broker can be
  launched.
- The relay queues writes to a writer thread rather than writing itself,
  so a broker that stops reading is caught by the deadline. It reports its
  end before any slow cleanup.
