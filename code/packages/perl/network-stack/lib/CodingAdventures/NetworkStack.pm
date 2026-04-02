package CodingAdventures::NetworkStack;

# ============================================================================
# CodingAdventures::NetworkStack — Layered Network Protocol Stack
# ============================================================================
#
# Implements the TCP/IP model from Ethernet frames at Layer 2 up to
# TCP/UDP sockets at Layer 4.
#
# ## The Postal Analogy
#
#   Ethernet  — local mail carrier; delivers between houses on the same street.
#   IP        — postal routing; which city and post office?
#   TCP       — registered mail with tracking; guaranteed delivery and order.
#   UDP       — a postcard; fast, no guarantee it arrives.
#
# ## Packet Encapsulation
#
# As data moves DOWN the stack, each layer wraps the data from above:
#
#   Application: "Hello!"
#   TCP layer:   [TCP Header 20 B] + "Hello!"
#   IP layer:    [IP Header  20 B] + TCP segment
#   Ethernet:    [Eth Header 14 B] + IP packet
#   Wire:        raw bytes
#
# ## Module Map (all defined in this file)
#
#   EthernetFrame  — Layer 2 frame serialise/deserialise
#   ARPTable       — IP→MAC address resolution cache
#   IPv4Header     — Layer 3 header with ones'-complement checksum
#   RoutingTable   — Longest-prefix-match route lookup
#   IPLayer        — Creates and parses IP packets
#   TCPSegment     — Layer 4 TCP header (20 bytes)
#   UDPDatagram    — Layer 4 UDP header (8 bytes)
#   TCPConnection  — Full TCP state machine (11 states)
#   NetworkStack   — Top-level send/receive facade
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

# Protocol constants — shared across sub-modules
use constant PROTO_TCP      => 6;
use constant PROTO_UDP      => 17;
use constant ETHERTYPE_IPV4 => 0x0800;
use constant ETHERTYPE_ARP  => 0x0806;

# TCP flag bits
use constant TCP_FIN => 0x01;   # "I am done sending"
use constant TCP_SYN => 0x02;   # "Let's synchronise sequence numbers"
use constant TCP_RST => 0x04;   # "Abort this connection"
use constant TCP_PSH => 0x08;   # "Push data to application immediately"
use constant TCP_ACK => 0x10;   # "The ack_num field is valid"
use constant TCP_URG => 0x20;   # "Urgent pointer is valid"

# Broadcast MAC address — reaches every device on the local network
use constant MAC_BROADCAST => [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];

# TCP connection states
use constant TCP_CLOSED       => 'closed';
use constant TCP_LISTEN       => 'listen';
use constant TCP_SYN_SENT     => 'syn_sent';
use constant TCP_SYN_RECEIVED => 'syn_received';
use constant TCP_ESTABLISHED  => 'established';
use constant TCP_FIN_WAIT_1   => 'fin_wait_1';
use constant TCP_FIN_WAIT_2   => 'fin_wait_2';
use constant TCP_CLOSE_WAIT   => 'close_wait';
use constant TCP_LAST_ACK     => 'last_ack';
use constant TCP_TIME_WAIT    => 'time_wait';
use constant TCP_CLOSING      => 'closing';

# ============================================================================
# EthernetFrame — Layer 2: Local Delivery
# ============================================================================
#
# Every NIC has a unique 48-bit MAC address burned in at the factory.
# Ethernet frames carry data between devices on the SAME local network.
#
# Wire format (big-endian throughout):
#
#   ┌────────────────┬───────────────┬────────────┬──────────────────┐
#   │ Dest MAC (6 B) │ Src MAC (6 B) │ EtherType  │ Payload (N B)    │
#   │                │               │ (2 B)      │                  │
#   └────────────────┴───────────────┴────────────┴──────────────────┘
#
# EtherType 0x0800 = IPv4, 0x0806 = ARP
#
# Arrays of integers represent MAC addresses: [0xAA, 0xBB, 0xCC, ...]
# Arrays of integers represent payloads:      [0x45, 0x00, ...]

package CodingAdventures::NetworkStack::EthernetFrame;

sub new {
    my ($class, %opts) = @_;
    return bless {
        dest_mac   => $opts{dest_mac}   // [0,0,0,0,0,0],
        src_mac    => $opts{src_mac}    // [0,0,0,0,0,0],
        ether_type => $opts{ether_type} // CodingAdventures::NetworkStack::ETHERTYPE_IPV4,
        payload    => $opts{payload}    // [],
    }, $class;
}

sub serialize {
    my ($self) = @_;
    my $et_hi = ($self->{ether_type} >> 8) & 0xFF;
    my $et_lo =  $self->{ether_type}       & 0xFF;
    return [
        @{ $self->{dest_mac} },
        @{ $self->{src_mac}  },
        $et_hi, $et_lo,
        @{ $self->{payload}  },
    ];
}

sub deserialize {
    my ($class, $bytes) = @_;
    my @b = @$bytes;
    my @dest_mac = @b[0..5];
    my @src_mac  = @b[6..11];
    my $ether_type = ($b[12] << 8) | $b[13];
    my @payload  = @b[14..$#b];
    return $class->new(
        dest_mac   => \@dest_mac,
        src_mac    => \@src_mac,
        ether_type => $ether_type,
        payload    => \@payload,
    );
}

# ============================================================================
# ARPTable — IP-to-MAC Address Resolution Cache
# ============================================================================
#
# When a computer wants to send a packet to 192.168.1.5, it needs the MAC
# address of that device's NIC. ARP (Address Resolution Protocol) maintains
# a table of IP-string → MAC-array mappings.

package CodingAdventures::NetworkStack::ARPTable;

sub new {
    my ($class) = @_;
    return bless { table => {} }, $class;
}

sub lookup {
    my ($self, $ip) = @_;
    return $self->{table}{$ip};   # undef if not found
}

sub insert {
    my ($self, $ip, $mac) = @_;
    $self->{table}{$ip} = $mac;
    return $self;
}

sub size {
    my ($self) = @_;
    return scalar keys %{ $self->{table} };
}

# ============================================================================
# IPv4Header — Layer 3: Routing Across Networks
# ============================================================================
#
# Every device has an IP address. IP headers tell routers where to send each
# packet. The standard IPv4 header is exactly 20 bytes (IHL=5, no options).
#
# Header layout:
#
#   Byte 0:     version (4 bits = 4) | IHL (4 bits = 5)  →  0x45
#   Byte 1:     DSCP / ECN (ToS) — we always write 0
#   Bytes 2-3:  Total length (header + payload)
#   Bytes 4-5:  Identification — 0
#   Bytes 6-7:  Flags + Fragment Offset — 0
#   Byte 8:     TTL (Time To Live) — default 64
#   Byte 9:     Protocol (6=TCP, 17=UDP)
#   Bytes 10-11:Header Checksum (ones' complement of all 16-bit words)
#   Bytes 12-15:Source IP address
#   Bytes 16-19:Destination IP address
#
# ## Ones' Complement Checksum
#
#   1. Treat header as 10 × 16-bit big-endian words.
#   2. Set checksum field to 0 before computing.
#   3. Sum all words; fold any carry bits back into the low 16 bits.
#   4. Bitwise NOT → 16-bit result.
#
# A correctly received header gives 0xFFFF when all words (including
# the embedded checksum) are summed and folded.

package CodingAdventures::NetworkStack::IPv4Header;

sub new {
    my ($class, %opts) = @_;
    return bless {
        version         => $opts{version}  // 4,
        ihl             => $opts{ihl}      // 5,
        total_length    => $opts{total_length},
        ttl             => $opts{ttl}      // 64,
        protocol        => $opts{protocol},
        header_checksum => $opts{header_checksum} // 0,
        src_ip          => $opts{src_ip},
        dst_ip          => $opts{dst_ip},
    }, $class;
}

sub serialize {
    my ($self) = @_;
    my @src = @{ $self->{src_ip} };
    my @dst = @{ $self->{dst_ip} };
    return [
        (($self->{version} << 4) | $self->{ihl}) & 0xFF,
        0,  # ToS
        ($self->{total_length} >> 8) & 0xFF,
         $self->{total_length}       & 0xFF,
        0, 0,  # identification
        0, 0,  # flags + fragment offset
        $self->{ttl},
        $self->{protocol},
        ($self->{header_checksum} >> 8) & 0xFF,
         $self->{header_checksum}       & 0xFF,
        @src,
        @dst,
    ];
}

sub deserialize {
    my ($class, $bytes) = @_;
    my @b = @$bytes;
    my $b0 = $b[0];
    my $version      = ($b0 >> 4) & 0x0F;
    my $ihl          =  $b0       & 0x0F;
    my $total_length = ($b[2] << 8) | $b[3];
    my $ttl          =  $b[8];
    my $protocol     =  $b[9];
    my $checksum     = ($b[10] << 8) | $b[11];
    my @src = @b[12..15];
    my @dst = @b[16..19];
    return $class->new(
        version         => $version,
        ihl             => $ihl,
        total_length    => $total_length,
        ttl             => $ttl,
        protocol        => $protocol,
        header_checksum => $checksum,
        src_ip          => \@src,
        dst_ip          => \@dst,
    );
}

sub compute_checksum {
    my ($self) = @_;
    my $zero_cksum = ref($self)->new(
        version      => $self->{version},
        ihl          => $self->{ihl},
        total_length => $self->{total_length},
        ttl          => $self->{ttl},
        protocol     => $self->{protocol},
        header_checksum => 0,
        src_ip       => $self->{src_ip},
        dst_ip       => $self->{dst_ip},
    );
    my $bytes = $zero_cksum->serialize();
    my $sum = _sum_words($bytes);
    $sum = _fold_carry($sum);
    return (~$sum) & 0xFFFF;
}

sub verify_checksum {
    my ($self) = @_;
    my $bytes = $self->serialize();
    my $sum = _sum_words($bytes);
    $sum = _fold_carry($sum);
    return ($sum & 0xFFFF) == 0xFFFF;
}

sub _sum_words {
    my ($bytes) = @_;
    my $sum = 0;
    my @b   = @$bytes;
    for (my $i = 0; $i < $#b; $i += 2) {
        $sum += ($b[$i] << 8) | $b[$i+1];
    }
    return $sum;
}

sub _fold_carry {
    my ($sum) = @_;
    while ($sum > 0xFFFF) {
        $sum = ($sum & 0xFFFF) + ($sum >> 16);
    }
    return $sum;
}

# ============================================================================
# RoutingTable — Longest-Prefix-Match Route Lookup
# ============================================================================
#
# Each entry says: "If the destination matches network/mask, send to gateway
# via interface."
#
#   Longest prefix match: when multiple routes match, pick the most specific
#   one (the route whose mask has the most 1-bits).
#
# Example routing table:
#
#   Network      Mask             Gateway     Iface
#   10.0.0.0     255.255.255.0    10.0.0.1    eth0
#   0.0.0.0      0.0.0.0         192.168.1.1  eth1   ← default route

package CodingAdventures::NetworkStack::RoutingTable;

sub new {
    my ($class) = @_;
    return bless { routes => [] }, $class;
}

sub add_route {
    my ($self, $network, $mask, $gateway, $iface) = @_;
    push @{ $self->{routes} }, {
        network => $network,
        mask    => $mask,
        gateway => $gateway,
        iface   => $iface,
    };
    return $self;
}

sub lookup {
    my ($self, $dst_ip) = @_;
    my $best       = undef;
    my $best_bits  = -1;

    for my $entry (@{ $self->{routes} }) {
        if (_matches($dst_ip, $entry)) {
            my $bits = _count_mask_bits($entry->{mask});
            if ($bits > $best_bits) {
                $best_bits = $bits;
                $best      = $entry;
            }
        }
    }
    return $best;  # undef if no route found
}

sub _matches {
    my ($dst_ip, $entry) = @_;
    my @d = @$dst_ip;
    my @m = @{ $entry->{mask}    };
    my @n = @{ $entry->{network} };
    for my $i (0..3) {
        return 0 if (($d[$i] & $m[$i]) != $n[$i]);
    }
    return 1;
}

sub _count_mask_bits {
    my ($mask) = @_;
    my $bits = 0;
    for my $byte (@$mask) {
        my $b = $byte;
        while ($b) {
            $bits += $b & 1;
            $b >>= 1;
        }
    }
    return $bits;
}

# ============================================================================
# IPLayer — Layer 3: The Routing Engine
# ============================================================================
#
# Sits between Ethernet (below) and TCP/UDP (above). Builds IP packets with
# correct headers and checksums, and parses incoming packets.

package CodingAdventures::NetworkStack::IPLayer;

sub new {
    my ($class, %opts) = @_;
    return bless {
        local_ip      => $opts{local_ip},
        routing_table => CodingAdventures::NetworkStack::RoutingTable->new(),
        arp_table     => CodingAdventures::NetworkStack::ARPTable->new(),
    }, $class;
}

sub create_packet {
    my ($self, $dst_ip, $protocol, $payload) = @_;
    my $total_length = 20 + scalar(@$payload);
    my $hdr = CodingAdventures::NetworkStack::IPv4Header->new(
        src_ip       => $self->{local_ip},
        dst_ip       => $dst_ip,
        protocol     => $protocol,
        total_length => $total_length,
    );
    my $checksum = $hdr->compute_checksum();
    $hdr->{header_checksum} = $checksum;
    return [ @{ $hdr->serialize() }, @$payload ];
}

sub parse_packet {
    my ($class, $bytes) = @_;
    my @hdr_bytes = @{$bytes}[0..19];
    my $hdr = CodingAdventures::NetworkStack::IPv4Header->deserialize(\@hdr_bytes);
    return ('error', undef, undef, undef) unless $hdr->verify_checksum();
    my @payload = @{$bytes}[20..$#$bytes];
    return ('ok', $hdr->{src_ip}, $hdr->{protocol}, \@payload);
}

# ============================================================================
# TCPSegment — Layer 4 TCP Header (20 bytes)
# ============================================================================
#
# Carries everything needed for reliable delivery:
#
#   Bytes 0-1:   Source port
#   Bytes 2-3:   Destination port
#   Bytes 4-7:   Sequence number (32-bit, big-endian)
#   Bytes 8-11:  Acknowledgment number (32-bit, big-endian)
#   Byte 12:     Data offset (upper 4 bits, lower 4 reserved)
#   Byte 13:     Flags (FIN|SYN|RST|PSH|ACK|URG)
#   Bytes 14-15: Window size
#   Bytes 16-17: Checksum (0 in simulation)
#   Bytes 18-19: Urgent pointer (0)

package CodingAdventures::NetworkStack::TCPSegment;

sub new {
    my ($class, %opts) = @_;
    return bless {
        src_port    => $opts{src_port}    // 0,
        dst_port    => $opts{dst_port}    // 0,
        seq_num     => $opts{seq_num}     // 0,
        ack_num     => $opts{ack_num}     // 0,
        data_offset => $opts{data_offset} // 5,
        flags       => $opts{flags}       // 0,
        window_size => $opts{window_size} // 65535,
        payload     => $opts{payload}     // [],
    }, $class;
}

sub has_flag {
    my ($self, $flag) = @_;
    return ($self->{flags} & $flag) != 0;
}

sub serialize {
    my ($self) = @_;
    my $sp  = $self->{src_port};
    my $dp  = $self->{dst_port};
    my $seq = $self->{seq_num};
    my $ack = $self->{ack_num};
    my $ws  = $self->{window_size};
    my @hdr = (
        ($sp  >> 8) & 0xFF,  $sp  & 0xFF,
        ($dp  >> 8) & 0xFF,  $dp  & 0xFF,
        ($seq >> 24) & 0xFF, ($seq >> 16) & 0xFF,
        ($seq >>  8) & 0xFF,  $seq         & 0xFF,
        ($ack >> 24) & 0xFF, ($ack >> 16) & 0xFF,
        ($ack >>  8) & 0xFF,  $ack         & 0xFF,
        ($self->{data_offset} << 4) & 0xF0,
        $self->{flags} & 0xFF,
        ($ws >> 8) & 0xFF,   $ws & 0xFF,
        0, 0,   # checksum
        0, 0,   # urgent pointer
    );
    return [ @hdr, @{ $self->{payload} } ];
}

sub deserialize {
    my ($class, $bytes) = @_;
    my @b = @$bytes;
    my $src_port    = ($b[0]  << 8)  | $b[1];
    my $dst_port    = ($b[2]  << 8)  | $b[3];
    my $seq_num     = ($b[4]  << 24) | ($b[5] << 16) | ($b[6] << 8) | $b[7];
    my $ack_num     = ($b[8]  << 24) | ($b[9] << 16) | ($b[10]<< 8) | $b[11];
    my $data_offset = ($b[12] >> 4) & 0x0F;
    my $flags       =  $b[13];
    my $window_size = ($b[14] << 8) | $b[15];
    my @payload     = @b[20..$#b];
    return $class->new(
        src_port    => $src_port,
        dst_port    => $dst_port,
        seq_num     => $seq_num,
        ack_num     => $ack_num,
        data_offset => $data_offset,
        flags       => $flags,
        window_size => $window_size,
        payload     => \@payload,
    );
}

# ============================================================================
# UDPDatagram — Layer 4 UDP Header (8 bytes)
# ============================================================================
#
# No handshake, no acknowledgments, no ordering. Just source port,
# destination port, length, and optional checksum.
#
#   Bytes 0-1: Source port
#   Bytes 2-3: Destination port
#   Bytes 4-5: Length (header + data)
#   Bytes 6-7: Checksum (0 in simulation)

package CodingAdventures::NetworkStack::UDPDatagram;

sub new {
    my ($class, %opts) = @_;
    my $payload = $opts{payload} // [];
    return bless {
        src_port => $opts{src_port} // 0,
        dst_port => $opts{dst_port} // 0,
        length   => $opts{length}   // (8 + scalar(@$payload)),
        checksum => $opts{checksum} // 0,
        payload  => $payload,
    }, $class;
}

sub serialize {
    my ($self) = @_;
    my $sp = $self->{src_port};
    my $dp = $self->{dst_port};
    my $ln = $self->{length};
    my $cs = $self->{checksum};
    return [
        ($sp >> 8) & 0xFF, $sp & 0xFF,
        ($dp >> 8) & 0xFF, $dp & 0xFF,
        ($ln >> 8) & 0xFF, $ln & 0xFF,
        ($cs >> 8) & 0xFF, $cs & 0xFF,
        @{ $self->{payload} },
    ];
}

sub deserialize {
    my ($class, $bytes) = @_;
    my @b       = @$bytes;
    my $src_port = ($b[0] << 8) | $b[1];
    my $dst_port = ($b[2] << 8) | $b[3];
    my $length   = ($b[4] << 8) | $b[5];
    my $checksum = ($b[6] << 8) | $b[7];
    my @payload  = @b[8..$#b];
    return $class->new(
        src_port => $src_port,
        dst_port => $dst_port,
        length   => $length,
        checksum => $checksum,
        payload  => \@payload,
    );
}

# ============================================================================
# TCPConnection — The State Machine
# ============================================================================
#
# Manages the full lifecycle of a single TCP connection.
#
# State diagram (simplified):
#
#   CLOSED  → (listen)  → LISTEN      → (SYN)     → SYN_RECEIVED → (ACK) → ESTABLISHED
#   CLOSED  → (connect) → SYN_SENT    → (SYN+ACK) → ESTABLISHED
#   ESTABLISHED → (FIN) → CLOSE_WAIT  → (send FIN) → LAST_ACK → (ACK) → CLOSED
#   ESTABLISHED → (initiate close) → FIN_WAIT_1 → (ACK) → FIN_WAIT_2
#                                                → (FIN) → TIME_WAIT → CLOSED

package CodingAdventures::NetworkStack::TCPConnection;

sub new {
    my ($class, %opts) = @_;
    return bless {
        state       => CodingAdventures::NetworkStack::TCP_CLOSED,
        local_port  => $opts{local_port}  // 0,
        remote_port => $opts{remote_port} // 0,
        local_ip    => $opts{local_ip}    // '0.0.0.0',
        remote_ip   => $opts{remote_ip}   // '0.0.0.0',
        send_seq    => 0,
        recv_next   => 0,
        send_buffer => [],
        recv_buffer => [],
    }, $class;
}

sub set_listen {
    my ($self) = @_;
    $self->{state} = CodingAdventures::NetworkStack::TCP_LISTEN;
    return $self;
}

# Initiate an outbound connection: generate ISN, send SYN, → SYN_SENT
sub initiate_connect {
    my ($self) = @_;
    return ($self, undef) unless $self->{state} eq CodingAdventures::NetworkStack::TCP_CLOSED;
    $self->{send_seq} = 1000;
    $self->{state}    = CodingAdventures::NetworkStack::TCP_SYN_SENT;
    my $syn = CodingAdventures::NetworkStack::TCPSegment->new(
        src_port => $self->{local_port},
        dst_port => $self->{remote_port},
        seq_num  => $self->{send_seq},
        flags    => CodingAdventures::NetworkStack::TCP_SYN,
    );
    return ($self, $syn);
}

# Handle an incoming segment — the core TCP state machine
sub handle_segment {
    my ($self, $seg) = @_;
    my $state = $self->{state};

    if ($state eq CodingAdventures::NetworkStack::TCP_LISTEN) {
        if ($seg->has_flag(CodingAdventures::NetworkStack::TCP_SYN)) {
            $self->{remote_port} = $seg->{src_port};
            $self->{recv_next}   = $seg->{seq_num} + 1;
            $self->{send_seq}    = 3000;
            $self->{state}       = CodingAdventures::NetworkStack::TCP_SYN_RECEIVED;
            my $response = CodingAdventures::NetworkStack::TCPSegment->new(
                src_port => $self->{local_port},
                dst_port => $self->{remote_port},
                seq_num  => $self->{send_seq},
                ack_num  => $self->{recv_next},
                flags    => CodingAdventures::NetworkStack::TCP_SYN |
                            CodingAdventures::NetworkStack::TCP_ACK,
            );
            return ($self, $response);
        }
    }

    elsif ($state eq CodingAdventures::NetworkStack::TCP_SYN_SENT) {
        if ($seg->has_flag(CodingAdventures::NetworkStack::TCP_SYN) &&
            $seg->has_flag(CodingAdventures::NetworkStack::TCP_ACK)) {
            $self->{recv_next} = $seg->{seq_num} + 1;
            $self->{send_seq}  = $self->{send_seq} + 1;
            $self->{state}     = CodingAdventures::NetworkStack::TCP_ESTABLISHED;
            my $response = CodingAdventures::NetworkStack::TCPSegment->new(
                src_port => $self->{local_port},
                dst_port => $self->{remote_port},
                seq_num  => $self->{send_seq},
                ack_num  => $self->{recv_next},
                flags    => CodingAdventures::NetworkStack::TCP_ACK,
            );
            return ($self, $response);
        }
    }

    elsif ($state eq CodingAdventures::NetworkStack::TCP_SYN_RECEIVED) {
        if ($seg->has_flag(CodingAdventures::NetworkStack::TCP_ACK)) {
            $self->{send_seq} = $self->{send_seq} + 1;
            $self->{state}    = CodingAdventures::NetworkStack::TCP_ESTABLISHED;
            return ($self, undef);
        }
    }

    elsif ($state eq CodingAdventures::NetworkStack::TCP_ESTABLISHED) {
        if ($seg->has_flag(CodingAdventures::NetworkStack::TCP_FIN)) {
            $self->{recv_next} = $seg->{seq_num} + 1;
            $self->{state}     = CodingAdventures::NetworkStack::TCP_CLOSE_WAIT;
            my $response = CodingAdventures::NetworkStack::TCPSegment->new(
                src_port => $self->{local_port},
                dst_port => $self->{remote_port},
                seq_num  => $self->{send_seq},
                ack_num  => $self->{recv_next},
                flags    => CodingAdventures::NetworkStack::TCP_ACK,
            );
            return ($self, $response);
        }
        elsif (@{ $seg->{payload} }) {
            push @{ $self->{recv_buffer} }, @{ $seg->{payload} };
            $self->{recv_next} = $seg->{seq_num} + scalar(@{ $seg->{payload} });
            my $response = CodingAdventures::NetworkStack::TCPSegment->new(
                src_port => $self->{local_port},
                dst_port => $self->{remote_port},
                seq_num  => $self->{send_seq},
                ack_num  => $self->{recv_next},
                flags    => CodingAdventures::NetworkStack::TCP_ACK,
            );
            return ($self, $response);
        }
    }

    elsif ($state eq CodingAdventures::NetworkStack::TCP_FIN_WAIT_1) {
        if ($seg->has_flag(CodingAdventures::NetworkStack::TCP_ACK)) {
            $self->{state} = CodingAdventures::NetworkStack::TCP_FIN_WAIT_2;
            return ($self, undef);
        }
    }

    elsif ($state eq CodingAdventures::NetworkStack::TCP_FIN_WAIT_2) {
        if ($seg->has_flag(CodingAdventures::NetworkStack::TCP_FIN)) {
            $self->{recv_next} = $seg->{seq_num} + 1;
            $self->{state}     = CodingAdventures::NetworkStack::TCP_TIME_WAIT;
            my $response = CodingAdventures::NetworkStack::TCPSegment->new(
                src_port => $self->{local_port},
                dst_port => $self->{remote_port},
                seq_num  => $self->{send_seq},
                ack_num  => $self->{recv_next},
                flags    => CodingAdventures::NetworkStack::TCP_ACK,
            );
            return ($self, $response);
        }
    }

    elsif ($state eq CodingAdventures::NetworkStack::TCP_LAST_ACK) {
        if ($seg->has_flag(CodingAdventures::NetworkStack::TCP_ACK)) {
            $self->{state} = CodingAdventures::NetworkStack::TCP_CLOSED;
            return ($self, undef);
        }
    }

    return ($self, undef);
}

# Send data over an established connection
sub send_data {
    my ($self, $data) = @_;
    return ($self, undef, [])
        unless $self->{state} eq CodingAdventures::NetworkStack::TCP_ESTABLISHED;
    my $header = CodingAdventures::NetworkStack::TCPSegment->new(
        src_port => $self->{local_port},
        dst_port => $self->{remote_port},
        seq_num  => $self->{send_seq},
        ack_num  => $self->{recv_next},
        flags    => CodingAdventures::NetworkStack::TCP_PSH |
                    CodingAdventures::NetworkStack::TCP_ACK,
    );
    push @{ $self->{send_buffer} }, @$data;
    $self->{send_seq} += scalar(@$data);
    return ($self, $header, $data);
}

# Read all buffered received data and clear the buffer
sub recv_data {
    my ($self) = @_;
    my @data = @{ $self->{recv_buffer} };
    $self->{recv_buffer} = [];
    return (\@data, $self);
}

# Initiate connection teardown by sending FIN
sub initiate_close {
    my ($self) = @_;
    my $state = $self->{state};

    if ($state eq CodingAdventures::NetworkStack::TCP_ESTABLISHED) {
        my $header = CodingAdventures::NetworkStack::TCPSegment->new(
            src_port => $self->{local_port},
            dst_port => $self->{remote_port},
            seq_num  => $self->{send_seq},
            ack_num  => $self->{recv_next},
            flags    => CodingAdventures::NetworkStack::TCP_FIN |
                        CodingAdventures::NetworkStack::TCP_ACK,
        );
        $self->{state} = CodingAdventures::NetworkStack::TCP_FIN_WAIT_1;
        return ($self, $header);
    }
    elsif ($state eq CodingAdventures::NetworkStack::TCP_CLOSE_WAIT) {
        my $header = CodingAdventures::NetworkStack::TCPSegment->new(
            src_port => $self->{local_port},
            dst_port => $self->{remote_port},
            seq_num  => $self->{send_seq},
            ack_num  => $self->{recv_next},
            flags    => CodingAdventures::NetworkStack::TCP_FIN |
                        CodingAdventures::NetworkStack::TCP_ACK,
        );
        $self->{state} = CodingAdventures::NetworkStack::TCP_LAST_ACK;
        return ($self, $header);
    }

    return ($self, undef);
}

# ============================================================================
# NetworkStack — Top-Level Facade
# ============================================================================
#
# Composes all layers.  Provides send_udp / send_tcp / receive.

package CodingAdventures::NetworkStack::NetworkStack;

sub new {
    my ($class, %opts) = @_;
    return bless {
        local_ip    => $opts{local_ip},
        local_mac   => $opts{local_mac},
        arp_table   => CodingAdventures::NetworkStack::ARPTable->new(),
        routing_table => CodingAdventures::NetworkStack::RoutingTable->new(),
    }, $class;
}

sub add_route {
    my ($self, $network, $mask, $gateway, $iface) = @_;
    $self->{routing_table}->add_route($network, $mask, $gateway, $iface);
    return $self;
}

sub add_arp {
    my ($self, $ip, $mac) = @_;
    $self->{arp_table}->insert($ip, $mac);
    return $self;
}

sub send_udp {
    my ($self, $dst_ip, $dst_mac, $src_port, $dst_port, $data) = @_;
    my $udp = CodingAdventures::NetworkStack::UDPDatagram->new(
        src_port => $src_port,
        dst_port => $dst_port,
        payload  => $data,
    );
    my $udp_bytes = $udp->serialize();
    my $ip_layer  = CodingAdventures::NetworkStack::IPLayer->new(local_ip => $self->{local_ip});
    my $ip_bytes  = $ip_layer->create_packet($dst_ip, CodingAdventures::NetworkStack::PROTO_UDP, $udp_bytes);
    my $eth = CodingAdventures::NetworkStack::EthernetFrame->new(
        dest_mac   => $dst_mac,
        src_mac    => $self->{local_mac},
        ether_type => CodingAdventures::NetworkStack::ETHERTYPE_IPV4,
        payload    => $ip_bytes,
    );
    return $eth->serialize();
}

sub send_tcp {
    my ($self, $dst_ip, $dst_mac, $src_port, $dst_port, $seq, $ack, $flags, $data) = @_;
    my $tcp = CodingAdventures::NetworkStack::TCPSegment->new(
        src_port => $src_port,
        dst_port => $dst_port,
        seq_num  => $seq,
        ack_num  => $ack,
        flags    => $flags,
        payload  => $data,
    );
    my $tcp_bytes = $tcp->serialize();
    my $ip_layer  = CodingAdventures::NetworkStack::IPLayer->new(local_ip => $self->{local_ip});
    my $ip_bytes  = $ip_layer->create_packet($dst_ip, CodingAdventures::NetworkStack::PROTO_TCP, $tcp_bytes);
    my $eth = CodingAdventures::NetworkStack::EthernetFrame->new(
        dest_mac   => $dst_mac,
        src_mac    => $self->{local_mac},
        ether_type => CodingAdventures::NetworkStack::ETHERTYPE_IPV4,
        payload    => $ip_bytes,
    );
    return $eth->serialize();
}

# Parse an incoming wire frame.  Returns a hashref with parsed layers.
sub receive {
    my ($self, $wire_bytes) = @_;
    my $frame = CodingAdventures::NetworkStack::EthernetFrame->deserialize($wire_bytes);
    my $result = {
        ether_type => $frame->{ether_type},
        dest_mac   => $frame->{dest_mac},
        src_mac    => $frame->{src_mac},
    };

    if ($frame->{ether_type} == CodingAdventures::NetworkStack::ETHERTYPE_IPV4) {
        my ($status, $src_ip, $proto, $payload) =
            CodingAdventures::NetworkStack::IPLayer->parse_packet($frame->{payload});
        if ($status eq 'ok') {
            $result->{ip_status} = 'ok';
            $result->{src_ip}    = $src_ip;
            $result->{protocol}  = $proto;
            if ($proto == CodingAdventures::NetworkStack::PROTO_UDP) {
                my $udp = CodingAdventures::NetworkStack::UDPDatagram->deserialize($payload);
                $result->{src_port} = $udp->{src_port};
                $result->{dst_port} = $udp->{dst_port};
                $result->{data}     = $udp->{payload};
            }
            elsif ($proto == CodingAdventures::NetworkStack::PROTO_TCP) {
                my $tcp = CodingAdventures::NetworkStack::TCPSegment->deserialize($payload);
                $result->{src_port}  = $tcp->{src_port};
                $result->{dst_port}  = $tcp->{dst_port};
                $result->{tcp_flags} = $tcp->{flags};
                $result->{data}      = $tcp->{payload};
            }
        } else {
            $result->{ip_status} = 'checksum_error';
        }
    }

    return $result;
}

# ============================================================================
# Top-level package
# ============================================================================

package CodingAdventures::NetworkStack;

=head1 NAME

CodingAdventures::NetworkStack - Layered TCP/IP network protocol stack

=head1 SYNOPSIS

  use CodingAdventures::NetworkStack;

  my $stack = CodingAdventures::NetworkStack::NetworkStack->new(
      local_ip  => [192, 168, 1, 10],
      local_mac => [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF],
  );
  $stack->add_arp([192, 168, 1, 1], [0x11, 0x22, 0x33, 0x44, 0x55, 0x66]);
  my $wire = $stack->send_udp(
      [192, 168, 1, 1],  # dst IP
      [0x11, 0x22, 0x33, 0x44, 0x55, 0x66], # dst MAC
      5000, 53,          # src_port, dst_port
      [0xDE, 0xAD],      # payload
  );

=cut

1;
