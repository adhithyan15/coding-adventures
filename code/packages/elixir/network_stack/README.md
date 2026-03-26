# CodingAdventures.NetworkStack

A complete network stack implementation in Elixir, covering every layer of the TCP/IP model from raw Ethernet frames to HTTP requests.

## Where It Fits

```
Layer 7: HTTP, DNS     — Application protocols
Layer 4: TCP, UDP      — Transport protocols
Layer 3: IP            — Network routing
Layer 2: Ethernet, ARP — Local delivery
Layer 1: NetworkWire   — Simulated physical medium
```

## Components

- **EthernetFrame** — Frame serialization/deserialization with MAC addressing
- **ARPTable** — IP-to-MAC address resolution cache (immutable map)
- **IPv4Header** — IP packet headers with ones' complement checksum
- **RoutingTable** — Longest prefix match routing
- **IPLayer** — Packet creation and parsing
- **TCPConnection** — Full TCP state machine (11 states) using immutable structs
- **UDPSocket** — Connectionless datagram communication
- **SocketManager** — Berkeley sockets API (immutable, returns updated state)
- **DNSResolver** — Static hostname-to-IP resolution
- **HTTPRequest/HTTPResponse** — HTTP/1.1 message serialization
- **HTTPClient** — URL parsing and request building
- **NetworkWire** — Bidirectional simulated network cable (Agent-based)

## Elixir Idioms

This implementation uses idiomatic Elixir patterns:
- All data structures are immutable structs
- State changes return new structs (no mutation)
- NetworkWire uses an Agent for shared mutable state
- TCP states are atoms (`:closed`, `:established`, etc.)
- Pattern matching drives the TCP state machine

## Usage

```elixir
alias CodingAdventures.NetworkStack.{TCPConnection, HTTPClient, NetworkWire}

# Build an HTTP request
{request, host, port} = HTTPClient.build_request("http://example.com/")
IO.puts(HTTPRequest.serialize(request))

# Simulate a network cable
wire = NetworkWire.new()
NetworkWire.send_a(wire, [1, 2, 3])
data = NetworkWire.receive_b(wire)  # [1, 2, 3]
NetworkWire.stop(wire)
```

## Testing

```bash
mix deps.get
mix test
```

## License

MIT
