package CodingAdventures::Argon2d;

# ============================================================================
# CodingAdventures::Argon2d -- Pure Perl Argon2d (RFC 9106)
# ============================================================================
#
# Argon2d is the data-DEPENDENT variant of the Argon2 memory-hard password
# hashing family.  For every new block, the index of the reference block is
# read from the first 64 bits of the previously computed block -- so the
# memory-access pattern is correlated with the password.  That maximises
# resistance against GPU/ASIC cracking but leaks a side channel through
# memory-access timing.  Use Argon2d only when side-channel attacks are NOT
# in the threat model (e.g. proof-of-work).  For password hashing prefer
# CodingAdventures::Argon2id.
#
# Reference: https://datatracker.ietf.org/doc/html/rfc9106
# See also:  code/specs/KD03-argon2.md
#
# PERL 64-BIT NOTES
# -----------------
# Every 64-bit arithmetic step is wrapped with `use integer` (signed 64-bit
# wrap-on-overflow) and masked with MASK64 where unsigned semantics matter.
# Bitwise ops (&, |, ^, >>, <<) already work natively on 64-bit integers.
# `pack("Q<", $w)` and `unpack("Q<", $s)` give us little-endian 64-bit I/O.

use strict;
use warnings;
no warnings 'portable';   # Allow 64-bit hex literals (> 0xFFFFFFFF).

use Config;
BEGIN {
    die "CodingAdventures::Argon2d requires 64-bit Perl (ivsize >= 8); "
      . "this build has ivsize=$Config{ivsize}\n"
        if $Config{ivsize} < 8;
}

use CodingAdventures::Blake2b ();

our $VERSION = '0.01';

# --- Constants -------------------------------------------------------------
use constant MASK32         => 0xFFFFFFFF;
use constant MASK64         => 0xFFFFFFFFFFFFFFFF;
use constant BLOCK_SIZE     => 1024;     # bytes per Argon2 memory block
use constant BLOCK_WORDS    => 128;      # 64-bit words per block
use constant SYNC_POINTS    => 4;        # slices per pass
use constant ARGON2_VERSION => 0x13;     # v1.3 is the only approved version
use constant TYPE_D         => 0;        # primitive type code for Argon2d

# ---------------------------------------------------------------------------
# _rotr64($x, $n) -- right-rotate a 64-bit word by $n bits.
# ---------------------------------------------------------------------------
sub _rotr64 {
    my ($x, $n) = @_;
    return ((($x >> $n) | ($x << (64 - $n))) & MASK64);
}

# ---------------------------------------------------------------------------
# _gb(\@v, $a, $b, $c, $d) -- the Argon2 G-mixer (RFC 9106 §3.5).
#
# Identical to the BLAKE2 quarter-round EXCEPT each addition has an extra
# `2 * trunc32(a) * trunc32(b)` term.  That cross-term is what makes
# memory-hard attacks on Argon2 quadratic in block size, raising the cost
# of any "prune-and-extend" strategy.
# ---------------------------------------------------------------------------
sub _gb {
    my ($v, $a, $b, $c, $d) = @_;

    my ($va, $vb, $vc, $vd) = ($v->[$a], $v->[$b], $v->[$c], $v->[$d]);

    {
        use integer;
        $va = ($va + $vb + 2 * ($va & MASK32) * ($vb & MASK32)) & MASK64;
    }
    $vd = _rotr64($vd ^ $va, 32);
    {
        use integer;
        $vc = ($vc + $vd + 2 * ($vc & MASK32) * ($vd & MASK32)) & MASK64;
    }
    $vb = _rotr64($vb ^ $vc, 24);
    {
        use integer;
        $va = ($va + $vb + 2 * ($va & MASK32) * ($vb & MASK32)) & MASK64;
    }
    $vd = _rotr64($vd ^ $va, 16);
    {
        use integer;
        $vc = ($vc + $vd + 2 * ($vc & MASK32) * ($vd & MASK32)) & MASK64;
    }
    $vb = _rotr64($vb ^ $vc, 63);

    ($v->[$a], $v->[$b], $v->[$c], $v->[$d]) = ($va, $vb, $vc, $vd);
}

# ---------------------------------------------------------------------------
# _permutation_p(\@v, $off) -- eight G-rounds across a 16-word slice.
#
# Four "column" rounds on (0,4,8,12) etc. followed by four "diagonal"
# rounds on (0,5,10,15) etc. -- the classic double-round layout.
# ---------------------------------------------------------------------------
sub _permutation_p {
    my ($v, $off) = @_;
    $off //= 0;
    _gb($v, $off + 0, $off + 4, $off +  8, $off + 12);
    _gb($v, $off + 1, $off + 5, $off +  9, $off + 13);
    _gb($v, $off + 2, $off + 6, $off + 10, $off + 14);
    _gb($v, $off + 3, $off + 7, $off + 11, $off + 15);
    _gb($v, $off + 0, $off + 5, $off + 10, $off + 15);
    _gb($v, $off + 1, $off + 6, $off + 11, $off + 12);
    _gb($v, $off + 2, $off + 7, $off +  8, $off + 13);
    _gb($v, $off + 3, $off + 4, $off +  9, $off + 14);
}

# ---------------------------------------------------------------------------
# _compress(\@x, \@y) -> \@out  --  the Argon2 compression function G.
#
# Treats the 128-word block as an 8x8 matrix of 128-bit registers.  First
# performs a "row pass" by permuting each of the eight 16-word rows in
# place; then a "column pass" by gathering offset-2c pairs across all
# eight rows, permuting, and scattering back.  The result is XORed with
# the original input (r := x XOR y) to make G a Davies-Meyer-style
# one-way function.
# ---------------------------------------------------------------------------
sub _compress {
    my ($x, $y) = @_;

    my @r = map { $x->[$_] ^ $y->[$_] } 0 .. BLOCK_WORDS - 1;
    my @q = @r;

    # Row pass
    for my $i (0 .. 7) {
        _permutation_p(\@q, $i * 16);
    }

    # Column pass -- gather, permute, scatter.
    my @col;
    for my $c (0 .. 7) {
        for my $rr (0 .. 7) {
            $col[2 * $rr]     = $q[$rr * 16 + 2 * $c];
            $col[2 * $rr + 1] = $q[$rr * 16 + 2 * $c + 1];
        }
        _permutation_p(\@col, 0);
        for my $rr (0 .. 7) {
            $q[$rr * 16 + 2 * $c]     = $col[2 * $rr];
            $q[$rr * 16 + 2 * $c + 1] = $col[2 * $rr + 1];
        }
    }

    return [ map { $r[$_] ^ $q[$_] } 0 .. BLOCK_WORDS - 1 ];
}

# --- Block <-> byte-string helpers ----------------------------------------
sub _block_to_bytes { pack("Q<" . BLOCK_WORDS, @{$_[0]}) }
sub _bytes_to_block { [ unpack("Q<" . BLOCK_WORDS, $_[0]) ] }
sub _le32           { pack("V", $_[0]) }

# ---------------------------------------------------------------------------
# _blake2b_long($t, $x)  --  Argon2 variable-length hash H' (RFC 9106 §3.3).
#
# Produces $t output bytes by chaining 32-byte halves of repeated 64-byte
# BLAKE2b calls, with the last call sized to fit exactly.  The initial
# input is `LE32($t) || $x`.
# ---------------------------------------------------------------------------
sub _blake2b_long {
    my ($t, $x) = @_;
    die "H' output length must be positive\n" if $t <= 0;

    my $input = _le32($t) . $x;

    return CodingAdventures::Blake2b::blake2b($input, digest_size => $t)
        if $t <= 64;

    my $r = int(($t + 31) / 32) - 2;
    my $v = CodingAdventures::Blake2b::blake2b($input, digest_size => 64);
    my $out = substr($v, 0, 32);
    for (1 .. $r - 1) {
        $v = CodingAdventures::Blake2b::blake2b($v, digest_size => 64);
        $out .= substr($v, 0, 32);
    }
    my $final_size = $t - 32 * $r;
    $v = CodingAdventures::Blake2b::blake2b($v, digest_size => $final_size);
    $out .= $v;
    return $out;
}

# ---------------------------------------------------------------------------
# _index_alpha($j1, $r, $sl, $c, $same_lane, $q, $sl_len)
#
# Maps the 32-bit pseudo-random J1 value to a reference-block column
# inside the chosen lane (RFC 9106 §3.4.1.1).  The window [start, start+W)
# describes which previously computed columns are eligible to be
# referenced; J1 is biased toward recent blocks via `(W * x) >> 32`.
# ---------------------------------------------------------------------------
sub _index_alpha {
    my ($j1, $r, $sl, $c, $same_lane, $q, $sl_len) = @_;
    my ($w, $start);

    if ($r == 0 && $sl == 0) {
        # First slice of first pass: eligible columns are (0 .. c-1).
        $w     = $c - 1;
        $start = 0;
    } elsif ($r == 0) {
        # First pass, later slice: eligible = everything built so far,
        # minus at most one spot depending on lane-locality edge cases.
        $w = $same_lane ? $sl * $sl_len + $c - 1
            : $c == 0   ? $sl * $sl_len - 1
            :             $sl * $sl_len;
        $start = 0;
    } else {
        # Later pass: always q - sl_len eligible blocks.
        $w = $same_lane ? $q - $sl_len + $c - 1
            : $c == 0   ? $q - $sl_len - 1
            :             $q - $sl_len;
        $start = (($sl + 1) * $sl_len) % $q;
    }

    my $x   = ($j1 * $j1) >> 32;
    my $y   = ($w * $x)   >> 32;
    my $rel = $w - 1 - $y;
    return ($start + $rel) % $q;
}

# ---------------------------------------------------------------------------
# _fill_segment(\%memory, $r, $lane, $sl, $q, $sl_len, $p)
#
# Fills one (pass, slice, lane) segment.  Argon2d is entirely
# data-dependent: the 64 low/high bits of the previous block supply J1/J2.
# ---------------------------------------------------------------------------
sub _fill_segment {
    my ($memory, $r, $lane, $sl, $q, $sl_len, $p) = @_;

    my $starting_c = ($r == 0 && $sl == 0) ? 2 : 0;

    for my $i ($starting_c .. $sl_len - 1) {
        my $col = $sl * $sl_len + $i;
        my $prev_col = $col == 0 ? $q - 1 : $col - 1;
        my $prev_block = $memory->{$lane}[$prev_col];

        my $pseudo_rand = $prev_block->[0];
        my $j1 = $pseudo_rand & MASK32;
        my $j2 = ($pseudo_rand >> 32) & MASK32;

        my $l_prime = $lane;
        $l_prime = $j2 % $p unless ($r == 0 && $sl == 0);

        my $z_prime = _index_alpha($j1, $r, $sl, $i, $l_prime == $lane,
                                   $q, $sl_len);
        my $ref_block = $memory->{$l_prime}[$z_prime];

        my $new_block = _compress($prev_block, $ref_block);
        if ($r == 0) {
            $memory->{$lane}[$col] = $new_block;
        } else {
            my $existing = $memory->{$lane}[$col];
            $memory->{$lane}[$col] =
                [ map { $existing->[$_] ^ $new_block->[$_] }
                      0 .. BLOCK_WORDS - 1 ];
        }
    }
}

# ---------------------------------------------------------------------------
# _validate(...) -- enforce the RFC 9106 parameter bounds.
# ---------------------------------------------------------------------------
sub _validate {
    my ($password, $salt, $t, $m, $p, $tag_length, $key, $ad, $version) = @_;

    die "password length must fit in 32 bits\n" if length($password) > MASK32;
    die "salt must be at least 8 bytes\n"       if length($salt) < 8;
    die "salt length must fit in 32 bits\n"     if length($salt) > MASK32;
    die "key length must fit in 32 bits\n"      if length($key)  > MASK32;
    die "associated_data length must fit in 32 bits\n"
        if length($ad) > MASK32;
    die "tag_length must be >= 4\n"             if $tag_length < 4;
    die "tag_length must fit in 32 bits\n"      if $tag_length > MASK32;
    die "parallelism must be in [1, 2^24-1]\n"
        unless defined $p && $p =~ /\A\d+\z/ && $p >= 1 && $p <= 0xFFFFFF;
    die "memory_cost must be >= 8*parallelism\n" if $m < 8 * $p;
    die "memory_cost must fit in 32 bits\n"      if $m > MASK32;
    die "time_cost must be >= 1\n"               if $t < 1;
    die "only Argon2 v1.3 (0x13) is supported\n"
        unless $version == ARGON2_VERSION;
}

# ---------------------------------------------------------------------------
# argon2d($password, $salt, $time_cost, $memory_cost, $parallelism,
#         $tag_length, %opts) -> raw tag bytes
#
# %opts keys:
#   key              -- optional MAC secret (default '')
#   associated_data  -- optional context bytes (default '')
#   version          -- only 0x13 supported (default)
# ---------------------------------------------------------------------------
sub argon2d {
    my ($password, $salt, $time_cost, $memory_cost, $parallelism,
        $tag_length, %opts) = @_;

    my $key     = $opts{key}             // '';
    my $ad      = $opts{associated_data} // '';
    my $version = $opts{version}         // ARGON2_VERSION;

    # Treat every input as a raw byte string.  A caller that hands us a
    # scalar with wide (> 0xFF) codepoints is almost always a bug -- the
    # alternative is to silently byte-encode it as UTF-8 and produce a
    # tag that no reference Argon2 implementation can reproduce.  Refuse.
    for my $name_and_val (
        [ password        => \$password ],
        [ salt            => \$salt     ],
        [ key             => \$key      ],
        [ associated_data => \$ad       ],
    ) {
        my ($name, $ref) = @$name_and_val;
        utf8::downgrade($$ref, 1)
            or die "$name must be a byte string; refusing wide characters\n";
    }

    _validate($password, $salt, $time_cost, $memory_cost, $parallelism,
              $tag_length, $key, $ad, $version);

    my $segment_length = int($memory_cost / (SYNC_POINTS * $parallelism));
    my $m_prime        = $segment_length * SYNC_POINTS * $parallelism;
    my $q              = int($m_prime / $parallelism);
    my $sl_len         = $segment_length;
    my $p              = $parallelism;
    my $t              = $time_cost;

    # H0 = BLAKE2b( LE32(p) || LE32(T) || LE32(m) || LE32(t) || LE32(v)
    #             || LE32(type) || LE32(|P|)||P || LE32(|S|)||S
    #             || LE32(|K|)||K || LE32(|X|)||X )
    my $h0_in =
          _le32($p) . _le32($tag_length) . _le32($memory_cost) . _le32($t)
        . _le32($version) . _le32(TYPE_D)
        . _le32(length $password) . $password
        . _le32(length $salt)     . $salt
        . _le32(length $key)      . $key
        . _le32(length $ad)       . $ad;
    my $h0 = CodingAdventures::Blake2b::blake2b($h0_in, digest_size => 64);

    # Memory is addressed as memory{lane}[col].  Using a hash-of-arrays
    # rather than an array-of-arrays avoids autovivification surprises.
    my %memory;
    for my $i (0 .. $p - 1) {
        $memory{$i} = [];
        my $b0 = _blake2b_long(BLOCK_SIZE, $h0 . _le32(0) . _le32($i));
        my $b1 = _blake2b_long(BLOCK_SIZE, $h0 . _le32(1) . _le32($i));
        $memory{$i}[0] = _bytes_to_block($b0);
        $memory{$i}[1] = _bytes_to_block($b1);
    }

    for my $r (0 .. $t - 1) {
        for my $sl (0 .. SYNC_POINTS - 1) {
            for my $lane (0 .. $p - 1) {
                _fill_segment(\%memory, $r, $lane, $sl, $q, $sl_len, $p);
            }
        }
    }

    # Final block = XOR of last column across all lanes.
    my @final = @{ $memory{0}[$q - 1] };
    for my $lane (1 .. $p - 1) {
        my $last = $memory{$lane}[$q - 1];
        for my $k (0 .. BLOCK_WORDS - 1) {
            $final[$k] ^= $last->[$k];
        }
    }

    return _blake2b_long($tag_length, _block_to_bytes(\@final));
}

# ---------------------------------------------------------------------------
# argon2d_hex(...)  --  like argon2d but returns lowercase hex.
# ---------------------------------------------------------------------------
sub argon2d_hex {
    return unpack("H*", argon2d(@_));
}

1;

__END__

=head1 NAME

CodingAdventures::Argon2d - Pure Perl Argon2d password hashing (RFC 9106)

=head1 SYNOPSIS

  use CodingAdventures::Argon2d;

  my $tag = CodingAdventures::Argon2d::argon2d(
      $password, $salt, 3, 32, 4, 32,
      key => $key, associated_data => $ad,
  );

  my $hex = CodingAdventures::Argon2d::argon2d_hex(
      $password, $salt, 3, 32, 4, 32,
  );

=head1 DESCRIPTION

Pure Perl implementation of Argon2d, the data-dependent Argon2 variant
(RFC 9106).  Argon2d maximises resistance to GPU/ASIC cracking by
choosing every reference block based on the password-derived state;
this makes memory-access timing a side channel, so Argon2d is unsuited
to password hashing in adversarial environments.  Use
L<CodingAdventures::Argon2id> for password hashing.

=head1 SECURITY

This is a pure-Perl reference implementation.  Argon2 is designed to
burn memory and CPU on purpose; callers control the DoS boundary via
the C<memory_cost>, C<time_cost>, and C<parallelism> parameters.  The
module does not attempt constant-time tag comparison -- callers MUST
compare tags with a constant-time equality function of their own.

=head1 SEE ALSO

L<https://datatracker.ietf.org/doc/html/rfc9106>

=cut
