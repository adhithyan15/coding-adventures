# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-19

### Added

- Added a transport-agnostic UDP client around `std::net::UdpSocket`.
- Added bind, connect, explicit send, connected send, receive, local address,
  and one-shot send/receive APIs.
- Added configurable read/write timeouts and deterministic datagram-size guards.
- Added loopback-only tests for unconnected UDP, connected UDP, echo round
  trips, empty datagrams, receive timeouts, truncation detection, IPv6 when
  available, and parallel ephemeral sockets.
