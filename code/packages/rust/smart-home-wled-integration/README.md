# smart-home-wled-integration

This package connects WLED devices directly to D23 over the local network:

- `_wled._tcp.local` mDNS discovery through `smart-home-discovery`;
- bounded HTTP/1.1 inspection of the WLED `/json/si` state and information endpoint;
- normalized master and segment light entities with capability-bit-aware RGB and CCT support; and
- capability-caged power, brightness, RGB, and color-temperature mutations through `/json/state`.

The first host slice polls the documented JSON API. It does not claim WebSocket
push support. Run `cargo run -p smart-home-wled-integration -- discover` to scan
the local network, or `cargo run -p smart-home-wled-integration -- inspect
<host> [port]` to emit a sanitized device and segment summary.
