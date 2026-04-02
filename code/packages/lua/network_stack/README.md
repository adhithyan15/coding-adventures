# network_stack (Lua)

Layered network protocol stack for the coding-adventures simulated computer.

## What It Does

Implements the TCP/IP model end-to-end:

| Layer | Component     | Responsibility                    |
|-------|---------------|-----------------------------------|
| 4     | TCPSegment    | Reliable ordered byte stream      |
| 4     | UDPDatagram   | Fast unreliable datagram          |
| 3     | IPv4Header    | Routing, TTL, checksum            |
| 3     | IPLayer       | Packet construction and parsing   |
| 3     | RoutingTable  | Longest-prefix-match routing      |
| 2     | EthernetFrame | MAC addresses, local delivery     |
| 2     | ARPTable      | IP-to-MAC address cache           |

## Usage

```lua
local NS = require("coding_adventures.network_stack")

local sender   = NS.NetworkStack.new({ 192, 168, 1, 10 }, { 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0x01 })
local receiver = NS.NetworkStack.new({ 192, 168, 1, 20 }, { 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0x02 })

-- Send UDP
local wire = sender:send_udp(
  { 192, 168, 1, 20 },               -- dst_ip
  { 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0x02 },  -- dst_mac
  5000, 9000,                         -- src_port, dst_port
  { 72, 101, 108, 108, 111 }          -- "Hello"
)

-- Receive
local status, proto, src_ip, src_port, dst_port, payload = receiver:receive(wire)
-- status = "ok", proto = "udp", payload = { 72, 101, 108, 108, 111 }
```

## Packet Encapsulation

```
Application:  [data]
UDP:          [8-byte UDP header] + [data]
IP:           [20-byte IP header] + [UDP segment]
Ethernet:     [14-byte Eth header] + [IP packet]
Wire:         raw bytes
```
