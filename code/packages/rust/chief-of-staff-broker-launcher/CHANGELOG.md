# Changelog

## Unreleased

### Added

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
