# smart-home-tasmota-local-integration

This package adds a native local HTTP path for Tasmota devices alongside the
existing MQTT delegation:

- filtered `_http._tcp.local` mDNS and manual discovery;
- optional WebUser/WebPassword authentication backed by a runtime `VaultRef`;
- bounded `Status 0` inspection of relay, light, and sensor state; and
- capability-caged `Power`, `Dimmer`, `HSBColor`, and `CT` commands followed by
  a fresh status read and confirmed D23 state installation.

Run `cargo run -p smart-home-tasmota-local-integration -- discover` to scan the
local network or `inspect <host> [port]` for a sanitized snapshot. Set
`TASMOTA_USERNAME`, `TASMOTA_PASSWORD`, and optionally
`TASMOTA_CREDENTIAL_REF` when the device web API requires authentication.

The existing MQTT runtime remains the preferred push path. This package covers
direct local polling and command fallback without claiming event push.
