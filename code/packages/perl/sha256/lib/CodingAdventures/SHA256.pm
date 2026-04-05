package CodingAdventures::SHA256;

# ============================================================================
# CodingAdventures::SHA256 — Pure Perl SHA-256 cryptographic hash function
# ============================================================================
#
# SHA-256 is a member of the SHA-2 family, designed by the NSA and published
# by NIST in 2001 as FIPS 180-2. It produces a 256-bit (32-byte) digest.
# Unlike MD5 (broken 2004) and SHA-1 (broken 2017), SHA-256 remains secure
# with no known practical attacks.
#
# Algorithm overview (FIPS 180-4):
#
#   1. Pre-processing (Merkle-Damgard padding, big-endian):
#      Append 0x80, pad with zeros until bit-length = 448 (mod 512), then
#      append the original 64-bit bit-length as BIG-endian. The padded
#      message is a multiple of 512 bits (64 bytes).
#
#   2. Initialize eight 32-bit state words H0..H7:
#      First 32 bits of the fractional parts of the square roots of the
#      first 8 primes (2, 3, 5, 7, 11, 13, 17, 19).
#
#   3. Round constants K0..K63:
#      First 32 bits of the fractional parts of the cube roots of the
#      first 64 primes (2, 3, 5, ..., 311).
#
#   4. For each 512-bit block, build a 64-word message schedule W[0..63]:
#      - W[0..15]  = the 16 big-endian 32-bit words of the block
#      - W[t] = sigma1(W[t-2]) + W[t-7] + sigma0(W[t-15]) + W[t-16]
#
#   5. Run 64 rounds with auxiliary functions:
#      - Ch(e,f,g) = (e AND f) XOR (NOT e AND g)
#      - Maj(a,b,c) = (a AND b) XOR (a AND c) XOR (b AND c)
#      - Sigma0(a) = ROTR(2,a) XOR ROTR(13,a) XOR ROTR(22,a)
#      - Sigma1(e) = ROTR(6,e) XOR ROTR(11,e) XOR ROTR(25,e)
#      - sigma0(x) = ROTR(7,x) XOR ROTR(18,x) XOR SHR(3,x)
#      - sigma1(x) = ROTR(17,x) XOR ROTR(19,x) XOR SHR(10,x)
#
#   6. Output: pack H0..H7 as 8 big-endian 32-bit words = 32 bytes.
#
# Perl-specific notes:
#   - Perl 64-bit integers require masking with & 0xFFFFFFFF after every
#     arithmetic and bitwise operation to simulate 32-bit unsigned arithmetic.
#   - Right rotate: (($x >> $n) | (($x & 0xFFFFFFFF) << (32-$n))) & 0xFFFFFFFF
#   - pack("N*", @words) packs 32-bit big-endian unsigned integers.
#   - unpack("N*", $str) unpacks 32-bit big-endian words.

use strict;
use warnings;

our $VERSION = '0.01';

# ---------------------------------------------------------------------------
# Initial hash values H0..H7 (FIPS 180-4, Section 5.3.3)
#
# Derived from: floor(frac(sqrt(prime)) * 2^32) for primes 2..19
# ---------------------------------------------------------------------------
my @H_INIT = (
    0x6A09E667,  # sqrt(2)
    0xBB67AE85,  # sqrt(3)
    0x3C6EF372,  # sqrt(5)
    0xA54FF53A,  # sqrt(7)
    0x510E527F,  # sqrt(11)
    0x9B05688C,  # sqrt(13)
    0x1F83D9AB,  # sqrt(17)
    0x5BE0CD19,  # sqrt(19)
);

# ---------------------------------------------------------------------------
# Round constants K0..K63 (FIPS 180-4, Section 4.2.2)
#
# Derived from: floor(frac(cbrt(prime)) * 2^32) for first 64 primes
# ---------------------------------------------------------------------------
my @K = (
    0x428A2F98, 0x71374491, 0xB5C0FBCF, 0xE9B5DBA5,
    0x3956C25B, 0x59F111F1, 0x923F82A4, 0xAB1C5ED5,
    0xD807AA98, 0x12835B01, 0x243185BE, 0x550C7DC3,
    0x72BE5D74, 0x80DEB1FE, 0x9BDC06A7, 0xC19BF174,
    0xE49B69C1, 0xEFBE4786, 0x0FC19DC6, 0x240CA1CC,
    0x2DE92C6F, 0x4A7484AA, 0x5CB0A9DC, 0x76F988DA,
    0x983E5152, 0xA831C66D, 0xB00327C8, 0xBF597FC7,
    0xC6E00BF3, 0xD5A79147, 0x06CA6351, 0x14292967,
    0x27B70A85, 0x2E1B2138, 0x4D2C6DFC, 0x53380D13,
    0x650A7354, 0x766A0ABB, 0x81C2C92E, 0x92722C85,
    0xA2BFE8A1, 0xA81A664B, 0xC24B8B70, 0xC76C51A3,
    0xD192E819, 0xD6990624, 0xF40E3585, 0x106AA070,
    0x19A4C116, 0x1E376C08, 0x2748774C, 0x34B0BCB5,
    0x391C0CB3, 0x4ED8AA4A, 0x5B9CCA4F, 0x682E6FF3,
    0x748F82EE, 0x78A5636F, 0x84C87814, 0x8CC70208,
    0x90BEFFFA, 0xA4506CEB, 0xBEF9A3F7, 0xC67178F2,
);

# ---------------------------------------------------------------------------
# _rotr($x, $n) — rotate right 32-bit value $x by $n bits
#
# SHA-256 uses right rotations (unlike SHA-1 which uses left).
# The left-shift component recovers the bits that "fall off" the right end.
# ---------------------------------------------------------------------------
sub _rotr {
    my ($x, $n) = @_;
    return ( (($x & 0xFFFFFFFF) >> $n) | (($x & 0xFFFFFFFF) << (32 - $n)) ) & 0xFFFFFFFF;
}

# ---------------------------------------------------------------------------
# SHA-256 Auxiliary Functions (FIPS 180-4, Section 4.1.2)
# ---------------------------------------------------------------------------

# Ch(x,y,z) — "Choice": for each bit, if x=1 pick y, else pick z
sub _ch {
    my ($x, $y, $z) = @_;
    return (($x & $y) ^ ((~$x) & $z)) & 0xFFFFFFFF;
}

# Maj(x,y,z) — "Majority": output is the majority vote of the 3 inputs
sub _maj {
    my ($x, $y, $z) = @_;
    return (($x & $y) ^ ($x & $z) ^ ($y & $z)) & 0xFFFFFFFF;
}

# Sigma0(x) — "Big Sigma 0": used on variable 'a' in compression rounds
sub _big_sigma0 {
    my ($x) = @_;
    return (_rotr($x, 2) ^ _rotr($x, 13) ^ _rotr($x, 22)) & 0xFFFFFFFF;
}

# Sigma1(x) — "Big Sigma 1": used on variable 'e' in compression rounds
sub _big_sigma1 {
    my ($x) = @_;
    return (_rotr($x, 6) ^ _rotr($x, 11) ^ _rotr($x, 25)) & 0xFFFFFFFF;
}

# sigma0(x) — "Small sigma 0": used in message schedule expansion
# Note: third term is a right SHIFT (not rotate) — bits fall off permanently
sub _small_sigma0 {
    my ($x) = @_;
    return (_rotr($x, 7) ^ _rotr($x, 18) ^ (($x & 0xFFFFFFFF) >> 3)) & 0xFFFFFFFF;
}

# sigma1(x) — "Small sigma 1": used in message schedule expansion
sub _small_sigma1 {
    my ($x) = @_;
    return (_rotr($x, 17) ^ _rotr($x, 19) ^ (($x & 0xFFFFFFFF) >> 10)) & 0xFFFFFFFF;
}

# ---------------------------------------------------------------------------
# _sha256_block(\@state, $chunk_bytes)
#
# Process one 64-byte block. @state = (H0..H7). $chunk_bytes is a 64-byte
# substring of the padded message.
# ---------------------------------------------------------------------------
sub _sha256_block {
    my ($state, $chunk) = @_;

    # Build the 64-word message schedule
    my @W = unpack("N16", $chunk);  # W[0..15] from the chunk (big-endian)

    for my $t ( 16 .. 63 ) {
        # Message schedule extension: mix four prior words through sigma functions
        $W[$t] = (_small_sigma1($W[$t-2]) + $W[$t-7]
                 + _small_sigma0($W[$t-15]) + $W[$t-16]) & 0xFFFFFFFF;
    }

    my ($a, $b, $c, $d, $e, $f, $g, $h) = @{$state};

    for my $t ( 0 .. 63 ) {
        # T1 combines: h, non-linear function of (e,f,g), round constant, schedule word
        my $T1 = ($h + _big_sigma1($e) + _ch($e, $f, $g) + $K[$t] + $W[$t]) & 0xFFFFFFFF;

        # T2 combines: non-linear function of (a,b,c)
        my $T2 = (_big_sigma0($a) + _maj($a, $b, $c)) & 0xFFFFFFFF;

        # Rotate working variables
        $h = $g;
        $g = $f;
        $f = $e;
        $e = ($d + $T1) & 0xFFFFFFFF;
        $d = $c;
        $c = $b;
        $b = $a;
        $a = ($T1 + $T2) & 0xFFFFFFFF;
    }

    # Davies-Meyer feed-forward: add compressed output to running state
    $state->[0] = ($state->[0] + $a) & 0xFFFFFFFF;
    $state->[1] = ($state->[1] + $b) & 0xFFFFFFFF;
    $state->[2] = ($state->[2] + $c) & 0xFFFFFFFF;
    $state->[3] = ($state->[3] + $d) & 0xFFFFFFFF;
    $state->[4] = ($state->[4] + $e) & 0xFFFFFFFF;
    $state->[5] = ($state->[5] + $f) & 0xFFFFFFFF;
    $state->[6] = ($state->[6] + $g) & 0xFFFFFFFF;
    $state->[7] = ($state->[7] + $h) & 0xFFFFFFFF;
}

# ---------------------------------------------------------------------------
# sha256($message) -> \@bytes   (32-element arrayref of 0-255 integers)
#
# Compute SHA-256 of $message. Can be called as:
#   CodingAdventures::SHA256::sha256($msg)     — functional
#   CodingAdventures::SHA256->sha256($msg)     — class method
#   $obj->sha256($msg)                         — instance method
# ---------------------------------------------------------------------------
sub sha256 {
    my $msg;
    if ( @_ == 2 ) {
        $msg = $_[1];   # OO call: (class/object, message)
    }
    else {
        $msg = $_[0];   # function call: (message)
    }
    $msg //= '';

    # ------------------------------------------------------------------
    # Step 1: Padding (Merkle-Damgard, big-endian)
    # ------------------------------------------------------------------
    my $original_len   = length($msg);
    my $original_bits  = $original_len * 8;

    $msg .= "\x80";

    my $pad = (56 - length($msg) % 64) % 64;
    $msg .= "\x00" x $pad;

    # Append 64-bit big-endian length
    my $hi = int($original_bits / 2**32) & 0xFFFFFFFF;
    my $lo = $original_bits & 0xFFFFFFFF;
    $msg .= pack("NN", $hi, $lo);

    # ------------------------------------------------------------------
    # Step 2: Initialize state
    # ------------------------------------------------------------------
    my @state = @H_INIT;

    # ------------------------------------------------------------------
    # Step 3: Process each 64-byte block
    # ------------------------------------------------------------------
    my $num_blocks = length($msg) / 64;
    for my $b ( 0 .. $num_blocks - 1 ) {
        _sha256_block(\@state, substr($msg, $b * 64, 64));
    }

    # ------------------------------------------------------------------
    # Step 4: Produce the 32-byte digest (big-endian)
    # ------------------------------------------------------------------
    my $raw = pack("N8", @state);
    return [ unpack("C*", $raw) ];
}

# ---------------------------------------------------------------------------
# sha256_hex($message) -> $hex_string   (64 lowercase hex characters)
# ---------------------------------------------------------------------------
sub sha256_hex {
    my @args = @_;
    my $bytes = sha256(@args);
    return join('', map { sprintf('%02x', $_) } @{$bytes});
}

# ============================================================================
# Streaming Hasher (OO interface)
# ============================================================================
#
# When the full message is not available at once, the streaming API allows
# incremental updates. Internally tracks:
#   - _state: the eight-word running hash
#   - _buffer: unprocessed bytes (less than 64)
#   - _total_len: total bytes fed (needed for padding length)
# ============================================================================

# ---------------------------------------------------------------------------
# new() — constructor
# ---------------------------------------------------------------------------
sub new {
    my ($class) = @_;
    return bless {
        _state     => [ @H_INIT ],
        _buffer    => '',
        _total_len => 0,
    }, $class;
}

# ---------------------------------------------------------------------------
# update($data) — feed more bytes into the hasher (chainable)
# ---------------------------------------------------------------------------
sub update {
    my ($self, $data) = @_;
    $data //= '';

    $self->{_buffer}    .= $data;
    $self->{_total_len} += length($data);

    # Process complete 64-byte blocks
    while ( length($self->{_buffer}) >= 64 ) {
        my $chunk = substr($self->{_buffer}, 0, 64);
        $self->{_buffer} = substr($self->{_buffer}, 64);
        _sha256_block($self->{_state}, $chunk);
    }

    return $self;  # chainable
}

# ---------------------------------------------------------------------------
# digest() -> \@bytes   (32-element arrayref, non-destructive)
# ---------------------------------------------------------------------------
sub digest {
    my ($self) = @_;

    # If called as class/function method with a message argument, dispatch to
    # the one-shot function
    if ( !ref($self) ) {
        return sha256(@_);
    }

    # Work on copies so the internal state is not modified
    my @state = @{ $self->{_state} };
    my $buf   = $self->{_buffer};
    my $bit_len = $self->{_total_len} * 8;

    # Padding
    $buf .= "\x80";
    my $pad = (56 - length($buf) % 64) % 64;
    $buf .= "\x00" x $pad;
    my $hi = int($bit_len / 2**32) & 0xFFFFFFFF;
    my $lo = $bit_len & 0xFFFFFFFF;
    $buf .= pack("NN", $hi, $lo);

    # Process remaining blocks
    my $num_blocks = length($buf) / 64;
    for my $b ( 0 .. $num_blocks - 1 ) {
        _sha256_block(\@state, substr($buf, $b * 64, 64));
    }

    my $raw = pack("N8", @state);
    return [ unpack("C*", $raw) ];
}

# ---------------------------------------------------------------------------
# hex_digest() -> $hex_string   (64 lowercase hex chars, non-destructive)
# ---------------------------------------------------------------------------
sub hex_digest {
    my ($self) = @_;
    my $bytes = $self->digest();
    return join('', map { sprintf('%02x', $_) } @{$bytes});
}

# ---------------------------------------------------------------------------
# copy() -> new independent hasher with the same state
# ---------------------------------------------------------------------------
sub copy {
    my ($self) = @_;
    return bless {
        _state     => [ @{ $self->{_state} } ],
        _buffer    => $self->{_buffer},
        _total_len => $self->{_total_len},
    }, ref($self);
}

1;

__END__

=head1 NAME

CodingAdventures::SHA256 - Pure Perl SHA-256 cryptographic hash function

=head1 SYNOPSIS

    use CodingAdventures::SHA256;

    # Functional interface (one-shot)
    my $hex   = CodingAdventures::SHA256::sha256_hex("hello");
    # => "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"

    my $bytes = CodingAdventures::SHA256::sha256("hello");
    # => arrayref of 32 integers (0-255)

    # Streaming interface
    my $hasher = CodingAdventures::SHA256->new();
    $hasher->update("hello ");
    $hasher->update("world");
    my $hex = $hasher->hex_digest();

    # Copy for branching
    my $branch = $hasher->copy();
    $branch->update("!");

=head1 DESCRIPTION

A pure-Perl implementation of the SHA-256 secure hash algorithm (FIPS 180-4).
No XS or external modules required. Provides both one-shot and streaming APIs
for computing 256-bit (32-byte) digests.

=head1 VERSION

Version 0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
