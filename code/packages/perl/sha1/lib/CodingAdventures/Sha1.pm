package CodingAdventures::Sha1;

# ============================================================================
# CodingAdventures::Sha1 — Pure Perl SHA-1 cryptographic hash function
# ============================================================================
#
# SHA-1 (Secure Hash Algorithm 1) produces a 160-bit (20-byte) digest from
# an arbitrary-length input. Defined in FIPS PUB 180-4. While SHA-1 is
# deprecated for collision-sensitive uses (Google's SHAttered attack, 2017),
# it remains widely deployed in HMAC, TLS, and Git's object model.
#
# Algorithm overview:
#
#   1. Pre-processing (padding) — same structure as MD5 but BIG-endian:
#      Append 0x80, pad with zeros until bit-length ≡ 448 (mod 512), then
#      append the original 64-bit bit-length as BIG-endian. The padded
#      message is a multiple of 512 bits (64 bytes).
#
#   2. Initialize five 32-bit state words:
#      H0 = 0x67452301
#      H1 = 0xEFCDAB89
#      H2 = 0x98BADCFE
#      H3 = 0x10325476
#      H4 = 0xC3D2E1F0
#
#   3. For each 512-bit chunk, build an 80-word message schedule W[0..79]:
#      - W[0..15]   = the 16 big-endian 32-bit words of the chunk
#      - W[t] = ROTL1(W[t-3] ^ W[t-8] ^ W[t-14] ^ W[t-16])  for t=16..79
#
#   4. Run 80 rounds in four groups of 20, each with a different auxiliary
#      function (Ch, Parity, Maj, Parity) and constant (K0..K3).
#
#   5. Add the chunk result back into H0..H4.
#
#   6. Output: pack the five state words big-endian — that is the 20-byte digest.
#
# Perl-specific notes:
#   - All 32-bit arithmetic must be masked with & 0xFFFFFFFF.
#   - Left rotate: (($x << $n) | (($x & 0xFFFFFFFF) >> (32-$n))) & 0xFFFFFFFF
#   - pack("N*", @words) packs 32-bit big-endian unsigned integers.
#   - unpack("N*", $str) unpacks 32-bit big-endian words.
#
# This module is part of the coding-adventures project, an educational
# computing stack built from logic gates up through interpreters and
# compilers.

use strict;
use warnings;

our $VERSION = '0.01';

# ---------------------------------------------------------------------------
# SHA-1 round constants (FIPS 180-4 §4.2.1)
#
#   K0 = 0x5A827999  (rounds  0-19) — sqrt(2) * 2^30
#   K1 = 0x6ED9EBA1  (rounds 20-39) — sqrt(3) * 2^30
#   K2 = 0x8F1BBCDC  (rounds 40-59) — sqrt(5) * 2^30
#   K3 = 0xCA62C1D6  (rounds 60-79) — sqrt(10)* 2^30
# ---------------------------------------------------------------------------
my @K = (0x5A827999, 0x6ED9EBA1, 0x8F1BBCDC, 0xCA62C1D6);

# ---------------------------------------------------------------------------
# _rotl($x, $n) — rotate left 32-bit value $x by $n bits
#
# SHA-1 uses 5-bit left rotations. The right-shift component recovers the
# bits that "fall off" the left end and wraps them to the right.
# ---------------------------------------------------------------------------
sub _rotl {
    my ($x, $n) = @_;
    return ( ($x << $n) | ( ($x & 0xFFFFFFFF) >> (32 - $n) ) ) & 0xFFFFFFFF;
}

# ---------------------------------------------------------------------------
# _sha1_block(\@state, $chunk_bytes)
#
# Process one 64-byte block. @state = (H0..H4). $chunk_bytes is a 64-byte
# substring of the padded message.
#
# SHA-1 auxiliary functions by round group:
#   0-19  Ch(B,C,D)     = (B & C) | (~B & D)  — "choice"
#   20-39 Parity(B,C,D) = B ^ C ^ D            — XOR parity
#   40-59 Maj(B,C,D)    = (B & C) | (B & D) | (C & D)  — "majority"
#   60-79 Parity(B,C,D) = B ^ C ^ D            — XOR parity again
# ---------------------------------------------------------------------------
sub _sha1_block {
    my ($state, $chunk) = @_;

    # Build the 80-word message schedule
    my @W = unpack("N16", $chunk);  # W[0..15] from the chunk (big-endian)
    for my $t ( 16 .. 79 ) {
        # Message expansion: XOR four previous words and rotate left 1
        $W[$t] = _rotl($W[$t-3] ^ $W[$t-8] ^ $W[$t-14] ^ $W[$t-16], 1);
    }

    my ($A, $B, $C, $D, $E) = @{$state};

    for my $t ( 0 .. 79 ) {
        my $f;
        my $k;
        if ( $t < 20 ) {
            $f = ($B & $C) | ((~$B) & $D);  # Ch
            $k = $K[0];
        }
        elsif ( $t < 40 ) {
            $f = $B ^ $C ^ $D;              # Parity
            $k = $K[1];
        }
        elsif ( $t < 60 ) {
            $f = ($B & $C) | ($B & $D) | ($C & $D);  # Maj
            $k = $K[2];
        }
        else {
            $f = $B ^ $C ^ $D;              # Parity
            $k = $K[3];
        }

        my $temp = (_rotl($A, 5) + $f + $E + $k + $W[$t]) & 0xFFFFFFFF;
        $E = $D;
        $D = $C;
        $C = _rotl($B, 30);
        $B = $A;
        $A = $temp;
    }

    # Accumulate: add block result back into running state
    $state->[0] = ($state->[0] + $A) & 0xFFFFFFFF;
    $state->[1] = ($state->[1] + $B) & 0xFFFFFFFF;
    $state->[2] = ($state->[2] + $C) & 0xFFFFFFFF;
    $state->[3] = ($state->[3] + $D) & 0xFFFFFFFF;
    $state->[4] = ($state->[4] + $E) & 0xFFFFFFFF;
}

# ---------------------------------------------------------------------------
# digest($message) -> \@bytes   (20-element arrayref of 0-255 integers)
#
# Compute SHA-1 of $message. Can be called as:
#   CodingAdventures::Sha1::digest($msg)     — functional
#   CodingAdventures::Sha1->digest($msg)     — class method
#   $sha1->digest($msg)                      — instance method
# ---------------------------------------------------------------------------
sub digest {
    my $msg;
    if ( @_ == 2 ) {
        $msg = $_[1];   # OO call: (class/object, message)
    }
    else {
        $msg = $_[0];   # function call: (message)
    }
    $msg //= '';

    # ------------------------------------------------------------------
    # Step 1: Padding (big-endian, same structure as MD5)
    #
    # Append 0x80, then zero bytes until length ≡ 56 (mod 64), then the
    # original 64-bit bit-length in big-endian order.
    # ------------------------------------------------------------------
    my $original_len   = length($msg);
    my $original_bits  = $original_len * 8;

    $msg .= "\x80";

    my $pad = (56 - length($msg) % 64) % 64;
    $msg .= "\x00" x $pad;

    # Append 64-bit big-endian length. Perl's integers are 64-bit, but
    # pack("N") only handles 32 bits, so we split into two 32-bit words.
    my $hi = int($original_bits / 2**32) & 0xFFFFFFFF;
    my $lo = $original_bits & 0xFFFFFFFF;
    $msg .= pack("NN", $hi, $lo);

    # ------------------------------------------------------------------
    # Step 2: Initialize state (FIPS 180-4 §6.1)
    # ------------------------------------------------------------------
    my @state = (0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0);

    # ------------------------------------------------------------------
    # Step 3: Process each 64-byte block
    # ------------------------------------------------------------------
    my $num_blocks = length($msg) / 64;
    for my $b ( 0 .. $num_blocks - 1 ) {
        _sha1_block(\@state, substr($msg, $b * 64, 64));
    }

    # ------------------------------------------------------------------
    # Step 4: Produce the 20-byte digest (big-endian)
    # ------------------------------------------------------------------
    my $raw = pack("N5", @state);
    return [ unpack("C*", $raw) ];
}

# ---------------------------------------------------------------------------
# hex($message) -> $hex_string   (40 lowercase hex characters)
# ---------------------------------------------------------------------------
sub hex {
    my @args = @_;
    my $bytes = digest(@args);
    return join('', map { sprintf('%02x', $_) } @{$bytes});
}

# ---------------------------------------------------------------------------
# new() — constructor for OO usage
# ---------------------------------------------------------------------------
sub new {
    my ($class) = @_;
    return bless {}, $class;
}

1;

__END__

=head1 NAME

CodingAdventures::Sha1 - Pure Perl SHA-1 cryptographic hash function

=head1 SYNOPSIS

    use CodingAdventures::Sha1;

    # Functional interface
    my $hex   = CodingAdventures::Sha1::hex("hello");
    # => "aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d"

    my $bytes = CodingAdventures::Sha1::digest("hello");
    # => arrayref of 20 integers (0-255)

    # OO interface
    my $sha1 = CodingAdventures::Sha1->new();
    my $hex  = $sha1->hex("The quick brown fox jumps over the lazy dog");

=head1 DESCRIPTION

A pure-Perl implementation of the SHA-1 secure hash algorithm (FIPS 180-4).
No XS or external modules required. Demonstrates the Merkle–Damgård
construction with Davies–Meyer compression, big-endian message scheduling,
and 80-round mixing with four auxiliary functions.

=head1 VERSION

Version 0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
