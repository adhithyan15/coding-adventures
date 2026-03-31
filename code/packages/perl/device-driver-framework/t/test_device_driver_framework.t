use strict;
use warnings;
use Test2::V0;

use CodingAdventures::DeviceDriverFramework;

my $DDF    = 'CodingAdventures::DeviceDriverFramework';
my $Disk   = 'CodingAdventures::DeviceDriverFramework::SimulatedDisk';
my $Serial = 'CodingAdventures::DeviceDriverFramework::SimulatedSerial';
my $NIC    = 'CodingAdventures::DeviceDriverFramework::SimulatedNIC';
my $Reg    = 'CodingAdventures::DeviceDriverFramework::Registry';

# ============================================================================
# Constants
# ============================================================================

subtest 'constants' => sub {
    is($DDF->TYPE_CHARACTER, 0, 'character=0');
    is($DDF->TYPE_BLOCK,     1, 'block=1');
    is($DDF->TYPE_NETWORK,   2, 'network=2');
    is($DDF->DEFAULT_BLOCK_SIZE,   512,  'block_size 512');
    is($DDF->DEFAULT_TOTAL_BLOCKS, 2048, 'total_blocks 2048');
};

# ============================================================================
# SimulatedDisk tests
# ============================================================================

subtest 'SimulatedDisk — new with defaults' => sub {
    my $d = $Disk->new();
    is($d->{name},        'disk0',                   'name');
    is($d->{device_type}, $DDF->TYPE_BLOCK,          'type block');
    is($d->{major},       3,                         'major 3');
    is($d->{minor},       0,                         'minor 0');
    is($d->{block_size},  512,                       'block_size 512');
    is($d->{total_blocks}, 2048,                     'total_blocks 2048');
    is($d->{initialized}, 0,                         'not initialized');
};

subtest 'SimulatedDisk — custom options' => sub {
    my $d = $Disk->new(name => 'disk1', major => 8, minor => 16, block_size => 256, total_blocks => 4);
    is($d->{name},        'disk1', 'name');
    is($d->{block_size},  256,    'block_size 256');
    is($d->{total_blocks}, 4,     'total_blocks 4');
};

subtest 'SimulatedDisk — initialize' => sub {
    my $d = $Disk->new();
    my ($st, $d2) = $d->initialize();
    is($st, 'ok', 'ok');
    is($d2->{initialized}, 1, 'initialized');
};

subtest 'SimulatedDisk — open before initialize' => sub {
    my $d = $Disk->new();
    my ($st, $ignored) = $d->open();
    is($st, 'not_initialized', 'not_initialized');
};

subtest 'SimulatedDisk — open and close' => sub {
    my $d = $Disk->new();
    $d->initialize();
    my ($st1, $ignored) = $d->open();
    is($st1, 'ok', 'open ok');
    is($d->{open_count}, 1, 'count 1');
    my ($st2, $_2) = $d->close();
    is($st2, 'ok', 'close ok');
    is($d->{open_count}, 0, 'count 0');
};

subtest 'SimulatedDisk — close when not open' => sub {
    my $d = $Disk->new();
    $d->initialize();
    my ($st, $ignored) = $d->close();
    is($st, 'not_open', 'not_open');
};

subtest 'SimulatedDisk — write_block and read_block' => sub {
    my $d = $Disk->new(block_size => 8, total_blocks => 4);
    my ($ws, $ignored) = $d->write_block(0, 'ABCDEFGH');
    is($ws, 'ok', 'write ok');
    my ($rs, $self, $data) = $d->read_block(0);
    is($rs,   'ok',       'read ok');
    is($data, 'ABCDEFGH', 'data correct');
};

subtest 'SimulatedDisk — write to non-zero block' => sub {
    my $d = $Disk->new(block_size => 4, total_blocks => 8);
    $d->write_block(3, 'TEST');
    my (undef, undef, $r0) = $d->read_block(0);
    my (undef, undef, $r3) = $d->read_block(3);
    is($r3, 'TEST',     'block 3 correct');
    is($r0, "\x00" x 4, 'block 0 unchanged');
};

subtest 'SimulatedDisk — write out of bounds' => sub {
    my $d = $Disk->new(block_size => 4, total_blocks => 2);
    my ($st, $ignored) = $d->write_block(2, 'XXXX');
    is($st, 'out_of_bounds', 'out_of_bounds');
    ($st, $_) = $d->write_block(-1, 'XXXX');
    is($st, 'out_of_bounds', 'negative block');
};

subtest 'SimulatedDisk — write wrong size' => sub {
    my $d = $Disk->new(block_size => 8, total_blocks => 4);
    my ($st, $ignored) = $d->write_block(0, 'short');
    is($st, 'wrong_size', 'wrong_size');
};

subtest 'SimulatedDisk — read out of bounds' => sub {
    my $d = $Disk->new(block_size => 4, total_blocks => 2);
    my ($st, $self, $data) = $d->read_block(5);
    is($st, 'out_of_bounds', 'out_of_bounds');
    is($data, undef,         'undef data');
};

subtest 'SimulatedDisk — ioctl' => sub {
    my $d = $Disk->new(block_size => 256, total_blocks => 100);
    my ($st1, $bs) = $d->ioctl('get_block_size');
    is($st1, 'ok', 'ok');
    is($bs,  256,  '256');
    my ($st2, $tb) = $d->ioctl('get_total_blocks');
    is($st2, 'ok', 'ok');
    is($tb,  100,  '100');
    my ($st3, $ignored) = $d->ioctl('format');
    is($st3, 'unsupported', 'unsupported');
};

# ============================================================================
# SimulatedSerial tests
# ============================================================================

subtest 'SimulatedSerial — new defaults' => sub {
    my $s = $Serial->new();
    is($s->{name},        'serial0',              'name');
    is($s->{device_type}, $DDF->TYPE_CHARACTER,   'character type');
    is($s->{baud_rate},   9600,                   'baud 9600');
    is($s->{initialized}, 0,                      'not initialized');
};

subtest 'SimulatedSerial — lifecycle' => sub {
    my $s = $Serial->new();
    $s->initialize();
    ok($s->{initialized}, 'initialized');
    my ($st1, $ignored) = $s->open();
    is($st1, 'ok', 'open ok');
    is($s->{open_count}, 1, 'count 1');
    my ($st2, $_2) = $s->close();
    is($st2, 'ok', 'close ok');
    is($s->{open_count}, 0, 'count 0');
};

subtest 'SimulatedSerial — open before initialize' => sub {
    my $s = $Serial->new();
    my ($st, $ignored) = $s->open();
    is($st, 'not_initialized', 'error');
};

subtest 'SimulatedSerial — close when not open' => sub {
    my $s = $Serial->new();
    $s->initialize();
    my ($st, $ignored) = $s->close();
    is($st, 'not_open', 'error');
};

subtest 'SimulatedSerial — write and tx_contents' => sub {
    my $s = $Serial->new();
    $s->initialize();
    my ($st, $self, $n) = $s->write('hello');
    is($st, 'ok',    'ok');
    is($n,  5,       '5 bytes');
    is($s->tx_contents(), 'hello', 'tx buffer');
};

subtest 'SimulatedSerial — inject_rx and read' => sub {
    my $s = $Serial->new();
    $s->initialize();
    $s->inject_rx('world');
    my ($st, $self, $data) = $s->read(10);
    is($st,   'ok',   'ok');
    is($data, 'world', 'data');
    is(length($s->{rx_buffer}), 0, 'rx empty');
};

subtest 'SimulatedSerial — read from empty' => sub {
    my $s = $Serial->new();
    my ($st, $self, $data) = $s->read(4);
    is($st,   'empty', 'empty');
    is($data, '',      'empty string');
};

subtest 'SimulatedSerial — partial read' => sub {
    my $s = $Serial->new();
    $s->inject_rx('hello world');
    my ($st, $self, $data) = $s->read(5);
    is($st,   'ok',   'ok');
    is($data, 'hello', '5 bytes');
    is(length($s->{rx_buffer}), 6, '6 remaining');
};

subtest 'SimulatedSerial — ioctl baud rate' => sub {
    my $s = $Serial->new(baud_rate => 115200);
    my ($st1, $br) = $s->ioctl('get_baud_rate');
    is($st1, 'ok',    'ok');
    is($br,  115200,  '115200');
    my ($st2, $s2) = $s->ioctl('set_baud_rate', 9600);
    is($st2, 'ok', 'ok');
    is($s2->{baud_rate}, 9600, 'updated');
};

subtest 'SimulatedSerial — ioctl unsupported' => sub {
    my $s = $Serial->new();
    my ($st, $ignored) = $s->ioctl('flush');
    is($st, 'unsupported', 'unsupported');
};

# ============================================================================
# SimulatedNIC tests
# ============================================================================

subtest 'SimulatedNIC — new defaults' => sub {
    my $n = $NIC->new();
    is($n->{name},        'eth0',              'name');
    is($n->{device_type}, $DDF->TYPE_NETWORK,  'network type');
    is($n->{major},       5,                   'major 5');
    is(scalar @{ $n->{mac_address} }, 6,       '6-byte MAC');
    is($n->{initialized}, 0,                   'not initialized');
};

subtest 'SimulatedNIC — lifecycle' => sub {
    my $n = $NIC->new();
    $n->initialize();
    ok($n->{initialized}, 'initialized');
    my ($st1, $ignored) = $n->open();
    is($st1, 'ok', 'open');
    is($n->{open_count}, 1, 'count 1');
    my ($st2, $_2) = $n->close();
    is($st2, 'ok', 'close');
    is($n->{open_count}, 0, 'count 0');
};

subtest 'SimulatedNIC — open before initialize' => sub {
    my $n = $NIC->new();
    my ($st, $ignored) = $n->open();
    is($st, 'not_initialized', 'error');
};

subtest 'SimulatedNIC — close when not open' => sub {
    my $n = $NIC->new();
    $n->initialize();
    my ($st, $ignored) = $n->close();
    is($st, 'not_open', 'error');
};

subtest 'SimulatedNIC — send and receive' => sub {
    my $n = $NIC->new();
    $n->send([0xAA, 0xBB, 0xCC]);
    is(scalar @{ $n->{tx_queue} }, 1, 'tx queue has 1');
    $n->inject_rx([0x01, 0x02]);
    my ($st, $self, $pkt) = $n->receive();
    is($st, 'ok', 'ok');
    is($pkt, [0x01, 0x02], 'packet correct');
    is(scalar @{ $n->{rx_queue} }, 0, 'rx empty');
};

subtest 'SimulatedNIC — receive from empty' => sub {
    my $n = $NIC->new();
    my ($st, $self, $pkt) = $n->receive();
    is($st,  'empty', 'empty');
    is($pkt, undef,   'undef');
};

subtest 'SimulatedNIC — FIFO order' => sub {
    my $n = $NIC->new();
    $n->inject_rx([1]);
    $n->inject_rx([2]);
    $n->inject_rx([3]);
    my (undef, undef, $p1) = $n->receive();
    my (undef, undef, $p2) = $n->receive();
    my (undef, undef, $p3) = $n->receive();
    is($p1->[0], 1, 'first');
    is($p2->[0], 2, 'second');
    is($p3->[0], 3, 'third');
};

subtest 'SimulatedNIC — ioctl get_mac' => sub {
    my $mac = [0x11, 0x22, 0x33, 0x44, 0x55, 0x66];
    my $n = $NIC->new(mac_address => $mac);
    my ($st, $got) = $n->ioctl('get_mac');
    is($st, 'ok', 'ok');
    is($got, $mac, 'mac correct');
};

subtest 'SimulatedNIC — ioctl unsupported' => sub {
    my $n = $NIC->new();
    my ($st, $ignored) = $n->ioctl('reset');
    is($st, 'unsupported', 'unsupported');
};

# ============================================================================
# Registry tests
# ============================================================================

subtest 'Registry — new is empty' => sub {
    my $r = $Reg->new();
    my @names = $r->list();
    is(scalar @names, 0, 'empty');
};

subtest 'Registry — register and get' => sub {
    my $r = $Reg->new();
    my $d = $Disk->new();
    is($r->register($d), 'ok', 'register ok');
    my ($st, $got) = $r->get('disk0');
    is($st, 'ok', 'ok');
    is($got->{name}, 'disk0', 'name');
};

subtest 'Registry — double register returns already_registered' => sub {
    my $r = $Reg->new();
    my $d = $Disk->new();
    $r->register($d);
    is($r->register($d), 'already_registered', 'already_registered');
};

subtest 'Registry — get not_found' => sub {
    my $r = $Reg->new();
    my ($st, $ignored) = $r->get('nodev');
    is($st, 'not_found', 'not_found');
};

subtest 'Registry — get_by_major_minor' => sub {
    my $r = $Reg->new();
    my $d = $Disk->new(major => 8, minor => 0);
    $r->register($d);
    my ($st, $got) = $r->get_by_major_minor(8, 0);
    is($st, 'ok', 'ok');
    is($got->{name}, 'disk0', 'name');
};

subtest 'Registry — get_by_major_minor not_found' => sub {
    my $r = $Reg->new();
    my ($st, $ignored) = $r->get_by_major_minor(99, 0);
    is($st, 'not_found', 'not_found');
};

subtest 'Registry — update' => sub {
    my $r = $Reg->new();
    my $d = $Disk->new(block_size => 4, total_blocks => 4);
    $r->register($d);
    $d->write_block(0, 'ABCD');
    is($r->update('disk0', $d), 'ok', 'update ok');
    my (undef, $got) = $r->get('disk0');
    my (undef, undef, $data) = $got->read_block(0);
    is($data, 'ABCD', 'updated data');
};

subtest 'Registry — update not_found' => sub {
    my $r = $Reg->new();
    my $d = $Disk->new();
    is($r->update('disk0', $d), 'not_found', 'not_found');
};

subtest 'Registry — unregister' => sub {
    my $r = $Reg->new();
    my $d = $Disk->new(major => 3, minor => 0);
    $r->register($d);
    is($r->unregister('disk0'), 'ok', 'unregistered');
    my ($st, $ignored) = $r->get('disk0');
    is($st, 'not_found', 'gone');
    # major:minor also removed
    my ($st2, $_2) = $r->get_by_major_minor(3, 0);
    is($st2, 'not_found', 'major:minor gone');
};

subtest 'Registry — unregister not_found' => sub {
    my $r = $Reg->new();
    is($r->unregister('nodev'), 'not_found', 'not_found');
};

subtest 'Registry — list returns sorted names' => sub {
    my $r    = $Reg->new();
    my $disk = $Disk->new(name => 'disk0', major => 3, minor => 0);
    my $ser  = $Serial->new(name => 'serial0', major => 4, minor => 0);
    my $nic  = $NIC->new(name => 'eth0', major => 5, minor => 0);
    $r->register($disk);
    $r->register($ser);
    $r->register($nic);
    my @names = $r->list();
    is(\@names, ['disk0', 'eth0', 'serial0'], 'sorted names');
};

subtest 'Registry — full driver lifecycle' => sub {
    my $d = $Disk->new(block_size => 8, total_blocks => 4);
    $d->initialize();
    $d->open();
    $d->write_block(1, 'HELLO!!!');
    my (undef, undef, $data) = $d->read_block(1);
    is($data, 'HELLO!!!', 'data correct');
    my ($st, $ignored) = $d->close();
    is($st, 'ok', 'closed');
};

done_testing();
