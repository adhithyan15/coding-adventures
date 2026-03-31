# network-stack (Perl)

Layered TCP/IP network protocol stack for the coding-adventures simulated OS.

## What It Does

- `EthernetFrame` — Layer 2 frame serialise/deserialise (14-byte header)
- `ARPTable` — IP-to-MAC address resolution cache
- `IPv4Header` — Layer 3 header with ones'-complement checksum
- `RoutingTable` — Longest-prefix-match route lookup
- `IPLayer` — Creates and parses full IP packets
- `TCPSegment` — Layer 4 TCP header (20 bytes) with flag helpers
- `UDPDatagram` — Layer 4 UDP header (8 bytes)
- `TCPConnection` — Full TCP state machine (CLOSED → ESTABLISHED → CLOSED)
- `NetworkStack` — Top-level send/receive facade

## Stack Layers

```
Application  "Hello, World!"
TCP/UDP      [TCP/UDP Header] + data
IP           [IP Header 20 B] + TCP/UDP segment
Ethernet     [Eth Header 14 B] + IP packet
Wire         raw bytes (list of integers)
```

## Usage

```perl
use CodingAdventures::NetworkStack;

my $stack = CodingAdventures::NetworkStack::NetworkStack->new(
    local_ip  => [192, 168, 1, 10],
    local_mac => [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF],
);

# Send a UDP datagram
my $wire = $stack->send_udp(
    [192, 168, 1, 1],                           # dst IP
    [0x11, 0x22, 0x33, 0x44, 0x55, 0x66],      # dst MAC
    5000, 53,                                   # src_port, dst_port
    [0xDE, 0xAD],                               # payload bytes
);

# Parse an incoming frame
my $result = $stack->receive($wire);
# $result->{ip_status}, $result->{protocol}, $result->{data}, ...

# TCP state machine
my $conn = CodingAdventures::NetworkStack::TCPConnection->new(
    local_port => 5000, remote_port => 80,
);
my ($c, $syn) = $conn->initiate_connect();
# ... handle SYN+ACK, reach ESTABLISHED, send_data, recv_data
```
