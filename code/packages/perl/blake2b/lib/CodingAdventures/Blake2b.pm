package CodingAdventures::Blake2b;

# ============================================================================
# CodingAdventures::Blake2b -- Pure Perl BLAKE2b (RFC 7693) cryptographic hash
# ============================================================================
#
# BLAKE2b is the 64-bit variant of the BLAKE2 family: faster than MD5 on
# modern hardware and at least as secure as SHA-3 against every known attack.
# Designed in 2012 as a drop-in replacement for SHA-2 in performance-sensitive
# contexts, it is the hash used internally by Argon2, libsodium, WireGuard,
# Noise Protocol, and IPFS.
#
# WHY THIS MODULE EXISTS
# ----------------------
# The "HF06" spec in this repo lands BLAKE2b across ten languages because it
# is a hard prerequisite for Argon2.  This Perl port mirrors the Python, Go,
# TypeScript, Rust, Ruby, Elixir, Swift, and Lua siblings: same KATs, same
# public surface, same parameterization.
#
# HOW BLAKE2b WORKS -- A GUIDED TOUR
# ----------------------------------
# 1. PARAMETER-BLOCK INITIALIZATION (RFC 7693 section 2.5)
#    Build an 8-word initial state by XOR-ing the SHA-512 IVs with a 64-byte
#    parameter block encoding digest_size, key_length, fanout=1, depth=1, a
#    16-byte salt, and a 16-byte personalization string.
#
# 2. COMPRESSION (F function, RFC 7693 section 3.2)
#    Each 128-byte input block is absorbed into the state by running twelve
#    ARX rounds over a 16-word working vector v = h || IV, with a 128-bit
#    byte counter folded into v[12..13] and a final-flag inversion applied
#    to v[14] on the last block only.
#
# 3. QUARTER-ROUND G
#    The ARX quarter-round mixes four state words and two message words
#    using add / XOR / rotate-right (rotation constants 32, 24, 16, 63).
#
# 4. FINAL-BLOCK FLAGGING (the classic BLAKE2 off-by-one)
#    Only the LAST real block is flagged final.  If the message is an exact
#    multiple of 128 bytes, DO NOT add an empty padding block -- flag the
#    last real block.  `update()` must therefore keep at least one byte in
#    its internal buffer: it can only flush a full block when more data is
#    known to follow.
#
# PERL 5.26+ 64-BIT ARITHMETIC NOTES
# ----------------------------------
# Perl integers on a 64-bit build are 64-bit, but unqualified arithmetic can
# silently promote large unsigned values to floating-point NVs, destroying
# the low bits.  We therefore wrap every addition in `use integer` (which
# forces signed 64-bit wrapping arithmetic) and then mask with MASK64 only
# where we care about the unsigned interpretation.
#
# Bitwise operators (&, |, ^, ~, >>, <<) work correctly on 64-bit values
# without `use integer`.  `pack("Q<", $w)` and `unpack("Q<", $s)` give us
# little-endian 64-bit packing natively on every 64-bit Perl.

use strict;
use warnings;
no warnings 'portable';   # Allow 64-bit hex literals (> 0xFFFFFFFF)

our $VERSION = '0.01';

# 64-bit mask.  Used after bitwise shifts to keep results in [0, 2^64).
use constant MASK64 => 0xFFFFFFFFFFFFFFFF;

# Block size in bytes.  Every compression call consumes exactly 128 bytes.
use constant BLOCK_SIZE => 128;

# ---------------------------------------------------------------------------
# Initial Hash Values (IVs) -- identical to SHA-512
#
# First 64 bits of the fractional parts of the square roots of the first
# eight primes (2, 3, 5, 7, 11, 13, 17, 19).  Reusing SHA-512's IVs is a
# deliberate "nothing up my sleeve" choice -- anyone can verify there is
# no hidden backdoor simply by checking SHA-512's constants.
# ---------------------------------------------------------------------------
my @IV = (
    0x6a09e667f3bcc908,
    0xbb67ae8584caa73b,
    0x3c6ef372fe94f82b,
    0xa54ff53a5f1d36f1,
    0x510e527fade682d1,
    0x9b05688c2b3e6c1f,
    0x1f83d9abfb41bd6b,
    0x5be0cd19137e2179,
);

# ---------------------------------------------------------------------------
# Message-schedule permutations (SIGMA).
#
# Ten permutations of 0..15.  Round i uses SIGMA[i % 10].  Twelve rounds
# total, so rounds 10 and 11 reuse rows 0 and 1.
# ---------------------------------------------------------------------------
my @SIGMA = (
    [  0,  1,  2,  3,  4,  5,  6,  7,  8,  9, 10, 11, 12, 13, 14, 15 ],
    [ 14, 10,  4,  8,  9, 15, 13,  6,  1, 12,  0,  2, 11,  7,  5,  3 ],
    [ 11,  8, 12,  0,  5,  2, 15, 13, 10, 14,  3,  6,  7,  1,  9,  4 ],
    [  7,  9,  3,  1, 13, 12, 11, 14,  2,  6,  5, 10,  4,  0, 15,  8 ],
    [  9,  0,  5,  7,  2,  4, 10, 15, 14,  1, 11, 12,  6,  8,  3, 13 ],
    [  2, 12,  6, 10,  0, 11,  8,  3,  4, 13,  7,  5, 15, 14,  1,  9 ],
    [ 12,  5,  1, 15, 14, 13,  4, 10,  0,  7,  6,  3,  9,  2,  8, 11 ],
    [ 13, 11,  7, 14, 12,  1,  3,  9,  5,  0, 15,  4,  8,  6,  2, 10 ],
    [  6, 15, 14,  9, 11,  3,  0,  8, 12,  2, 13,  7,  1,  4, 10,  5 ],
    [ 10,  2,  8,  4,  7,  6,  1,  5, 15, 11,  9, 14,  3, 12, 13,  0 ],
);

# ---------------------------------------------------------------------------
# _add64 -- 64-bit wrapping addition.
#
# Without `use integer`, Perl promotes large unsigned sums to doubles,
# losing low-order bits.  Under `use integer`, arithmetic is signed 64-bit
# with wrap-on-overflow -- which, after a `& MASK64` mask, is exactly the
# unsigned `mod 2^64` that BLAKE2b requires.
# ---------------------------------------------------------------------------
sub _add64 {
    use integer;
    return ($_[0] + $_[1]) & MASK64;
}

sub _add64_3 {
    use integer;
    return ($_[0] + $_[1] + $_[2]) & MASK64;
}

# ---------------------------------------------------------------------------
# _rotr64($x, $n) -- right-rotate a 64-bit word by $n bits.
# ---------------------------------------------------------------------------
sub _rotr64 {
    my ($x, $n) = @_;
    return ((($x >> $n) | ($x << (64 - $n))) & MASK64);
}

# ---------------------------------------------------------------------------
# _G(\@v, $a, $b, $c, $d, $x, $y)
#
# The BLAKE2b quarter-round.  Mutates four words v[a..d] of the 16-word
# working vector by mixing them with two message words x, y.  Uses only
# addition, XOR, and rotation -- the "ARX" primitive family.
#
# Rotation constants (32, 24, 16, 63) are from RFC 7693 Appendix D;
# changing any one breaks compatibility with every BLAKE2b implementation.
# ---------------------------------------------------------------------------
sub _G {
    my ($v, $a, $b, $c, $d, $x, $y) = @_;

    $v->[$a] = _add64_3($v->[$a], $v->[$b], $x);
    $v->[$d] = _rotr64($v->[$d] ^ $v->[$a], 32);
    $v->[$c] = _add64($v->[$c], $v->[$d]);
    $v->[$b] = _rotr64($v->[$b] ^ $v->[$c], 24);
    $v->[$a] = _add64_3($v->[$a], $v->[$b], $y);
    $v->[$d] = _rotr64($v->[$d] ^ $v->[$a], 16);
    $v->[$c] = _add64($v->[$c], $v->[$d]);
    $v->[$b] = _rotr64($v->[$b] ^ $v->[$c], 63);
}

# ---------------------------------------------------------------------------
# _F(\@h, $block, $t, $final)
#
# The BLAKE2b compression function.  Absorbs one 128-byte block into the
# 8-word state @h (passed by arrayref, mutated in place).  $t is the total
# byte count fed through the hash so far (INCLUDING the bytes in this
# block, even if zero-padded); $final is true iff this is the last
# compression call for the hash.
# ---------------------------------------------------------------------------
sub _F {
    my ($h, $block, $t, $final) = @_;

    # Parse the block as sixteen little-endian 64-bit words.
    my @m = unpack("Q<16", $block);

    # Working vector: state (0..7) followed by IVs (8..15).
    my @v = (@{$h}, @IV);

    # Fold the byte counter into v[12..13].  For any practical message
    # (< 2^64 bytes) the high 64 bits are zero, so we only XOR the low
    # half.  This matches the reference behaviour exactly.
    $v[12] ^= $t & MASK64;
    # v[13] ^= 0  (no-op for messages < 2^64 bytes)

    # Final-block domain separation.
    if ($final) {
        $v[14] ^= MASK64;
    }

    # Twelve rounds: columns, then diagonals -- the same "double-round"
    # pattern ChaCha20 uses.
    for my $i (0 .. 11) {
        my $s = $SIGMA[$i % 10];
        # Columns
        _G(\@v, 0, 4,  8, 12, $m[$s->[ 0]], $m[$s->[ 1]]);
        _G(\@v, 1, 5,  9, 13, $m[$s->[ 2]], $m[$s->[ 3]]);
        _G(\@v, 2, 6, 10, 14, $m[$s->[ 4]], $m[$s->[ 5]]);
        _G(\@v, 3, 7, 11, 15, $m[$s->[ 6]], $m[$s->[ 7]]);
        # Diagonals
        _G(\@v, 0, 5, 10, 15, $m[$s->[ 8]], $m[$s->[ 9]]);
        _G(\@v, 1, 6, 11, 12, $m[$s->[10]], $m[$s->[11]]);
        _G(\@v, 2, 7,  8, 13, $m[$s->[12]], $m[$s->[13]]);
        _G(\@v, 3, 4,  9, 14, $m[$s->[14]], $m[$s->[15]]);
    }

    # Davies-Meyer-style feed-forward: XOR both halves of v into the
    # state.  Makes F one-way even if an attacker could invert G.
    for my $i (0 .. 7) {
        $h->[$i] ^= $v[$i] ^ $v[$i + 8];
    }
}

# ---------------------------------------------------------------------------
# _validate_params(%opts)
#
# Rejects out-of-range parameters with a descriptive die().  Mirrors the
# validation logic of every sibling port.
# ---------------------------------------------------------------------------
sub _validate_params {
    my (%o) = @_;
    my $ds = $o{digest_size};
    my $key = $o{key} // '';
    my $salt = $o{salt} // '';
    my $personal = $o{personal} // '';

    if (!defined $ds || $ds !~ /\A-?\d+\z/ || $ds < 1 || $ds > 64) {
        die "digest_size must be an integer in [1, 64], got "
            . (defined $ds ? $ds : 'undef') . "\n";
    }
    if (length($key) > 64) {
        die "key length must be in [0, 64], got " . length($key) . "\n";
    }
    if (length($salt) != 0 && length($salt) != 16) {
        die "salt must be exactly 16 bytes (or empty), got "
            . length($salt) . "\n";
    }
    if (length($personal) != 0 && length($personal) != 16) {
        die "personal must be exactly 16 bytes (or empty), got "
            . length($personal) . "\n";
    }
}

# ---------------------------------------------------------------------------
# _initial_state($digest_size, $key_len, $salt, $personal) -> \@state
#
# Builds the parameter-block-XORed initial state.  The parameter block is
# 64 bytes (8 LE 64-bit words) containing digest_size, key_length,
# fanout=1, depth=1, leaf_length=0, node_offset=0, node_depth=0,
# inner_length=0, 14 reserved zero bytes, a 16-byte salt, and a 16-byte
# personalization string (RFC 7693 section 2.5).
# ---------------------------------------------------------------------------
sub _initial_state {
    my ($digest_size, $key_len, $salt, $personal) = @_;

    my $p = "\x00" x 64;
    substr($p, 0, 1) = chr($digest_size);
    substr($p, 1, 1) = chr($key_len);
    substr($p, 2, 1) = chr(1);    # fanout = 1 (sequential)
    substr($p, 3, 1) = chr(1);    # depth  = 1 (sequential)
    # bytes 4..31 stay zero
    if (length($salt) == 16) {
        substr($p, 32, 16) = $salt;
    }
    if (length($personal) == 16) {
        substr($p, 48, 16) = $personal;
    }

    my @pw = unpack("Q<8", $p);
    my @h = map { $IV[$_] ^ $pw[$_] } 0 .. 7;
    return \@h;
}

# ===========================================================================
# Streaming Hasher (OO API)
# ===========================================================================

sub new {
    my ($class, %opts) = @_;
    $opts{digest_size} //= 64;
    $opts{key}         //= '';
    $opts{salt}        //= '';
    $opts{personal}    //= '';

    _validate_params(%opts);

    my $self = {
        digest_size => $opts{digest_size},
        state       => _initial_state(
            $opts{digest_size},
            length($opts{key}),
            $opts{salt},
            $opts{personal},
        ),
        # Buffer is a byte string.  After `update()` returns it holds
        # strictly less than one full block -- we never flush eagerly,
        # because the last block is the one that must be flagged final.
        buffer     => '',
        byte_count => 0,
    };

    if (length($opts{key}) > 0) {
        # Keyed mode: zero-pad the key to BLOCK_SIZE and treat it as the
        # first block of input.
        $self->{buffer} =
            $opts{key} . ("\x00" x (BLOCK_SIZE - length($opts{key})));
    }

    return bless $self, $class;
}

sub update {
    my ($self, $data) = @_;
    $self->{buffer} .= $data;

    # Flush only when the buffer STRICTLY exceeds BLOCK_SIZE.  This
    # preserves at least one byte for the final-flagged compression.
    while (length($self->{buffer}) > BLOCK_SIZE) {
        $self->{byte_count} += BLOCK_SIZE;
        _F(
            $self->{state},
            substr($self->{buffer}, 0, BLOCK_SIZE),
            $self->{byte_count},
            0,
        );
        substr($self->{buffer}, 0, BLOCK_SIZE) = '';
    }
    return $self;
}

sub digest {
    my ($self) = @_;

    # Non-destructive: copy state, run one final compression on a
    # zero-padded copy of the buffer, serialize, truncate.  The original
    # hasher is unchanged so further update()/digest() calls keep working.
    my @state = @{ $self->{state} };
    my $buffer = $self->{buffer};
    my $byte_count = $self->{byte_count} + length($buffer);
    my $final_block =
        $buffer . ("\x00" x (BLOCK_SIZE - length($buffer)));
    _F(\@state, $final_block, $byte_count, 1);

    my $full = pack("Q<8", @state);
    return substr($full, 0, $self->{digest_size});
}

sub hex_digest {
    my ($self) = @_;
    return unpack("H*", $self->digest);
}

sub copy {
    my ($self) = @_;
    my @state_copy = @{ $self->{state} };
    return bless {
        digest_size => $self->{digest_size},
        state       => \@state_copy,
        buffer      => $self->{buffer},
        byte_count  => $self->{byte_count},
    }, ref($self);
}

# ===========================================================================
# One-shot functional API
# ===========================================================================

# blake2b($data, %opts) -> raw digest bytes
sub blake2b {
    my ($data, %opts) = @_;
    my $h = __PACKAGE__->new(%opts);
    $h->update($data);
    return $h->digest;
}

# blake2b_hex($data, %opts) -> lowercase hex digest
sub blake2b_hex {
    my ($data, %opts) = @_;
    return unpack("H*", blake2b($data, %opts));
}

1;

__END__

=head1 NAME

CodingAdventures::Blake2b - Pure Perl BLAKE2b (RFC 7693) cryptographic hash

=head1 SYNOPSIS

  use CodingAdventures::Blake2b;

  # One-shot
  my $hex = CodingAdventures::Blake2b::blake2b_hex("hello");
  my $raw = CodingAdventures::Blake2b::blake2b("hello", digest_size => 32);

  # Keyed (MAC mode)
  my $tag = CodingAdventures::Blake2b::blake2b_hex(
      $msg, key => "shared-secret", digest_size => 32,
  );

  # Streaming
  my $h = CodingAdventures::Blake2b->new(digest_size => 32);
  $h->update("hello ");
  $h->update("world");
  my $out = $h->hex_digest;

=head1 DESCRIPTION

Pure Perl implementation of BLAKE2b, the 64-bit variant of the BLAKE2
family (RFC 7693).  Sequential mode only -- no tree hashing, no
BLAKE2s/BLAKE2bp/BLAKE2Xb.  Requires a 64-bit Perl (ivsize == 8).

=head1 SEE ALSO

The BLAKE2b spec at
L<https://datatracker.ietf.org/doc/html/rfc7693>.

=cut
