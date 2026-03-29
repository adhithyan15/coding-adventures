package CodingAdventures::BlockRam;

# ============================================================================
# CodingAdventures::BlockRam — Block RAM (memory) simulation in Pure Perl
# ============================================================================
#
# This module simulates a block of random-access memory (RAM). It models the
# memory subsystem you find in every computer: a flat array of bytes, each
# uniquely addressable, supporting byte-level and word-level reads/writes.
#
# # What is Block RAM?
#
# RAM (Random-Access Memory) is storage where every location can be read or
# written in constant time regardless of where it is. The "block" in "Block
# RAM" refers to the fact that FPGA designers carve their on-chip memory into
# discrete configurable chunks ("blocks") rather than one giant array.
#
# For this simulation, "block RAM" simply means a contiguous, byte-addressable
# memory region — exactly what a CPU sees as its address space.
#
# # Memory Model
#
#   * Byte-addressable: every address identifies one 8-bit byte.
#   * Addresses are 0-based integers.
#   * Values are unsigned bytes: 0–255.
#   * Words are multiple bytes read/written together.
#   * Endianness controls byte order within a word.
#
# # Endianness: Little vs. Big
#
# When a multi-byte integer is stored in memory, we must decide which byte
# of the integer goes at the lowest address. Two conventions exist:
#
#   Big-endian (network byte order, used by Motorola 68k, SPARC):
#     Most significant byte at the lowest address.
#     The value 0x1234 at address 100:  byte 100=0x12, byte 101=0x34
#     Mnemonic: "big end first"
#
#   Little-endian (used by x86, ARM in default mode, RISC-V):
#     Least significant byte at the lowest address.
#     The value 0x1234 at address 100:  byte 100=0x34, byte 101=0x12
#     Mnemonic: "little end first" — the "little" (less significant) byte is first
#
# Example for 0xDEADBEEF stored as a 4-byte word at address 0:
#
#   Big-endian:    [0xDE, 0xAD, 0xBE, 0xEF]   (natural reading order)
#   Little-endian: [0xEF, 0xBE, 0xAD, 0xDE]   (reversed byte order)
#
# Most modern desktop/server CPUs (x86, x86-64, ARM) are little-endian.
# Network protocols (TCP/IP) define big-endian as the standard, which is
# why big-endian is also called "network byte order".
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

# ============================================================================
# Constants
# ============================================================================

# Endianness constants for read_word/write_word.
# We use strings rather than integers so error messages are self-documenting.
use constant LITTLE => 'little';
use constant BIG    => 'big';

# ============================================================================
# Constructor
# ============================================================================

# ----------------------------------------------------------------------------
# new($size_bytes) → BlockRam instance
#
# Create a new block of RAM with $size_bytes bytes of storage.
# All bytes are initialized to 0.
#
# Internal representation:
#   _mem  — Perl array of integers (0–255), one per byte
#   _size — total number of bytes
#
# @param $size_bytes  Total memory size in bytes (must be >= 1)
# @return blessed hashref
# ----------------------------------------------------------------------------
sub new {
    my ($class, $size_bytes) = @_;
    die "BlockRam: size must be >= 1" unless defined $size_bytes && $size_bytes >= 1;

    # Allocate a flat array of $size_bytes zeros.
    # Perl arrays are sparse by nature, but pre-allocating with (0) x N
    # gives us a true dense array with defined values at every index.
    my @mem = (0) x $size_bytes;

    return bless {
        _mem  => \@mem,
        _size => $size_bytes,
    }, $class;
}

# ============================================================================
# Byte-level operations
# ============================================================================

# ----------------------------------------------------------------------------
# read_byte($addr) → integer (0–255)
#
# Read a single byte from the given address.
#
# This is the most primitive memory operation. Every other operation
# (read_word, dump, etc.) is built on top of this.
#
# @param $addr  Byte address (0 to size-1)
# @return integer in range 0–255
# ----------------------------------------------------------------------------
sub read_byte {
    my ($self, $addr) = @_;
    $self->_validate_addr($addr);
    return $self->{_mem}[$addr];
}

# ----------------------------------------------------------------------------
# write_byte($addr, $val) → void
#
# Write a single byte to the given address.
#
# @param $addr  Byte address (0 to size-1)
# @param $val   Byte value (0–255)
# ----------------------------------------------------------------------------
sub write_byte {
    my ($self, $addr, $val) = @_;
    $self->_validate_addr($addr);
    $self->_validate_byte($val);
    $self->{_mem}[$addr] = $val & 0xFF;
}

# ============================================================================
# Word-level operations
# ============================================================================

# ----------------------------------------------------------------------------
# read_word($addr, $nbytes, $endian) → integer
#
# Read a multi-byte integer from memory starting at $addr.
#
# The bytes at addresses [$addr .. $addr+$nbytes-1] are assembled into a
# single integer according to the specified endianness.
#
# Example: read_word(0, 2, 'big') with memory [0x12, 0x34]
#   → result = 0x12 * 256 + 0x34 = 0x1234 = 4660
#
# Example: read_word(0, 2, 'little') with memory [0x34, 0x12]
#   → result = 0x34 + 0x12 * 256 = 0x1234 = 4660
#   (same integer, opposite byte order in memory)
#
# @param $addr    Starting byte address
# @param $nbytes  Number of bytes (1, 2, 4, 8, etc.)
# @param $endian  'little' or 'big' (default: 'little')
# @return unsigned integer
# ----------------------------------------------------------------------------
sub read_word {
    my ($self, $addr, $nbytes, $endian) = @_;
    $nbytes //= 4;
    $endian //= LITTLE;

    $self->_validate_range($addr, $nbytes);

    my @bytes = map { $self->{_mem}[$addr + $_] } 0 .. $nbytes - 1;

    return _bytes_to_int(\@bytes, $endian);
}

# ----------------------------------------------------------------------------
# write_word($addr, $val, $nbytes, $endian) → void
#
# Write a multi-byte integer to memory starting at $addr.
#
# Splits $val into $nbytes bytes and stores them at consecutive addresses,
# in the order determined by $endian.
#
# @param $addr    Starting byte address
# @param $val     Unsigned integer to write
# @param $nbytes  Number of bytes (default 4)
# @param $endian  'little' or 'big' (default: 'little')
# ----------------------------------------------------------------------------
sub write_word {
    my ($self, $addr, $val, $nbytes, $endian) = @_;
    $nbytes //= 4;
    $endian //= LITTLE;

    $self->_validate_range($addr, $nbytes);
    die "BlockRam: write_word value must be >= 0" unless $val >= 0;

    my @bytes = _int_to_bytes($val, $nbytes, $endian);
    for my $i (0 .. $nbytes - 1) {
        $self->{_mem}[$addr + $i] = $bytes[$i];
    }
}

# ============================================================================
# Bulk operations
# ============================================================================

# ----------------------------------------------------------------------------
# dump($start, $len) → \@bytes
#
# Return a copy of $len bytes starting from address $start as an array ref.
#
# This is useful for debugging, checksumming, or serializing a region of
# memory. The original memory is not modified.
#
# @param $start  Starting byte address (default 0)
# @param $len    Number of bytes to dump (default: whole memory)
# @return arrayref of integers (0–255)
# ----------------------------------------------------------------------------
sub dump {
    my ($self, $start, $len) = @_;
    $start //= 0;
    $len   //= $self->{_size} - $start;

    $self->_validate_range($start, $len);

    my @result = @{ $self->{_mem} }[$start .. $start + $len - 1];
    return \@result;
}

# ----------------------------------------------------------------------------
# load(\@bytes, $offset) → void
#
# Write a sequence of bytes into memory starting at $offset.
#
# This models the "initial ROM load" or "DMA transfer" operation: loading
# a pre-defined sequence of bytes (e.g., a firmware image, a test fixture)
# into memory before the CPU begins execution.
#
# @param \@bytes  Array ref of byte values (0–255)
# @param $offset  Destination start address (default 0)
# ----------------------------------------------------------------------------
sub load {
    my ($self, $bytes_ref, $offset) = @_;
    $offset //= 0;

    my @bytes = @$bytes_ref;
    $self->_validate_range($offset, scalar @bytes) if @bytes;

    for my $i (0 .. $#bytes) {
        $self->_validate_byte($bytes[$i]);
        $self->{_mem}[$offset + $i] = $bytes[$i] & 0xFF;
    }
}

# ----------------------------------------------------------------------------
# size() → integer
#
# Return the total number of bytes in this block of RAM.
#
# @return integer
# ----------------------------------------------------------------------------
sub size {
    my ($self) = @_;
    return $self->{_size};
}

# ----------------------------------------------------------------------------
# fill($val) → void
#
# Fill all bytes with the given value. Useful for initialising memory to a
# known pattern (e.g., 0xFF "erased" EEPROM, 0x00 zeroed RAM).
#
# @param $val  Byte value (0–255)
# ----------------------------------------------------------------------------
sub fill {
    my ($self, $val) = @_;
    $self->_validate_byte($val);
    $self->{_mem}[$_] = $val for 0 .. $self->{_size} - 1;
}

# ============================================================================
# Private helpers
# ============================================================================

# Validate that $addr is within bounds.
sub _validate_addr {
    my ($self, $addr) = @_;
    die "BlockRam: address $addr out of range [0, @{[$self->{_size}-1]}]"
        if $addr < 0 || $addr >= $self->{_size};
}

# Validate that [$addr .. $addr+$len-1] is entirely within bounds.
sub _validate_range {
    my ($self, $addr, $len) = @_;
    die "BlockRam: address $addr out of range"
        if $addr < 0 || $addr >= $self->{_size};
    die "BlockRam: range [$addr .. @{[$addr+$len-1]}] out of bounds (size=$self->{_size})"
        if $addr + $len > $self->{_size};
}

# Validate that a value is a byte (0–255).
sub _validate_byte {
    my ($self, $val) = @_;
    die "BlockRam: byte value must be 0–255, got $val"
        if !defined $val || $val < 0 || $val > 255;
}

# Convert a byte array to an integer.
#
# Big-endian:    first byte is the most significant
# Little-endian: first byte is the least significant
#
# @param \@bytes  Array of byte values
# @param $endian  'big' or 'little'
# @return integer
sub _bytes_to_int {
    my ($bytes_ref, $endian) = @_;
    my @bytes = @$bytes_ref;
    my $result = 0;

    if ($endian eq BIG) {
        # Big-endian: bytes[0] is MSB
        # result = bytes[0] * 256^(n-1) + bytes[1] * 256^(n-2) + ...
        for my $b (@bytes) {
            $result = ($result << 8) | $b;
        }
    } elsif ($endian eq LITTLE) {
        # Little-endian: bytes[0] is LSB
        # result = bytes[0] + bytes[1] * 256 + bytes[2] * 65536 + ...
        my $shift = 0;
        for my $b (@bytes) {
            $result |= ($b << $shift);
            $shift += 8;
        }
    } else {
        die "BlockRam: unknown endian '$endian' (use 'little' or 'big')";
    }
    return $result;
}

# Convert an integer to a byte array.
#
# @param $val     The integer to encode
# @param $nbytes  Number of bytes to produce
# @param $endian  'big' or 'little'
# @return list of bytes
sub _int_to_bytes {
    my ($val, $nbytes, $endian) = @_;

    # Extract bytes from LSB to MSB (natural order for bit manipulation)
    my @lsb_first;
    my $v = $val;
    for (1 .. $nbytes) {
        push @lsb_first, $v & 0xFF;
        $v >>= 8;
    }

    if ($endian eq LITTLE) {
        # Little-endian: LSB first — already in the right order
        return @lsb_first;
    } elsif ($endian eq BIG) {
        # Big-endian: MSB first — reverse
        return reverse @lsb_first;
    } else {
        die "BlockRam: unknown endian '$endian' (use 'little' or 'big')";
    }
}

1;

__END__

=head1 NAME

CodingAdventures::BlockRam - Block RAM (memory) simulation in Pure Perl

=head1 SYNOPSIS

    use CodingAdventures::BlockRam;

    # Create 256 bytes of RAM
    my $ram = CodingAdventures::BlockRam->new(256);

    # Byte operations
    $ram->write_byte(0, 0xAB);
    my $val = $ram->read_byte(0);  # 0xAB

    # Word operations (default: 4 bytes, little-endian)
    $ram->write_word(4, 0xDEADBEEF, 4, 'little');
    my $word = $ram->read_word(4, 4, 'little');  # 0xDEADBEEF

    # Bulk load
    $ram->load([0x01, 0x02, 0x03], 10);

    # Dump a region
    my $bytes = $ram->dump(10, 3);  # [0x01, 0x02, 0x03]

=head1 DESCRIPTION

Simulates a byte-addressable block of RAM supporting:

=over 4

=item * C<read_byte($addr)> / C<write_byte($addr, $val)>

=item * C<read_word($addr, $nbytes, $endian)> / C<write_word($addr, $val, $nbytes, $endian)>

=item * C<dump($start, $len)> — snapshot a region as an arrayref

=item * C<load(\@bytes, $offset)> — bulk load bytes into memory

=item * C<fill($val)> — fill all memory with a constant byte

=back

Endianness: pass C<'little'> or C<'big'> (default: C<'little'>).

=head1 VERSION

Version 0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
