# smart-home-shelly-integration

This package connects Shelly Gen2 and Gen3 devices directly to D23 over the
local network:

- `_shelly._tcp.local` mDNS discovery through `smart-home-discovery`;
- bounded HTTP/1.1 reads of `/shelly` and `Shelly.GetStatus`;
- normalized switch, light, input, temperature, humidity, and energy entities;
- capability-caged `Switch.Set` and `Light.Set` command execution; and
- explicit rejection of authentication-enabled devices until a credential host
  is configured, so unauthenticated operation is never mistaken for pairing.

Run `cargo run -p smart-home-shelly-integration -- discover` to scan the local
network, or `cargo run -p smart-home-shelly-integration -- inspect <host> [port]`
to emit a sanitized device and component summary.
