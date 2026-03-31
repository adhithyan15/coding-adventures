package CodingAdventures::CpuSimulator;
# =============================================================================
# CodingAdventures::CpuSimulator — CPU simulator building blocks
# =============================================================================
#
# Packages:
#   CodingAdventures::CpuSimulator::Memory        — dense array-backed RAM
#   CodingAdventures::CpuSimulator::SparseMemory  — hash-backed sparse RAM
#   CodingAdventures::CpuSimulator::RegisterFile  — configurable register file

use strict;
use warnings;

our $VERSION = '0.01';

1;

# =============================================================================

package CodingAdventures::CpuSimulator::Memory;
# -----------------------------------------------------------------------------
# Byte-addressable dense RAM.
# All multi-byte accesses use LITTLE-ENDIAN byte order.
# -----------------------------------------------------------------------------

use strict;
use warnings;
use Carp qw(croak);

sub new {
    my ($class, $size) = @_;
    croak "size must be >= 1" unless defined $size && $size >= 1;
    return bless {
        size => $size,
        data => [(0) x $size],
    }, $class;
}

sub read_byte {
    my ($self, $addr) = @_;
    croak sprintf("read_byte: address 0x%X out of range [0, %d)", $addr, $self->{size})
        if $addr < 0 || $addr >= $self->{size};
    return $self->{data}[$addr];
}

sub write_byte {
    my ($self, $addr, $val) = @_;
    croak sprintf("write_byte: address 0x%X out of range [0, %d)", $addr, $self->{size})
        if $addr < 0 || $addr >= $self->{size};
    $self->{data}[$addr] = $val & 0xFF;
}

sub read_word {
    my ($self, $addr) = @_;
    my $b0 = $self->read_byte($addr);
    my $b1 = $self->read_byte($addr + 1);
    my $b2 = $self->read_byte($addr + 2);
    my $b3 = $self->read_byte($addr + 3);
    return ($b0 | ($b1 << 8) | ($b2 << 16) | ($b3 << 24)) & 0xFFFFFFFF;
}

sub write_word {
    my ($self, $addr, $val) = @_;
    $val &= 0xFFFFFFFF;
    $self->write_byte($addr,     $val & 0xFF);
    $self->write_byte($addr + 1, ($val >> 8)  & 0xFF);
    $self->write_byte($addr + 2, ($val >> 16) & 0xFF);
    $self->write_byte($addr + 3, ($val >> 24) & 0xFF);
}

sub load_bytes {
    my ($self, $addr, $bytes) = @_;
    for my $i (0 .. $#$bytes) {
        $self->write_byte($addr + $i, $bytes->[$i]);
    }
}

sub dump {
    my ($self, $start, $length) = @_;
    return [ map { $self->read_byte($start + $_) } 0 .. $length - 1 ];
}

1;

# =============================================================================

package CodingAdventures::CpuSimulator::SparseMemory;
# -----------------------------------------------------------------------------
# Hash-backed sparse memory.  Unwritten addresses return 0.
# Writing 0 removes the entry to keep the hash sparse.
# -----------------------------------------------------------------------------

use strict;
use warnings;

sub new {
    my ($class) = @_;
    return bless { data => {} }, $class;
}

sub read_byte {
    my ($self, $addr) = @_;
    return $self->{data}{$addr} // 0;
}

sub write_byte {
    my ($self, $addr, $val) = @_;
    $val &= 0xFF;
    if ($val == 0) {
        delete $self->{data}{$addr};
    } else {
        $self->{data}{$addr} = $val;
    }
}

sub read_word {
    my ($self, $addr) = @_;
    my $b0 = $self->read_byte($addr);
    my $b1 = $self->read_byte($addr + 1);
    my $b2 = $self->read_byte($addr + 2);
    my $b3 = $self->read_byte($addr + 3);
    return ($b0 | ($b1 << 8) | ($b2 << 16) | ($b3 << 24)) & 0xFFFFFFFF;
}

sub write_word {
    my ($self, $addr, $val) = @_;
    $val &= 0xFFFFFFFF;
    $self->write_byte($addr,     $val & 0xFF);
    $self->write_byte($addr + 1, ($val >> 8)  & 0xFF);
    $self->write_byte($addr + 2, ($val >> 16) & 0xFF);
    $self->write_byte($addr + 3, ($val >> 24) & 0xFF);
}

sub load_bytes {
    my ($self, $addr, $bytes) = @_;
    for my $i (0 .. $#$bytes) {
        $self->write_byte($addr + $i, $bytes->[$i]);
    }
}

sub dump {
    my ($self, $start, $length) = @_;
    return [ map { $self->read_byte($start + $_) } 0 .. $length - 1 ];
}

1;

# =============================================================================

package CodingAdventures::CpuSimulator::RegisterFile;
# -----------------------------------------------------------------------------
# General-purpose register file.
#
# Registers are 0-indexed.  Writes are masked to bit_width bits.
# All registers initialised to 0.
# -----------------------------------------------------------------------------

use strict;
use warnings;
use Carp qw(croak);

sub new {
    my ($class, $num_regs, $bit_width) = @_;
    $num_regs  //= 16;
    $bit_width //= 32;

    # Compute mask.  Perl integers are at least 64-bit on modern systems.
    my $max_value;
    if ($bit_width >= 64) {
        $max_value = 0xFFFF_FFFF_FFFF_FFFF;
    } else {
        $max_value = (1 << $bit_width) - 1;
    }

    return bless {
        num_registers => $num_regs,
        bit_width     => $bit_width,
        max_value     => $max_value,
        values        => [(0) x $num_regs],
    }, $class;
}

sub read {
    my ($self, $idx) = @_;
    croak sprintf("read: index %d out of range [0, %d)", $idx, $self->{num_registers})
        if $idx < 0 || $idx >= $self->{num_registers};
    return $self->{values}[$idx];
}

sub write {
    my ($self, $idx, $val) = @_;
    croak sprintf("write: index %d out of range [0, %d)", $idx, $self->{num_registers})
        if $idx < 0 || $idx >= $self->{num_registers};
    $self->{values}[$idx] = $val & $self->{max_value};
}

sub dump {
    my ($self) = @_;
    my %result;
    for my $i (0 .. $self->{num_registers} - 1) {
        $result{ "R$i" } = $self->{values}[$i];
    }
    return \%result;
}

1;
