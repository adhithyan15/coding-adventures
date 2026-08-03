# Changelog

## 0.2.0

- Add bounded HEOS change-event registration and collection over a dedicated
  production TCP connection.
- Normalize unsolicited player state, volume, mute, progress, repeat, shuffle,
  topology, queue, and playback-error frames into D23 events.
- Require D23 subscribe authorization before event transport I/O and prove the
  registration and event path with a real loopback connection.

## 0.1.0

- Add verified HEOS SSDP and manual host discovery.
- Add bounded TCP/JSON player inventory and state inspection.
- Normalize player identity, playback, volume, mute, and media state into D23.
- Add authorization-before-transport and real UDP/TCP loopback coverage.
