# smart-home-heos-cli-integration

First-party HEOS CLI inspection and change-event integration for D23.

It discovers Denon and Marantz HEOS systems through their documented SSDP
search target or accepts a manual host. A bounded TCP client connects to port
1255, retrieves the player inventory, and reads playback, now-playing, volume,
and mute state for each player before installing normalized D23 devices and
entities. Authorization is checked before any socket I/O.

A separate bounded TCP connection can register for documented HEOS change
events and project unsolicited player state, volume, mute, progress, repeat,
shuffle, queue, topology, and playback-error frames into D23. Subscribe
authorization is checked before the event socket is opened.

The slice intentionally excludes HEOS account access and playback, volume,
grouping, or queue mutation commands until D23 has explicit media command
types and capability mappings.

```bash
cargo run -p smart-home-heos-cli-integration -- discover
cargo run -p smart-home-heos-cli-integration -- inspect 192.0.2.30
```
