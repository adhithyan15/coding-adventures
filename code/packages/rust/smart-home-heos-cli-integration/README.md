# smart-home-heos-cli-integration

First-party HEOS CLI inspection, change-event, and media-control integration
for D23.

It discovers Denon and Marantz HEOS systems through their documented SSDP
search target or accepts a manual host. A bounded TCP client connects to port
1255, retrieves the player inventory, and reads playback, now-playing, volume,
and mute state for each player before installing normalized D23 devices and
entities. Authorization is checked before any socket I/O.

A separate bounded TCP connection can register for documented HEOS change
events and project unsolicited player state, volume, mute, progress, repeat,
shuffle, queue, topology, and playback-error frames into D23. Subscribe
authorization is checked before the event socket is opened.

D23 media commands route playback state, next/previous, volume, mute, grouping,
queue clearing, queue playback, removal, and reordering through the same bounded
TCP host. Every affected group member is authorized before network I/O, and the
host validates the command-correlated HEOS success response. Account-backed
source browsing and queue insertion remain outside this credential-free slice.

```bash
cargo run -p smart-home-heos-cli-integration -- discover
cargo run -p smart-home-heos-cli-integration -- inspect 192.0.2.30
```
