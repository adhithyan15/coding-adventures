use strict;
use warnings;
use Test2::V0;
use lib '../lib';
use CodingAdventures::CpuSimulator;

my $Memory       = 'CodingAdventures::CpuSimulator::Memory';
my $SparseMemory = 'CodingAdventures::CpuSimulator::SparseMemory';
my $RegisterFile = 'CodingAdventures::CpuSimulator::RegisterFile';

# ---------------------------------------------------------------------------
# Memory tests
# ---------------------------------------------------------------------------

subtest 'Memory::new initialises to zero' => sub {
    my $m = $Memory->new(64);
    is $m->read_byte($_), 0, "byte $_ is 0" for 0..63;
};

subtest 'Memory read_byte / write_byte roundtrip' => sub {
    my $m = $Memory->new(64);
    $m->write_byte(0, 0xAB);
    is $m->read_byte(0), 0xAB, 'read back 0xAB';
};

subtest 'Memory write_byte masks to 8 bits' => sub {
    my $m = $Memory->new(64);
    $m->write_byte(0, 0x1FF);
    is $m->read_byte(0), 0xFF, '0x1FF & 0xFF = 0xFF';
};

subtest 'Memory read_word / write_word little-endian' => sub {
    my $m = $Memory->new(64);
    $m->write_word(0, 0xDEADBEEF);
    is $m->read_byte(0), 0xEF, 'byte 0 = 0xEF (LSB)';
    is $m->read_byte(1), 0xBE, 'byte 1 = 0xBE';
    is $m->read_byte(2), 0xAD, 'byte 2 = 0xAD';
    is $m->read_byte(3), 0xDE, 'byte 3 = 0xDE (MSB)';
    is $m->read_word(0), 0xDEADBEEF, 'read_word roundtrip';
};

subtest 'Memory write_word at non-zero offset' => sub {
    my $m = $Memory->new(64);
    $m->write_word(8, 0x12345678);
    is $m->read_word(8), 0x12345678, 'roundtrip at offset 8';
};

subtest 'Memory load_bytes' => sub {
    my $m = $Memory->new(64);
    $m->load_bytes(4, [0x01, 0x02, 0x03, 0x04]);
    is $m->read_byte(4), 0x01, 'byte 4 = 0x01';
    is $m->read_byte(7), 0x04, 'byte 7 = 0x04';
};

subtest 'Memory dump' => sub {
    my $m = $Memory->new(64);
    $m->write_byte(0, 0xAA);
    $m->write_byte(1, 0xBB);
    $m->write_byte(2, 0xCC);
    my $d = $m->dump(0, 3);
    is $d->[0], 0xAA, 'dump[0]';
    is $d->[1], 0xBB, 'dump[1]';
    is $d->[2], 0xCC, 'dump[2]';
};

subtest 'Memory out-of-range read dies' => sub {
    my $m = $Memory->new(4);
    ok dies { $m->read_byte(4) }, 'out-of-range read dies';
};

subtest 'Memory out-of-range write dies' => sub {
    my $m = $Memory->new(4);
    ok dies { $m->write_byte(4, 0) }, 'out-of-range write dies';
};

# ---------------------------------------------------------------------------
# SparseMemory tests
# ---------------------------------------------------------------------------

subtest 'SparseMemory unwritten returns 0' => sub {
    my $m = $SparseMemory->new();
    is $m->read_byte(0),         0, 'addr 0 = 0';
    is $m->read_byte(0xFFFF),    0, 'addr 0xFFFF = 0';
    is $m->read_word(0x1000),    0, 'word at 0x1000 = 0';
};

subtest 'SparseMemory read_byte / write_byte' => sub {
    my $m = $SparseMemory->new();
    $m->write_byte(0x100, 0x42);
    is $m->read_byte(0x100), 0x42, 'read back 0x42';
};

subtest 'SparseMemory read_word / write_word' => sub {
    my $m = $SparseMemory->new();
    $m->write_word(0x200, 0xCAFEBABE);
    is $m->read_word(0x200), 0xCAFEBABE, 'roundtrip';
};

subtest 'SparseMemory load_bytes' => sub {
    my $m = $SparseMemory->new();
    $m->load_bytes(0, [0x11, 0x22, 0x33]);
    is $m->read_byte(0), 0x11, 'byte 0';
    is $m->read_byte(2), 0x33, 'byte 2';
    is $m->read_byte(3), 0,    'byte 3 = 0';
};

subtest 'SparseMemory writing 0 removes entry' => sub {
    my $m = $SparseMemory->new();
    $m->write_byte(5, 0x55);
    is $m->read_byte(5), 0x55, 'written 0x55';
    $m->write_byte(5, 0);
    is $m->read_byte(5), 0, 'reads 0 after zero-write';
    ok !exists $m->{data}{5}, 'entry removed from hash';
};

# ---------------------------------------------------------------------------
# RegisterFile tests
# ---------------------------------------------------------------------------

subtest 'RegisterFile initialises to zero' => sub {
    my $rf = $RegisterFile->new(16, 32);
    is $rf->read($_), 0, "R$_ = 0" for 0..15;
};

subtest 'RegisterFile write / read roundtrip' => sub {
    my $rf = $RegisterFile->new(16, 32);
    $rf->write(3, 0xDEAD);
    is $rf->read(3), 0xDEAD, 'R3 = 0xDEAD';
};

subtest 'RegisterFile masks 32-bit values' => sub {
    my $rf = $RegisterFile->new(16, 32);
    $rf->write(0, 0x1FFFFFFFF);  # larger than 32 bits
    is $rf->read(0), 0xFFFFFFFF, 'truncated to 32 bits';
};

subtest 'RegisterFile 8-bit width' => sub {
    my $rf = $RegisterFile->new(4, 8);
    $rf->write(0, 0x1FF);
    is $rf->read(0), 0xFF, '0x1FF & 0xFF = 0xFF';
};

subtest 'RegisterFile dump' => sub {
    my $rf = $RegisterFile->new(4, 32);
    $rf->write(1, 100);
    my $d = $rf->dump();
    is $d->{R0}, 0,   'R0 = 0';
    is $d->{R1}, 100, 'R1 = 100';
    ok defined $d->{R3}, 'R3 exists';
};

subtest 'RegisterFile out-of-range read dies' => sub {
    my $rf = $RegisterFile->new(8, 32);
    ok dies { $rf->read(8) }, 'out-of-range read dies';
};

subtest 'RegisterFile out-of-range write dies' => sub {
    my $rf = $RegisterFile->new(8, 32);
    ok dies { $rf->write(-1, 0) }, 'negative index dies';
};

subtest 'RegisterFile independent registers' => sub {
    my $rf = $RegisterFile->new(16, 32);
    $rf->write(0, 10);
    $rf->write(1, 20);
    is $rf->read(0), 10, 'R0 = 10';
    is $rf->read(1), 20, 'R1 = 20';
};

done_testing;
