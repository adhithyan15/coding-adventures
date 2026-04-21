package CodingAdventures::Argon2i;

# ============================================================================
# CodingAdventures::Argon2i -- Pure Perl Argon2i (RFC 9106)
# ============================================================================
#
# Argon2i is the data-INDEPENDENT variant.  Reference-block indices are
# derived from a deterministic pseudo-random stream that is seeded purely
# from public parameters: (pass, lane, slice, total memory, total passes,
# type, counter).  The access pattern is therefore constant across secrets,
# which defeats memory-access side channels at the cost of making Argon2i
# the easiest variant for GPUs/ASICs to parallelise.
#
# For general password hashing prefer CodingAdventures::Argon2id.  Use
# Argon2i only when side-channel resistance dominates the threat model.
#
# Reference: https://datatracker.ietf.org/doc/html/rfc9106
# See also:  code/specs/KD03-argon2.md
#
# The arithmetic core (G mixer, permutation P, compression G, H', index
# alpha, parameter validation) is identical to Argon2d -- only the
# fill_segment routine differs.  The duplication is deliberate: each
# package is self-contained and publishable on its own.

use strict;
use warnings;
no warnings 'portable';

use Config;
BEGIN {
    die "CodingAdventures::Argon2i requires 64-bit Perl (ivsize >= 8); "
      . "this build has ivsize=$Config{ivsize}\n"
        if $Config{ivsize} < 8;
}

use CodingAdventures::Blake2b ();

our $VERSION = '0.01';

use constant MASK32              => 0xFFFFFFFF;
use constant MASK64              => 0xFFFFFFFFFFFFFFFF;
use constant BLOCK_SIZE          => 1024;
use constant BLOCK_WORDS         => 128;
use constant ADDRESSES_PER_BLOCK => 128;   # = BLOCK_WORDS
use constant SYNC_POINTS         => 4;
use constant ARGON2_VERSION      => 0x13;
use constant TYPE_I              => 1;

sub _rotr64 {
    my ($x, $n) = @_;
    return ((($x >> $n) | ($x << (64 - $n))) & MASK64);
}

sub _gb {
    my ($v, $a, $b, $c, $d) = @_;
    my ($va, $vb, $vc, $vd) = ($v->[$a], $v->[$b], $v->[$c], $v->[$d]);
    { use integer;
      $va = ($va + $vb + 2 * ($va & MASK32) * ($vb & MASK32)) & MASK64; }
    $vd = _rotr64($vd ^ $va, 32);
    { use integer;
      $vc = ($vc + $vd + 2 * ($vc & MASK32) * ($vd & MASK32)) & MASK64; }
    $vb = _rotr64($vb ^ $vc, 24);
    { use integer;
      $va = ($va + $vb + 2 * ($va & MASK32) * ($vb & MASK32)) & MASK64; }
    $vd = _rotr64($vd ^ $va, 16);
    { use integer;
      $vc = ($vc + $vd + 2 * ($vc & MASK32) * ($vd & MASK32)) & MASK64; }
    $vb = _rotr64($vb ^ $vc, 63);
    ($v->[$a], $v->[$b], $v->[$c], $v->[$d]) = ($va, $vb, $vc, $vd);
}

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

sub _compress {
    my ($x, $y) = @_;
    my @r = map { $x->[$_] ^ $y->[$_] } 0 .. BLOCK_WORDS - 1;
    my @q = @r;
    for my $i (0 .. 7) { _permutation_p(\@q, $i * 16); }
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

sub _block_to_bytes { pack("Q<" . BLOCK_WORDS, @{$_[0]}) }
sub _bytes_to_block { [ unpack("Q<" . BLOCK_WORDS, $_[0]) ] }
sub _le32           { pack("V", $_[0]) }

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

sub _index_alpha {
    my ($j1, $r, $sl, $c, $same_lane, $q, $sl_len) = @_;
    my ($w, $start);
    if ($r == 0 && $sl == 0) {
        $w = $c - 1; $start = 0;
    } elsif ($r == 0) {
        $w = $same_lane ? $sl * $sl_len + $c - 1
            : $c == 0   ? $sl * $sl_len - 1
            :             $sl * $sl_len;
        $start = 0;
    } else {
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
# _fill_segment  (Argon2i-specific)
#
# J1/J2 come from a deterministic address stream generated by
# double-G(0, compress(0, input_block)), where input_block carries
# (pass r, lane, slice sl, m', t_total, TYPE_I, counter) in its first
# seven words.  The counter is bumped once per 128-word chunk of the
# segment; the address block is re-derived lazily when the previous
# chunk is exhausted.
# ---------------------------------------------------------------------------
sub _fill_segment {
    my ($memory, $r, $lane, $sl, $q, $sl_len, $p, $m_prime, $t_total) = @_;

    my @input   = (0) x BLOCK_WORDS;
    my @address = (0) x BLOCK_WORDS;
    my @zero    = (0) x BLOCK_WORDS;
    $input[0] = $r;
    $input[1] = $lane;
    $input[2] = $sl;
    $input[3] = $m_prime;
    $input[4] = $t_total;
    $input[5] = TYPE_I;

    my $refresh = sub {
        $input[6]++;
        my $z = _compress(\@zero, \@input);
        @address = @{ _compress(\@zero, $z) };
    };

    my $starting_c = ($r == 0 && $sl == 0) ? 2 : 0;
    $refresh->() if $starting_c != 0;

    for my $i ($starting_c .. $sl_len - 1) {
        if ($i % ADDRESSES_PER_BLOCK == 0
            && !($r == 0 && $sl == 0 && $i == 2))
        {
            $refresh->();
        }

        my $col = $sl * $sl_len + $i;
        my $prev_col = $col == 0 ? $q - 1 : $col - 1;
        my $prev_block = $memory->{$lane}[$prev_col];

        my $pseudo_rand = $address[$i % ADDRESSES_PER_BLOCK];
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

sub argon2i {
    my ($password, $salt, $time_cost, $memory_cost, $parallelism,
        $tag_length, %opts) = @_;

    my $key     = $opts{key}             // '';
    my $ad      = $opts{associated_data} // '';
    my $version = $opts{version}         // ARGON2_VERSION;

    # Refuse wide-character input rather than silently UTF-8-encoding it
    # into a tag that no reference Argon2 implementation can reproduce.
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

    my $h0_in =
          _le32($p) . _le32($tag_length) . _le32($memory_cost) . _le32($t)
        . _le32($version) . _le32(TYPE_I)
        . _le32(length $password) . $password
        . _le32(length $salt)     . $salt
        . _le32(length $key)      . $key
        . _le32(length $ad)       . $ad;
    my $h0 = CodingAdventures::Blake2b::blake2b($h0_in, digest_size => 64);

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
                _fill_segment(\%memory, $r, $lane, $sl, $q, $sl_len,
                              $p, $m_prime, $t);
            }
        }
    }

    my @final = @{ $memory{0}[$q - 1] };
    for my $lane (1 .. $p - 1) {
        my $last = $memory{$lane}[$q - 1];
        for my $k (0 .. BLOCK_WORDS - 1) {
            $final[$k] ^= $last->[$k];
        }
    }

    return _blake2b_long($tag_length, _block_to_bytes(\@final));
}

sub argon2i_hex {
    return unpack("H*", argon2i(@_));
}

1;

__END__

=head1 NAME

CodingAdventures::Argon2i - Pure Perl Argon2i password hashing (RFC 9106)

=head1 SYNOPSIS

  use CodingAdventures::Argon2i;

  my $tag = CodingAdventures::Argon2i::argon2i(
      $password, $salt, 3, 32, 4, 32,
      key => $key, associated_data => $ad,
  );

=head1 DESCRIPTION

Argon2i derives reference-block indices from a deterministic public
stream, giving side-channel resistance at the cost of GPU/ASIC
hardening.  For general-purpose password hashing prefer
L<CodingAdventures::Argon2id>.

=head1 SEE ALSO

L<https://datatracker.ietf.org/doc/html/rfc9106>

=cut
