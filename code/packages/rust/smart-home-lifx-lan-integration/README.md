# smart-home-lifx-lan-integration

This package connects LIFX lights directly to D23 with the documented binary
LAN protocol:

- UDP broadcast `GetService` discovery on port `56700`;
- direct `GetColor` inspection with source, sequence, target, and packet-size validation;
- normalized light state and capabilities; and
- capability-caged `SetLightPower` and `SetColor` commands followed by a fresh
  `GetColor` query that verifies each mutation.

No LIFX cloud account or credential is accepted by this package.

Protocol source: [LIFX LAN protocol](https://lan.developer.lifx.com/docs/introduction).

Run `cargo run -p smart-home-lifx-lan-integration -- discover` to scan the
local network, or `cargo run -p smart-home-lifx-lan-integration -- inspect
<host> <serial> [port]` to emit sanitized current state.
