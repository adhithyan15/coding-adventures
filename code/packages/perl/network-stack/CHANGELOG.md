# Changelog — network-stack (Perl)

## 0.01 — 2026-03-31

### Added
- `EthernetFrame` — Layer 2 frame with serialize/deserialize (14-byte header)
- `ARPTable` — IP-string to MAC-array resolution cache
- `IPv4Header` — 20-byte header with ones'-complement checksum compute/verify
- `RoutingTable` — Longest-prefix-match routing with multi-route support
- `IPLayer` — Creates and parses full IP packets with checksum validation
- `TCPSegment` — 20-byte TCP header with per-flag accessors and round-trip serialisation
- `UDPDatagram` — 8-byte UDP header with round-trip serialisation
- `TCPConnection` — Full 11-state TCP state machine including three-way handshake,
  data transfer, and graceful teardown (active and passive close)
- `NetworkStack` — Top-level facade for send_udp, send_tcp, and receive
- 95%+ test coverage via Test2::V0
