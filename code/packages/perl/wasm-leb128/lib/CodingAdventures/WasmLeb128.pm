package CodingAdventures::WasmLeb128;

# ============================================================================
# CodingAdventures::WasmLeb128 — LEB128 variable-length integer encoding
# ============================================================================
#
# LEB128 (Little Endian Base 128) is the variable-length integer encoding
# used extensively in WebAssembly binary format, DWARF debug info, and
# Android's DEX files.
#
# The key insight: instead of always using 4 or 8 bytes for an integer, we
# use only as many bytes as needed. Small numbers (0-127) encode in 1 byte;
# larger numbers use more bytes. This compresses typical WebAssembly modules
# significantly.
#
# Encoding scheme:
#   - Each byte contributes 7 bits of the value.
#   - The high bit (bit 7, value 0x80) is a "continuation bit":
#       1 means "more bytes follow"
#       0 means "this is the last byte"
#   - Bytes are ordered LITTLE-ENDIAN: least-significant group first.
#
# Example: encode_unsigned(624485)
#   624485 in binary: 0010 0110 0001 1110 0101  (19 bits)
#   Split into 7-bit groups (LSB first): 1100101  0111100  0100110
#   Set continuation bits:               11100101 10001110 00100110
#   In hex: 0xE5 0x8E 0x26  — three bytes instead of four!
#
# Signed LEB128 uses two's-complement and sign extension. The sign bit is
# the MSB of the last byte's 7-bit payload (bit 6 of the last byte).
#
# Perl-specific notes:
#   - For unsigned: use & 0x7F and >> 7 (unsigned right shift on positives)
#   - For signed: Perl's >> on negative integers IS arithmetic (sign-extends),
#     so (-128 >> 7) == -1, which is exactly what we want.
#   - Use POSIX::floor for signed right-shift alternative if needed.
#
# This module is part of the coding-adventures project, an educational
# computing stack built from logic gates up through interpreters and
# compilers.

use strict;
use warnings;
use Carp qw(croak);
use POSIX qw(floor);
use Exporter 'import';

our $VERSION = '0.01';
our @EXPORT_OK = qw(encode_unsigned encode_signed decode_unsigned decode_signed);

# ---------------------------------------------------------------------------
# encode_unsigned($value) -> @bytes
#
# Encode a non-negative integer as unsigned LEB128. Returns a list of byte
# values (0-255).
#
# Algorithm:
#   Repeat:
#     Take the low 7 bits: $byte = $value & 0x7F
#     Right-shift value by 7: $value >>= 7
#     If more bytes remain ($value != 0), set the continuation bit: $byte |= 0x80
#     Emit $byte
#   Until done.
#
# The minimum encoding length is 1 byte (even for value 0).
# ---------------------------------------------------------------------------
sub encode_unsigned {
    my ($value) = @_;
    croak "encode_unsigned: value must be non-negative" if $value < 0;

    my @bytes;
    do {
        my $byte = $value & 0x7F;    # extract low 7 bits
        $value >>= 7;                # consume those 7 bits
        if ($value != 0) {
            $byte |= 0x80;           # set continuation bit
        }
        push @bytes, $byte;
    } while ($value != 0);

    return @bytes;
}

# ---------------------------------------------------------------------------
# encode_signed($value) -> @bytes
#
# Encode a signed integer as signed LEB128. Returns a list of byte values.
#
# Signed LEB128 uses two's complement. The loop continues until the remaining
# value is either 0 (and the sign bit of the last 7-bit group is 0) or -1
# (and the sign bit of the last 7-bit group is 1).
#
# Sign bit check: the sign bit of a 7-bit two's-complement group is bit 6
# (value 0x40). If the remaining value is 0 and the sign bit is clear,
# or the remaining value is -1 and the sign bit is set, we are done.
#
# Perl note: Perl's >> operator on NEGATIVE integers does NOT perform an
# arithmetic (sign-extending) right shift on 64-bit platforms — instead it
# treats the value as an unsigned integer and shifts logically. For example:
#   (-1 >> 7) gives 144115188075855871, NOT -1.
# To get arithmetic right shift for signed LEB128 we use POSIX::floor:
#   floor(-1 / 128) == -1    ✓
#   floor(-128 / 128) == -1  ✓
# ---------------------------------------------------------------------------
sub encode_signed {
    my ($value) = @_;

    my @bytes;
    my $more = 1;

    while ($more) {
        my $byte = $value & 0x7F;  # low 7 bits
        # Arithmetic right shift using floor-division (correct for negative values)
        $value = POSIX::floor($value / 128);

        # Determine if we're done:
        #   - If remaining value is 0 and the sign bit (bit 6) of $byte is 0
        #     (meaning the number was positive and is now exhausted), stop.
        #   - If remaining value is -1 and the sign bit of $byte is 1
        #     (meaning the number was negative and is now exhausted), stop.
        if ( ($value == 0  && !($byte & 0x40))
          || ($value == -1 &&  ($byte & 0x40)) ) {
            $more = 0;
        }
        else {
            $byte |= 0x80;   # set continuation bit
        }
        push @bytes, $byte;
    }

    return @bytes;
}

# ---------------------------------------------------------------------------
# decode_unsigned(\@bytes, $offset) -> ($value, $bytes_consumed)
#
# Decode unsigned LEB128 from an arrayref of bytes, starting at $offset
# (default 0). Returns the decoded integer value and the number of bytes
# consumed.
#
# Algorithm:
#   For each byte (starting at $offset):
#     Take the low 7 bits and shift them into position: $result |= ($byte & 0x7F) << $shift
#     Increment $shift by 7
#     If the high bit is 0, we're done.
#     Otherwise continue to the next byte.
#
# Dies with "LEB128: unterminated sequence" if the end of the array is
# reached before finding a terminating byte (high bit == 0).
# ---------------------------------------------------------------------------
sub decode_unsigned {
    my ($bytes, $offset) = @_;
    $offset //= 0;

    my $result = 0;
    my $shift  = 0;
    my $count  = 0;

    while (1) {
        my $idx = $offset + $count;
        croak "LEB128: unterminated sequence" if $idx >= scalar(@{$bytes});

        my $byte = $bytes->[$idx];
        $count++;

        $result |= ($byte & 0x7F) << $shift;
        $shift  += 7;

        last unless $byte & 0x80;   # continuation bit clear -> done
    }

    return ($result, $count);
}

# ---------------------------------------------------------------------------
# decode_signed(\@bytes, $offset) -> ($value, $bytes_consumed)
#
# Decode signed LEB128. Same loop as unsigned, but after the loop we check
# whether the sign bit of the last 7-bit group is set; if so, we sign-extend
# to produce a proper negative Perl IV.
#
# Sign extension strategy:
#   If the last byte had bit 6 set (0x40), the encoded value is negative.
#   We use SUBTRACTION rather than bitwise OR to produce a Perl IV (signed
#   integer), because OR with -(1 << $shift) produces a UV (unsigned) bit
#   pattern that Perl represents as a large positive number in string context,
#   which causes Test2 comparisons to fail.
#
#   The subtraction form: $result -= (1 << $shift)
#   This is equivalent to sign extension because:
#     result (unsigned) + (all-ones mask) + 1 = result - 2^shift
#   Example: result=0x7E (126), shift=7 → 126 - 128 = -2  ✓
# ---------------------------------------------------------------------------
sub decode_signed {
    my ($bytes, $offset) = @_;
    $offset //= 0;

    my $result = 0;
    my $shift  = 0;
    my $count  = 0;
    my $byte;

    while (1) {
        my $idx = $offset + $count;
        croak "LEB128: unterminated sequence" if $idx >= scalar(@{$bytes});

        $byte = $bytes->[$idx];
        $count++;

        $result |= ($byte & 0x7F) << $shift;
        $shift  += 7;

        last unless $byte & 0x80;   # continuation bit clear -> done
    }

    # Sign extension: if the sign bit (bit 6 of the last byte) is set,
    # subtract 2^shift to convert the unsigned accumulator to a signed IV.
    if ($byte & 0x40) {
        $result -= (1 << $shift);
    }

    return ($result, $count);
}

1;

__END__

=head1 NAME

CodingAdventures::WasmLeb128 - LEB128 variable-length integer encoding for WebAssembly

=head1 SYNOPSIS

    use CodingAdventures::WasmLeb128;

    # Encode
    my @bytes = CodingAdventures::WasmLeb128::encode_unsigned(624485);
    # => (0xE5, 0x8E, 0x26)

    my @bytes = CodingAdventures::WasmLeb128::encode_signed(-2);
    # => (0x7E)

    # Decode
    my ($value, $count) = CodingAdventures::WasmLeb128::decode_unsigned([0xE5, 0x8E, 0x26]);
    # => (624485, 3)

    my ($value, $count) = CodingAdventures::WasmLeb128::decode_signed([0x7E]);
    # => (-2, 1)

    # With offset (for parsing embedded in a larger byte stream)
    my ($value, $count) = CodingAdventures::WasmLeb128::decode_unsigned(
        [0x00, 0xE5, 0x8E, 0x26], 1
    );
    # => (624485, 3)

=head1 DESCRIPTION

Pure-Perl implementation of LEB128 (Little Endian Base 128) variable-length
integer encoding as used in WebAssembly binary format (§5.2.2).

Both unsigned and signed variants are supported. The signed variant uses
two's-complement representation with sign extension on decode.

=head1 VERSION

Version 0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
