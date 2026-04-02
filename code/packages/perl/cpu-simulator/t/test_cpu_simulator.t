use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::CpuSimulator; 1 },
    'CodingAdventures::CpuSimulator loads' );

my $Memory        = 'CodingAdventures::CpuSimulator::Memory';
my $SparseMemory  = 'CodingAdventures::CpuSimulator::SparseMemory';
my $RegisterFile  = 'CodingAdventures::CpuSimulator::RegisterFile';

# ============================================================================
# Memory tests
# ============================================================================

subtest 'Memory: initializes all bytes to 0' => sub {
    my $m = $Memory->new(64);
    for my $i (0 .. 63) {
        is($m->read_byte($i), 0, "byte $i is 0");
    }
};

subtest 'Memory: read_byte/write_byte round-trip' => sub {
    my $m = $Memory->new(16);
    $m->write_byte(0,  0xFF);
    $m->write_byte(1,  0xAB);
    $m->write_byte(15, 0x42);
    is($m->read_byte(0),  0xFF, 'byte 0 = 0xFF');
    is($m->read_byte(1),  0xAB, 'byte 1 = 0xAB');
    is($m->read_byte(15), 0x42, 'byte 15 = 0x42');
};

subtest 'Memory: write_byte masks to 8 bits' => sub {
    my $m = $Memory->new(8);
    $m->write_byte(0, 0x1FF);
    is($m->read_byte(0), 0xFF, 'masked to 0xFF');
};

subtest 'Memory: read_word is little-endian' => sub {
    my $m = $Memory->new(8);
    $m->write_byte(0, 0x78);
    $m->write_byte(1, 0x56);
    $m->write_byte(2, 0x34);
    $m->write_byte(3, 0x12);
    is($m->read_word(0), 0x12345678, 'little-endian read_word');
};

subtest 'Memory: write_word stores little-endian bytes' => sub {
    my $m = $Memory->new(8);
    $m->write_word(0, 0xDEADBEEF);
    is($m->read_byte(0), 0xEF, 'byte 0 = LSB');
    is($m->read_byte(1), 0xBE, 'byte 1');
    is($m->read_byte(2), 0xAD, 'byte 2');
    is($m->read_byte(3), 0xDE, 'byte 3 = MSB');
};

subtest 'Memory: write_word/read_word round-trip' => sub {
    my $m = $Memory->new(16);
    $m->write_word(0, 0xCAFEBABE);
    $m->write_word(4, 0x00000001);
    $m->write_word(8, 0xFFFFFFFF);
    is($m->read_word(0), 0xCAFEBABE, 'first word');
    is($m->read_word(4), 0x00000001, 'second word');
    is($m->read_word(8), 0xFFFFFFFF, 'third word (all ones)');
};

subtest 'Memory: load_bytes stores bytes sequentially' => sub {
    my $m = $Memory->new(16);
    $m->load_bytes(4, [0x01, 0x02, 0x03, 0x04]);
    is($m->read_byte(4), 0x01, 'byte[4] = 0x01');
    is($m->read_byte(5), 0x02, 'byte[5] = 0x02');
    is($m->read_byte(6), 0x03, 'byte[6] = 0x03');
    is($m->read_byte(7), 0x04, 'byte[7] = 0x04');
};

subtest 'Memory: dump returns correct slice' => sub {
    my $m = $Memory->new(16);
    $m->load_bytes(2, [0xAA, 0xBB, 0xCC]);
    my $bytes = $m->dump(2, 3);
    is($bytes, [0xAA, 0xBB, 0xCC], 'dump returns correct bytes');
};

subtest 'Memory: out-of-bounds read dies' => sub {
    my $m = $Memory->new(8);
    ok(dies { $m->read_byte(8) }, 'out-of-bounds read dies');
};

subtest 'Memory: out-of-bounds write dies' => sub {
    my $m = $Memory->new(8);
    ok(dies { $m->write_byte(-1, 0) }, 'negative address write dies');
};

subtest 'Memory: new() with size 0 dies' => sub {
    ok(dies { $Memory->new(0) }, 'size=0 dies');
};

# ============================================================================
# SparseMemory tests
# ============================================================================

subtest 'SparseMemory: reads 0 from unwritten addresses' => sub {
    my $m = $SparseMemory->new(1024 * 1024);
    is($m->read_byte(0),      0, 'address 0 = 0');
    is($m->read_byte(65535),  0, 'address 65535 = 0');
    is($m->read_word(0),      0, 'word at 0 = 0');
};

subtest 'SparseMemory: read_byte/write_byte round-trip' => sub {
    my $m = $SparseMemory->new(1024);
    $m->write_byte(500, 0x7F);
    is($m->read_byte(500), 0x7F, 'sparse byte round-trip');
};

subtest 'SparseMemory: writing 0 removes entry' => sub {
    my $m = $SparseMemory->new(1024);
    $m->write_byte(100, 0x42);
    $m->write_byte(100, 0x00);
    is($m->read_byte(100), 0, 'byte is 0 after writing 0');
    ok(!exists $m->{data}{100}, 'entry removed from hash');
};

subtest 'SparseMemory: read_word/write_word round-trip' => sub {
    my $m = $SparseMemory->new(65536);
    $m->write_word(1000, 0xABCDEF01);
    is($m->read_word(1000), 0xABCDEF01, 'sparse word round-trip');
};

subtest 'SparseMemory: load_bytes and dump' => sub {
    my $m = $SparseMemory->new(65536);
    $m->load_bytes(200, [0x11, 0x22, 0x33]);
    my $bytes = $m->dump(200, 3);
    is($bytes, [0x11, 0x22, 0x33], 'sparse dump correct');
};

# ============================================================================
# RegisterFile tests
# ============================================================================

subtest 'RegisterFile: initializes all registers to 0' => sub {
    my $rf = $RegisterFile->new(16, 32);
    for my $i (0 .. 15) {
        is($rf->read($i), 0, "R$i = 0");
    }
};

subtest 'RegisterFile: read/write round-trip' => sub {
    my $rf = $RegisterFile->new(16, 32);
    $rf->write(0,  0xDEADBEEF);
    $rf->write(7,  12345);
    $rf->write(15, 0);
    is($rf->read(0),  0xDEADBEEF, 'R0 = 0xDEADBEEF');
    is($rf->read(7),  12345,      'R7 = 12345');
    is($rf->read(15), 0,          'R15 = 0');
};

subtest 'RegisterFile: masks writes to 32-bit max' => sub {
    my $rf = $RegisterFile->new(4, 32);
    $rf->write(0, 0x1FFFFFFFF);  # 5 bytes — only lower 4 should be kept
    is($rf->read(0), 0xFFFFFFFF, 'masked to 0xFFFFFFFF');
};

subtest 'RegisterFile: masks writes for 8-bit registers' => sub {
    my $rf = $RegisterFile->new(4, 8);
    $rf->write(0, 0x1FF);
    is($rf->read(0), 0xFF, 'masked to 0xFF for 8-bit');
};

subtest 'RegisterFile: num_regs() returns count' => sub {
    my $rf = $RegisterFile->new(32, 64);
    is($rf->num_regs(), 32, 'num_regs = 32');
};

subtest 'RegisterFile: dump() returns keyed hashref' => sub {
    my $rf = $RegisterFile->new(4, 32);
    $rf->write(0, 10);
    $rf->write(1, 20);
    $rf->write(2, 30);
    $rf->write(3, 40);
    my $d = $rf->dump();
    is($d->{R0}, 10, 'R0=10');
    is($d->{R1}, 20, 'R1=20');
    is($d->{R2}, 30, 'R2=30');
    is($d->{R3}, 40, 'R3=40');
};

subtest 'RegisterFile: out-of-bounds read dies' => sub {
    my $rf = $RegisterFile->new(4, 32);
    ok(dies { $rf->read(4) }, 'out-of-bounds read dies');
};

subtest 'RegisterFile: out-of-bounds write dies' => sub {
    my $rf = $RegisterFile->new(4, 32);
    ok(dies { $rf->write(-1, 0) }, 'negative index write dies');
};

subtest 'RegisterFile: default constructor creates 16 32-bit registers' => sub {
    my $rf = $RegisterFile->new();
    is($rf->num_regs(),    16, 'default num_regs = 16');
    is($rf->{bit_width},   32, 'default bit_width = 32');
};

done_testing;
