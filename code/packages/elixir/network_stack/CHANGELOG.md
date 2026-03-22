# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-21

### Added
- **Layer 2 (Ethernet):** EthernetFrame with serialize/deserialize, ARPTable for IP-to-MAC mapping
- **Layer 3 (IP):** IPv4Header with ones' complement checksum, RoutingTable with longest prefix match, IPLayer for packet creation/parsing
- **Layer 4 (TCP):** Full TCPConnection state machine covering all 11 TCP states as atoms, three-way handshake, data transfer with sequence numbers, four-way connection teardown — all using immutable structs
- **Layer 4 (UDP):** UDPHeader serialize/deserialize, UDPSocket with send_to/receive_from/deliver
- **Socket API:** SocketManager implementing Berkeley sockets with immutable state
- **Layer 7 (DNS):** DNSResolver with static hostname-to-IP table
- **Layer 7 (HTTP):** HTTPRequest and HTTPResponse serialize/deserialize, HTTPClient with URL parsing
- **Physical:** NetworkWire using Agent for bidirectional simulated network cable
- Comprehensive ExUnit test suite covering all layers
- Knuth-style literate programming with extensive documentation
