use strict;
use warnings;
use Test2::V0;
use lib 'lib';

use CodingAdventures::NetworkStack;

my $EF  = 'CodingAdventures::NetworkStack::EthernetFrame';
my $ARP = 'CodingAdventures::NetworkStack::ARPTable';
my $IP  = 'CodingAdventures::NetworkStack::IPv4Header';
my $RT  = 'CodingAdventures::NetworkStack::RoutingTable';
my $IPL = 'CodingAdventures::NetworkStack::IPLayer';
my $TCP = 'CodingAdventures::NetworkStack::TCPSegment';
my $UDP = 'CodingAdventures::NetworkStack::UDPDatagram';
my $TCN = 'CodingAdventures::NetworkStack::TCPConnection';
my $NS  = 'CodingAdventures::NetworkStack::NetworkStack';

# ------------------------------------------------------------------ constants

subtest 'constants are correct' => sub {
    is(CodingAdventures::NetworkStack::PROTO_TCP,      6,      'PROTO_TCP');
    is(CodingAdventures::NetworkStack::PROTO_UDP,      17,     'PROTO_UDP');
    is(CodingAdventures::NetworkStack::ETHERTYPE_IPV4, 0x0800, 'ETHERTYPE_IPV4');
    is(CodingAdventures::NetworkStack::ETHERTYPE_ARP,  0x0806, 'ETHERTYPE_ARP');
    is(CodingAdventures::NetworkStack::TCP_SYN, 0x02, 'TCP_SYN');
    is(CodingAdventures::NetworkStack::TCP_ACK, 0x10, 'TCP_ACK');
    is(CodingAdventures::NetworkStack::TCP_FIN, 0x01, 'TCP_FIN');
    is(CodingAdventures::NetworkStack::TCP_PSH, 0x08, 'TCP_PSH');
    is(CodingAdventures::NetworkStack::TCP_RST, 0x04, 'TCP_RST');
};

# ------------------------------------------------------------------ EthernetFrame

subtest 'EthernetFrame — new' => sub {
    my $f = $EF->new(
        dest_mac   => [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
        src_mac    => [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF],
        ether_type => 0x0800,
        payload    => [1, 2, 3],
    );
    is($f->{ether_type}, 0x0800, 'ether_type stored');
    is($f->{payload},    [1,2,3], 'payload stored');
};

subtest 'EthernetFrame — serialize' => sub {
    my $f = $EF->new(
        dest_mac   => [0x01, 0x02, 0x03, 0x04, 0x05, 0x06],
        src_mac    => [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF],
        ether_type => 0x0800,
        payload    => [0x45, 0x00],
    );
    my $bytes = $f->serialize();
    is(scalar(@$bytes), 16, '14-byte header + 2-byte payload = 16');
    # dest MAC is bytes 0..5
    is($bytes->[0], 0x01, 'dest MAC byte 0');
    is($bytes->[5], 0x06, 'dest MAC byte 5');
    # src MAC is bytes 6..11
    is($bytes->[6], 0xAA, 'src MAC byte 0');
    # EtherType big-endian at bytes 12..13
    is($bytes->[12], 0x08, 'EtherType high byte');
    is($bytes->[13], 0x00, 'EtherType low byte');
    # payload starts at byte 14
    is($bytes->[14], 0x45, 'payload byte 0');
};

subtest 'EthernetFrame — deserialize round-trip' => sub {
    my $original = $EF->new(
        dest_mac   => [0x11, 0x22, 0x33, 0x44, 0x55, 0x66],
        src_mac    => [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF],
        ether_type => 0x0806,
        payload    => [10, 20, 30],
    );
    my $bytes = $original->serialize();
    my $parsed = $EF->deserialize($bytes);
    is($parsed->{ether_type}, 0x0806, 'ether_type round-trips');
    is($parsed->{dest_mac},   [0x11, 0x22, 0x33, 0x44, 0x55, 0x66], 'dest_mac round-trips');
    is($parsed->{src_mac},    [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF], 'src_mac round-trips');
    is($parsed->{payload},    [10, 20, 30], 'payload round-trips');
};

# ------------------------------------------------------------------ ARPTable

subtest 'ARPTable — insert and lookup' => sub {
    my $arp = $ARP->new();
    is($arp->size(), 0, 'initially empty');
    is($arp->lookup('192.168.1.1'), undef, 'unknown IP returns undef');

    $arp->insert('192.168.1.1', [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);
    is($arp->size(), 1, 'one entry');
    is($arp->lookup('192.168.1.1'), [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF], 'lookup works');
};

subtest 'ARPTable — overwrite existing entry' => sub {
    my $arp = $ARP->new();
    $arp->insert('10.0.0.1', [0x11, 0x22, 0x33, 0x44, 0x55, 0x66]);
    $arp->insert('10.0.0.1', [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);
    is($arp->size(), 1, 'still one entry after overwrite');
    is($arp->lookup('10.0.0.1'), [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF], 'updated entry returned');
};

# ------------------------------------------------------------------ IPv4Header

subtest 'IPv4Header — serialize is 20 bytes' => sub {
    my $hdr = $IP->new(
        src_ip       => [192, 168, 1, 10],
        dst_ip       => [8, 8, 8, 8],
        protocol     => 17,
        total_length => 40,
    );
    my $bytes = $hdr->serialize();
    is(scalar(@$bytes), 20, 'exactly 20 bytes');
    is($bytes->[0], 0x45, 'version=4, IHL=5 → 0x45');
    is($bytes->[9], 17,   'protocol field');
    is($bytes->[12], 192, 'src IP byte 0');
    is($bytes->[16], 8,   'dst IP byte 0');
};

subtest 'IPv4Header — deserialize round-trip' => sub {
    my $orig = $IP->new(
        src_ip       => [10, 0, 0, 1],
        dst_ip       => [10, 0, 0, 2],
        protocol     => 6,
        total_length => 60,
        ttl          => 128,
    );
    my $bytes  = $orig->serialize();
    my $parsed = $IP->deserialize($bytes);
    is($parsed->{src_ip},       [10, 0, 0, 1], 'src_ip round-trips');
    is($parsed->{dst_ip},       [10, 0, 0, 2], 'dst_ip round-trips');
    is($parsed->{protocol},     6,   'protocol round-trips');
    is($parsed->{total_length}, 60,  'total_length round-trips');
    is($parsed->{ttl},          128, 'ttl round-trips');
};

subtest 'IPv4Header — checksum compute and verify' => sub {
    my $hdr = $IP->new(
        src_ip       => [192, 168, 1, 1],
        dst_ip       => [192, 168, 1, 2],
        protocol     => 17,
        total_length => 28,
    );
    my $cksum = $hdr->compute_checksum();
    ok($cksum != 0, 'checksum non-zero');

    $hdr->{header_checksum} = $cksum;
    ok($hdr->verify_checksum(), 'verify_checksum returns true on valid header');
};

subtest 'IPv4Header — corrupted checksum fails verify' => sub {
    my $hdr = $IP->new(
        src_ip       => [10, 0, 0, 1],
        dst_ip       => [10, 0, 0, 2],
        protocol     => 6,
        total_length => 40,
    );
    my $cksum = $hdr->compute_checksum();
    $hdr->{header_checksum} = $cksum ^ 0xFFFF;  # corrupt it
    ok(!$hdr->verify_checksum(), 'corrupted checksum fails verify');
};

# ------------------------------------------------------------------ RoutingTable

subtest 'RoutingTable — no routes → undef' => sub {
    my $rt = $RT->new();
    is($rt->lookup([192, 168, 1, 5]), undef, 'no routes returns undef');
};

subtest 'RoutingTable — single route match' => sub {
    my $rt = $RT->new();
    $rt->add_route([192, 168, 1, 0], [255, 255, 255, 0], [192, 168, 1, 1], 'eth0');
    my $entry = $rt->lookup([192, 168, 1, 5]);
    ok(defined $entry, 'route found');
    is($entry->{iface}, 'eth0', 'correct interface');
};

subtest 'RoutingTable — no match' => sub {
    my $rt = $RT->new();
    $rt->add_route([192, 168, 1, 0], [255, 255, 255, 0], [192, 168, 1, 1], 'eth0');
    is($rt->lookup([10, 0, 0, 1]), undef, 'different subnet → no match');
};

subtest 'RoutingTable — longest prefix match' => sub {
    my $rt = $RT->new();
    # Default route
    $rt->add_route([0, 0, 0, 0], [0, 0, 0, 0], [192, 168, 1, 1], 'eth0');
    # More specific route
    $rt->add_route([10, 0, 0, 0], [255, 0, 0, 0], [10, 0, 0, 254], 'eth1');
    # Even more specific
    $rt->add_route([10, 0, 1, 0], [255, 255, 255, 0], [10, 0, 1, 254], 'eth2');

    my $e1 = $rt->lookup([10, 0, 1, 5]);
    is($e1->{iface}, 'eth2', 'most specific /24 wins');

    my $e2 = $rt->lookup([10, 0, 2, 5]);
    is($e2->{iface}, 'eth1', '/8 wins over default');

    my $e3 = $rt->lookup([8, 8, 8, 8]);
    is($e3->{iface}, 'eth0', 'default route used');
};

# ------------------------------------------------------------------ IPLayer

subtest 'IPLayer — create_packet returns valid IP bytes' => sub {
    my $layer = $IPL->new(local_ip => [192, 168, 1, 10]);
    my $payload = [0xDE, 0xAD, 0xBE, 0xEF];
    my $packet  = $layer->create_packet([8, 8, 8, 8], 17, $payload);
    is(scalar(@$packet), 24, '20 hdr + 4 payload = 24 bytes');
    is($packet->[0], 0x45, 'version/IHL correct');
};

subtest 'IPLayer — parse_packet on valid data' => sub {
    my $layer   = $IPL->new(local_ip => [192, 168, 1, 10]);
    my $payload = [0xAA, 0xBB];
    my $packet  = $layer->create_packet([10, 0, 0, 1], 17, $payload);
    my ($st, $src, $proto, $pl) = $IPL->parse_packet($packet);
    is($st,    'ok', 'parse succeeds');
    is($src,   [192, 168, 1, 10], 'src IP correct');
    is($proto, 17, 'protocol correct');
    is($pl,    [0xAA, 0xBB], 'payload correct');
};

subtest 'IPLayer — parse_packet with bad checksum returns error' => sub {
    my $layer  = $IPL->new(local_ip => [1, 2, 3, 4]);
    my $packet = $layer->create_packet([5, 6, 7, 8], 6, [0x01]);
    $packet->[10] ^= 0xFF;  # corrupt checksum
    my ($st) = $IPL->parse_packet($packet);
    is($st, 'error', 'bad checksum → error');
};

# ------------------------------------------------------------------ UDPDatagram

subtest 'UDPDatagram — new and serialize' => sub {
    my $udp = $UDP->new(
        src_port => 12345,
        dst_port => 53,
        payload  => [0xDE, 0xAD],
    );
    is($udp->{length}, 10, 'length = 8 header + 2 payload');
    my $bytes = $udp->serialize();
    is(scalar(@$bytes), 10, '10 bytes total');
    is($bytes->[0], (12345 >> 8) & 0xFF, 'src port high byte');
    is($bytes->[1], 12345 & 0xFF,        'src port low byte');
    is($bytes->[2], 0, 'dst port high byte (53 = 0x0035)');
    is($bytes->[3], 53, 'dst port low byte');
};

subtest 'UDPDatagram — deserialize round-trip' => sub {
    my $orig  = $UDP->new(src_port => 5000, dst_port => 80, payload => [1, 2, 3]);
    my $bytes = $orig->serialize();
    my $parsed = $UDP->deserialize($bytes);
    is($parsed->{src_port}, 5000, 'src_port round-trips');
    is($parsed->{dst_port}, 80,   'dst_port round-trips');
    is($parsed->{length},   11,   'length round-trips');
    is($parsed->{payload},  [1,2,3], 'payload round-trips');
};

# ------------------------------------------------------------------ TCPSegment

subtest 'TCPSegment — new and flags' => sub {
    my $seg = $TCP->new(
        src_port => 1024,
        dst_port => 80,
        seq_num  => 100,
        ack_num  => 0,
        flags    => CodingAdventures::NetworkStack::TCP_SYN,
    );
    ok($seg->has_flag(CodingAdventures::NetworkStack::TCP_SYN), 'SYN flag set');
    ok(!$seg->has_flag(CodingAdventures::NetworkStack::TCP_ACK), 'ACK not set');
};

subtest 'TCPSegment — serialize is 20+ bytes' => sub {
    my $seg = $TCP->new(
        src_port => 1024, dst_port => 80,
        seq_num  => 0,    ack_num  => 0,
        flags    => CodingAdventures::NetworkStack::TCP_SYN,
        payload  => [0xAA],
    );
    my $bytes = $seg->serialize();
    is(scalar(@$bytes), 21, '20 hdr + 1 payload = 21');
    is($bytes->[13], CodingAdventures::NetworkStack::TCP_SYN, 'flags byte correct');
};

subtest 'TCPSegment — deserialize round-trip' => sub {
    my $orig = $TCP->new(
        src_port => 9999,
        dst_port => 443,
        seq_num  => 0xDEADBEEF,
        ack_num  => 0x12345678,
        flags    => CodingAdventures::NetworkStack::TCP_ACK |
                    CodingAdventures::NetworkStack::TCP_PSH,
        window_size => 8192,
        payload  => [0x48, 0x69],
    );
    my $bytes  = $orig->serialize();
    my $parsed = $TCP->deserialize($bytes);
    is($parsed->{src_port},    9999,         'src_port round-trips');
    is($parsed->{dst_port},    443,          'dst_port round-trips');
    is($parsed->{seq_num},     0xDEADBEEF,   'seq_num round-trips');
    is($parsed->{ack_num},     0x12345678,   'ack_num round-trips');
    is($parsed->{window_size}, 8192,         'window_size round-trips');
    ok($parsed->has_flag(CodingAdventures::NetworkStack::TCP_ACK), 'ACK flag survives');
    ok($parsed->has_flag(CodingAdventures::NetworkStack::TCP_PSH), 'PSH flag survives');
    is($parsed->{payload}, [0x48, 0x69], 'payload round-trips');
};

# ------------------------------------------------------------------ TCPConnection state machine

subtest 'TCPConnection — initial state is closed' => sub {
    my $conn = $TCN->new(local_port => 80, remote_port => 12345);
    is($conn->{state}, 'closed', 'starts in CLOSED');
};

subtest 'TCPConnection — set_listen' => sub {
    my $conn = $TCN->new(local_port => 80);
    $conn->set_listen();
    is($conn->{state}, 'listen', 'transitions to LISTEN');
};

subtest 'TCPConnection — active open: CLOSED → SYN_SENT' => sub {
    my $conn = $TCN->new(local_port => 5000, remote_port => 80);
    my ($c, $syn) = $conn->initiate_connect();
    is($c->{state}, 'syn_sent', 'state → SYN_SENT');
    ok(defined $syn, 'SYN segment returned');
    ok($syn->has_flag(CodingAdventures::NetworkStack::TCP_SYN), 'segment has SYN');
    ok(!$syn->has_flag(CodingAdventures::NetworkStack::TCP_ACK), 'no ACK in first SYN');
};

subtest 'TCPConnection — passive open: LISTEN → SYN_RECEIVED on incoming SYN' => sub {
    my $conn = $TCN->new(local_port => 80);
    $conn->set_listen();
    my $syn_in = $TCP->new(
        src_port => 5000, dst_port => 80,
        seq_num  => 999,
        flags    => CodingAdventures::NetworkStack::TCP_SYN,
    );
    my ($c, $response) = $conn->handle_segment($syn_in);
    is($c->{state}, 'syn_received', 'state → SYN_RECEIVED');
    ok(defined $response, 'SYN+ACK response generated');
    ok($response->has_flag(CodingAdventures::NetworkStack::TCP_SYN), 'response has SYN');
    ok($response->has_flag(CodingAdventures::NetworkStack::TCP_ACK), 'response has ACK');
    is($c->{recv_next}, 1000, 'recv_next = peer_seq + 1');
};

subtest 'TCPConnection — three-way handshake (client side)' => sub {
    my $client = $TCN->new(local_port => 5000, remote_port => 80);
    # Step 1: client sends SYN
    my ($c, $syn) = $client->initiate_connect();
    is($c->{state}, 'syn_sent', 'step 1: SYN_SENT');

    # Step 2: server replies with SYN+ACK
    my $synack = $TCP->new(
        src_port => 80, dst_port => 5000,
        seq_num  => 3000, ack_num => $c->{send_seq} + 1,
        flags    => CodingAdventures::NetworkStack::TCP_SYN |
                    CodingAdventures::NetworkStack::TCP_ACK,
    );
    my ($c2, $ack) = $c->handle_segment($synack);
    is($c2->{state}, 'established', 'step 2: ESTABLISHED after SYN+ACK');
    ok(defined $ack, 'ACK response generated');
    ok($ack->has_flag(CodingAdventures::NetworkStack::TCP_ACK), 'response is ACK');
};

subtest 'TCPConnection — three-way handshake (server side)' => sub {
    my $server = $TCN->new(local_port => 80);
    $server->set_listen();

    # Server receives SYN
    my $syn = $TCP->new(src_port => 5000, dst_port => 80, seq_num => 1000,
                        flags => CodingAdventures::NetworkStack::TCP_SYN);
    my ($s, $synack) = $server->handle_segment($syn);
    is($s->{state}, 'syn_received', 'step 1: SYN_RECEIVED');

    # Server receives ACK from client
    my $ack = $TCP->new(src_port => 5000, dst_port => 80,
                        seq_num => 1001, ack_num => $s->{send_seq} + 1,
                        flags => CodingAdventures::NetworkStack::TCP_ACK);
    my ($s2, $resp) = $s->handle_segment($ack);
    is($s2->{state}, 'established', 'step 2: ESTABLISHED after client ACK');
    is($resp, undef, 'no response needed');
};

subtest 'TCPConnection — send_data and recv_data' => sub {
    my $conn = _make_established_conn();
    my ($c, $hdr, $data) = $conn->send_data([0x68, 0x69]);  # "hi"
    ok(defined $hdr, 'header generated');
    ok($hdr->has_flag(CodingAdventures::NetworkStack::TCP_PSH), 'PSH set');
    ok($hdr->has_flag(CodingAdventures::NetworkStack::TCP_ACK), 'ACK set');
    is($data, [0x68, 0x69], 'data returned');

    # Receive incoming data
    my $seg = $TCP->new(src_port => 80, dst_port => 5000,
                        seq_num => 0, flags => CodingAdventures::NetworkStack::TCP_ACK,
                        payload => [0x48, 0x65, 0x6C, 0x6C, 0x6F]);  # "Hello"
    my ($c2, $ack) = $c->handle_segment($seg);
    my ($rdata, $c3) = $c2->recv_data();
    is($rdata, [0x48, 0x65, 0x6C, 0x6C, 0x6F], 'received data buffered');
    is(scalar(@{ $c3->{recv_buffer} }), 0, 'buffer cleared after recv_data');
};

subtest 'TCPConnection — connection teardown (active close)' => sub {
    my $conn = _make_established_conn();
    # initiate close → FIN_WAIT_1
    my ($c, $fin) = $conn->initiate_close();
    is($c->{state}, 'fin_wait_1', 'state → FIN_WAIT_1');
    ok($fin->has_flag(CodingAdventures::NetworkStack::TCP_FIN), 'FIN set');

    # peer ACKs our FIN → FIN_WAIT_2
    my $ack = $TCP->new(flags => CodingAdventures::NetworkStack::TCP_ACK,
                        src_port => 80, dst_port => 5000, seq_num => 0, ack_num => 0);
    my ($c2, $r2) = $c->handle_segment($ack);
    is($c2->{state}, 'fin_wait_2', 'state → FIN_WAIT_2');

    # peer sends FIN → TIME_WAIT
    my $peer_fin = $TCP->new(flags => CodingAdventures::NetworkStack::TCP_FIN,
                             src_port => 80, dst_port => 5000, seq_num => 10, ack_num => 0);
    my ($c3, $final_ack) = $c2->handle_segment($peer_fin);
    is($c3->{state}, 'time_wait', 'state → TIME_WAIT');
    ok($final_ack->has_flag(CodingAdventures::NetworkStack::TCP_ACK), 'final ACK sent');
};

subtest 'TCPConnection — passive close (CLOSE_WAIT → LAST_ACK → CLOSED)' => sub {
    my $conn = _make_established_conn();

    # Peer sends FIN → CLOSE_WAIT
    my $fin = $TCP->new(flags => CodingAdventures::NetworkStack::TCP_FIN,
                        src_port => 80, dst_port => 5000, seq_num => 0, ack_num => 0);
    my ($c, $ack) = $conn->handle_segment($fin);
    is($c->{state}, 'close_wait', 'state → CLOSE_WAIT');

    # We send our FIN → LAST_ACK
    my ($c2, $our_fin) = $c->initiate_close();
    is($c2->{state}, 'last_ack', 'state → LAST_ACK');
    ok($our_fin->has_flag(CodingAdventures::NetworkStack::TCP_FIN), 'FIN set');

    # Peer ACKs our FIN → CLOSED
    my $final = $TCP->new(flags => CodingAdventures::NetworkStack::TCP_ACK,
                          src_port => 80, dst_port => 5000, seq_num => 0, ack_num => 0);
    my ($c3, $r) = $c2->handle_segment($final);
    is($c3->{state}, 'closed', 'state → CLOSED');
};

subtest 'TCPConnection — send_data fails when not established' => sub {
    my $conn = $TCN->new(local_port => 5000, remote_port => 80);
    my ($c, $hdr, $data) = $conn->send_data([0x01]);
    is($hdr, undef, 'no header when not established');
    is($data, [], 'empty data when not established');
};

subtest 'TCPConnection — initiate_connect fails if not CLOSED' => sub {
    my $conn = $TCN->new(local_port => 5000, remote_port => 80);
    $conn->set_listen();
    my ($c, $syn) = $conn->initiate_connect();
    is($syn, undef, 'no SYN when not CLOSED');
};

# ------------------------------------------------------------------ NetworkStack (facade)

subtest 'NetworkStack — send_udp produces valid frame' => sub {
    my $stack = $NS->new(
        local_ip  => [192, 168, 1, 10],
        local_mac => [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF],
    );
    my $wire = $stack->send_udp(
        [192, 168, 1, 1],
        [0x11, 0x22, 0x33, 0x44, 0x55, 0x66],
        5000, 53,
        [0xDE, 0xAD],
    );
    ok(scalar(@$wire) > 14, 'wire has more than 14 bytes');
    # First 6 bytes are destination MAC
    is($wire->[0], 0x11, 'dst MAC byte 0');
    is($wire->[5], 0x66, 'dst MAC byte 5');
    # EtherType at bytes 12-13
    is($wire->[12], 0x08, 'EtherType high byte = IPv4');
    is($wire->[13], 0x00, 'EtherType low byte');
    # IP version/IHL at byte 14
    is($wire->[14], 0x45, 'IP version/IHL = 0x45');
};

subtest 'NetworkStack — send_tcp produces valid frame' => sub {
    my $stack = $NS->new(
        local_ip  => [10, 0, 0, 1],
        local_mac => [0xCA, 0xFE, 0xBA, 0xBE, 0x00, 0x01],
    );
    my $wire = $stack->send_tcp(
        [10, 0, 0, 2],
        [0x11, 0x22, 0x33, 0x44, 0x55, 0x66],
        1234, 80,
        1000, 0,
        CodingAdventures::NetworkStack::TCP_SYN,
        [],
    );
    ok(scalar(@$wire) > 14, 'wire has data');
    # Protocol byte in IP header is at wire[14+9] = wire[23]
    is($wire->[23], CodingAdventures::NetworkStack::PROTO_TCP, 'protocol = TCP');
};

subtest 'NetworkStack — receive parses UDP frame' => sub {
    my $stack = $NS->new(
        local_ip  => [192, 168, 1, 10],
        local_mac => [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF],
    );
    my $wire = $stack->send_udp(
        [192, 168, 1, 1],
        [0x11, 0x22, 0x33, 0x44, 0x55, 0x66],
        5000, 53,
        [0xDE, 0xAD],
    );
    # Build a "reply" by swapping src/dst — just parse the same frame
    my $result = $stack->receive($wire);
    is($result->{ether_type}, CodingAdventures::NetworkStack::ETHERTYPE_IPV4, 'IPv4 frame');
    is($result->{ip_status},  'ok', 'IP checksum valid');
    is($result->{protocol},   CodingAdventures::NetworkStack::PROTO_UDP, 'protocol UDP');
    is($result->{src_port},   5000, 'src_port parsed');
    is($result->{dst_port},   53,   'dst_port parsed');
    is($result->{data},       [0xDE, 0xAD], 'payload intact');
};

subtest 'NetworkStack — receive parses TCP frame' => sub {
    my $stack = $NS->new(
        local_ip  => [10, 0, 0, 1],
        local_mac => [0xCA, 0xFE, 0x00, 0x00, 0x00, 0x01],
    );
    my $wire = $stack->send_tcp(
        [10, 0, 0, 2],
        [0x00, 0x11, 0x22, 0x33, 0x44, 0x55],
        1234, 80,
        0, 0,
        CodingAdventures::NetworkStack::TCP_SYN,
        [],
    );
    my $result = $stack->receive($wire);
    is($result->{ip_status}, 'ok', 'IP checksum valid');
    is($result->{protocol},  CodingAdventures::NetworkStack::PROTO_TCP, 'protocol TCP');
    is($result->{src_port},  1234, 'src_port parsed');
    is($result->{dst_port},  80,   'dst_port parsed');
    ok($result->{tcp_flags} & CodingAdventures::NetworkStack::TCP_SYN, 'SYN flag in tcp_flags');
};

subtest 'NetworkStack — receive with corrupt IP checksum' => sub {
    my $stack = $NS->new(
        local_ip  => [1, 2, 3, 4],
        local_mac => [0x01, 0x02, 0x03, 0x04, 0x05, 0x06],
    );
    my $wire = $stack->send_udp(
        [5, 6, 7, 8],
        [0x11, 0x22, 0x33, 0x44, 0x55, 0x66],
        9999, 53,
        [0xFF],
    );
    # Corrupt the IP checksum (at wire offset 14+10 = 24)
    $wire->[24] ^= 0xFF;
    my $result = $stack->receive($wire);
    is($result->{ip_status}, 'checksum_error', 'bad checksum detected');
};

subtest 'NetworkStack — add_route and add_arp' => sub {
    my $stack = $NS->new(
        local_ip  => [192, 168, 1, 10],
        local_mac => [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF],
    );
    $stack->add_arp('192.168.1.1', [0x11, 0x22, 0x33, 0x44, 0x55, 0x66]);
    $stack->add_route([192, 168, 1, 0], [255, 255, 255, 0], [192, 168, 1, 1], 'eth0');

    is($stack->{arp_table}->lookup('192.168.1.1'),
       [0x11, 0x22, 0x33, 0x44, 0x55, 0x66], 'ARP entry stored');
    my $route = $stack->{routing_table}->lookup([192, 168, 1, 5]);
    is($route->{iface}, 'eth0', 'route entry stored');
};

# ------------------------------------------------------------------ helpers

sub _make_established_conn {
    my $conn = $TCN->new(local_port => 5000, remote_port => 80);
    my ($c, $syn) = $conn->initiate_connect();
    my $synack = $TCP->new(
        src_port => 80, dst_port => 5000,
        seq_num  => 3000, ack_num => $c->{send_seq} + 1,
        flags    => CodingAdventures::NetworkStack::TCP_SYN |
                    CodingAdventures::NetworkStack::TCP_ACK,
    );
    my ($c2, $ack) = $c->handle_segment($synack);
    return $c2;  # ESTABLISHED
}

done_testing();
