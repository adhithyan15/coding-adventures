# Network Stack

A full layered networking stack implementation in Rust, from raw Ethernet frames (Layer 2) through HTTP (Layer 7). This crate simulates a complete network protocol stack, allowing you to see actual packets flow between a client and server without needing real hardware.

## Layers Implemented

- **Ethernet** (`ethernet.rs`) — Frame serialization/deserialization, ARP table for IP-to-MAC resolution
- **IPv4** (`ipv4.rs`) — Header with ones' complement checksum, routing table with longest-prefix match
- **TCP** (`tcp.rs`) — Full 11-state state machine, 3-way handshake, data transfer, connection teardown
- **UDP** (`udp.rs`) — Connectionless datagram delivery with send/receive queues
- **Socket API** (`socket_api.rs`) — Berkeley sockets interface (socket, bind, listen, accept, connect, send, recv, close)
- **DNS** (`dns.rs`) — Static hostname-to-IP resolver
- **HTTP** (`http.rs`) — Request/response serialization for HTTP/1.1, URL parsing, client builder
- **NetworkWire** (`network_wire.rs`) — Bidirectional in-memory channel simulating an Ethernet cable

## Usage

```rust
use network_stack::ethernet::{EthernetFrame, ETHER_TYPE_IPV4};
use network_stack::dns::DnsResolver;
use network_stack::http::HttpClient;
use network_stack::network_wire::NetworkWire;

// Create a DNS resolver
let mut resolver = DnsResolver::new();
resolver.add_static("example.com", [93, 184, 216, 34]);

// Build an HTTP request
let client = HttpClient::with_resolver(resolver);
let (hostname, port, request) = client.build_request(
    "http://example.com/index.html", "GET", "", None
).unwrap();

// Simulate a network wire
let mut wire = NetworkWire::new();
let frame = EthernetFrame::new([0xBB; 6], [0xAA; 6], ETHER_TYPE_IPV4, vec![1, 2, 3]);
wire.send_a(frame.serialize());
let received = wire.receive_b();
```

## Testing

```bash
cargo test -p network-stack
```
