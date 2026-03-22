# Network Stack

A complete layered networking stack implementation — from raw Ethernet frames to HTTP requests — built as a teaching tool using Knuth-style literate programming.

## Where It Fits

This package implements the TCP/IP networking stack that sits between user applications and the simulated hardware:

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

## Modules

| Module | Layer | Description |
|--------|-------|-------------|
| `ethernet` | 2 | Frame serialization, ARP table |
| `ipv4` | 3 | IP headers, checksums, routing table |
| `tcp` | 4 | Full TCP state machine, three-way handshake |
| `udp` | 4 | Connectionless datagrams |
| `socket_api` | - | BSD socket interface (STREAM/DGRAM) |
| `dns` | 7 | Static hostname-to-IP resolution |
| `http` | 7 | HTTP request/response serialization |
| `network_wire` | 1 | Simulated Ethernet cable |

## Usage

```python
from network_stack import (
    DNSResolver, HTTPClient, HTTPRequest, HTTPResponse,
    TCPConnection, TCPState, SocketManager, SocketType,
    EthernetFrame, IPv4Header, NetworkWire,
)

# DNS resolution
resolver = DNSResolver()
resolver.add_static("myserver.local", 0x0A000001)
ip = resolver.resolve("myserver.local")  # 0x0A000001

# HTTP request building
client = HTTPClient(resolver)
host, port, req = client.build_request("GET", "http://myserver.local/page")
raw_request = req.serialize()

# TCP three-way handshake
client_conn = TCPConnection(local_port=49152, remote_port=80)
syn = client_conn.initiate_connect()  # SYN

server_conn = TCPConnection(local_port=80)
server_conn.initiate_listen()
synack = server_conn.handle_segment(syn)  # SYN+ACK
ack = client_conn.handle_segment(synack)   # ACK -> ESTABLISHED

# Socket API
mgr = SocketManager()
fd = mgr.socket(SocketType.DGRAM)
mgr.bind(fd, 0, 12345)
mgr.sendto(fd, b"hello", 0x08080808, 53)
```

## Development

```bash
uv venv
uv pip install -e ".[dev]"
python -m pytest tests/ -v
```

## Specification

See [D17-network-stack.md](../../../specs/D17-network-stack.md) for the full specification.
