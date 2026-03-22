# Changelog

All notable changes to the network-stack package will be documented in this file.

## [0.1.0] - 2026-03-21

### Added

- **Ethernet layer** (`ethernet.py`): `EthernetFrame` with serialize/deserialize, `ARPTable` for IP-to-MAC mapping, EtherType constants for IPv4 and ARP.
- **IPv4 layer** (`ipv4.py`): `IPv4Header` with serialize/deserialize and Internet checksum, `RoutingTable` with longest-prefix matching, `IPLayer` for packet creation and parsing.
- **TCP layer** (`tcp.py`): Full `TCPConnection` state machine with all 11 states (CLOSED through TIME_WAIT), three-way handshake, data send/receive with sequence numbers, four-way connection teardown, `TCPHeader` serialize/deserialize.
- **UDP layer** (`udp.py`): `UDPHeader` serialize/deserialize, `UDPSocket` with send_to/receive_from/deliver for connectionless datagrams.
- **Socket API** (`socket_api.py`): `SocketManager` with BSD socket interface (socket/bind/listen/accept/connect/send/recv/sendto/recvfrom/close), `SocketType` enum (STREAM/DGRAM), `Socket` internal representation.
- **DNS resolver** (`dns.py`): `DNSResolver` with static hostname-to-IP mappings, pre-populated with localhost -> 127.0.0.1.
- **HTTP layer** (`http.py`): `HTTPRequest` and `HTTPResponse` serialization/deserialization, `HTTPClient` with URL parsing and request building.
- **Network wire** (`network_wire.py`): `NetworkWire` simulating a full-duplex Ethernet cable with bidirectional queues.
- Comprehensive test suite covering all layers.
- Knuth-style literate programming with detailed explanations throughout.
