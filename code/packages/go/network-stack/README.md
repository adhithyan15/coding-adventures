# Network Stack (Go)

A complete layered networking stack implementation in Go — from raw Ethernet frames to HTTP requests — built as a teaching tool using Knuth-style literate programming.

## Where It Fits

```
User Program (HTTP client, web server, etc.)
    |
Socket API          <-- application interface
    |
+-- HTTP Client     <-- Layer 7 (Application)
+-- DNS Resolver    <-- Layer 7 (Application)
+-- TCP             <-- Layer 4 (Transport, reliable)
+-- UDP             <-- Layer 4 (Transport, fast)
+-- IP              <-- Layer 3 (Network, routing)
+-- ARP             <-- Layer 2.5 (IP-to-MAC resolution)
+-- Ethernet        <-- Layer 2 (Data Link, local delivery)
    |
NetworkWire         <-- Simulated physical medium
```

## Files

| File | Layer | Description |
|------|-------|-------------|
| `ethernet.go` | 2 | EthernetFrame, ARPTable |
| `ipv4.go` | 3 | IPv4Header, RoutingTable, IPLayer |
| `tcp.go` | 4 | TCPConnection state machine, TCPHeader |
| `udp.go` | 4 | UDPHeader, UDPSocket |
| `socket.go` | - | SocketManager (BSD socket interface) |
| `dns.go` | 7 | DNSResolver (static lookups) |
| `http.go` | 7 | HTTPRequest, HTTPResponse, HTTPClient |
| `wire.go` | 1 | NetworkWire (simulated cable) |

## Usage

```go
import ns "github.com/adhithyan15/coding-adventures/code/packages/go/network-stack"

// DNS resolution
resolver := ns.NewDNSResolver()
resolver.AddStatic("myserver.local", 0x0A000001)
ip, ok := resolver.Resolve("myserver.local")

// TCP three-way handshake
client := ns.NewTCPConnection(49152, 0x0A000002, 80)
syn := client.InitiateConnect()

server := ns.NewTCPConnection(80, 0, 0)
server.InitiateListen()
synack := server.HandleSegment(syn, nil)
ack := client.HandleSegment(synack, nil)
server.HandleSegment(ack, nil)
// Both are now ESTABLISHED

// Socket API
mgr := ns.NewSocketManager()
fd := mgr.CreateSocket(ns.SocketDgram)
mgr.Bind(fd, 0, 12345)
mgr.SendTo(fd, []byte("hello"), 0x08080808, 53)
```

## Testing

```bash
go test ./... -v -cover
```

## Specification

See [D17-network-stack.md](../../../specs/D17-network-stack.md) for the full specification.
