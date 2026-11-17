package CodingAdventures::Trig;

# ============================================================================
# CodingAdventures::Trig — Trigonometric functions from first principles
#                          (TRIG-REF-1)
# ============================================================================
#
# This module provides sine, cosine, tangent, and angle-conversion functions
# built entirely from scratch.  We do NOT use Perl's built-in sin()/cos() or
# POSIX — the goal is to understand *how* these functions work at the
# mathematical level.
#
# Since TRIG-CONV (PHY00a, step TC-15) this port follows the ONE reference
# algorithm, TRIG-REF-1, that every language port of `trig` shares, so that
# the same input produces the same 64 bits in every language.
#
# The analogy: seventeen workshops each cut a "one metre" rod from the same
# drawing.  All the rods look right and they differ by a hair.  TRIG-REF-1 is
# the master bar, the shared test vectors (code/specs/fixtures/
# trig-reference-v1) are its calibration marks, and this file is one workshop
# being re-cut to match.
#
# ## Why Build Trig from Scratch?
#
# Every language ships sin() and cos() in its standard library. Reimplementing
# them reveals:
#   1. How infinite series approximate transcendental functions.
#   2. Why "range reduction" is critical for numerical accuracy.
#   3. Why the ORDER in which you add things matters (Horner's rule).
#
# ## The Core Idea: Maclaurin Series
#
# A Maclaurin series expresses a function as an infinite sum evaluated at 0:
#
#     sin(x) = x - x^3/3! + x^5/5! - x^7/7! + ...
#     cos(x) = 1 - x^2/2! + x^4/4! - x^6/6! + ...
#
# Each successive term is much smaller than the last for |x| <= pi/4, so ten
# terms of each series are already more than a double can resolve.
#
# ## The arithmetic model
#
# Every value is an IEEE 754 binary64 number — exactly what a Perl NV is.
# Every step is ONE addition, subtraction or multiplication, rounded to
# nearest-even once.  Perl interprets one operator at a time, so a multiply
# can never be fused into the following add (no FMA), and nothing is computed
# in extended precision on a 64-bit-NV Perl.  sin, cos, radians and degrees
# use no division, no square root and no library call.
#
# ## Perl-specific care (the part that is easy to get wrong)
#
# 1. NV must be 8 bytes.  A Perl built with long doubles (-Duselongdouble) or
#    with 32-bit NVs is refused at load time.
#
# 2. Perl does INTEGER arithmetic on integer-VALUED operands, however they are
#    written (PERL_PRESERVE_IVUV): when both operands of + - * hold whole
#    numbers, the operation is done on 64-bit integers, and an integer has no
#    negative zero.  Measured on this Perl: -0.0 * 1.0 is +0, but
#    -0.0 * 0.5 is -0.  So this file NEVER "normalises" an argument with
#    `* 1.0` or `+ 0.0` (either would turn -0.0 into +0.0), and never says
#    `use integer`.  An argument is passed through unchanged until the first
#    operation that genuinely needs a double.  Where a -0 result is promised
#    it is either the argument itself, returned by an early return (sin, atan,
#    sqrt), or the product with a NON-integer constant (RAD_PER_DEG,
#    DEG_PER_RAD), or a division (tan).  The only sums of two whole numbers
#    are t + MAGIC and (t + MAGIC) - MAGIC, which stay below 2^53 where the
#    integer and double results agree.
#
#    Perl has no negative-zero INTEGER: the literals 0 and -0 are both IV 0,
#    and sin(IV 0) is +0.  Only an NV -0.0 (a literal -0.0, or the value
#    decoded from bits 0x8000000000000000) carries a sign.
#
# 3. Perl's decimal-to-NV parser has historically not been correctly rounded
#    on every platform, so every constant is built from its bits, once, at
#    load time:  unpack("d>", pack("H16", "3ff921fb54442d18")).
#
# 4. Perl prints an NV with 15 significant digits, so nothing here (and no
#    test) ever passes a result through a string.  The tests compare packed
#    bits.
#
# 5. NaN is built from its bit pattern, not from a computation, so it is the
#    same quiet NaN on every platform.
#
# ## The plan for sin(x) and cos(x)
#
#   1. Guard: NaN, infinities and |x| > 2^29 are outside the domain -> NaN.
#   2. |x| < 2^-27: sin(x) is x itself (the next term is below half an ulp),
#      cos(x) is 1.  This also makes sin(-0.0) = -0.0.
#   3. Reduce: find the nearest whole number k of quarter-turns, so that
#      r = x - k*(pi/2) satisfies |r| <= pi/4 (Cody-Waite, below).
#   4. Evaluate a fixed-length Horner polynomial for sin(r) or cos(r).
#   5. Use the quadrant q = k mod 4 to pick sin(r), cos(r), -sin(r) or -cos(r).
#
# Usage:
#
#   use CodingAdventures::Trig qw(sin_approx cos_approx tan_approx
#                                  sin_deg cos_deg tan_deg
#                                  degrees_to_radians radians_to_degrees);
#
#   my $s = sin_approx(3.14159 / 6);  # => 0.5
#   my $d = radians_to_degrees(3.14159265);  # => ~180
#
# All functions are also available as class methods:
#
#   CodingAdventures::Trig->sin_approx(1.0);
#
# ============================================================================

use strict;
use warnings;
use Config;

BEGIN {
    # See "Perl-specific care", item 1.
    die "CodingAdventures::Trig needs an 8-byte NV (this perl has nvsize=$Config{nvsize}"
      . ( $Config{uselongdouble} ? ', uselongdouble' : '' ) . ")\n"
      unless $Config{nvsize} == 8 && !$Config{uselongdouble};
}

our $VERSION = '0.3.0';

use Exporter 'import';
our @EXPORT_OK = qw(
    sin_approx  cos_approx  tan_approx
    sqrt_approx
    atan_approx atan2_approx
    sin_deg     cos_deg     tan_deg
    degrees_to_radians  radians_to_degrees
);

# ============================================================================
# Bit-exact constants
# ============================================================================
#
# Every constant below is specified by its IEEE bit pattern (PHY00a 3.2), and
# the test suite asserts those bits against the shared manifest.  _from_bits
# reinterprets 16 hex digits as a double:
#
#   pack("H16", ...)  the eight bytes, most significant first
#   unpack("d>", ...) the same eight bytes read as a big-endian double
#
# so it is a pure bit cast (no decimal parsing, no rounding).

sub _from_bits { return unpack( 'd>', pack( 'H16', $_[0] ) ); }

# PI is the ratio of a circle's circumference to its diameter: the double
# nearest the real pi, which is slightly LESS than pi.
our $PI      = _from_bits('400921fb54442d18');    # 3.141592653589793
our $TWO_PI  = _from_bits('401921fb54442d18');    # one full turn
our $HALF_PI = _from_bits('3ff921fb54442d18');    # one quarter-turn
# TWO_PI and HALF_PI are PI times a power of two, so neither rounds.

# The quiet NaN every out-of-domain input returns.  (Its payload and sign are
# not part of the contract: comparisons treat every NaN as equal.)
our $NAN = _from_bits('7ff8000000000000');

# INV_PIO2 is the double nearest 2/pi.  Multiplying x by it estimates how many
# quarter-turns x contains.
our $INV_PIO2 = _from_bits('3fe45f306dc9c883');    # 0.6366197723675814

# pi/2 stored as a SUM of five pieces (PHY00a 3.2).  The first four have at
# most 24 significant bits each, so k * PIO2[i] is computed EXACTLY for any
# whole number |k| < 2^29 (24 + 29 = 53 bits fit a double).  The fifth is the
# remainder rounded to 53 bits.  Together they carry pi/2 to 149 bits.
our @PIO2 = (
    _from_bits('3ff921fb40000000'),    # 1.570796251296997
    _from_bits('3e74442d00000000'),    # 7.549789415861596e-08
    _from_bits('3cf8469880000000'),    # 5.390302529957765e-15
    _from_bits('3b78cc5160000000'),    # 3.282003415807913e-22
    _from_bits('39f01b839a25204a'),    # 1.2706558760139879e-29
);

# MAGIC is 1.5 * 2^52.  Adding it to t (|t| < 2^51) pushes every fractional
# bit of t off the end of the 53-bit significand, so (t + MAGIC) - MAGIC is t
# rounded to the NEAREST WHOLE NUMBER, ties to even, using only an add and a
# subtract: no floor, no round, no int() of an unchecked value.
our $MAGIC = _from_bits('4338000000000000');    # 6755399441055744.0

# LIMIT = 2^29: beyond it k no longer fits the exact products above, and
# recovering x mod pi/2 would need pi to a thousand bits (Payne-Hanek, a later
# amendment).  Such arguments are OUTSIDE THE DOMAIN and give NaN.
our $LIMIT = _from_bits('41c0000000000000');    # 536870912.0

# TINY = 2^-27: below it sin(x) == x and cos(x) == 1 to the last bit.
our $TINY = _from_bits('3e40000000000000');    # 7.450580596923828e-09

# ----------------------------------------------------------------------------
# The Maclaurin coefficients, stored (not recomputed)
# ----------------------------------------------------------------------------
#
#   sin(r) = r - r^3/3! + r^5/5! - ...    S[n] = (-1)^n / (2n+1)!,  n = 1..9
#   cos(r) = 1 - r^2/2! + r^4/4! - ...    C[n] = (-1)^n / (2n)!,    n = 2..9
#
# Each is the exact rational rounded ONCE to the nearest double.  The table of
# factorials is visible right here, which keeps the package educational:
#
#   n   S[n]    value                        C[n]    value
#   1   -1/3!   -0.16666666666666666         (-1/2!, applied as 0.5 * z)
#   2   +1/5!    0.008333333333333333        +1/4!    0.041666666666666664
#   3   -1/7!   -0.0001984126984126984       -1/6!   -0.001388888888888889
#   4   +1/9!    2.7557319223985893e-06      +1/8!    2.48015873015873e-05
#   5   -1/11!  -2.505210838544172e-08       -1/10!  -2.755731922398589e-07
#   6   +1/13!   1.6059043836821613e-10      +1/12!   2.08767569878681e-09
#   7   -1/15!  -7.647163731819816e-13       -1/14!  -1.1470745597729725e-11
#   8   +1/17!   2.8114572543455206e-15      +1/16!   4.779477332387385e-14
#   9   -1/19!  -8.22063524662433e-18        -1/18!  -1.5619206968586225e-16
#
# S[0], C[0] and C[1] are unused (undef): the leading terms r and 1 - z/2 are
# added by the kernels themselves.
our @S = (
    undef,
    _from_bits('bfc5555555555555'),
    _from_bits('3f81111111111111'),
    _from_bits('bf2a01a01a01a01a'),
    _from_bits('3ec71de3a556c734'),
    _from_bits('be5ae64567f544e4'),
    _from_bits('3de6124613a86d09'),
    _from_bits('bd6ae7f3e733b81f'),
    _from_bits('3ce952c77030ad4a'),
    _from_bits('bc62f49b46814157'),
);

our @C = (
    undef, undef,
    _from_bits('3fa5555555555555'),
    _from_bits('bf56c16c16c16c17'),
    _from_bits('3efa01a01a01a01a'),
    _from_bits('be927e4fb7789f5c'),
    _from_bits('3e21eed8eff8d898'),
    _from_bits('bda93974a8c07c9d'),
    _from_bits('3d2ae7f3e733b81f'),
    _from_bits('bca6827863b97d97'),
);

# Degree conversion constants: the doubles nearest pi/180 and 180/pi.
our $RAD_PER_DEG = _from_bits('3f91df46a2529d39');    # 0.017453292519943295
our $DEG_PER_RAD = _from_bits('404ca5dc1a63c1f8');    # 57.29577951308232

# Thresholds of the legacy contracts of tan, sqrt and atan (decimal in older
# releases, now pinned by bits like everything else).
our $TAN_POLE  = _from_bits('3cd203af9ee75616');    # 1e-15: |cos| below this is a pole
our $TAN_BIG   = _from_bits('7fe1ccf385ebc8a0');    # 1e308: the pole sentinel
our $ATAN_TERM = _from_bits('3c670ef54646d497');    # 1e-17: atan series stops below this
our $SQRT_REL  = _from_bits('3cd203af9ee75616');    # 1e-15: sqrt relative stopping test
our $SQRT_TINY = _from_bits('01a56e1fc2f8f359');    # 1e-300: sqrt absolute stopping floor
our $MAX_FINITE = _from_bits('7fefffffffffffff');   # 1.7976931348623157e308

# ============================================================================
# Range reduction: Cody-Waite with a nearest-integer quotient
# ============================================================================
#
# A sine wave repeats, so knowing sin on [-pi/4, pi/4] and which quarter-turn
# we are in is enough to know it everywhere.  Reduction answers two questions:
# how many quarter-turns fit in x (a whole number k), and what is left over
# (r = x - k*pi/2, with |r| <= pi/4)?
#
# Step 1, the quotient:
#
#   t = x * INV_PIO2            one rounding; roughly x / (pi/2)
#   k = (t + MAGIC) - MAGIC     nearest whole number to t (see MAGIC above)
#
# Step 2, the remainder.  pi/2 is irrational, so k*(pi/2) cannot be formed
# exactly.  We store pi/2 as five short slices and subtract one slice at a
# time; each product k*PIO2[i] (i <= 4) is exact, so only the subtractions
# round:
#
#   x ---------------------------------------------------------- the angle
#   |-- k*PIO2_1 --|                                              24 bits
#                  |-- k*PIO2_2 --|                               next 24
#                                 |-- k*PIO2_3 --|                next 24
#                                                |- k*PIO2_4 -|   next 24
#                                                             |k*PIO2_5|
#   r = what is left, |r| <= pi/4 (plus a few ulps)
#
# Every product is written as its own statement and then subtracted: this is
# the shape that must never be fused into a multiply-add (PHY00a 5).  Perl
# executes the two statements as two operators, so it cannot be.
#
# Returns ($k, $r), with $k a whole-number NV (or IV) of magnitude below 2^29.

sub _reduce {
    my ($x) = @_;
    my $t = $x * $INV_PIO2;
    $t = $t + $MAGIC;
    my $k = $t - $MAGIC;
    my $r = $x;
    for my $piece (@PIO2) {
        my $m = $k * $piece;
        $r = $r - $m;
    }
    return ( $k, $r );
}

# ============================================================================
# The series kernels, by Horner's rule
# ============================================================================
#
# On |r| <= pi/4 write z = r*r.  Then
#
#   sin(r) = r + r*z*(S1 + S2*z + S3*z^2 + ... + S9*z^8)    r^1 .. r^19
#   cos(r) = (1 - z/2) + z^2*(C2 + C3*z + ... + C9*z^7)      r^0 .. r^18
#
# Horner evaluates the bracket innermost-first: start with the highest
# coefficient and repeatedly "multiply by z, add the next coefficient".  That
# adds the SMALLEST terms first (the accurate order), needs no division, and
# cannot be grouped two ways.  The leading term is added last so the big part
# of the answer is rounded only once.  Ten terms are far more than rounding
# can resolve; the count is fixed so that every port adds the same terms.

sub _sin_kernel {
    my ($r) = @_;
    my $z = $r * $r;
    my $p = $S[9];
    for my $n ( reverse 1 .. 8 ) {
        $p = $p * $z;
        $p = $p + $S[$n];
    }
    my $w = $r * $z;
    $w = $w * $p;
    return $r + $w;
}

sub _cos_kernel {
    my ($r) = @_;
    my $z = $r * $r;
    my $q = $C[9];
    for my $n ( reverse 2 .. 8 ) {
        $q = $q * $z;
        $q = $q + $C[$n];
    }
    my $u = 0.5 * $z;    # exact: halving never rounds
    $u = 1.0 - $u;
    my $v = $z * $z;
    $v = $v * $q;
    return $u + $v;
}

# ============================================================================
# sin_approx — Sine
# ============================================================================
#
# With k the quarter-turn count and q = k mod 4, the identities
#
#     sin(r + k*pi/2) = { sin r, cos r, -sin r, -cos r }[q]
#
# give the answer.  k is a whole number of magnitude below 2^29, so int($k)
# is exact, and `& 3` on a two's-complement integer is the FLOOR modulus 4
# (k = -1 gives q = 3, k = -5 gives q = 3).  Perl's `&` works on the 64-bit
# two's-complement pattern of a negative integer, as long as `use integer` is
# NOT in force.
#
# Quadrant table (q: sin(x), cos(x)):
#
#     q = 0:   sin r      cos r
#     q = 1:   cos r     -sin r
#     q = 2:  -sin r     -cos r
#     q = 3:  -cos r      sin r
#
# The domain guard is written as a NEGATED <=, and must be: NaN fails every
# comparison, so `not (|x| <= LIMIT)` catches NaN, both infinities and every
# finite value beyond 2^29 with ONE test.  (Writing `|x| > LIMIT` would let a
# NaN through.)  The guard comes first, then TINY.
#
# The early return `return $x` is what makes sin(-0.0) = -0.0: the general
# path computes r + r*z*p, and -0 + +0 = +0.  It returns the argument itself,
# untouched (see "Perl-specific care", item 2).
#
# @param  $x   Angle in radians (or class name if called as method).
# @return      Sine of x; NaN if x is NaN, infinite or |x| > 2^29.

sub sin_approx {
    # Support both function call (sin_approx($x)) and method call
    # (CodingAdventures::Trig->sin_approx($x)).
    my $x = ( @_ == 2 ) ? $_[1] : $_[0];

    return $NAN unless abs($x) <= $LIMIT;    # NaN, +-inf, |x| > 2^29
    return $x   if abs($x) < $TINY;          # exact; keeps sin(-0.0) = -0.0

    my ( $k, $r ) = _reduce($x);
    my $q = int($k) & 3;
    return _sin_kernel($r)  if $q == 0;
    return _cos_kernel($r)  if $q == 1;
    return -_sin_kernel($r) if $q == 2;
    return -_cos_kernel($r);
}

# ============================================================================
# cos_approx — Cosine
# ============================================================================
#
# Same reduction and quadrant machinery as sine; see the table above.
#
#     cos(r + k*pi/2) = { cos r, -sin r, -cos r, sin r }[q]
#
# THE PYTHAGOREAN IDENTITY
#
# For all x:  sin^2(x) + cos^2(x) = 1.  The tests verify it across many angles
# as a sanity check on both implementations.
#
# @param  $x   Angle in radians.
# @return      Cosine of x; NaN if x is NaN, infinite or |x| > 2^29.

sub cos_approx {
    my $x = ( @_ == 2 ) ? $_[1] : $_[0];

    return $NAN unless abs($x) <= $LIMIT;
    return 1.0  if abs($x) < $TINY;

    my ( $k, $r ) = _reduce($x);
    my $q = int($k) & 3;
    return _cos_kernel($r)  if $q == 0;
    return -_sin_kernel($r) if $q == 1;
    return -_cos_kernel($r) if $q == 2;
    return _sin_kernel($r);
}

# ============================================================================
# tan_approx — Tangent
# ============================================================================
#
# DEFINITION
#
#     tan(x) = sin(x) / cos(x)
#
# GEOMETRIC INTERPRETATION
#
# On the unit circle, if you draw a vertical line tangent to the circle at
# (1, 0), then tan(x) is the y-coordinate where the angle's ray intersects
# that tangent line.  This is the literal origin of the name "tangent".
#
# WHERE TANGENT IS UNDEFINED
#
# tan(x) is undefined where cos(x) = 0, i.e., at x = pi/2 + k*pi.  PHY00 3.6
# documents a SENTINEL rather than +-Inf: when |cos| < 1e-15 the result is
# +-1e308, signed like the sine.  This is a pinned legacy contract, not an
# approximation (tan(fl(pi/2)) is really about 1.6e16), and NO accuracy claim
# is made for tan at all: it is sin / cos, about twice the error of either,
# plus the division.
#
# The domain guard makes cos = 0 unreachable, so Perl's "Illegal division by
# zero" can never fire: outside the domain both sin and cos are NaN, the
# sentinel test `|c| < 1e-15` is false for a NaN, and NaN / NaN is NaN.
#
# This is the only division in Part I.  tan(-0.0) is sin(-0.0) / 1 = -0.0.
#
# @param  $x   Angle in radians.
# @return      Tangent of x (see above).

sub tan_approx {
    my $x = ( @_ == 2 ) ? $_[1] : $_[0];
    my $s = sin_approx($x);
    my $c = cos_approx($x);
    if ( abs($c) < $TAN_POLE ) {    # false when c is NaN
        return ( $s > 0 ) ? $TAN_BIG : -$TAN_BIG;
    }
    return $s / $c;
}

# ============================================================================
# sqrt_approx — Square Root via Newton's (Babylonian) Method
# ============================================================================
#
# Newton's method for square roots has been known since Babylonian times
# (~1700 BCE). The recurrence is:
#
#     next_guess = (guess + x / guess) / 2.0
#
# This has *quadratic convergence* — the number of correct digits doubles
# each iteration. For x = 2:
#
#     iter | guess                | correct digits
#     -----|----------------------|---------------
#     0    | 2.0                  | 0
#     1    | 1.5                  | 1
#     2    | 1.41667              | 2
#     3    | 1.41422              | 5
#     4    | 1.41421356237...     | 11+ (full precision)
#
# DOMAIN
#
# The real square root is only defined for x >= 0. Negative inputs trigger
# a Perl die() (equivalent to throwing an exception).
#
# @param  $x   The radicand (must be >= 0).
# @return      sqrt(x) to double-precision accuracy.

sub sqrt_approx {
    my $x = ( @_ == 2 ) ? $_[1] : $_[0];

    die "sqrt_approx: domain error — input $x is negative\n" if $x < 0;

    # sqrt(0) = 0 exactly.
    return $x if $x == 0.0;
    return $x if $x > $MAX_FINITE;

    my $scaled = $x;
    my $result_scale = 1.0;
    while ( $scaled < 0.25 ) {
        $scaled *= 4.0;
        $result_scale *= 0.5;
    }
    while ( $scaled >= 4.0 ) {
        $scaled *= 0.25;
        $result_scale *= 2.0;
    }

    # Initial guess: x itself for x >= 1, else 1.0.
    # For large x, starting at x converges faster than starting at 1.
    # For x in (0, 1), starting at 1.0 avoids dividing by a tiny number.
    my $guess = ( $scaled >= 1.0 ) ? $scaled : 1.0;

    # Iterate to convergence (up to 60 steps as a safety cap).
    for my $i ( 1 .. 60 ) {
        my $next = ( $guess + $scaled / $guess ) / 2.0;

        # Stop when improvement is negligibly small.
        # 1e-15 * $guess handles relative precision for large values.
        # 1e-300 is an absolute floor for subnormal inputs.
        my $improvement = abs( $next - $guess );
        return $next * $result_scale if $improvement < $SQRT_REL * $guess + $SQRT_TINY;

        $guess = $next;
    }

    return $guess * $result_scale;
}

# ============================================================================
# _atan_core — Taylor Series for atan, |x| <= 1, with Half-Angle Reduction
# ============================================================================
#
# This is a private helper for atan_approx and atan2_approx.
#
# HALF-ANGLE REDUCTION
#
# The Taylor series for atan:
#
#     atan(x) = x - x^3/3 + x^5/5 - x^7/7 + ...   (for |x| <= 1)
#
# converges slowly near x = 1 (requires ~50 terms for full precision).
# We apply the half-angle identity first:
#
#     atan(x) = 2 * atan( x / (1 + sqrt(1 + x^2)) )
#
# After reduction, |reduced| <= tan(pi/8) ~= 0.414, and the series
# converges in ~15 terms with 17-digit accuracy.
#
# ITERATIVE TERM COMPUTATION
#
#     term_0 = reduced
#     term_n = term_{n-1} * (-t^2) * (2n-1) / (2n+1)
#
# We return 2 * result to undo the half-angle halving.

sub _atan_core {
    my ($x) = @_;

    # Half-angle reduction. We use our own sqrt_approx — no POSIX sqrt.
    my $reduced = $x / ( 1.0 + sqrt_approx( 1.0 + $x * $x ) );

    my $t     = $reduced;
    my $t_sq  = $t * $t;
    my $term  = $t;
    my $result = $t;

    for my $n ( 1 .. 30 ) {
        # term_n = term_{n-1} * (-t^2) * (2n-1) / (2n+1)
        $term   = $term * ( -$t_sq ) * ( 2 * $n - 1 ) / ( 2 * $n + 1 );
        $result += $term;

        # Early exit when term is negligibly small.
        last if abs($term) < $ATAN_TERM;
    }

    return 2.0 * $result;
}

# ============================================================================
# atan_approx — Arctangent (Single-Argument)
# ============================================================================
#
# Computes atan(x), the angle θ ∈ (-π/2, π/2) such that tan(θ) = x.
#
# RANGE REDUCTION (for |x| > 1)
#
# The Taylor series converges only for |x| <= 1. For |x| > 1 we use:
#
#     atan(x)  = π/2 - atan(1/x)    for x > 1
#     atan(x)  = -π/2 - atan(1/x)   for x < -1
#
# Proof: atan(x) + atan(1/x) = π/2 for x > 0.
# If θ = atan(x), then tan(π/2 - θ) = cot(θ) = 1/x, so atan(1/x) = π/2 - θ.
#
# @param  $x   Any real number.
# @return      atan(x) in radians, in (-π/2, π/2).

sub atan_approx {
    my $x = ( @_ == 2 ) ? $_[1] : $_[0];

    # atan(x) rounds exactly to x here; avoid halving subnormals and retain -0.
    return $x if abs($x) <= $TINY;

    if    ( $x >  1.0 ) { return $HALF_PI  - _atan_core( 1.0 / $x ); }
    elsif ( $x < -1.0 ) { return -$HALF_PI - _atan_core( 1.0 / $x ); }

    return _atan_core($x);
}

# ============================================================================
# atan2_approx — Four-Quadrant Arctangent
# ============================================================================
#
# atan2($y, $x) returns the angle in (-π, π] that the vector ($x, $y)
# makes with the positive x-axis.
#
# Unlike atan($y/$x), atan2 inspects the signs of both arguments separately,
# giving the correct result in all four quadrants.
#
# WHY atan($y/$x) IS INSUFFICIENT:
#
#     atan(-1 / 1) = -π/4     (Q4 — correct)
#     atan(-1 / -1) = atan(1) = π/4  (but (-1,-1) is in Q3 — should be -3π/4!)
#
# QUADRANT DIAGRAM:
#
#          y > 0
#      Q2  |  Q1        atan2 > 0 in Q1 and Q2
#    ------+------  x   atan2 < 0 in Q3 and Q4
#      Q3  |  Q4        atan2 = ±π on the negative x-axis
#          y < 0
#
# @param  $y   y-coordinate (the "opposite" side).
# @param  $x   x-coordinate (the "adjacent" side).
# @return      Angle in radians, in (-π, π].

sub atan2_approx {
    my ( $y, $x ) = ( @_ == 3 ) ? ( $_[1], $_[2] ) : ( $_[0], $_[1] );

    # A NaN in either position is a NaN answer.  (Before TRIG-REF-1 a NaN fell
    # through every comparison to the last branch and returned 0.0, a finite
    # answer to an undefined question.)  `$y != $y` is true only for NaN.
    return $NAN if $x != $x || $y != $y;

    if    ( $x > 0.0 )                { return atan_approx( $y / $x );         }
    elsif ( $x < 0.0 && $y >= 0.0 )  { return atan_approx( $y / $x ) + $PI;   }
    elsif ( $x < 0.0 && $y <  0.0 )  { return atan_approx( $y / $x ) - $PI;   }
    elsif ( $x == 0.0 && $y >  0.0 ) { return  $HALF_PI;                       }
    elsif ( $x == 0.0 && $y <  0.0 ) { return -$HALF_PI;                       }
    else                              { return 0.0; } # both zero: undefined → 0
}

# ============================================================================
# Angle Conversion
# ============================================================================
#
# BACKGROUND
#
# There are two natural ways to measure angles:
#
#   Degrees: a full circle = 360°. This number comes from Babylonian
#   astronomy, which used base-60 arithmetic. It has nice divisors
#   (360 = 2^3 × 3^2 × 5), which is why surveyors still prefer degrees.
#
#   Radians: a full circle = 2*pi radians. This is the "natural" measure
#   because the arc length of an angle θ on the unit circle equals θ itself.
#   Calculus formulas for derivatives of sin/cos only work cleanly when
#   angles are in radians.
#
# CONVERSION FORMULA
#
# Set up a proportion: degrees/360 = radians/(2*pi)
# Solve:               radians = degrees * (pi/180)
#
# TRIG-REF-1 does this with ONE multiply by a stored, bit-pinned constant:
#
#     radians(d) = d * RAD_PER_DEG        (RAD_PER_DEG = fl(pi/180))
#     degrees(r) = r * DEG_PER_RAD        (DEG_PER_RAD = fl(180/pi))
#
# One multiply is one rounding.  The older `d * PI / 180` rounded twice and
# its intermediate `d * PI` could overflow for d > 5.7e307 although the final
# answer is finite.  The measured worst error is 0.65 ulp (radians) and
# 0.81 ulp (degrees).
#
# NaN propagates, +-inf maps to +-inf, and the sign of zero is kept
# (-0.0 * a NON-integer constant is -0.0; this is why the constants must not
# be integer-valued: see "Perl-specific care", item 2).  An integer argument
# is fine: IV * non-integer NV is done in doubles.
#
# MEMORABLE EXAMPLES
#
#     0°    -> 0
#     90°   -> pi/2   (right angle)
#     180°  -> pi     (straight line; exactly PI)
#     360°  -> 2*pi   (full circle)
#
# @param $deg   Angle in degrees.
# @return       Angle in radians.

sub degrees_to_radians {
    my $deg = ( @_ == 2 ) ? $_[1] : $_[0];
    return $deg * $RAD_PER_DEG;
}

# radians_to_degrees — Convert radians to degrees.
#
# The inverse of degrees_to_radians:  degrees = radians * (180 / pi).
#
# @param $rad   Angle in radians.
# @return       Angle in degrees.

sub radians_to_degrees {
    my $rad = ( @_ == 2 ) ? $_[1] : $_[0];
    return $rad * $DEG_PER_RAD;
}

# ============================================================================
# Convenience wrappers — degree-input versions
# ============================================================================
#
# It is common to work in degrees (especially in geometry and navigation).
# These wrappers convert the input from degrees to radians, then delegate
# to the radian-based functions.

# sin_deg — Sine of an angle in degrees.
#
# EXAMPLE
#     sin_deg(30)  = 0.5   (sin(pi/6) = 0.5)
#     sin_deg(90)  = 1.0
#     sin_deg(180) = 0.0   (approximately — floating-point may give ~1e-16)
#
# @param  $deg   Angle in degrees.
# @return        Sine.

sub sin_deg {
    my $deg = ( @_ == 2 ) ? $_[1] : $_[0];
    return sin_approx( degrees_to_radians($deg) );
}

# cos_deg — Cosine of an angle in degrees.
#
# EXAMPLE
#     cos_deg(0)   = 1.0
#     cos_deg(60)  = 0.5
#     cos_deg(90)  = 0.0   (approximately)
#
# @param  $deg   Angle in degrees.
# @return        Cosine.

sub cos_deg {
    my $deg = ( @_ == 2 ) ? $_[1] : $_[0];
    return cos_approx( degrees_to_radians($deg) );
}

# tan_deg — Tangent of an angle in degrees.
#
# EXAMPLE
#     tan_deg(45)  = 1.0
#     tan_deg(0)   = 0.0
#
# @param  $deg   Angle in degrees.
# @return        Tangent.

sub tan_deg {
    my $deg = ( @_ == 2 ) ? $_[1] : $_[0];
    return tan_approx( degrees_to_radians($deg) );
}

1;

__END__

=encoding utf8

=head1 NAME

CodingAdventures::Trig - Trigonometric functions from first principles (TRIG-REF-1)

=head1 SYNOPSIS

    use CodingAdventures::Trig qw(sin_approx cos_approx tan_approx
                                   sqrt_approx atan_approx atan2_approx
                                   sin_deg cos_deg tan_deg
                                   degrees_to_radians radians_to_degrees);

    my $s = sin_approx(3.14159265 / 6);  # 0.5
    my $c = cos_deg(60);                  # 0.5
    my $r = degrees_to_radians(180);      # pi

=head1 DESCRIPTION

Pure-Perl trigonometric functions implemented using Maclaurin series without
calling any C library routines.  Intended as an educational illustration of
how sin() and cos() work under the hood.

Since version 0.3.0 the module follows the TRIG-REF-1 reference algorithm of
PHY00a (Cody-Waite range reduction, ten-term Horner kernels, bit-pinned
constants), so every result is bit-for-bit the same as in the other language
ports.  C<sin_approx>, C<cos_approx> and C<tan_approx> are defined for
C<|x| E<lt>= 2**29> radians and return NaN for NaN, an infinity, or a larger
magnitude.

=head1 FUNCTIONS

=over 4

=item B<sin_approx($x)>

Sine of C<$x> in radians (within 3 ulp).  C<sin_approx(-0.0)> is C<-0.0>.

=item B<cos_approx($x)>

Cosine of C<$x> in radians (within 3 ulp).

=item B<tan_approx($x)>

Tangent of C<$x> in radians.  At a pole (C<|cos| E<lt> 1e-15>) it returns
C<+-1e308> signed like the sine, a legacy sentinel and not an approximation.
No accuracy claim is made for C<tan>.

=item B<sqrt_approx($x)>

Square root of C<$x> via Newton's method.  Dies if C<$x < 0>.

=item B<atan_approx($x)>

Arctangent of C<$x> in radians.  Returns a value in C<(-π/2, π/2)>.

=item B<atan2_approx($y, $x)>

Four-quadrant arctangent of C<($y, $x)>.  Returns a value in C<(-π, π]>,
and NaN if either argument is NaN.

=item B<sin_deg($deg)>

Sine of C<$deg> degrees.

=item B<cos_deg($deg)>

Cosine of C<$deg> degrees.

=item B<tan_deg($deg)>

Tangent of C<$deg> degrees.

=item B<degrees_to_radians($deg)>

Convert degrees to radians: one multiply, C<$deg * (pi / 180)> (within 1 ulp).

=item B<radians_to_degrees($rad)>

Convert radians to degrees: one multiply, C<$rad * (180 / pi)> (within 1 ulp).

=back

=head1 CONSTANTS

=over 4

=item C<$CodingAdventures::Trig::PI>

3.141592653589793

=item C<$CodingAdventures::Trig::TWO_PI>

6.283185307179586

=back

=head1 VERSION

0.3.0

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
