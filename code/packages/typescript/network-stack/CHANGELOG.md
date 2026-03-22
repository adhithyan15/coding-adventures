# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-21

### Added
- **Layer 2 (Ethernet):** EthernetFrame with serialize/deserialize, ARPTable for IP-to-MAC mapping
- **Layer 3 (IP):** IPv4Header with ones' complement checksum, RoutingTable with longest prefix match, IPLayer for packet creation/parsing
- **Layer 4 (TCP):** Full TCPConnection state machine covering all 11 TCP states, three-way handshake, data transfer with sequence numbers, four-way connection teardown
- **Layer 4 (UDP):** UDPHeader serialize/deserialize, UDPSocket with send_to/receive_from/deliver
- **Socket API:** Socket and SocketManager implementing Berkeley sockets (socket, bind, listen, accept, connect, send, recv, sendto, recvfrom, close)
- **Layer 7 (DNS):** DNSResolver with static hostname-to-IP table (localhost pre-configured)
- **Layer 7 (HTTP):** HTTPRequest and HTTPResponse serialize/deserialize, HTTPClient with URL parsing
- **Physical:** NetworkWire bidirectional simulated network cable
- Comprehensive test suite covering all layers with 85%+ coverage target
- Knuth-style literate programming with extensive inline documentation
