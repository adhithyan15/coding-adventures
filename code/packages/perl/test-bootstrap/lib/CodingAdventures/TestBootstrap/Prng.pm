package CodingAdventures::TestBootstrap::Prng;

# Deterministic randomness and hashing for test runs (TF01 5.1, VER05 2,
# VER05p section 3).
#
# SplitMix64 and FNV-1a are defined on 64-bit words that WRAP.  Perl has no
# wrapping 64-bit arithmetic: `+` and `*` overflow into a double.  So a "word"
# here is a non-negative UV and the two wrapping operations are built from
# 32-bit halves (`add64`, `mul64`), where every intermediate is below 2**53 and
# therefore exact whatever the scalar's type.  Shifts, `^`, `&`, `|` and `~` on
# non-negative integers are defined and exact on UVs; `<` and `%` on UVs are
# exact unsigned operations.  Nothing here uses `use integer`, `/` or `**`.

use strict;
use warnings;
no warnings q(portable);    # 64-bit hex constants are the point

use Scalar::Util ();

use CodingAdventures::TestBootstrap::Int    ();
use CodingAdventures::TestBootstrap::Hooks  ();
use CodingAdventures::TestBootstrap::Limits ();

my $Int = 'CodingAdventures::TestBootstrap::Int';

use constant {
    M32          => 0xFFFFFFFF,
    GOLDEN_GAMMA => 0x9E3779B97F4A7C15,
    MIX_MUL1     => 0xBF58476D1CE4E5B9,
    MIX_MUL2     => 0x94D049BB133111EB,
    FNV_OFFSET   => 0xCBF29CE484222325,
    FNV_PRIME    => 0x00000100000001B3,
};

# (a + b) mod 2**64.  The low halves sum to at most 2**33, so the carry is exact.
sub add64 {
    my ( $a, $b ) = @_;
    my $lo = ( $a & M32 ) + ( $b & M32 );
    my $hi = ( ( $a >> 32 ) + ( $b >> 32 ) + ( $lo >> 32 ) ) & M32;
    return ( $hi << 32 ) | ( $lo & M32 );
}

# (x * y) mod 2**32 for 32-bit x, y, from 16-bit limbs (products below 2**32).
sub _mul32lo {
    my ( $x, $y ) = @_;
    my ( $xl, $xh, $yl, $yh ) = ( $x & 0xFFFF, $x >> 16, $y & 0xFFFF, $y >> 16 );
    return ( $xl * $yl + ( ( ( $xh * $yl + $xl * $yh ) & 0xFFFF ) << 16 ) ) & M32;
}

# (a * b) mod 2**64.  With a = a1*2**32 + a0 and b = b1*2**32 + b0 the result is
# a0*b0 + ((a1*b0 + a0*b1) mod 2**32) * 2**32, and the full 64-bit product a0*b0
# is assembled from 16-bit limbs so no partial product exceeds 2**33.
sub mul64 {
    my ( $a, $b ) = @_;
    my ( $a0, $a1, $b0, $b1 ) = ( $a & M32, $a >> 32, $b & M32, $b >> 32 );
    my ( $al, $ah, $bl, $bh ) = ( $a0 & 0xFFFF, $a0 >> 16, $b0 & 0xFFFF, $b0 >> 16 );
    my $ll  = $al * $bl;                 # < 2**32
    my $mid = $al * $bh + $ah * $bl;     # < 2**33
    my $hh  = $ah * $bh;                 # < 2**32
    my $p   = add64( add64( $ll, $mid << 16 ), $hh << 32 );
    my $cross = ( _mul32lo( $a1, $b0 ) + _mul32lo( $a0, $b1 ) ) & M32;
    return add64( $p, $cross << 32 );
}

# The rejection threshold t = (2**64 - n) mod n for a word n != 0, UNSIGNED and
# without forming 2**64: 2**64 - n = (~n) + 1, so t = ((~n % n) + 1) % n.
# (A signed `(-n) % n` is wrong for every n.)
sub threshold {
    my ($n) = @_;
    die "threshold of zero\n" if $n == 0;
    my $r = ( ~$n ) % $n;
    $r = $r + 1;
    return $r == $n ? 0 : $r;
}

# ---------------------------------------------------------------- Rng

package CodingAdventures::TestBootstrap::Prng::Rng;

use strict;
use warnings;

# The state IS the stream: never printed.
use overload '""' => sub { '#<Prng.SplitMix64>' }, fallback => 1;

# SplitMix64 step; returns the output WORD.
#
#   state += golden gamma
#   z1  = (z ^ (z >> 30)) * 0xbf58476d1ce4e5b9
#   z2  = (z1 ^ (z1 >> 27)) * 0x94d049bb133111eb
#   out = z2 ^ (z2 >> 31)
#
# Seed 0's first output is 0xe220a8397b1dcdaf.
sub next_word {
    my ($self) = @_;
    my $s = CodingAdventures::TestBootstrap::Prng::add64( $self->{state}, CodingAdventures::TestBootstrap::Prng::GOLDEN_GAMMA );
    $self->{state} = $s;
    my $z = CodingAdventures::TestBootstrap::Prng::mul64( $s ^ ( $s >> 30 ), CodingAdventures::TestBootstrap::Prng::MIX_MUL1 );
    $z = CodingAdventures::TestBootstrap::Prng::mul64( $z ^ ( $z >> 27 ), CodingAdventures::TestBootstrap::Prng::MIX_MUL2 );
    return $z ^ ( $z >> 31 );
}

sub next_u64 { return CodingAdventures::TestBootstrap::Int::from_u64( $_[0]->next_word ) }

# Uniform word in [0, n) for a word n != 0, by rejection sampling.  Raws below t
# are redrawn so the remaining 2**64 - t values split into whole residue classes.
# bounded(1) still consumes one word (the VER05 2.2 loop run once): the stream
# position must be identical in every lane.
#
# Worked example, n = 3: 2**64 = 3 * 6148914691236517205 + 1, so t = 1 and the
# single raw 0 is thrown away.
sub bounded_word {
    my ( $self, $n ) = @_;
    my $hook = $CodingAdventures::TestBootstrap::Hooks::H{threshold};
    my $t = $hook ? $hook->($n) : CodingAdventures::TestBootstrap::Prng::threshold($n);
    while (1) {
        my $x = $self->next_word;
        return $x % $n if !( $x < $t );
    }
}

# Uniform Int in [0, n) for an Int 1 <= n <= 2**64 (2**64 is one raw word).
sub bounded {
    my ( $self, $n ) = @_;
    $n = CodingAdventures::TestBootstrap::Int::check( $n, 'bounded(n)' );
    die "bounded(n) requires n >= 1\n" if CodingAdventures::TestBootstrap::Int::sign($n) < 1;
    my $c = CodingAdventures::TestBootstrap::Int::cmp( $n, CodingAdventures::TestBootstrap::Int::two64() );
    die "bounded(n) requires n <= 2^64\n" if $c > 0;
    return $self->next_u64 if $c == 0;
    return CodingAdventures::TestBootstrap::Int::from_u64( $self->bounded_word( CodingAdventures::TestBootstrap::Int::to_u64($n) ) );
}

# Uniform Int in [0, span) for any span >= 1 (VER05 7.2).
#
#   span <= 2**64   bounded(span)
#   span  > 2**64   k-word rejection: k = max(2, ceil(bitlen(span-1)/64)),
#                   first draw most significant, t = (2**(64k) - span) mod span
sub bounded_big {
    my ( $self, $span ) = @_;
    my $I = 'CodingAdventures::TestBootstrap::Int';
    $span = CodingAdventures::TestBootstrap::Int::check( $span, 'span' );
    die "span must be >= 1\n" if CodingAdventures::TestBootstrap::Int::sign($span) < 1;
    return $self->bounded($span) if CodingAdventures::TestBootstrap::Int::cmp( $span, CodingAdventures::TestBootstrap::Int::two64() ) <= 0;
    my $bits = CodingAdventures::TestBootstrap::Int::bit_length( CodingAdventures::TestBootstrap::Int::sub( $span, 1 ) );
    die "span wider than " . CodingAdventures::TestBootstrap::Limits::MAX_SPAN_BITS . " bits\n"
      if $bits > CodingAdventures::TestBootstrap::Limits::MAX_SPAN_BITS;
    my $words = int( ( $bits + 63 ) / 64 );
    $words = 2 if $words < 2;
    my $t = CodingAdventures::TestBootstrap::Int::mod( CodingAdventures::TestBootstrap::Int::sub( CodingAdventures::TestBootstrap::Int::pow2( 64 * $words ), $span ), $span );
    while (1) {
        my $raw = 0;
        for ( 1 .. $words ) {
            $raw = CodingAdventures::TestBootstrap::Int::add( CodingAdventures::TestBootstrap::Int::shl( $raw, 64 ), $self->next_u64 );
        }
        return CodingAdventures::TestBootstrap::Int::mod( $raw, $span ) if CodingAdventures::TestBootstrap::Int::cmp( $raw, $t ) >= 0;
    }
}

package CodingAdventures::TestBootstrap::Prng;

# A new SplitMix64 stream.  `$seed` is an Int of any size, reduced modulo 2**64
# (two's complement: -1 is 2**64 - 1).  A float, string or undef is refused.
sub new {
    my ($seed) = @_;
    $seed = CodingAdventures::TestBootstrap::Int::check( $seed, 'seed' );
    return bless { state => CodingAdventures::TestBootstrap::Int::to_u64($seed) }, 'CodingAdventures::TestBootstrap::Prng::Rng';
}

# The factory the engine uses; `Hooks::H{new_rng}` replaces it for sabotage tests.
sub new_rng {
    my ($seed) = @_;
    my $hook = $CodingAdventures::TestBootstrap::Hooks::H{new_rng};
    return $hook->($seed) if $hook;
    return new($seed);
}

# hash * 0x100000001b3 (mod 2**64) without the general `mul64`: the prime is
# 2**40 + 435, so the product is (hash << 40) + hash * 435.  The halves times 435
# stay below 2**42, so every intermediate is exact; the UV shift discards the high
# bits (that is the wrap).  `fnv_mul_general` is the reference the tests compare it with.
sub _fnv_mul {
    my ($h) = @_;
    my $lo = ( $h & M32 ) * 435;
    my $hi = ( $h >> 32 ) * 435;
    return add64( add64( $lo, ( $hi & M32 ) << 32 ), $h << 40 );
}

sub fnv_mul_general { return mul64( $_[0], FNV_PRIME ) }
sub fnv_mul         { return _fnv_mul( $_[0] ) }

# 64-bit FNV-1a of a byte string, as a word.
sub fnv1a64_word {
    my ($data) = @_;
    die "fnv1a64 expects a string\n" if !defined $data || ref $data;
    my $bytes = $data;
    if ( utf8::is_utf8($bytes) ) {
        utf8::downgrade( $bytes, 1 ) or utf8::encode($bytes);
    }
    my $hash = FNV_OFFSET;
    for my $b ( unpack 'C*', $bytes ) {
        $hash = _fnv_mul( $hash ^ $b );
    }
    return $hash;
}

sub fnv1a64 { return CodingAdventures::TestBootstrap::Int::from_u64( fnv1a64_word( $_[0] ) ) }

# sub_seed(run_seed, id) = SplitMix64(run_seed ^ fnv1a64(id)).next_u64  (VER05 2.3)
sub sub_seed {
    my ( $run_seed, $id ) = @_;
    $run_seed = CodingAdventures::TestBootstrap::Int::check( $run_seed, 'run_seed' );
    die "property_id must be a string\n" if !defined $id || ref $id;
    my $rng = new( CodingAdventures::TestBootstrap::Int::from_u64( CodingAdventures::TestBootstrap::Int::to_u64($run_seed) ^ fnv1a64_word($id) ) );
    return $rng->next_u64;
}

1;
