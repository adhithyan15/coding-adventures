# @coding-adventures/network-stack

A complete network stack implementation in TypeScript, covering every layer of the TCP/IP model from raw Ethernet frames to HTTP requests.

## Where It Fits

This package implements Layers 2 through 7 of the computing stack:

```
Layer 7: HTTP, DNS     — Application protocols
Layer 4: TCP, UDP      — Transport protocols
Layer 3: IP            — Network routing
Layer 2: Ethernet, ARP — Local delivery
Layer 1: NetworkWire   — Simulated physical medium
```

## Components

- **EthernetFrame** — Frame serialization/deserialization with MAC addressing
- **ARPTable** — IP-to-MAC address resolution cache
- **IPv4Header** — IP packet headers with ones' complement checksum
- **RoutingTable** — Longest prefix match routing
- **IPLayer** — Packet creation and parsing
- **TCPConnection** — Full TCP state machine (11 states), three-way handshake, data transfer, four-way close
- **UDPSocket** — Connectionless datagram communication
- **SocketManager** — Berkeley sockets API (socket, bind, listen, accept, connect, send, recv, close)
- **DNSResolver** — Static hostname-to-IP resolution
- **HTTPRequest/HTTPResponse** — HTTP/1.1 message serialization
- **HTTPClient** — URL parsing and request building
- **NetworkWire** — Bidirectional simulated network cable

## Usage

```typescript
import {
  SocketManager, SocketType, TCPConnection,
  HTTPClient, DNSResolver, NetworkWire
} from "@coding-adventures/network-stack";

// Create a socket
const mgr = new SocketManager();
const fd = mgr.socket(SocketType.STREAM);
mgr.bind(fd, "10.0.0.1", 49152);

// Build an HTTP request
const client = new HTTPClient();
const { request, host, port } = client.build_request("http://example.com/");
console.log(request.serialize());

// Simulate a network cable
const wire = new NetworkWire();
wire.send_a([1, 2, 3]);
const data = wire.receive_b(); // [1, 2, 3]
```

## Testing

```bash
npm install
npx vitest run --coverage
```

## License

MIT
