# Changelog

All notable changes to the network-stack package (Go) will be documented in this file.

## [0.1.0] - 2026-03-21

### Added

- **Ethernet layer** (`ethernet.go`): `EthernetFrame` with Serialize/Deserialize, `ARPTable` for IP-to-MAC mapping, EtherType constants for IPv4 and ARP.
- **IPv4 layer** (`ipv4.go`): `IPv4Header` with Serialize/Deserialize and Internet checksum, `RoutingTable` with longest-prefix matching, `IPLayer` for packet creation and parsing.
- **TCP layer** (`tcp.go`): Full `TCPConnection` state machine with all 11 states, three-way handshake, data Send/Receive with sequence numbers, four-way connection teardown, `TCPHeader` Serialize/Deserialize.
- **UDP layer** (`udp.go`): `UDPHeader` Serialize/Deserialize, `UDPSocket` with SendTo/ReceiveFrom/Deliver for connectionless datagrams.
- **Socket API** (`socket.go`): `SocketManager` with BSD socket interface (CreateSocket/Bind/Listen/Accept/Connect/Send/Recv/SendTo/RecvFrom/Close), `SocketType` constants (SocketStream/SocketDgram).
- **DNS resolver** (`dns.go`): `DNSResolver` with static hostname-to-IP mappings, pre-populated with localhost -> 127.0.0.1.
- **HTTP layer** (`http.go`): `HTTPRequest` and `HTTPResponse` serialization/deserialization, `HTTPClient` with URL parsing and request building.
- **Network wire** (`wire.go`): `NetworkWire` simulating a full-duplex Ethernet cable with bidirectional queues.
- Comprehensive test suite covering all layers.
- Knuth-style literate programming with detailed explanations throughout.
