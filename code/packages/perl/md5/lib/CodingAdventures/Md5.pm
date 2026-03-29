package CodingAdventures::Md5;

# ============================================================================
# CodingAdventures::Md5 — Pure Perl MD5 cryptographic hash function
# ============================================================================
#
# MD5 (Message-Digest Algorithm 5) produces a 128-bit (16-byte) hash from
# an arbitrary input. Though cryptographically broken for collision
# resistance, it remains widely used for checksums and data integrity checks.
#
# Algorithm overview (RFC 1321):
#
#   1. Pre-processing (padding):
#      Append a single 1-bit (0x80 byte), then zero bytes until the message
#      length in bits ≡ 448 (mod 512). Finally append the original message
#      length as a 64-bit little-endian integer. The padded message is a
#      multiple of 512 bits (64 bytes).
#
#   2. Initialize four 32-bit state registers:
#      A = 0x67452301
#      B = 0xEFCDAB89
#      C = 0x98BADCFE
#      D = 0x10325476
#
#   3. Process each 512-bit (16-word) chunk:
#      Run 64 rounds divided into four groups of 16. Each round mixes one
#      message word with the state registers using one of four auxiliary
#      functions (F, G, H, I), a precomputed sine-derived constant T[i],
#      and a left-rotation amount s[i].
#
#   4. Add the chunk's output back into the running state (Davies–Meyer).
#
#   5. Produce the 16-byte digest by concatenating A, B, C, D in
#      little-endian byte order.
#
# Perl-specific notes:
#   - Perl integers are 64-bit on modern platforms; all 32-bit arithmetic
#     must be masked with & 0xFFFFFFFF after each addition.
#   - pack("V", $n) packs a 32-bit little-endian unsigned integer.
#   - pack("N", $n) packs a 32-bit BIG-endian unsigned integer.
#   - unpack("C*", $str) unpacks bytes as unsigned 8-bit integers.
#
# This module is part of the coding-adventures project, an educational
# computing stack built from logic gates up through interpreters and
# compilers.

use strict;
use warnings;

our $VERSION = '0.01';

# ---------------------------------------------------------------------------
# MD5 per-round left-rotation amounts (s-table, RFC 1321 §3.4)
#
# There are 64 rounds. Each round uses a fixed left-shift amount:
#   Rounds  1-16: s = [7, 12, 17, 22] repeated 4 times
#   Rounds 17-32: s = [5,  9, 14, 20] repeated 4 times
#   Rounds 33-48: s = [4, 11, 16, 23] repeated 4 times
#   Rounds 49-64: s = [6, 10, 15, 21] repeated 4 times
# ---------------------------------------------------------------------------
my @S = (
    7, 12, 17, 22,  7, 12, 17, 22,  7, 12, 17, 22,  7, 12, 17, 22,
    5,  9, 14, 20,  5,  9, 14, 20,  5,  9, 14, 20,  5,  9, 14, 20,
    4, 11, 16, 23,  4, 11, 16, 23,  4, 11, 16, 23,  4, 11, 16, 23,
    6, 10, 15, 21,  6, 10, 15, 21,  6, 10, 15, 21,  6, 10, 15, 21,
);

# ---------------------------------------------------------------------------
# MD5 additive constants K[i] (RFC 1321 §3.4)
#
# K[i] = floor(2^32 * |sin(i+1)|) for i = 0..63
# These constants add "random-looking" bits to break symmetry.
# ---------------------------------------------------------------------------
my @K = (
    0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee,
    0xf57c0faf, 0x4787c62a, 0xa8304613, 0xfd469501,
    0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be,
    0x6b901122, 0xfd987193, 0xa679438e, 0x49b40821,
    0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa,
    0xd62f105d, 0x02441453, 0xd8a1e681, 0xe7d3fbc8,
    0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed,
    0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a,
    0xfffa3942, 0x8771f681, 0x6d9d6122, 0xfde5380c,
    0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70,
    0x289b7ec6, 0xeaa127fa, 0xd4ef3085, 0x04881d05,
    0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665,
    0xf4292244, 0x432aff97, 0xab9423a7, 0xfc93a039,
    0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
    0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1,
    0xf7537e82, 0xbd3af235, 0x2ad7d2bb, 0xeb86d391,
);

# ---------------------------------------------------------------------------
# _left_rotate($x, $n)
#
# Rotate the 32-bit value $x left by $n bits. This is the circular left
# shift used in every MD5 round. Because Perl integers are wider than 32
# bits we mask the result to 32 bits.
#
# Diagram:
#   Original: [ b31 b30 ... b1 b0 ]
#   After rot-left by 1: [ b30 ... b1 b0 b31 ]
# ---------------------------------------------------------------------------
sub _left_rotate {
    my ($x, $n) = @_;
    return ( ($x << $n) | ( ($x & 0xFFFFFFFF) >> (32 - $n) ) ) & 0xFFFFFFFF;
}

# ---------------------------------------------------------------------------
# _md5_block(\@state, \@words)
#
# Process one 512-bit (16-word) block. @state = (A, B, C, D). @words is
# the 16 little-endian 32-bit words from the current block.
#
# The four auxiliary functions encode different boolean combinations:
#   F(B,C,D) = (B & C) | (~B & D)  — "if B then C else D"  (rounds 1-16)
#   G(B,C,D) = (B & D) | (C & ~D)  — "if D then B else C"  (rounds 17-32)
#   H(B,C,D) = B ^ C ^ D           — XOR parity              (rounds 33-48)
#   I(B,C,D) = C ^ (B | ~D)        — ~D or B then XOR C     (rounds 49-64)
# ---------------------------------------------------------------------------
sub _md5_block {
    my ($state, $words) = @_;
    my ($A, $B, $C, $D) = @{$state};

    for my $i ( 0 .. 63 ) {
        my ($F, $g);
        if ( $i < 16 ) {
            # Round 1: F(B,C,D) = (B & C) | (~B & D)
            # Mask ~$B to 32 bits: Perl's ~ operator produces a 64-bit result
            # on 64-bit platforms, so we must AND with 0xFFFFFFFF after NOT.
            $F = ($B & $C) | ((~$B & 0xFFFFFFFF) & $D);
            $g = $i;
        }
        elsif ( $i < 32 ) {
            # Round 2: G(B,C,D) = (B & D) | (C & ~D)
            $F = ($B & $D) | ($C & (~$D & 0xFFFFFFFF));
            $g = (5 * $i + 1) % 16;
        }
        elsif ( $i < 48 ) {
            # Round 3: H(B,C,D) = B ^ C ^ D  (no complement needed)
            $F = $B ^ $C ^ $D;
            $g = (3 * $i + 5) % 16;
        }
        else {
            # Round 4: I(B,C,D) = C ^ (B | ~D)
            $F = $C ^ ($B | (~$D & 0xFFFFFFFF));
            $g = (7 * $i) % 16;
        }

        # All arithmetic is modulo 2^32; mask after each add
        $F = ($F + $A + $K[$i] + $words->[$g]) & 0xFFFFFFFF;
        $A = $D;
        $D = $C;
        $C = $B;
        $B = ($B + _left_rotate($F, $S[$i])) & 0xFFFFFFFF;
    }

    # Davies–Meyer construction: add the block result into the running state
    $state->[0] = ($state->[0] + $A) & 0xFFFFFFFF;
    $state->[1] = ($state->[1] + $B) & 0xFFFFFFFF;
    $state->[2] = ($state->[2] + $C) & 0xFFFFFFFF;
    $state->[3] = ($state->[3] + $D) & 0xFFFFFFFF;
}

# ---------------------------------------------------------------------------
# digest($message) -> \@bytes   (16-element arrayref of 0-255 integers)
#
# Compute the MD5 of $message and return the raw 16 bytes as an arrayref.
# ---------------------------------------------------------------------------
sub digest {
    my (undef, $message) = @_;  # ignore invocant (works as both class/func)
    # Allow calling as CodingAdventures::Md5::digest($msg) without OO
    if ( ref(\$_[0]) eq 'SCALAR' && !ref($_[0]) && $_[0] ne __PACKAGE__ ) {
        # Called as a plain function; shift didn't happen correctly above
        # This case is handled by the check below
    }

    # Normalize: if called as Md5->digest($msg), $_[0] is class name
    # If called as Md5::digest($msg), $_[0] is the message itself.
    # We handle both by checking if the first arg is our package name.
    my $msg;
    if ( @_ == 2 ) {
        $msg = $_[1];   # OO call: (class/obj, message)
    }
    else {
        $msg = $_[0];   # function call: (message)
    }
    $msg //= '';

    # ------------------------------------------------------------------
    # Step 1: Padding
    #
    # The message must be padded so that its bit-length ≡ 448 (mod 512),
    # i.e., byte-length ≡ 56 (mod 64).
    #
    # Append 0x80 (a 1-bit followed by zeros), then enough 0x00 bytes,
    # then the original length as a 64-bit little-endian integer.
    # ------------------------------------------------------------------
    my $original_bit_len = length($msg) * 8;

    $msg .= "\x80";   # append the single 1-bit

    # Pad with zeros until length ≡ 56 (mod 64)
    my $pad_bytes = (56 - length($msg) % 64) % 64;
    $msg .= "\x00" x $pad_bytes;

    # Append original length as 64-bit little-endian.
    # pack("V V", lo32, hi32) gives two 32-bit little-endian words.
    my $lo = $original_bit_len & 0xFFFFFFFF;
    my $hi = int($original_bit_len / 2**32) & 0xFFFFFFFF;
    $msg .= pack("VV", $lo, $hi);

    # ------------------------------------------------------------------
    # Step 2: Initialize state registers (RFC 1321 §3.3)
    # ------------------------------------------------------------------
    my @state = (0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476);

    # ------------------------------------------------------------------
    # Step 3: Process each 64-byte block
    # ------------------------------------------------------------------
    my $num_blocks = length($msg) / 64;
    for my $b ( 0 .. $num_blocks - 1 ) {
        # Unpack 16 little-endian 32-bit words from this block
        my @words = unpack("V16", substr($msg, $b * 64, 64));
        _md5_block(\@state, \@words);
    }

    # ------------------------------------------------------------------
    # Step 4: Produce the 16-byte digest
    #
    # Concatenate the four state words in little-endian byte order.
    # ------------------------------------------------------------------
    my $raw = pack("V4", @state);
    return [ unpack("C*", $raw) ];
}

# ---------------------------------------------------------------------------
# hex($message) -> $hex_string   (32 lowercase hex characters)
#
# Convenience wrapper around digest() that returns the familiar hex string.
# ---------------------------------------------------------------------------
sub hex {
    my @args = @_;
    my $bytes = digest(@args);
    return join('', map { sprintf('%02x', $_) } @{$bytes});
}

# ---------------------------------------------------------------------------
# new(\%opts) — constructor for OO usage
#
# The object stores no mutable state for MD5 (each hash is independent),
# but OO style can be convenient for polymorphism.
# ---------------------------------------------------------------------------
sub new {
    my ($class, $opts) = @_;
    return bless {}, $class;
}

1;

__END__

=head1 NAME

CodingAdventures::Md5 - Pure Perl MD5 cryptographic hash function

=head1 SYNOPSIS

    use CodingAdventures::Md5;

    # Functional interface
    my $hex   = CodingAdventures::Md5::hex("hello");
    # => "5d41402abc4b2a76b9719d911017c592"

    my $bytes = CodingAdventures::Md5::digest("hello");
    # => arrayref of 16 integers (0-255)

    # OO interface
    my $md5 = CodingAdventures::Md5->new();
    my $hex = $md5->hex("hello");

=head1 DESCRIPTION

A pure-Perl implementation of the MD5 message-digest algorithm (RFC 1321).
No XS or external modules are required. MD5 is no longer considered
cryptographically secure for collision resistance, but this implementation
is valuable as a learning exercise and for legacy checksums.

=head1 VERSION

Version 0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
