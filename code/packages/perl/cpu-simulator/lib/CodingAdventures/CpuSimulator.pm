package CodingAdventures::CpuSimulator;

# ============================================================================
# CodingAdventures::CpuSimulator — CPU Simulator Building Blocks
# ============================================================================
#
# This module provides the fundamental building blocks of a CPU simulator:
#
#   Memory        — byte-addressable RAM (fixed size, little-endian)
#   SparseMemory  — sparse address space (stores only non-zero locations)
#   RegisterFile  — fast CPU register storage with bit-width masking
#
# WHY DO WE NEED THESE?
#
# A CPU has two types of storage:
#
#   Registers — tiny, ultra-fast storage INSIDE the CPU chip
#               (~16-64 locations, ~0.3 ns access time)
#               All computation happens here.
#
#   Memory    — large, slower storage on a separate chip (DRAM)
#               (~gigabytes, ~100 ns access time)
#               Programs and data live here.
#
# The fetch-decode-execute cycle:
#
#   1. FETCH   — read instruction at PC from memory
#   2. DECODE  — parse the instruction bits
#   3. EXECUTE — run the ALU (uses registers)
#   4. STORE   — write result to register or memory
#   5. ADVANCE — PC += instruction_size
#
# BYTE ORDERING — LITTLE-ENDIAN:
#
#   We use little-endian byte order: the least-significant byte is stored
#   at the lowest address. This matches x86, ARM (LE mode), and RISC-V.
#
#     Address: 0x00  0x01  0x02  0x03
#     Value:   0x78  0x56  0x34  0x12   stores 32-bit value 0x12345678
#
# 32-BIT ARITHMETIC IN PERL:
#
#   Perl's native integers are 64-bit on 64-bit systems. We mask all
#   values to 32 bits using (& 0xFFFFFFFF) when reading words.
#   The bitwise NOT (~) requires masking: (~$x) & 0xFFFFFFFF.

use strict;
use warnings;

our $VERSION = '0.01';

# ============================================================================
# Memory — fixed-size byte-addressable RAM
# ============================================================================

package CodingAdventures::CpuSimulator::Memory;

sub new {
    my ($class, $size) = @_;
    die "memory size must be >= 1" unless $size >= 1;
    return bless {
        size => $size,
        data => [ (0) x $size ],
    }, $class;
}

sub read_byte {
    my ($self, $address) = @_;
    die "memory read out of bounds: address $address, size $self->{size}"
        if $address < 0 || $address >= $self->{size};
    return $self->{data}[$address];
}

sub write_byte {
    my ($self, $address, $value) = @_;
    die "memory write out of bounds: address $address, size $self->{size}"
        if $address < 0 || $address >= $self->{size};
    $self->{data}[$address] = $value & 0xFF;
}

# read_word — reads 4 bytes starting at address, returns a 32-bit little-endian value
#
# Little-endian means the LEAST significant byte is at the lowest address:
#   data[addr+0] = bits 7..0    (LSB)
#   data[addr+1] = bits 15..8
#   data[addr+2] = bits 23..16
#   data[addr+3] = bits 31..24  (MSB)
sub read_word {
    my ($self, $address) = @_;
    my $b0 = $self->read_byte($address);
    my $b1 = $self->read_byte($address + 1);
    my $b2 = $self->read_byte($address + 2);
    my $b3 = $self->read_byte($address + 3);
    return ($b0 | ($b1 << 8) | ($b2 << 16) | ($b3 << 24)) & 0xFFFFFFFF;
}

sub write_word {
    my ($self, $address, $value) = @_;
    my $v = $value & 0xFFFFFFFF;
    $self->write_byte($address,     $v & 0xFF);
    $self->write_byte($address + 1, ($v >> 8)  & 0xFF);
    $self->write_byte($address + 2, ($v >> 16) & 0xFF);
    $self->write_byte($address + 3, ($v >> 24) & 0xFF);
}

sub load_bytes {
    my ($self, $address, $bytes) = @_;
    for my $i (0 .. $#$bytes) {
        $self->write_byte($address + $i, $bytes->[$i]);
    }
}

sub dump {
    my ($self, $start, $length) = @_;
    my @result;
    for my $i (0 .. $length - 1) {
        push @result, $self->read_byte($start + $i);
    }
    return \@result;
}

# ============================================================================
# SparseMemory — sparse address space
# ============================================================================
#
# Stores only non-zero locations in a hash. Ideal for simulating a 4GB
# address space where most memory is zero. Writing 0 removes the entry.

package CodingAdventures::CpuSimulator::SparseMemory;

sub new {
    my ($class, $size) = @_;
    die "sparse memory size must be >= 1" unless $size >= 1;
    return bless {
        size => $size,
        data => {},
    }, $class;
}

sub read_byte {
    my ($self, $address) = @_;
    die "sparse memory read out of bounds: $address"
        if $address < 0 || $address >= $self->{size};
    return $self->{data}{$address} // 0;
}

sub write_byte {
    my ($self, $address, $value) = @_;
    die "sparse memory write out of bounds: $address"
        if $address < 0 || $address >= $self->{size};
    my $v = $value & 0xFF;
    if ($v == 0) {
        delete $self->{data}{$address};
    } else {
        $self->{data}{$address} = $v;
    }
}

sub read_word {
    my ($self, $address) = @_;
    my $b0 = $self->read_byte($address);
    my $b1 = $self->read_byte($address + 1);
    my $b2 = $self->read_byte($address + 2);
    my $b3 = $self->read_byte($address + 3);
    return ($b0 | ($b1 << 8) | ($b2 << 16) | ($b3 << 24)) & 0xFFFFFFFF;
}

sub write_word {
    my ($self, $address, $value) = @_;
    my $v = $value & 0xFFFFFFFF;
    $self->write_byte($address,     $v & 0xFF);
    $self->write_byte($address + 1, ($v >> 8)  & 0xFF);
    $self->write_byte($address + 2, ($v >> 16) & 0xFF);
    $self->write_byte($address + 3, ($v >> 24) & 0xFF);
}

sub load_bytes {
    my ($self, $address, $bytes) = @_;
    for my $i (0 .. $#$bytes) {
        $self->write_byte($address + $i, $bytes->[$i]);
    }
}

sub dump {
    my ($self, $start, $length) = @_;
    my @result;
    for my $i (0 .. $length - 1) {
        push @result, $self->read_byte($start + $i);
    }
    return \@result;
}

# ============================================================================
# RegisterFile — fast CPU register storage
# ============================================================================
#
# Registers are numbered 0 through (num_registers - 1). Writes are masked
# to the configured bit width (default 32 bits).
#
# For 32-bit registers: max value = 0xFFFFFFFF
# For 8-bit registers:  max value = 0xFF

package CodingAdventures::CpuSimulator::RegisterFile;

sub new {
    my ($class, $num_registers, $bit_width) = @_;
    $num_registers //= 16;
    $bit_width     //= 32;
    my $max_value = $bit_width >= 32 ? 0xFFFFFFFF : (1 << $bit_width) - 1;
    return bless {
        num_registers => $num_registers,
        bit_width     => $bit_width,
        max_value     => $max_value,
        values        => [ (0) x $num_registers ],
    }, $class;
}

sub read {
    my ($self, $index) = @_;
    die "register index $index out of range [0, $self->{num_registers})"
        if $index < 0 || $index >= $self->{num_registers};
    return $self->{values}[$index];
}

sub write {
    my ($self, $index, $value) = @_;
    die "register index $index out of range [0, $self->{num_registers})"
        if $index < 0 || $index >= $self->{num_registers};
    $self->{values}[$index] = $value & $self->{max_value};
}

sub num_regs { return $_[0]->{num_registers} }

sub dump {
    my ($self) = @_;
    my %result;
    for my $i (0 .. $self->{num_registers} - 1) {
        $result{"R$i"} = $self->{values}[$i];
    }
    return \%result;
}

# ============================================================================
# Top-level package: convenience constructors
# ============================================================================

package CodingAdventures::CpuSimulator;

sub new_memory        { return CodingAdventures::CpuSimulator::Memory->new($_[1]) }
sub new_sparse_memory { return CodingAdventures::CpuSimulator::SparseMemory->new($_[1]) }
sub new_register_file { return CodingAdventures::CpuSimulator::RegisterFile->new($_[1], $_[2]) }

1;
__END__

=head1 NAME

CodingAdventures::CpuSimulator - CPU simulator building blocks

=head1 SYNOPSIS

    use CodingAdventures::CpuSimulator;

    my $mem = CodingAdventures::CpuSimulator::Memory->new(65536);
    $mem->write_word(0, 0xDEADBEEF);
    printf "0x%08X\n", $mem->read_word(0);  # 0xDEADBEEF

    my $rf = CodingAdventures::CpuSimulator::RegisterFile->new(16, 32);
    $rf->write(0, 42);
    print $rf->read(0);  # 42

=head1 DESCRIPTION

CPU simulator building blocks: Memory, SparseMemory, RegisterFile.

=cut
