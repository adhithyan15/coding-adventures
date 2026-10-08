# Changelog

## Unreleased

- `vault put` writes the destinations from `--destination` into the sealed
  record. The end-to-end test now redeems through `consume_for` and shows an
  unprovisioned destination being refused.
- Add `vault put`, `vault delete` and `vault list` (D18U, P1.4b on #13980).
  These are local commands that open the Chief vault through the daemon
  crate's `open_chief_vault`, and never contact the daemon:
  - `put` reads the secret from stdin through a bounded, single-allocation
    zeroizing read, and refuses an interactive terminal before reading
    anything.
  - The vault is opened before stdin is read, so a configuration or KEK
    problem means the secret is never read into memory.
  - Output never contains a value.
- Add `run_with_input` and the `SecretInput` trait, so the stdin source can be
  injected and tested. `run` uses real stdin.
- The `chief-of-staff` binary now prints an error's whole `source()` chain
  through `describe_error`. Before, an operator saw only a generic
  "command failed" and not which flag or record was at fault. A cause the
  parent message already includes is not printed twice.

## 0.1.0 - 2026-08-03

- Add the concrete `chief-of-staff` executable over the declarative CLI core.
- Compose strict local configuration, owner-only credential loading,
  authenticated loopback WebSocket dispatch, and deterministic output.
- Add `install-daemon` composition for launchd, systemd user services, and
  Windows Task Scheduler.
- Expose the CLI core's typed authenticated pipeline `wire` and `unwire`
  commands without adding credentials or endpoints to argv.
