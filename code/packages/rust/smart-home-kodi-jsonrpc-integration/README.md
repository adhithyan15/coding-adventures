# smart-home-kodi-jsonrpc-integration

This package connects one explicitly configured local Kodi HTTP JSON-RPC
endpoint to D23. The bounded runtime surface:

- reads Kodi application identity, version, volume, mute state, and active
  player telemetry;
- normalizes one Kodi host into a D23 bridge, device, and media entity;
- authorizes D23 reads before TCP I/O; and
- routes only play, pause, stop, volume, and mute commands through D23 command
  authorization before the fixed native JSON-RPC method allowlist.

Run `cargo run -p smart-home-kodi-jsonrpc-integration -- inspect <ip:port>` for
a one-shot authorized inspection of a private, link-local, or loopback endpoint.

The integration accepts no credentials and exposes no WebSocket subscription,
library browse, item metadata or path, playback URL, queue mutation, arbitrary
JSON-RPC method, add-on execution, input action, power action, public endpoint,
or long-lived connection. Credentialed Kodi deployments require a separate
Vault-leased authentication and supervised session owner.
