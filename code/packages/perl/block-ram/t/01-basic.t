use strict;
use warnings;
use Test2::V0;

ok(eval { require CodingAdventures::BlockRam; 1 }, 'CodingAdventures::BlockRam loads');

# ============================================================================
# Constructor
# ============================================================================

subtest 'construction' => sub {
    my $ram = CodingAdventures::BlockRam->new(256);
    ok($ram, 'BlockRam created');
    is($ram->size(), 256, 'size() returns 256');

    # All bytes should initialise to 0
    for my $a (0, 1, 127, 255) {
        is($ram->read_byte($a), 0, "byte at $a initialised to 0");
    }

    # size=1 edge case
    my $tiny = CodingAdventures::BlockRam->new(1);
    is($tiny->size(), 1, 'size=1 RAM created');

    # size=0 should die
    ok(!eval { CodingAdventures::BlockRam->new(0); 1 }, 'size=0 dies');
};

# ============================================================================
# Byte read/write
# ============================================================================

subtest 'write_byte and read_byte' => sub {
    my $ram = CodingAdventures::BlockRam->new(16);

    $ram->write_byte(0, 0xAB);
    is($ram->read_byte(0), 0xAB, 'read_byte after write_byte');

    $ram->write_byte(15, 0xFF);
    is($ram->read_byte(15), 0xFF, 'last address writable');

    $ram->write_byte(7, 0);
    is($ram->read_byte(7), 0, 'write 0 clears byte');

    # Min and max byte values
    $ram->write_byte(3, 0);
    is($ram->read_byte(3), 0, 'write min byte value 0');
    $ram->write_byte(3, 255);
    is($ram->read_byte(3), 255, 'write max byte value 255');
};

subtest 'write_byte validates bounds' => sub {
    my $ram = CodingAdventures::BlockRam->new(8);
    ok(!eval { $ram->write_byte(8, 0); 1 },   'out-of-range address dies');
    ok(!eval { $ram->write_byte(-1, 0); 1 },  'negative address dies');
    ok(!eval { $ram->write_byte(0, 256); 1 }, 'value > 255 dies');
    ok(!eval { $ram->write_byte(0, -1); 1 },  'negative value dies');
};

# ============================================================================
# Word read/write — little-endian
# ============================================================================

subtest 'read_word / write_word little-endian' => sub {
    my $ram = CodingAdventures::BlockRam->new(16);

    # Write 0x12345678 as 4 bytes little-endian at address 0
    # Expected layout: [0x78, 0x56, 0x34, 0x12]
    $ram->write_word(0, 0x12345678, 4, 'little');
    is($ram->read_byte(0), 0x78, 'little-endian LSB at addr 0');
    is($ram->read_byte(1), 0x56, 'little-endian byte 1');
    is($ram->read_byte(2), 0x34, 'little-endian byte 2');
    is($ram->read_byte(3), 0x12, 'little-endian MSB at addr 3');

    # Round-trip
    my $val = $ram->read_word(0, 4, 'little');
    is($val, 0x12345678, 'read_word little-endian round-trip');
};

# ============================================================================
# Word read/write — big-endian
# ============================================================================

subtest 'read_word / write_word big-endian' => sub {
    my $ram = CodingAdventures::BlockRam->new(16);

    # Write 0x1234 as 2 bytes big-endian at address 4
    # Expected layout: [0x12, 0x34]
    $ram->write_word(4, 0x1234, 2, 'big');
    is($ram->read_byte(4), 0x12, 'big-endian MSB first');
    is($ram->read_byte(5), 0x34, 'big-endian LSB second');

    # Round-trip
    my $val = $ram->read_word(4, 2, 'big');
    is($val, 0x1234, 'read_word big-endian round-trip');
};

subtest 'endian cross-check' => sub {
    my $ram = CodingAdventures::BlockRam->new(8);
    # Write same value in both endiannesses and verify byte layout differs
    $ram->write_word(0, 0xABCD, 2, 'little');
    is($ram->read_byte(0), 0xCD, 'little: LSB at low address');
    is($ram->read_byte(1), 0xAB, 'little: MSB at high address');

    $ram->write_word(0, 0xABCD, 2, 'big');
    is($ram->read_byte(0), 0xAB, 'big: MSB at low address');
    is($ram->read_byte(1), 0xCD, 'big: LSB at high address');
};

# ============================================================================
# dump
# ============================================================================

subtest 'dump returns copy of memory region' => sub {
    my $ram = CodingAdventures::BlockRam->new(8);
    $ram->load([0x10, 0x20, 0x30, 0x40, 0x50, 0x60, 0x70, 0x80], 0);

    my $all = $ram->dump();
    is($all, [0x10, 0x20, 0x30, 0x40, 0x50, 0x60, 0x70, 0x80], 'dump entire memory');

    my $partial = $ram->dump(2, 3);
    is($partial, [0x30, 0x40, 0x50], 'dump partial region');

    # dump does not modify memory
    $all->[0] = 0xFF;
    is($ram->read_byte(0), 0x10, 'modifying dump copy does not change RAM');
};

# ============================================================================
# load
# ============================================================================

subtest 'load writes bytes at offset' => sub {
    my $ram = CodingAdventures::BlockRam->new(16);
    $ram->load([1, 2, 3], 5);
    is($ram->read_byte(4), 0, 'byte before load region unchanged');
    is($ram->read_byte(5), 1, 'load byte 0');
    is($ram->read_byte(6), 2, 'load byte 1');
    is($ram->read_byte(7), 3, 'load byte 2');
    is($ram->read_byte(8), 0, 'byte after load region unchanged');
};

# ============================================================================
# fill
# ============================================================================

subtest 'fill sets all bytes' => sub {
    my $ram = CodingAdventures::BlockRam->new(4);
    $ram->fill(0xAA);
    is($ram->dump(), [0xAA, 0xAA, 0xAA, 0xAA], 'fill sets all bytes');
    $ram->fill(0);
    is($ram->dump(), [0, 0, 0, 0], 'fill with 0 clears all bytes');
};

# ============================================================================
# 1-byte word edge case
# ============================================================================

subtest 'read_word / write_word with 1 byte' => sub {
    my $ram = CodingAdventures::BlockRam->new(4);
    $ram->write_word(0, 0xAB, 1, 'little');
    is($ram->read_word(0, 1, 'little'), 0xAB, '1-byte word little');
    $ram->write_word(0, 0xCD, 1, 'big');
    is($ram->read_word(0, 1, 'big'), 0xCD, '1-byte word big');
};

done_testing;
