# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-03-21

### Added

- **Ethernet layer** — `EthernetFrame` with serialize/deserialize, `ArpTable` for IP-to-MAC caching
- **IPv4 layer** — `Ipv4Header` with ones' complement checksum, `RoutingTable` with longest-prefix match, `IpLayer` for packet creation and parsing
- **TCP layer** — Full 11-state `TcpConnection` state machine, `TcpHeader` serialization, 3-way handshake, data transfer, 4-way connection teardown
- **UDP layer** — `UdpHeader` serialization, `UdpSocket` with connectionless send/receive queues
- **Socket API** — `SocketManager` implementing Berkeley sockets (socket, bind, listen, accept, connect, send, recv, close)
- **DNS resolver** — Static hostname-to-IP lookup table with `localhost` default
- **HTTP layer** — `HttpRequest` and `HttpResponse` serialization/deserialization, `HttpClient` with URL parsing
- **NetworkWire** — Bidirectional in-memory channel simulating a physical Ethernet cable
