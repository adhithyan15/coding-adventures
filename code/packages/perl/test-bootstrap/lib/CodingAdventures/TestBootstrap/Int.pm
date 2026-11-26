package CodingAdventures::TestBootstrap::Int;

# Arbitrary-size integers over Perl's 64-bit IVs and core Math::BigInt
# (VER05p section 3).
#
# THE TRAP.  A Perl scalar is an IV, a UV, an NV (a double) or a string, and
# `+`, `-`, `*` silently turn into NV arithmetic the moment a result leaves 64
# bits.  A double has 53 bits of mantissa, so `2**63 + 1` is `2**63`, and
# SplitMix64 quietly becomes a different (and worse) generator.  So the rules of
# this package are:
#
#   * an `Int` is EITHER a native integer in [-2**63, 2**63 - 1] OR a
#     Math::BigInt outside that range (the representation is NORMALIZED, so a
#     Math::BigInt is always out of native range and `cmp` between the two kinds
#     is decided by the sign alone);
#   * native arithmetic is attempted only when both operands are below 2**62 in
#     magnitude, so the result cannot overflow; everything else goes through
#     Math::BigInt and is normalized back;
#   * an Int must be a scalar Perl holds AS AN INTEGER (the IOK flag): the float
#     literal 3.0, the string "5" and undef are refused (`check`), so a float can
#     never reach the PRNG path even when it is integral;
#   * the digits of an integer given as text are counted BEFORE it is parsed.
#
# "Words" (unsigned 64-bit patterns used by the PRNG) are plain non-negative
# UV scalars and live in Prng.pm; `from_u64` / `to_u64` convert.

use strict;
use warnings;

use B ();
use Config ();
use Math::BigInt ();
use Scalar::Util ();

BEGIN {
    if ( $Config::Config{ivsize} != 8 || $Config::Config{uvsize} != 8 ) {
        die "CodingAdventures::TestBootstrap needs a perl with 64-bit integers "
          . "(ivsize=$Config::Config{ivsize}, uvsize=$Config::Config{uvsize})\n";
    }
}

use constant HALF    => 4_611_686_018_427_387_904;       # 2**62
use constant IV_MAX  => 9_223_372_036_854_775_807;       # 2**63 - 1
use constant MAX_DIGITS => 1000;
use constant MAX_SHIFT  => 1 << 20;
use constant MAX_CHECK_DIGITS => 5000;    # about the 16,384-bit span cap

my $BIG_MAX = Math::BigInt->new('9223372036854775807');
my $BIG_MIN = Math::BigInt->new('-9223372036854775808');
my $BIG_2_64 = Math::BigInt->new('18446744073709551616');
sub two64 { return $BIG_2_64->copy }

my $blessed = \&Scalar::Util::blessed;

# ---------------------------------------------------------------- predicates

# True for the Math::BigInt of an Int (exactly that class: a subclass is not ours).
sub is_big {
    my $c = $blessed->( $_[0] );
    return ( defined $c && $c eq 'Math::BigInt' ) ? 1 : 0;
}

# True for a native integer scalar (no reference, IOK, defined).
sub _is_native_scalar {
    return 0 if !defined $_[0] || ref $_[0];
    return ( B::svref_2object( \$_[0] )->FLAGS & B::SVf_IOK ) ? 1 : 0;
}

sub is { return ( is_big( $_[0] ) || _is_native_scalar( $_[0] ) ) ? 1 : 0 }

# Validates an argument and returns it as an Int (a UV above 2**63 - 1 becomes a
# big).  `die`s for a float, string, undef or any other reference.
sub check {
    my $name = defined $_[1] ? $_[1] : 'value';
    if ( ref $_[0] ) {
        die "$name must be an Integer, not a reference\n" if !is_big( $_[0] );
        # A caller's big is never trusted: refuse NaN and infinity, cap its digits
        # BEFORE any text conversion, and return a normalized private copy (so a later
        # mutation of the caller's object cannot change what the engine holds, and a
        # small value is native, as `cmp` assumes).
        my $b = $_[0];
        die "$name must be a finite Integer\n" if $b->is_nan || $b->is_inf;
        die "$name exceeds " . MAX_CHECK_DIGITS . " digits\n" if $b->length > MAX_CHECK_DIGITS;
        return _norm( $b->copy );
    }
    die "$name must be an Integer, not undef\n" if !defined $_[0];
    my $f = B::svref_2object( \$_[0] )->FLAGS;
    die "$name must be an Integer (a float or a string is never a number here)\n"
      if !( $f & B::SVf_IOK );
    if ( ( $f & B::SVf_IVisUV ) && $_[0] > IV_MAX ) {
        return from_u64( $_[0] );
    }
    return $_[0];
}

# --------------------------------------------------------------------- text

# Decimal text -> Int.  Canonical text only; the digit cap is applied first.
sub parse {
    my ($text) = @_;
    die "integer text must be a string\n" if !defined $text || ref $text;
    die "non-canonical integer\n" if $text !~ /\A(?:0|-?[1-9][0-9]*)\z/;
    my $digits = length($text) - ( substr( $text, 0, 1 ) eq '-' ? 1 : 0 );
    die "integer exceeds " . MAX_DIGITS . " digits\n" if $digits > MAX_DIGITS;
    return 0 + $text if $digits <= 18;
    return _norm( Math::BigInt->new($text) );
}

sub tostring {
    my ($x) = @_;
    return is_big($x) ? $x->bstr : "$x";
}

# ---------------------------------------------------------------- internals

sub _big { return is_big( $_[0] ) ? $_[0] : Math::BigInt->new( "$_[0]" ) }

# A Math::BigInt result -> a normalized Int (a fresh object when it stays big).
sub _norm {
    my ($b) = @_;
    my $c = $blessed->($b);
    die "integer arithmetic left Math::BigInt\n" if !defined $c || $c ne 'Math::BigInt';
    die "integer arithmetic produced NaN or infinity\n" if $b->is_nan || $b->is_inf;
    if ( $b->bcmp($BIG_MAX) <= 0 && $b->bcmp($BIG_MIN) >= 0 ) {
        return 0 + $b->bstr;
    }
    return $b;
}

sub _small { return $_[0] > -HALF && $_[0] < HALF }

# --------------------------------------------------------------- comparison

sub cmp {
    my ( $a, $b ) = @_;
    my ( $ba, $bb ) = ( is_big($a), is_big($b) );
    return $a <=> $b if !$ba && !$bb;
    # Normalized bigs are decided by sign alone, but a big that was never normalized
    # (a caller's small Math::BigInt that bypassed `check`) must still compare right.
    my ( $x, $y ) = ( _big($a), _big($b) );
    die "NaN and infinity are not comparable\n" if $x->is_nan || $x->is_inf || $y->is_nan || $y->is_inf;
    return $x->bcmp($y);
}

sub eq { return CodingAdventures::TestBootstrap::Int::cmp( $_[0], $_[1] ) == 0 ? 1 : 0 }
sub le { return CodingAdventures::TestBootstrap::Int::cmp( $_[0], $_[1] ) <= 0 ? 1 : 0 }

sub sign {
    my ($x) = @_;
    return $x->is_neg ? -1 : 1 if is_big($x);
    return $x <=> 0;
}

# --------------------------------------------------------------- arithmetic

sub add {
    my ( $a, $b ) = @_;
    return $a + $b if !is_big($a) && !is_big($b) && _small($a) && _small($b);
    return _norm( _big($a)->copy->badd( _big($b) ) );
}

sub sub {
    my ( $a, $b ) = @_;
    return $a - $b if !is_big($a) && !is_big($b) && _small($a) && _small($b);
    return _norm( _big($a)->copy->bsub( _big($b) ) );
}

sub neg {
    my ($x) = @_;
    return -$x if !is_big($x) && _small($x);
    return _norm( _big($x)->copy->bneg );
}

sub abs {
    my ($x) = @_;
    return $x < 0 ? -$x : $x if !is_big($x) && _small($x);
    return _norm( _big($x)->copy->babs );
}

# Floored modulo by a POSITIVE modulus: the result is in [0, m).
sub mod {
    my ( $a, $m ) = @_;
    die "modulus must be positive\n" if sign($m) < 1;
    return $a % $m if !is_big($a) && !is_big($m);
    return _norm( _big($a)->copy->bmod( _big($m) ) );
}

# floor(x / 2): `-81` is `-41`, not `-40` (truncation would give the wrong
# bisection candidate for a negative difference).
sub floor_half {
    my ($x) = @_;
    if ( !is_big($x) ) {
        use integer;    # C division: truncates toward zero, defined for these operands
        my $q = $x / 2;
        $q-- if ( $x % 2 ) < 0;
        return $q;
    }
    my $q = $x->copy->bdiv(2);    # Math::BigInt division is floored; scalar context gives the quotient
    return _norm($q);
}

sub is_even { return is_big( $_[0] ) ? ( $_[0]->is_even ? 1 : 0 ) : ( $_[0] % 2 == 0 ? 1 : 0 ) }

sub pow2 {
    my ($n) = @_;
    die "shift outside [0, " . MAX_SHIFT . "]\n" if $n < 0 || $n > MAX_SHIFT;
    return _norm( Math::BigInt->new(1)->blsft($n) );
}

sub shl {
    my ( $x, $n ) = @_;
    die "shift outside [0, " . MAX_SHIFT . "]\n" if $n < 0 || $n > MAX_SHIFT;
    return _norm( _big($x)->copy->blsft($n) );
}

# Number of bits of |x| (0 for 0).
sub bit_length {
    my ($x) = @_;
    if ( !is_big($x) && $x >= 0 ) {
        my ( $v, $n ) = ( $x, 0 );
        while ($v) { $v >>= 1; $n++ }
        return $n;
    }
    # Math::BigInt's as_bin / as_hex are quadratic (11 s for 200,000 bits), so the
    # length comes from the decimal text: estimate from the leading digits, then
    # correct the estimate exactly by comparing with powers of two.
    my $b = _big($x)->copy->babs;
    die "bit_length of NaN or infinity\n" if $b->is_nan || $b->is_inf;
    my $dec    = $b->bstr;
    my $digits = length $dec;
    my $lead   = substr( $dec, 0, 15 ) + 0;
    my $lg     = log($lead) / log(2) + ( $digits - length( substr( $dec, 0, 15 ) ) ) * ( log(10) / log(2) );
    my $est    = int($lg) + 1;
    $est = 1 if $est < 1;
    # Above 5,000 digits (about 16,600 bits, past every cap that asks) the estimate is
    # used as is, within one bit: the exact correction needs huge shifts that are slow.
    return $est if $digits > 5000;
    my $guard = 0;
    # every correction loop is bounded: the estimate is within a few bits
    $est-- while ++$guard < 64 && $est > 1 && $b->bcmp( Math::BigInt->new(1)->blsft( $est - 1 ) ) < 0;
    $est++ while ++$guard < 128 && $b->bcmp( Math::BigInt->new(1)->blsft($est) ) >= 0;
    die "bit_length did not converge\n" if $guard >= 64;
    return $est;
}

# ------------------------------------------------------------- word <-> Int

# A word (a non-negative UV, 0 .. 2**64 - 1) as an Int.
sub from_u64 {
    my ($w) = @_;
    return $w if $w <= IV_MAX;
    return Math::BigInt->new( sprintf( '%u', $w ) );
}

# An Int of any size reduced modulo 2**64 (two's complement: -1 is 2**64 - 1),
# as a word.
sub to_u64 {
    my ($x) = @_;
    if ( !is_big($x) ) {
        return $x if $x >= 0;
        return $x & ~0;    # ~0 is 2**64 - 1: the two's complement pattern
    }
    return 0 + $x->copy->bmod($BIG_2_64)->bstr;
}

1;
