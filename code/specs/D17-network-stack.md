# D17 — Network Stack

## Overview

Networking is the art of getting bytes from one computer to another. This sounds
simple, but the reality is deeply layered: electrical signals on a wire must be
framed into packets, packets must be routed across networks, data must arrive
reliably and in order, and applications must have a clean API to use it all
without caring about the details.

This package implements a full networking stack — from raw Ethernet frames to
HTTP requests — in roughly 1,000 lines per language. We simulate the physical
network with an in-memory "wire" connecting two network interface cards, so
you can see actual packets flow between a client and server without needing
real hardware.

**Analogy:** Sending data over a network is like sending a letter through the
postal system:
- **Ethernet** (Layer 2) is the local mail carrier — delivers between
  neighboring houses on the same street.
- **IP** (Layer 3) is the postal routing system — figures out which city and
  which post office the letter should go to.
- **TCP** (Layer 4) is registered mail with tracking — guarantees delivery,
  correct order, and acknowledgment.
- **UDP** (Layer 4) is a postcard — fast, no guarantee it arrives, no
  tracking.
- **HTTP** (Layer 7) is the letter itself — "Dear Server, please send me
  the homepage."

## Where It Fits

```
User Program
│   let sock = socket(STREAM)
│   connect(sock, "192.168.1.100", 80)
│   send(sock, "GET / HTTP/1.1\r\n...")
│   let response = recv(sock)
▼
OS Kernel — Syscall Dispatcher
│   sys_socket(198)    → SocketManager
│   sys_connect(203)   → SocketManager → TCP → IP → Ethernet
│   sys_sendto(206)    → SocketManager → TCP → IP → Ethernet
│   sys_recvfrom(207)  → SocketManager ← TCP ← IP ← Ethernet
▼
Socket API ← YOU ARE HERE (application interface)
│
├── HTTP Client         — Layer 7 (Application)
│   └── builds/parses HTTP requests and responses
│
├── DNS Resolver        — Layer 7 (Application)
│   └── hostname → IP address lookup
│
├── TCP                 — Layer 4 (Transport)
│   └── reliable, ordered, connection-oriented byte stream
│
├── UDP                 — Layer 4 (Transport)
│   └── unreliable, unordered, connectionless datagrams
│
├── IP                  — Layer 3 (Network)
│   └── routing, addressing, packet forwarding
│
├── ARP                 — Layer 2.5
│   └── IP address → MAC address resolution
│
├── Ethernet            — Layer 2 (Data Link)
│   └── framing, MAC addressing, local delivery
│
▼
NetworkWire             — Simulated physical medium
│   └── connects two SimulatedNIC devices
▼
Device Driver Framework (D14)
│   └── NetworkDevice trait
▼
Hardware (simulated NIC)
```

**Depends on:** Device Driver Framework (D14, `NetworkDevice` trait), File System
(D15, sockets are file descriptors)

**Used by:** User programs (via socket syscalls), future packages (web server,
email client, etc.)

## Key Concepts

### The Layered Model

Why is networking layered? Because each layer solves one problem and hides it
from the layers above. The HTTP layer does not need to know whether the bytes
travel over WiFi, Ethernet, or carrier pigeon — it just sends a request and
gets a response.

```
The TCP/IP Model (what the Internet actually uses)
══════════════════════════════════════════════════

  ┌─────────────────────────────────────────────────────────────┐
  │ Layer 7: Application                                        │
  │   HTTP, DNS, FTP, SMTP, SSH...                              │
  │   "What are we saying?"                                     │
  ├─────────────────────────────────────────────────────────────┤
  │ Layer 4: Transport                                          │
  │   TCP (reliable) or UDP (fast)                              │
  │   "How do we ensure delivery?"                              │
  ├─────────────────────────────────────────────────────────────┤
  │ Layer 3: Network                                            │
  │   IP (Internet Protocol)                                    │
  │   "How do we route across networks?"                        │
  ├─────────────────────────────────────────────────────────────┤
  │ Layer 2: Data Link                                          │
  │   Ethernet, WiFi                                            │
  │   "How do we talk to the next hop?"                         │
  ├─────────────────────────────────────────────────────────────┤
  │ Layer 1: Physical                                           │
  │   Electrical signals, fiber optics, radio waves             │
  │   "How do we transmit bits?" (simulated by NetworkWire)     │
  └─────────────────────────────────────────────────────────────┘
```

### Packet Encapsulation

As data moves down the stack, each layer wraps the data from the layer above
in its own header. This is **encapsulation**:

```
Application data: "Hello, World!"

Layer 7 (HTTP):
┌──────────────────────────────────────────────────────────────┐
│ GET / HTTP/1.1\r\nHost: example.com\r\n\r\nHello, World!    │
└──────────────────────────────────────────────────────────────┘

Layer 4 (TCP) wraps HTTP in a TCP segment:
┌────────────┬─────────────────────────────────────────────────┐
│ TCP Header │ GET / HTTP/1.1\r\nHost: example.com\r\n\r\n... │
│ src: 49152 │                                                 │
│ dst: 80    │                                                 │
│ seq: 1000  │                                                 │
│ ack: 0     │                                                 │
│ flags: PSH │                                                 │
└────────────┴─────────────────────────────────────────────────┘

Layer 3 (IP) wraps TCP segment in an IP packet:
┌────────────┬────────────┬────────────────────────────────────┐
│ IP Header  │ TCP Header │ HTTP payload...                    │
│ src: 10.0  │ src: 49152 │                                    │
│   .0.1     │ dst: 80    │                                    │
│ dst: 10.0  │ ...        │                                    │
│   .0.2     │            │                                    │
│ proto: TCP │            │                                    │
│ ttl: 64    │            │                                    │
└────────────┴────────────┴────────────────────────────────────┘

Layer 2 (Ethernet) wraps IP packet in an Ethernet frame:
┌──────────────┬────────────┬────────────┬─────────────────────┐
│ Eth Header   │ IP Header  │ TCP Header │ HTTP payload...     │
│ dst: AA:BB:  │ src: 10.0  │ src: 49152 │                     │
│   CC:DD:EE:  │   .0.1     │ dst: 80    │                     │
│   FF         │ dst: 10.0  │ ...        │                     │
│ src: 11:22:  │   .0.2     │            │                     │
│   33:44:55:  │ proto: TCP │            │                     │
│   66         │ ttl: 64    │            │                     │
│ type: 0x0800 │            │            │                     │
│ (IPv4)       │            │            │                     │
└──────────────┴────────────┴────────────┴─────────────────────┘

Wire: the entire frame is transmitted as raw bytes.

On the receiving side, the process reverses:
  Ethernet strips its header → passes IP packet up
  IP strips its header → passes TCP segment up
  TCP strips its header → passes HTTP data to application
```

## Data Structures

### Layer 2: Ethernet

#### EthernetFrame

```
EthernetFrame
══════════════

  ┌──────────────────┬────────────────────────────────────────────────┐
  │ Field            │ Description                                    │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ dest_mac         │ 6-byte destination MAC address. This is the    │
  │                  │ hardware address of the network card that      │
  │                  │ should receive this frame. Example:             │
  │                  │ [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]           │
  │                  │ Broadcast: [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]│
  ├──────────────────┼────────────────────────────────────────────────┤
  │ src_mac          │ 6-byte source MAC address. The sender's        │
  │                  │ hardware address.                               │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ ether_type       │ 2-byte protocol identifier:                    │
  │                  │   0x0800 = IPv4                                 │
  │                  │   0x0806 = ARP                                  │
  │                  │ Tells the receiver how to interpret the         │
  │                  │ payload.                                        │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ payload          │ The data being carried — an IP packet, an ARP  │
  │                  │ message, etc. Variable length.                  │
  └──────────────────┴────────────────────────────────────────────────┘

  Methods:
    serialize() → Vec<u8>     — convert frame to bytes for transmission
    deserialize(bytes) → Self — parse received bytes into a frame
```

**What is a MAC address?** Every network interface card (NIC) has a unique
48-bit address burned in at the factory. MAC addresses are Layer 2 identifiers
— they only matter on the local network segment. When a packet needs to cross
a router to another network, the MAC addresses change at each hop, but the
IP addresses stay the same.

#### ARP: Bridging IP and MAC

When a computer wants to send an IP packet to 192.168.1.5, it needs the MAC
address of 192.168.1.5's network card. **ARP** (Address Resolution Protocol)
solves this by broadcasting "Who has 192.168.1.5?" and waiting for a reply.

```
ARP Table
═════════

  ┌──────────────────┬────────────────────────────────────────────────┐
  │ Field            │ Description                                    │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ entries          │ HashMap<IPv4Address, MacAddress>                │
  │                  │ Maps IP addresses to MAC addresses.             │
  └──────────────────┴────────────────────────────────────────────────┘

  Methods:
    lookup(ip) → Option<MacAddress>   — check if we know this IP's MAC
    insert(ip, mac)                   — learn a new mapping
    request(target_ip) → EthernetFrame — build an ARP request broadcast
    reply(request) → EthernetFrame     — build an ARP reply

ARP Exchange:
  Host A (10.0.0.1, MAC AA:...) wants to send to Host B (10.0.0.2):

  1. A checks ARP table for 10.0.0.2 → not found
  2. A broadcasts ARP Request:
     ┌────────────────────────────────────┐
     │ "Who has 10.0.0.2? Tell 10.0.0.1" │
     │ dest_mac: FF:FF:FF:FF:FF:FF       │
     │ src_mac: AA:AA:AA:AA:AA:AA        │
     │ ether_type: 0x0806 (ARP)          │
     └────────────────────────────────────┘
  3. Host B sees the request, replies:
     ┌────────────────────────────────────┐
     │ "10.0.0.2 is at BB:BB:BB:BB:BB:BB"│
     │ dest_mac: AA:AA:AA:AA:AA:AA       │
     │ src_mac: BB:BB:BB:BB:BB:BB        │
     └────────────────────────────────────┘
  4. A stores (10.0.0.2 → BB:BB:...) in ARP table
  5. A can now send the IP packet in an Ethernet frame to BB:BB:...
```

### Layer 3: IP (Internet Protocol)

#### IPv4Header

```
IPv4Header
══════════

  ┌──────────────────┬────────────────────────────────────────────────┐
  │ Field            │ Description                                    │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ version          │ Always 4 (for IPv4). The "4" in "IPv4."        │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ ihl              │ Internet Header Length — the header size in    │
  │                  │ 32-bit words. Minimum 5 (= 20 bytes, no       │
  │                  │ options). We always use 5.                      │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ total_length     │ Total size of the IP packet (header + payload) │
  │                  │ in bytes. Maximum 65,535.                       │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ ttl              │ Time To Live — decremented by 1 at each        │
  │                  │ router. When it reaches 0, the packet is       │
  │                  │ discarded. Prevents infinite routing loops.     │
  │                  │ Typically starts at 64 or 128.                  │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ protocol         │ Which Layer 4 protocol the payload contains:   │
  │                  │   6  = TCP                                      │
  │                  │   17 = UDP                                      │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ header_checksum  │ Error-detection value computed over the header │
  │                  │ bytes. See checksum algorithm below.            │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ src_ip           │ 4-byte source IP address.                      │
  │                  │ Example: [10, 0, 0, 1] = "10.0.0.1"           │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ dst_ip           │ 4-byte destination IP address.                 │
  │                  │ Example: [10, 0, 0, 2] = "10.0.0.2"           │
  └──────────────────┴────────────────────────────────────────────────┘

  Methods:
    serialize() → Vec<u8>
    deserialize(bytes) → Self
    compute_checksum() → u16
    verify_checksum() → bool
```

#### IP Checksum Algorithm

The IP checksum is a simple error-detection mechanism. It is NOT a security
hash — it only catches accidental corruption (bit flips during transmission):

```
IP Checksum Algorithm
═════════════════════

1. Treat the header as a sequence of 16-bit words.
2. Set the checksum field to 0 before computing.
3. Sum all 16-bit words using ones'-complement addition:
   - Normal addition, but if there is a carry out of bit 15,
     add it back to bit 0.
4. Take the ones'-complement (bitwise NOT) of the sum.
5. Store the result in the checksum field.

Verification: repeat steps 1-4 on the received header (including
the checksum field). If the result is 0xFFFF (or 0x0000 depending on
convention), the header is valid.

Example with 4 words:
  Word 1: 0x4500
  Word 2: 0x003C
  Word 3: 0x1C46
  Word 4: 0x4000

  Sum:    0x4500 + 0x003C + 0x1C46 + 0x4000 = 0xBD1E + ...
  (continue for all header words)
  Ones'-complement of final sum → checksum
```

#### Routing Table

```
RoutingTable
════════════

  Each entry:
  ┌──────────────────┬────────────────────────────────────────────────┐
  │ Field            │ Description                                    │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ network          │ Network address (e.g., 10.0.0.0)               │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ mask             │ Subnet mask (e.g., 255.255.255.0). ANDed with │
  │                  │ the destination IP to check if it matches this │
  │                  │ route.                                          │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ gateway          │ Next-hop IP address. If the destination is on  │
  │                  │ the local network, gateway is 0.0.0.0 (direct  │
  │                  │ delivery).                                      │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ interface        │ Which network interface to use for this route. │
  └──────────────────┴────────────────────────────────────────────────┘

  Routing algorithm:
    1. For each entry in the table:
       if (dst_ip & entry.mask) == entry.network:
         this route matches.
    2. Among all matching routes, pick the one with the longest
       (most specific) mask — this is "longest prefix match."
    3. Send the packet to the gateway via the specified interface.
    4. If no route matches, drop the packet (or send ICMP
       "destination unreachable" in a real stack).
```

#### IPLayer

```
IPLayer
═══════

  ┌──────────────────┬────────────────────────────────────────────────┐
  │ Field            │ Description                                    │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ local_ip         │ This host's IP address.                        │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ routing_table    │ RoutingTable for outbound packets.              │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ arp_table        │ ARP cache for IP → MAC resolution.             │
  └──────────────────┴────────────────────────────────────────────────┘

  Methods:
    send(dst_ip, protocol, payload) → Result
      1. Build IPv4Header (src=local_ip, dst=dst_ip, proto=protocol).
      2. Compute checksum.
      3. Look up route in routing_table → get gateway and interface.
      4. Resolve next-hop IP to MAC via ARP table (send ARP request
         if not cached).
      5. Wrap in EthernetFrame and transmit on the interface.

    receive(frame) → (src_ip, protocol, payload)
      1. Parse IPv4Header from frame payload.
      2. Verify checksum.
      3. If dst_ip != local_ip, drop (we are not a router).
      4. Decrement TTL. If TTL == 0, drop.
      5. Return (src_ip, protocol, payload) to the transport layer.
```

### Layer 4: TCP (Transmission Control Protocol)

TCP provides a **reliable, ordered, byte-stream** service. It is what makes
the Internet work for web browsing, email, file transfers — anything where
you cannot afford to lose or reorder data.

#### TCP States

A TCP connection goes through a well-defined state machine. This is one of the
most important diagrams in all of computer networking:

```
TCP State Machine (simplified)
══════════════════════════════

                    ┌────────┐
                    │ CLOSED │ ← initial state
                    └───┬────┘
                        │
          ┌─────────────┼─────────────┐
          │ (server)    │             │ (client)
          ▼             │             ▼
     ┌────────┐         │        ┌──────────┐
     │ LISTEN │         │        │ SYN_SENT │──── send SYN
     └───┬────┘         │        └─────┬────┘
         │              │              │
    recv SYN,           │         recv SYN+ACK,
    send SYN+ACK        │         send ACK
         │              │              │
         ▼              │              ▼
    ┌──────────┐        │       ┌─────────────┐
    │ SYN_RCVD │────────┼──────→│ ESTABLISHED │ ← data transfer
    └──────────┘   recv │       └──────┬──────┘
                   ACK  │              │
                        │         close / recv FIN
                        │              │
                        │    ┌─────────┼──────────┐
                        │    │ (active │close)    │ (passive close)
                        │    ▼         │          ▼
                        │ ┌─────────┐  │   ┌────────────┐
                        │ │FIN_WAIT1│  │   │ CLOSE_WAIT │
                        │ └────┬────┘  │   └─────┬──────┘
                        │ send FIN     │    send FIN
                        │      │       │         │
                        │      ▼       │         ▼
                        │ ┌─────────┐  │   ┌──────────┐
                        │ │FIN_WAIT2│  │   │ LAST_ACK │
                        │ └────┬────┘  │   └─────┬────┘
                        │ recv FIN     │    recv ACK
                        │      │       │         │
                        │      ▼       │         ▼
                        │ ┌──────────┐ │    ┌────────┐
                        │ │TIME_WAIT │ │    │ CLOSED │
                        │ └────┬─────┘ │    └────────┘
                        │      │ (2MSL │timeout)
                        │      ▼       │
                        │ ┌────────┐   │
                        └─│ CLOSED │───┘
                          └────────┘

  All 11 states:
    CLOSED, LISTEN, SYN_SENT, SYN_RECEIVED,
    ESTABLISHED, FIN_WAIT_1, FIN_WAIT_2,
    CLOSE_WAIT, LAST_ACK, TIME_WAIT, CLOSING
```

#### TCPHeader

```
TCPHeader
═════════

  ┌──────────────────┬────────────────────────────────────────────────┐
  │ Field            │ Description                                    │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ src_port         │ 16-bit source port (0-65535). Identifies the   │
  │                  │ sending application. Ephemeral ports (49152-   │
  │                  │ 65535) are typically assigned to clients.       │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ dst_port         │ 16-bit destination port. Well-known ports:     │
  │                  │   80 = HTTP, 443 = HTTPS, 22 = SSH,            │
  │                  │   53 = DNS, 25 = SMTP                          │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ seq_num          │ 32-bit sequence number. Identifies the first   │
  │                  │ byte of data in this segment. Used for         │
  │                  │ ordering and detecting duplicates.              │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ ack_num          │ 32-bit acknowledgment number. "I have received │
  │                  │ all bytes up to ack_num - 1. Send me byte      │
  │                  │ ack_num next."                                  │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ flags            │ Control bits:                                   │
  │                  │   SYN — synchronize sequence numbers (connect) │
  │                  │   ACK — acknowledgment field is valid           │
  │                  │   FIN — sender is finished sending              │
  │                  │   RST — reset the connection (abort)            │
  │                  │   PSH — push data to application immediately   │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ window_size      │ 16-bit flow control window. "I have this many │
  │                  │ bytes of buffer space left. Do not send more   │
  │                  │ than this." Prevents a fast sender from        │
  │                  │ overwhelming a slow receiver.                   │
  └──────────────────┴────────────────────────────────────────────────┘

  Methods:
    serialize() → Vec<u8>
    deserialize(bytes) → Self
```

#### Three-Way Handshake (Connection Setup)

```
Client                                    Server
  │                                          │
  │ ──── SYN (seq=100) ──────────────────→   │  "I want to connect.
  │                                          │   My starting sequence
  │                                          │   number is 100."
  │                                          │
  │ ←── SYN+ACK (seq=300, ack=101) ──────   │  "OK. My starting seq
  │                                          │   is 300. I acknowledge
  │                                          │   your seq 100 (next
  │                                          │   byte I expect is 101)."
  │                                          │
  │ ──── ACK (seq=101, ack=301) ─────────→   │  "I acknowledge your
  │                                          │   seq 300."
  │                                          │
  │         ESTABLISHED ←────────────────→   │  Both sides ready
  │                                          │   for data transfer.
```

**Why three messages?** Both sides need to establish initial sequence numbers
and confirm they can reach each other. SYN sends "here is my sequence number,"
SYN+ACK says "I got yours, here is mine," and the final ACK says "I got yours
too." Two messages would leave one side unconfirmed.

#### Connection Teardown (Four-Way)

```
Client                                    Server
  │                                          │
  │ ──── FIN (seq=500) ──────────────────→   │  "I am done sending."
  │                                          │
  │ ←── ACK (ack=501) ───────────────────   │  "Got it."
  │                                          │
  │     (server may still send data...)      │
  │                                          │
  │ ←── FIN (seq=800) ───────────────────   │  "I am also done."
  │                                          │
  │ ──── ACK (ack=801) ──────────────────→   │  "Got it."
  │                                          │
  │         CLOSED ←─────────────────────→   │  Connection fully closed.
```

#### Send and Receive Buffers

Each TCP connection maintains two buffers:

```
TCPConnection
═════════════

  ┌──────────────────┬────────────────────────────────────────────────┐
  │ Field            │ Description                                    │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ state            │ Current TCPState (CLOSED, ESTABLISHED, etc.)   │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ local_port       │ Our port number.                                │
  │ remote_port      │ The other side's port number.                   │
  │ local_ip         │ Our IP address.                                 │
  │ remote_ip        │ The other side's IP address.                    │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ send_seq         │ Next sequence number we will send.              │
  │ send_unacked     │ Oldest unacknowledged sequence number.          │
  │ recv_next        │ Next sequence number we expect to receive.      │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ send_buffer      │ Bytes the application wants to send but have   │
  │                  │ not yet been acknowledged by the remote side.   │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ recv_buffer      │ Bytes received from the network but not yet    │
  │                  │ read by the application.                        │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ retransmit_queue │ Segments sent but not yet acknowledged. If an  │
  │                  │ ACK does not arrive within a timeout, these    │
  │                  │ segments are re-sent. This is how TCP achieves │
  │                  │ reliability.                                    │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ retransmit_timer │ Countdown timer. When it expires, resend the   │
  │                  │ oldest unacknowledged segment.                  │
  └──────────────────┴────────────────────────────────────────────────┘
```

#### Retransmission

TCP achieves reliability through acknowledgments and retransmission:

```
1. Sender transmits segment with seq=100, data="hello" (5 bytes).
2. Sender starts retransmit_timer (e.g., 200ms).
3. Receiver gets the segment, sends ACK with ack=105
   ("I received up to byte 104, send me byte 105 next").
4. Sender receives ACK, removes segment from retransmit_queue.

If the ACK does not arrive before the timer expires:
  5. Sender resends the segment from retransmit_queue.
  6. Sender doubles the timer (exponential backoff).
  7. Repeat until ACK received or max retries exceeded.
```

### Layer 4: UDP (User Datagram Protocol)

UDP is the "anti-TCP" — it provides no reliability, no ordering, no flow
control, and no connection state. You send a datagram; it either arrives or
it does not. This simplicity makes UDP fast and lightweight, ideal for:

- DNS lookups (one question, one answer)
- Video streaming (a dropped frame is better than waiting for retransmission)
- Games (latest position update matters, old ones do not)

#### UDPHeader

```
UDPHeader
═════════

  ┌──────────────────┬────────────────────────────────────────────────┐
  │ Field            │ Description                                    │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ src_port         │ 16-bit source port.                             │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ dst_port         │ 16-bit destination port.                        │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ length           │ Total length of header + payload in bytes.     │
  │                  │ Minimum 8 (header only, no payload).            │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ checksum         │ Optional error-detection checksum. Can be 0    │
  │                  │ (meaning "not computed").                        │
  └──────────────────┴────────────────────────────────────────────────┘

  Methods:
    serialize() → Vec<u8>
    deserialize(bytes) → Self
```

UDP is so simple, the entire header is only 8 bytes. Compare to TCP's minimum
20 bytes.

### Socket API

The Socket API is the application-facing interface. It is modeled after the
Berkeley Sockets API, which has been the standard network programming
interface since the 1980s.

#### SocketType

```
SocketType
══════════

  STREAM — TCP (reliable, connection-oriented)
  DGRAM  — UDP (unreliable, connectionless)
```

#### Socket

```
Socket
══════

  ┌──────────────────┬────────────────────────────────────────────────┐
  │ Field            │ Description                                    │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ fd               │ File descriptor number (sockets are fds).      │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ socket_type      │ STREAM (TCP) or DGRAM (UDP).                   │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ local_ip         │ Bound local IP address (set by bind()).         │
  │ local_port       │ Bound local port (set by bind()).               │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ remote_ip        │ Connected remote IP (set by connect/accept).   │
  │ remote_port      │ Connected remote port.                          │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ tcp_connection   │ For STREAM sockets: the associated             │
  │                  │ TCPConnection (state machine, buffers).         │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ is_listening     │ True if this is a server socket (called         │
  │                  │ listen()). Listening sockets accept new         │
  │                  │ connections; they do not transfer data.          │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ accept_queue     │ For listening sockets: queue of completed      │
  │                  │ TCP connections waiting to be accepted.          │
  └──────────────────┴────────────────────────────────────────────────┘
```

#### SocketManager

```
SocketManager
═════════════

  Methods:
    socket(socket_type) → fd
      Create a new socket, allocate an fd.

    bind(fd, ip, port)
      Assign a local address and port to the socket.
      Fails if the port is already in use.

    listen(fd, backlog)
      Mark the socket as a server socket. backlog is the max number
      of pending connections in the accept_queue.

    accept(fd) → (new_fd, remote_ip, remote_port)
      Wait for an incoming TCP connection on a listening socket.
      When a connection completes the 3-way handshake, dequeue it
      and return a new socket fd for that specific connection.

    connect(fd, remote_ip, remote_port)
      Initiate a TCP connection (send SYN, wait for SYN+ACK,
      send ACK). For UDP, just record the remote address.

    send(fd, data) → bytes_sent
      For TCP: add data to send_buffer, transmit segments.
      For UDP: send a single datagram.

    recv(fd, buffer, max_len) → bytes_received
      For TCP: read from recv_buffer.
      For UDP: receive a single datagram.

    close(fd)
      For TCP: initiate connection teardown (send FIN).
      For UDP: just free the socket.
```

### DNS: Domain Name Resolution

DNS maps human-readable names ("example.com") to IP addresses (93.184.216.34).
We implement a simplified resolver with a static lookup table and a basic
UDP query format:

```
DNSResolver
═══════════

  ┌──────────────────┬────────────────────────────────────────────────┐
  │ Field            │ Description                                    │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ static_table     │ HashMap<String, IPv4Address>                   │
  │                  │ Pre-loaded name → IP mappings. In our          │
  │                  │ simulation, this avoids needing a real DNS      │
  │                  │ server.                                         │
  │                  │ Example: "example.com" → [93, 184, 216, 34]   │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ dns_server_ip    │ IP of the DNS server to query if not in the   │
  │                  │ static table.                                   │
  └──────────────────┴────────────────────────────────────────────────┘

  Methods:
    resolve(hostname) → Option<IPv4Address>
      1. Check static_table. If found, return immediately.
      2. Otherwise, build a DNS query (UDP, port 53), send to
         dns_server_ip, parse the response.
```

### HTTP: Hypertext Transfer Protocol

HTTP is the protocol of the web. It is a text-based request-response protocol
built on top of TCP:

```
HTTPRequest
═══════════

  ┌──────────────────┬────────────────────────────────────────────────┐
  │ Field            │ Description                                    │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ method           │ The action: GET (retrieve), POST (submit),     │
  │                  │ PUT, DELETE, etc.                               │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ path             │ The resource path: "/index.html", "/api/users" │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ headers          │ Key-value pairs of metadata:                   │
  │                  │   "Host: example.com"                          │
  │                  │   "Content-Type: text/plain"                   │
  │                  │   "Content-Length: 13"                          │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ body             │ Optional payload (for POST/PUT requests).      │
  └──────────────────┴────────────────────────────────────────────────┘

  Wire format (what actually goes over TCP):
    GET /index.html HTTP/1.1\r\n
    Host: example.com\r\n
    \r\n

  Methods:
    serialize() → String
    deserialize(text) → Self


HTTPResponse
════════════

  ┌──────────────────┬────────────────────────────────────────────────┐
  │ Field            │ Description                                    │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ status_code      │ 3-digit result code:                           │
  │                  │   200 = OK                                      │
  │                  │   404 = Not Found                               │
  │                  │   500 = Internal Server Error                   │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ status_text      │ Human-readable status: "OK", "Not Found"       │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ headers          │ Same as request headers.                        │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ body             │ The response payload (HTML, JSON, etc.).        │
  └──────────────────┴────────────────────────────────────────────────┘

  Wire format:
    HTTP/1.1 200 OK\r\n
    Content-Type: text/html\r\n
    Content-Length: 13\r\n
    \r\n
    Hello, World!

  Methods:
    serialize() → String
    deserialize(text) → Self


HTTPClient
══════════

  Methods:
    get(url) → HTTPResponse
      1. Parse URL into (hostname, port, path).
      2. Resolve hostname → IP via DNSResolver.
      3. Create TCP socket, connect to IP:port.
      4. Build HTTPRequest (method=GET, path=path).
      5. Send serialized request over TCP.
      6. Receive response bytes, deserialize into HTTPResponse.
      7. Close socket.
      8. Return HTTPResponse.

    post(url, body, content_type) → HTTPResponse
      Same as get(), but method=POST with body and Content-Type
      header.
```

### NetworkWire and SimulatedNIC

Since we do not have real network hardware, we simulate it:

```
NetworkWire
═══════════

  A NetworkWire is a bidirectional connection between two SimulatedNIC
  devices. When NIC A sends a frame, the wire delivers it to NIC B,
  and vice versa. Think of it as an Ethernet cable.

  ┌─────────────┐        ┌──────────────┐        ┌─────────────┐
  │ SimulatedNIC│◄──────►│ NetworkWire  │◄──────►│ SimulatedNIC│
  │   (Client)  │  send  │  (buffer of  │  send  │  (Server)   │
  │ MAC: AA:... │  recv  │   frames)    │  recv  │ MAC: BB:... │
  │ IP: 10.0.0.1│        │              │        │ IP: 10.0.0.2│
  └─────────────┘        └──────────────┘        └─────────────┘

  The wire has two queues: one for each direction.
  send() enqueues a frame; recv() dequeues from the other queue.


SimulatedServer
═══════════════

  Sits on the server-side NIC. Runs a simple event loop:

  1. Receive Ethernet frame from wire.
  2. Strip Ethernet header → IP packet.
  3. Strip IP header → TCP segment (or UDP datagram).
  4. Process at transport layer (TCP handshake, data transfer).
  5. If data received, pass to HTTP handler.
  6. HTTP handler generates response.
  7. Send response back down the stack.

  The SimulatedServer has a configurable set of routes:
    "/" → "Hello, World!"
    "/about" → "About page"
    (404 for anything else)
```

## Algorithms

### Full Request Flow

Here is what happens when a client sends `GET / HTTP/1.1` to a server:

```
Client                    Wire                     Server
  │                         │                         │
  │ 1. socket(STREAM)       │                         │
  │ 2. connect(10.0.0.2:80) │                         │
  │    ──SYN──────────────────────────────────────→    │ 3. accept()
  │    ←─SYN+ACK──────────────────────────────────    │
  │    ──ACK──────────────────────────────────────→    │
  │                         │                         │
  │ 4. send("GET / ...")    │                         │
  │    ──[ETH[IP[TCP[HTTP]]]]─────────────────────→   │ 5. recv()
  │    ←─[ETH[IP[TCP[ACK]]]]─────────────────────    │    parse HTTP
  │                         │                         │    build response
  │ 6. recv()               │                         │
  │    ←─[ETH[IP[TCP[HTTP 200 OK...]]]]───────────    │ 7. send(response)
  │    ──[ETH[IP[TCP[ACK]]]]──────────────────────→   │
  │                         │                         │
  │ 8. close()              │                         │
  │    ──FIN──────────────────────────────────────→    │ 9. close()
  │    ←─ACK──────────────────────────────────────    │
  │    ←─FIN──────────────────────────────────────    │
  │    ──ACK──────────────────────────────────────→    │
  │                         │                         │
```

### Demultiplexing: Routing Received Packets

When a frame arrives at a NIC, the stack must figure out which socket should
receive the data. This is called **demultiplexing**:

```
Incoming Ethernet frame
│
├── ether_type = 0x0806 → ARP handler
│
├── ether_type = 0x0800 → IP layer
│   │
│   ├── protocol = 6 (TCP) → TCP layer
│   │   │
│   │   └── match (src_ip, src_port, dst_ip, dst_port)
│   │       to an existing TCPConnection → deliver to that socket
│   │
│   ├── protocol = 17 (UDP) → UDP layer
│   │   │
│   │   └── match (dst_ip, dst_port) to a bound socket → deliver
│   │
│   └── unknown protocol → drop
│
└── unknown ether_type → drop
```

## Syscalls

```
Syscall Table Additions
═══════════════════════

  ┌──────────────┬─────────┬──────────────────────────────────────────┐
  │ Name         │ Number  │ Arguments                                │
  ├──────────────┼─────────┼──────────────────────────────────────────┤
  │ sys_socket   │ 198     │ (socket_type) → fd                       │
  │              │         │ Create a new socket (TCP or UDP).         │
  ├──────────────┼─────────┼──────────────────────────────────────────┤
  │ sys_bind     │ 200     │ (fd, ip, port) → 0                       │
  │              │         │ Assign a local address to a socket.       │
  ├──────────────┼─────────┼──────────────────────────────────────────┤
  │ sys_listen   │ 201     │ (fd, backlog) → 0                        │
  │              │         │ Mark socket as a listening server socket. │
  ├──────────────┼─────────┼──────────────────────────────────────────┤
  │ sys_accept   │ 202     │ (fd) → new_fd                            │
  │              │         │ Accept an incoming TCP connection.         │
  │              │         │ Returns a new socket fd for the conn.     │
  ├──────────────┼─────────┼──────────────────────────────────────────┤
  │ sys_connect  │ 203     │ (fd, remote_ip, remote_port) → 0         │
  │              │         │ Initiate TCP connection (3-way handshake).│
  │              │         │ For UDP, records the default destination. │
  ├──────────────┼─────────┼──────────────────────────────────────────┤
  │ sys_sendto   │ 206     │ (fd, buf_ptr, buf_len, dest_ip,          │
  │              │         │  dest_port) → bytes_sent                  │
  │              │         │ Send data. For connected sockets (TCP or  │
  │              │         │ connected UDP), dest can be omitted.       │
  ├──────────────┼─────────┼──────────────────────────────────────────┤
  │ sys_recvfrom │ 207     │ (fd, buf_ptr, buf_len) → bytes_received  │
  │              │         │ Receive data from a socket. For UDP, also │
  │              │         │ returns the sender's IP and port.          │
  └──────────────┴─────────┴──────────────────────────────────────────┘
```

### Sockets as File Descriptors

In Unix, "everything is a file." Sockets are no exception — they are
represented as file descriptors. This means:

- `sys_read` and `sys_write` work on sockets (as an alternative to
  `sys_sendto` / `sys_recvfrom`)
- `sys_close` closes a socket
- `sys_dup` / `sys_dup2` can duplicate socket fds
- The kernel's fd table does not distinguish between files, pipes, and
  sockets — they are all entries in the same table

## Dependencies

```
D17 Network Stack
│
├── depends on ──→ Device Driver Framework (D14)
│                   └── NetworkDevice trait (send_frame, recv_frame)
│
├── depends on ──→ File System (D15)
│                   └── Sockets use file descriptors
│                   └── sys_read/sys_write/sys_close work on sockets
│
└── used by ───→ User programs
                  └── HTTP clients
                  └── DNS lookups
                  └── Any networked application
```

## Testing Strategy

### Unit Tests — Layer 2

1. **Ethernet frame serialize/deserialize**: Build a frame, serialize to bytes,
   deserialize, verify all fields match.
2. **ARP table**: Insert mappings, look up, verify. Look up unknown IP, verify
   None.
3. **ARP request/reply**: Build ARP request for unknown IP, verify broadcast
   MAC. Build reply, verify correct MAC.

### Unit Tests — Layer 3

4. **IPv4 header serialize/deserialize**: Round-trip test.
5. **IP checksum**: Compute checksum, verify. Flip a bit in the header, verify
   checksum fails.
6. **Routing table**: Add routes with different masks. Verify longest-prefix
   match is selected. Verify no-match returns error.
7. **IP send/receive**: Send a packet, verify correct Ethernet frame is
   produced with right MACs.

### Unit Tests — Layer 4

8. **TCP three-way handshake**: Simulate SYN → SYN+ACK → ACK, verify both
   sides reach ESTABLISHED state.
9. **TCP data transfer**: Send data, verify sequence numbers advance. Verify
   ACK advances recv_next.
10. **TCP connection teardown**: FIN → ACK → FIN → ACK, verify both sides
    reach CLOSED.
11. **TCP retransmission**: Send data, do not ACK, verify retransmission after
    timeout.
12. **UDP send/receive**: Send datagram, verify it arrives. Verify no
    connection state is created.
13. **UDP header serialize/deserialize**: Round-trip test.

### Unit Tests — Socket API

14. **socket() + bind()**: Create socket, bind to port, verify. Bind to
    already-used port, verify error.
15. **listen() + accept() + connect()**: Server listens, client connects,
    verify both get valid sockets.
16. **send() + recv()**: Send data over TCP connection, receive on other end,
    verify contents.
17. **close()**: Close socket, verify resources freed. Verify FIN is sent
    for TCP.

### Unit Tests — Application Layer

18. **DNS resolve**: Static table lookup, verify correct IP.
19. **HTTP request serialize/deserialize**: Build GET request, serialize,
    deserialize, verify.
20. **HTTP response serialize/deserialize**: Build 200 OK response, round-trip.
21. **HTTPClient.get()**: Full end-to-end through simulated stack and wire.

### Integration Tests

22. **Full HTTP request**: Client creates socket → connects → sends GET →
    receives 200 OK with body → closes. Verify every layer is exercised.
23. **Multiple connections**: Two clients connect to the same server
    simultaneously. Verify independent data streams.
24. **NetworkWire bidirectional**: Both sides send and receive data. Verify
    no cross-talk.
25. **404 response**: Request a nonexistent path from SimulatedServer, verify
    404 response.

### Coverage Target

Target 90%+ line coverage. Every protocol state (all 11 TCP states), every
error path (connection refused, timeout, checksum failure), and every layer
(Ethernet through HTTP) must be exercised.
