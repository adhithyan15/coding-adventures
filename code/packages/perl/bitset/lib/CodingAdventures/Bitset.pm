package CodingAdventures::Bitset;

# ============================================================================
# CodingAdventures::Bitset — Compact boolean bitset packed into 32-bit words
# ============================================================================
#
# A bitset (also called a bit array or bitmap) stores a sequence of boolean
# values in a highly compact form: one bit per element, rather than one byte
# (or more) per element.
#
# Internal representation:
#   We store bits in an array of 32-bit unsigned integers (Perl IV masked to
#   32 bits). Each element of @words holds 32 consecutive bits.
#
#   For bit index $i:
#     word index  = int($i / 32)        ← which element of @words
#     bit offset  = $i % 32             ← which bit within that word (LSB = 0)
#     mask        = 1 << bit_offset
#
#   Memory layout for a 100-bit bitset: 4 words (⌈100/32⌉ = 4).
#
#   Bit 0   lives in words[0], mask 0x00000001
#   Bit 31  lives in words[0], mask 0x80000000
#   Bit 32  lives in words[1], mask 0x00000001
#   Bit 99  lives in words[3], mask 0x00000008
#
# API design: immutable/functional style
#   set() and clear() return a new Bitset object (the original is unchanged).
#   This makes it safe to use bitsets as values in maps and pipelines.
#
# Popcount (counting set bits) uses Brian Kernighan's algorithm:
#   Each iteration clears the lowest set bit with: $n &= ($n - 1)
#   Count how many iterations until $n == 0. That count is the popcount.
#
# This module is part of the coding-adventures project, an educational
# computing stack built from logic gates up through interpreters and
# compilers.

use strict;
use warnings;
use Carp qw(croak confess);

our $VERSION = '0.01';

use constant BITS_PER_WORD => 32;

# ---------------------------------------------------------------------------
# new($n_bits) -> $bitset
#
# Create a new bitset with capacity for $n_bits bits, all initialized to 0.
# $n_bits must be a positive integer.
#
# The number of 32-bit words needed is ⌈$n_bits / 32⌉.
# ---------------------------------------------------------------------------
sub new {
    my ($class, $n_bits) = @_;
    croak "Bitset::new requires a positive integer" unless defined $n_bits && $n_bits > 0;
    my $num_words = int(($n_bits + BITS_PER_WORD - 1) / BITS_PER_WORD);
    return bless {
        words => [(0) x $num_words],
        len   => $n_bits,
    }, $class;
}

# ---------------------------------------------------------------------------
# _clone($self) -> $new_bitset
#
# Internal helper: return a deep copy of the bitset.
# ---------------------------------------------------------------------------
sub _clone {
    my ($self) = @_;
    return bless {
        words => [@{ $self->{words} }],   # copy the array
        len   => $self->{len},
    }, ref($self);
}

# ---------------------------------------------------------------------------
# _check_index($self, $i)
#
# Die if $i is out of range [0, len).
# ---------------------------------------------------------------------------
sub _check_index {
    my ($self, $i) = @_;
    croak "Bitset: index $i out of range [0, $self->{len})"
        unless defined $i && $i >= 0 && $i < $self->{len};
}

# ---------------------------------------------------------------------------
# set($i) -> $new_bitset
#
# Return a new bitset identical to this one but with bit $i set to 1.
# Purely functional: the original bitset is not modified.
# ---------------------------------------------------------------------------
sub set {
    my ($self, $i) = @_;
    $self->_check_index($i);
    my $copy = $self->_clone();
    my $word = int($i / BITS_PER_WORD);
    my $off  = $i % BITS_PER_WORD;
    $copy->{words}[$word] = ($copy->{words}[$word] | (1 << $off)) & 0xFFFFFFFF;
    return $copy;
}

# ---------------------------------------------------------------------------
# clear($i) -> $new_bitset
#
# Return a new bitset with bit $i cleared (set to 0).
# ---------------------------------------------------------------------------
sub clear {
    my ($self, $i) = @_;
    $self->_check_index($i);
    my $copy = $self->_clone();
    my $word = int($i / BITS_PER_WORD);
    my $off  = $i % BITS_PER_WORD;
    # AND with the bitwise complement of the mask to clear just this bit.
    # We keep the result 32-bit.
    my $mask = (1 << $off) & 0xFFFFFFFF;
    $copy->{words}[$word] = ($copy->{words}[$word] & ~$mask) & 0xFFFFFFFF;
    return $copy;
}

# ---------------------------------------------------------------------------
# test($i) -> 0 or 1
#
# Return 1 if bit $i is set, 0 otherwise.
# ---------------------------------------------------------------------------
sub test {
    my ($self, $i) = @_;
    $self->_check_index($i);
    my $word = int($i / BITS_PER_WORD);
    my $off  = $i % BITS_PER_WORD;
    return ($self->{words}[$word] >> $off) & 1;
}

# ---------------------------------------------------------------------------
# popcount() -> $count
#
# Count the number of bits that are set to 1.
#
# Uses Brian Kernighan's algorithm: the expression $n &= ($n-1) clears the
# lowest set bit. Count how many times we can do this before $n reaches 0.
#
# Example: popcount(0b1010) = 2
#   iter1: 0b1010 & 0b1001 = 0b1000  (cleared bit 1)
#   iter2: 0b1000 & 0b0111 = 0b0000  (cleared bit 3) → done, count = 2
# ---------------------------------------------------------------------------
sub popcount {
    my ($self) = @_;
    my $count = 0;
    for my $w (@{ $self->{words} }) {
        my $n = $w & 0xFFFFFFFF;
        while ($n) {
            $n &= ($n - 1);   # clear the lowest set bit
            $count++;
        }
    }
    return $count;
}

# ---------------------------------------------------------------------------
# size() -> $n_bits
#
# Return the capacity of this bitset (the $n_bits passed to new()).
# ---------------------------------------------------------------------------
sub size {
    my ($self) = @_;
    return $self->{len};
}

# ---------------------------------------------------------------------------
# set_bits() -> @indices
#
# Return a sorted list of indices where bits are set to 1.
# Iterates only over valid indices (0 .. len-1), not padding bits.
# ---------------------------------------------------------------------------
sub set_bits {
    my ($self) = @_;
    my @result;
    for my $i (0 .. $self->{len} - 1) {
        my $word = int($i / BITS_PER_WORD);
        my $off  = $i % BITS_PER_WORD;
        if (($self->{words}[$word] >> $off) & 1) {
            push @result, $i;
        }
    }
    return @result;
}

# ---------------------------------------------------------------------------
# _bitwise_op($self, $other, $op) -> $new_bitset
#
# Internal helper for AND, OR, XOR. Applies $op to each pair of words.
# Both bitsets must have the same len; dies otherwise.
# ---------------------------------------------------------------------------
sub _bitwise_op {
    my ($self, $other, $op) = @_;
    croak "Bitset: size mismatch ($self->{len} vs $other->{len})"
        unless $self->{len} == $other->{len};

    my $result = $self->_clone();
    my $nw = scalar @{ $self->{words} };
    for my $i (0 .. $nw - 1) {
        if ($op eq 'and') {
            $result->{words}[$i] = ($self->{words}[$i] & $other->{words}[$i]) & 0xFFFFFFFF;
        }
        elsif ($op eq 'or') {
            $result->{words}[$i] = ($self->{words}[$i] | $other->{words}[$i]) & 0xFFFFFFFF;
        }
        elsif ($op eq 'xor') {
            $result->{words}[$i] = ($self->{words}[$i] ^ $other->{words}[$i]) & 0xFFFFFFFF;
        }
    }
    return $result;
}

# ---------------------------------------------------------------------------
# bitwise_and($other) -> $new_bitset
#
# Return a new bitset where bit i = (self[i] AND other[i]).
# Both bitsets must have the same size.
# ---------------------------------------------------------------------------
sub bitwise_and {
    my ($self, $other) = @_;
    return $self->_bitwise_op($other, 'and');
}

# ---------------------------------------------------------------------------
# bitwise_or($other) -> $new_bitset
#
# Return a new bitset where bit i = (self[i] OR other[i]).
# ---------------------------------------------------------------------------
sub bitwise_or {
    my ($self, $other) = @_;
    return $self->_bitwise_op($other, 'or');
}

# ---------------------------------------------------------------------------
# bitwise_xor($other) -> $new_bitset
#
# Return a new bitset where bit i = (self[i] XOR other[i]).
# ---------------------------------------------------------------------------
sub bitwise_xor {
    my ($self, $other) = @_;
    return $self->_bitwise_op($other, 'xor');
}

1;

__END__

=head1 NAME

CodingAdventures::Bitset - Compact boolean bitset packed into 32-bit integer words

=head1 SYNOPSIS

    use CodingAdventures::Bitset;

    my $bs = CodingAdventures::Bitset->new(100);  # 100-bit bitset, all zero

    $bs = $bs->set(42);    # returns new bitset with bit 42 set
    $bs = $bs->set(0);
    $bs = $bs->set(99);

    $bs->test(42);         # => 1
    $bs->test(1);          # => 0
    $bs->popcount();       # => 3

    $bs = $bs->clear(42);  # returns new bitset with bit 42 cleared

    my @indices = $bs->set_bits();  # => (0, 99)
    my $size    = $bs->size();      # => 100

    # Bitwise operations (same-size bitsets)
    my $a = CodingAdventures::Bitset->new(8)->set(0)->set(1);
    my $b = CodingAdventures::Bitset->new(8)->set(1)->set(2);
    my $and = $a->bitwise_and($b);  # bit 1 set
    my $or  = $a->bitwise_or($b);   # bits 0,1,2 set
    my $xor = $a->bitwise_xor($b);  # bits 0,2 set

=head1 DESCRIPTION

An immutable bitset (bit array) backed by an array of 32-bit integers.
Each method that modifies the bitset returns a new object, leaving the
original unchanged. Useful for set operations, bit flags, and Bloom filters.

=head1 VERSION

Version 0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
