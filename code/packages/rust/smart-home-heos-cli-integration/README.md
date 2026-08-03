# smart-home-heos-cli-integration

First-party, read-only HEOS CLI integration for D23.

It discovers Denon and Marantz HEOS systems through their documented SSDP
search target or accepts a manual host. A bounded TCP client connects to port
1255, retrieves the player inventory, and reads playback, now-playing, volume,
and mute state for each player before installing normalized D23 devices and
entities. Authorization is checked before any socket I/O.

The slice intentionally excludes HEOS account access, playback or volume
commands, grouping, queue mutation, and change-event subscriptions.

```bash
cargo run -p smart-home-heos-cli-integration -- discover
cargo run -p smart-home-heos-cli-integration -- inspect 192.0.2.30
```
