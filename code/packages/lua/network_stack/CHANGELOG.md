# Changelog — network_stack (Lua)

## 0.1.0 — 2026-03-31

### Added
- `EthernetFrame` — Layer 2: MAC header serialize/deserialize
- `ARPTable` — IP-to-MAC address resolution cache
- `IPv4Header` — Layer 3: IP header with ones'-complement checksum
- `RoutingTable` — longest-prefix-match routing
- `IPLayer` — IP packet creation and parsing
- `TCPSegment` — Layer 4: TCP header with flags, seq/ack numbers, serialize/deserialize
- `UDPDatagram` — Layer 4: UDP header serialize/deserialize
- `NetworkStack` — full stack: send_udp, send_tcp, receive (with full decapsulation)
- 95%+ test coverage via busted test suite
