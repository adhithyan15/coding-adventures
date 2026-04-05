package CodingAdventures::Sha512;

# ============================================================================
# CodingAdventures::Sha512 -- Pure Perl SHA-512 cryptographic hash function
# ============================================================================
#
# SHA-512 (Secure Hash Algorithm 512) is the 64-bit sibling of SHA-256 in
# the SHA-2 family, defined in FIPS PUB 180-4. It produces a 512-bit
# (64-byte) digest from an arbitrary-length input. On 64-bit platforms,
# SHA-512 is often faster than SHA-256 because it processes 128-byte blocks
# using native 64-bit arithmetic.
#
# Algorithm overview:
#
#   1. Pre-processing (padding) -- big-endian with 128-bit length field:
#      Append 0x80, pad with zeros until bit-length = 896 (mod 1024), then
#      append the original 128-bit bit-length as big-endian. The padded
#      message is a multiple of 1024 bits (128 bytes).
#
#   2. Initialize eight 64-bit state words (fractional parts of square
#      roots of the first 8 primes: 2, 3, 5, 7, 11, 13, 17, 19).
#
#   3. For each 1024-bit chunk, build an 80-word message schedule:
#      - W[0..15] = the 16 big-endian 64-bit words of the chunk
#      - W[t] = sigma1(W[t-2]) + W[t-7] + sigma0(W[t-15]) + W[t-16]
#
#   4. Run 80 rounds with Ch, Maj, Sigma0, Sigma1 functions:
#      T1 = h + Sigma1(e) + Ch(e,f,g) + K[t] + W[t]
#      T2 = Sigma0(a) + Maj(a,b,c)
#      Rotate working variables and apply T1, T2.
#
#   5. Output: pack the eight state words big-endian -- 64-byte digest.
#
# Perl-specific notes:
#   - Perl 5.26+ on 64-bit platforms has native 64-bit integers.
#   - All arithmetic must be masked with & 0xFFFFFFFFFFFFFFFF.
#   - Right rotation: (($x >> $n) | ($x << (64 - $n))) & MASK64
#   - pack("Q>*", @words) packs 64-bit big-endian unsigned integers.
#     However, Q> requires Perl 5.10+ and 64-bit. For portability in
#     unpacking, we use "N2" (two 32-bit words) per 64-bit value.
#   - "use integer" would make arithmetic signed; we avoid it and mask
#     explicitly instead.

use strict;
use warnings;
no warnings 'portable';   # SHA-512 requires 64-bit hex literals (>0xFFFFFFFF)

our $VERSION = '0.01';

# ---------------------------------------------------------------------------
# 64-bit wrapping addition
#
# CRITICAL PERL GOTCHA FOR SHA-512:
# Perl integers are *signed* 64-bit (ivsize=8). When two large unsigned
# 64-bit values are added, Perl may promote the result to a floating-point
# NV (double), losing precision. For example:
#   0x6a09e667f3bcc908 + 0xbb67ae8584caa73b  => 2.114e+19 (float!)
#
# The fix: use the `use integer` pragma *locally* inside an addition
# function. Under `use integer`, Perl does signed 64-bit arithmetic that
# wraps on overflow -- exactly what we need for modular addition mod 2^64.
#
# We only use `use integer` for addition. Bitwise ops (&, |, ^, ~, >>, <<)
# already work correctly on 64-bit values without `use integer`.
# ---------------------------------------------------------------------------
sub _add64 {
    use integer;
    return $_[0] + $_[1];
}

# ---------------------------------------------------------------------------
# Multi-argument wrapping addition (convenience for SHA-512 round function)
# ---------------------------------------------------------------------------
sub _add64_multi {
    use integer;
    my $sum = 0;
    for my $v (@_) {
        $sum += $v;
    }
    return $sum;
}

# ---------------------------------------------------------------------------
# 64-bit mask -- used by bitwise operations to stay in 64-bit range
# ---------------------------------------------------------------------------
use constant MASK64 => 0xFFFFFFFFFFFFFFFF;

# ---------------------------------------------------------------------------
# Round constants K[0..79] (FIPS 180-4 Section 4.2.3)
#
# First 64 bits of the fractional parts of the cube roots of the first
# 80 prime numbers.
# ---------------------------------------------------------------------------
my @K = (
    0x428a2f98d728ae22, 0x7137449123ef65cd, 0xb5c0fbcfec4d3b2f, 0xe9b5dba58189dbbc,
    0x3956c25bf348b538, 0x59f111f1b605d019, 0x923f82a4af194f9b, 0xab1c5ed5da6d8118,
    0xd807aa98a3030242, 0x12835b0145706fbe, 0x243185be4ee4b28c, 0x550c7dc3d5ffb4e2,
    0x72be5d74f27b896f, 0x80deb1fe3b1696b1, 0x9bdc06a725c71235, 0xc19bf174cf692694,
    0xe49b69c19ef14ad2, 0xefbe4786384f25e3, 0x0fc19dc68b8cd5b5, 0x240ca1cc77ac9c65,
    0x2de92c6f592b0275, 0x4a7484aa6ea6e483, 0x5cb0a9dcbd41fbd4, 0x76f988da831153b5,
    0x983e5152ee66dfab, 0xa831c66d2db43210, 0xb00327c898fb213f, 0xbf597fc7beef0ee4,
    0xc6e00bf33da88fc2, 0xd5a79147930aa725, 0x06ca6351e003826f, 0x142929670a0e6e70,
    0x27b70a8546d22ffc, 0x2e1b21385c26c926, 0x4d2c6dfc5ac42aed, 0x53380d139d95b3df,
    0x650a73548baf63de, 0x766a0abb3c77b2a8, 0x81c2c92e47edaee6, 0x92722c851482353b,
    0xa2bfe8a14cf10364, 0xa81a664bbc423001, 0xc24b8b70d0f89791, 0xc76c51a30654be30,
    0xd192e819d6ef5218, 0xd69906245565a910, 0xf40e35855771202a, 0x106aa07032bbd1b8,
    0x19a4c116b8d2d0c8, 0x1e376c085141ab53, 0x2748774cdf8eeb99, 0x34b0bcb5e19b48a8,
    0x391c0cb3c5c95a63, 0x4ed8aa4ae3418acb, 0x5b9cca4f7763e373, 0x682e6ff3d6b2b8a3,
    0x748f82ee5defb2fc, 0x78a5636f43172f60, 0x84c87814a1f0ab72, 0x8cc702081a6439ec,
    0x90befffa23631e28, 0xa4506cebde82bde9, 0xbef9a3f7b2c67915, 0xc67178f2e372532b,
    0xca273eceea26619c, 0xd186b8c721c0c207, 0xeada7dd6cde0eb1e, 0xf57d4f7fee6ed178,
    0x06f067aa72176fba, 0x0a637dc5a2c898a6, 0x113f9804bef90dae, 0x1b710b35131c471b,
    0x28db77f523047d84, 0x32caab7b40c72493, 0x3c9ebe0a15c9bebc, 0x431d67c49c100d4c,
    0x4cc5d4becb3e42b6, 0x597f299cfc657e2a, 0x5fcb6fab3ad6faec, 0x6c44198c4a475817,
);

# ---------------------------------------------------------------------------
# _rotr64($x, $n) -- rotate right a 64-bit value by $n bits
#
# SHA-512 uses right rotations extensively. The left-shift component wraps
# the bits that "fall off" the right end back to the left.
# ---------------------------------------------------------------------------
sub _rotr64 {
    my ($x, $n) = @_;
    return (($x >> $n) | ($x << (64 - $n))) & MASK64;
}

# ---------------------------------------------------------------------------
# SHA-512 auxiliary functions (FIPS 180-4 Section 4.1.3)
#
# Sigma0(x) = ROTR(28,x) XOR ROTR(34,x) XOR ROTR(39,x)
# Sigma1(x) = ROTR(14,x) XOR ROTR(18,x) XOR ROTR(41,x)
# sigma0(x) = ROTR(1,x) XOR ROTR(8,x) XOR SHR(7,x)
# sigma1(x) = ROTR(19,x) XOR ROTR(61,x) XOR SHR(6,x)
# Ch(x,y,z)  = (x AND y) XOR (NOT x AND z)
# Maj(x,y,z) = (x AND y) XOR (x AND z) XOR (y AND z)
# ---------------------------------------------------------------------------
sub _big_sigma0   { _rotr64($_[0], 28) ^ _rotr64($_[0], 34) ^ _rotr64($_[0], 39) }
sub _big_sigma1   { _rotr64($_[0], 14) ^ _rotr64($_[0], 18) ^ _rotr64($_[0], 41) }
sub _small_sigma0 { _rotr64($_[0],  1) ^ _rotr64($_[0],  8) ^ (($_[0] >> 7)  & MASK64) }
sub _small_sigma1 { _rotr64($_[0], 19) ^ _rotr64($_[0], 61) ^ (($_[0] >> 6)  & MASK64) }
sub _ch           { (($_[0] & $_[1]) ^ ((~$_[0]) & $_[2])) & MASK64 }
sub _maj          { (($_[0] & $_[1]) ^ ($_[0] & $_[2]) ^ ($_[1] & $_[2])) & MASK64 }

# ---------------------------------------------------------------------------
# _unpack_u64_be($str, $offset) -- read a 64-bit big-endian value
#
# pack/unpack with "Q>" requires 64-bit Perl. We use it directly since
# this module requires a 64-bit Perl anyway.
# ---------------------------------------------------------------------------
sub _unpack_u64_be {
    my ($str, $offset) = @_;
    my ($hi, $lo) = unpack("NN", substr($str, $offset, 8));
    return ($hi << 32) | $lo;
}

# ---------------------------------------------------------------------------
# _pack_u64_be($value) -- pack a 64-bit value as 8 bytes big-endian
# ---------------------------------------------------------------------------
sub _pack_u64_be {
    my ($val) = @_;
    my $hi = ($val >> 32) & 0xFFFFFFFF;
    my $lo = $val & 0xFFFFFFFF;
    return pack("NN", $hi, $lo);
}

# ---------------------------------------------------------------------------
# _sha512_block(\@state, $chunk_bytes)
#
# Process one 128-byte block. @state = (H0..H7) as 64-bit integers.
# $chunk_bytes is a 128-byte substring of the padded message.
# ---------------------------------------------------------------------------
sub _sha512_block {
    my ($state, $chunk) = @_;

    # Build the 80-word message schedule
    my @W;
    for my $i (0 .. 15) {
        $W[$i] = _unpack_u64_be($chunk, $i * 8);
    }
    for my $t (16 .. 79) {
        $W[$t] = _add64_multi(_small_sigma1($W[$t-2]), $W[$t-7],
                              _small_sigma0($W[$t-15]), $W[$t-16]);
    }

    my ($A, $B, $C, $D, $E, $F, $G, $H) = @{$state};

    for my $t (0 .. 79) {
        my $T1 = _add64_multi($H, _big_sigma1($E), _ch($E, $F, $G), $K[$t], $W[$t]);
        my $T2 = _add64(_big_sigma0($A), _maj($A, $B, $C));
        $H = $G;
        $G = $F;
        $F = $E;
        $E = _add64($D, $T1);
        $D = $C;
        $C = $B;
        $B = $A;
        $A = _add64($T1, $T2);
    }

    # Accumulate: add block result back into running state
    $state->[0] = _add64($state->[0], $A);
    $state->[1] = _add64($state->[1], $B);
    $state->[2] = _add64($state->[2], $C);
    $state->[3] = _add64($state->[3], $D);
    $state->[4] = _add64($state->[4], $E);
    $state->[5] = _add64($state->[5], $F);
    $state->[6] = _add64($state->[6], $G);
    $state->[7] = _add64($state->[7], $H);
}

# ---------------------------------------------------------------------------
# digest($message) -> \@bytes   (64-element arrayref of 0-255 integers)
#
# Compute SHA-512 of $message. Can be called as:
#   CodingAdventures::Sha512::digest($msg)     -- functional
#   CodingAdventures::Sha512->digest($msg)     -- class method
#   $obj->digest($msg)                         -- instance method
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
    # Step 1: Padding (FIPS 180-4 Section 5.1.2)
    #
    # Append 0x80, then zero bytes until length = 112 (mod 128), then the
    # original 128-bit bit-length in big-endian order.
    # ------------------------------------------------------------------
    my $original_len  = length($msg);
    my $original_bits = $original_len * 8;

    $msg .= "\x80";

    my $pad = (112 - length($msg) % 128) % 128;
    $msg .= "\x00" x $pad;

    # Append 128-bit big-endian length.
    # For messages < 2^64 bits, the high 64 bits are zero.
    $msg .= "\x00" x 8;  # high 64 bits
    $msg .= _pack_u64_be($original_bits);  # low 64 bits

    # ------------------------------------------------------------------
    # Step 2: Initialize state (FIPS 180-4 Section 5.3.5)
    # ------------------------------------------------------------------
    my @state = (
        0x6a09e667f3bcc908,
        0xbb67ae8584caa73b,
        0x3c6ef372fe94f82b,
        0xa54ff53a5f1d36f1,
        0x510e527fade682d1,
        0x9b05688c2b3e6c1f,
        0x1f83d9abfb41bd6b,
        0x5be0cd19137e2179,
    );

    # ------------------------------------------------------------------
    # Step 3: Process each 128-byte block
    # ------------------------------------------------------------------
    my $num_blocks = length($msg) / 128;
    for my $b (0 .. $num_blocks - 1) {
        _sha512_block(\@state, substr($msg, $b * 128, 128));
    }

    # ------------------------------------------------------------------
    # Step 4: Produce the 64-byte digest (big-endian)
    # ------------------------------------------------------------------
    my $raw = '';
    for my $w (@state) {
        $raw .= _pack_u64_be($w);
    }
    return [ unpack("C*", $raw) ];
}

# ---------------------------------------------------------------------------
# hex($message) -> $hex_string   (128 lowercase hex characters)
# ---------------------------------------------------------------------------
sub hex {
    my @args = @_;
    my $bytes = digest(@args);
    return join('', map { sprintf('%02x', $_) } @{$bytes});
}

# ---------------------------------------------------------------------------
# new() -- constructor for OO usage
# ---------------------------------------------------------------------------
sub new {
    my ($class) = @_;
    return bless {}, $class;
}

1;

__END__

=head1 NAME

CodingAdventures::Sha512 - Pure Perl SHA-512 cryptographic hash function

=head1 SYNOPSIS

    use CodingAdventures::Sha512;

    # Functional interface
    my $hex   = CodingAdventures::Sha512::hex("hello");
    # => "9b71d224bd62f3785d96d46ad3ea3d73..."

    my $bytes = CodingAdventures::Sha512::digest("hello");
    # => arrayref of 64 integers (0-255)

    # OO interface
    my $sha512 = CodingAdventures::Sha512->new();
    my $hex    = $sha512->hex("The quick brown fox jumps over the lazy dog");

=head1 DESCRIPTION

A pure-Perl implementation of the SHA-512 secure hash algorithm (FIPS 180-4).
No XS or external modules required. Requires a 64-bit Perl build.

SHA-512 uses 8 x 64-bit state words, 80 rounds of compression, and 128-byte
blocks. It produces a 512-bit (64-byte) digest.

=head1 VERSION

Version 0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
