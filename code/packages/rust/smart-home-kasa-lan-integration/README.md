# smart-home-kasa-lan-integration

This package connects credential-free TP-Link Kasa devices to D23 through the
legacy XOR-obfuscated JSON LAN protocol on UDP port `9999`:

- bounded broadcast `get_sysinfo` discovery;
- direct device inspection with source-address and response validation;
- normalized plug, switch, and bulb state and capabilities; and
- D23-authorized power, brightness, RGB, and color-temperature commands with a
  fresh state read that verifies each mutation.

This package does not accept TP-Link cloud credentials and does not claim the
newer authenticated KLAP/Tapo protocol. Devices that do not answer the legacy
local protocol fail closed.

Run `cargo run -p smart-home-kasa-lan-integration -- discover` to scan the LAN,
or `cargo run -p smart-home-kasa-lan-integration -- inspect <host> [port]` to
emit sanitized current state.
