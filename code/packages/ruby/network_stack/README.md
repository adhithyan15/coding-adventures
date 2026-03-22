# Network Stack

A full layered networking stack implementation in Ruby, from raw Ethernet frames (Layer 2) through HTTP (Layer 7). This package simulates a complete network protocol stack, allowing you to see actual packets flow between a client and server without needing real hardware.

## Where It Fits

```
Application (HTTP Client)
    |
    v
Socket API (Berkeley Sockets)
    |
    +-- HTTP (Layer 7)
    +-- DNS  (Layer 7)
    +-- TCP  (Layer 4)
    +-- UDP  (Layer 4)
    +-- IP   (Layer 3)
    +-- ARP  (Layer 2.5)
    +-- Ethernet (Layer 2)
    |
    v
NetworkWire (simulated physical medium)
```

## Layers Implemented

- **Ethernet** — Frame serialization/deserialization, ARP table for IP-to-MAC resolution
- **IPv4** — Header with checksum (ones' complement), routing table with longest-prefix match
- **TCP** — Full 11-state state machine, 3-way handshake, data transfer, connection teardown
- **UDP** — Connectionless datagram delivery with send/receive queues
- **Socket API** — Berkeley sockets interface (socket, bind, listen, accept, connect, send, recv, close)
- **DNS** — Static hostname-to-IP resolver
- **HTTP** — Request/response serialization for HTTP/1.1, URL parsing, client builder
- **NetworkWire** — Bidirectional in-memory channel simulating an Ethernet cable

## Usage

```ruby
require "coding_adventures_network_stack"

include CodingAdventures::NetworkStack

# Create a DNS resolver
resolver = DNSResolver.new
resolver.add_static("example.com", [93, 184, 216, 34])

# Build an HTTP request
client = HTTPClient.new(dns_resolver: resolver)
hostname, port, request = client.build_request("http://example.com/index.html")
puts request.serialize

# Simulate a network wire between two hosts
wire = NetworkWire.new
frame = EthernetFrame.new(
  dest_mac: [0xBB] * 6,
  src_mac: [0xAA] * 6,
  ether_type: ETHER_TYPE_IPV4,
  payload: [1, 2, 3]
)
wire.send_a(frame.serialize)
received = wire.receive_b
```

## Testing

```bash
bundle install
bundle exec rake test
```
