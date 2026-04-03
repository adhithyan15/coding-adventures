package CodingAdventures::Trig;

# ============================================================================
# CodingAdventures::Trig — Trigonometric functions from first principles
# ============================================================================
#
# This module provides sine, cosine, tangent, and angle-conversion functions
# built entirely from scratch using Maclaurin (Taylor) series. We do NOT use
# Perl's built-in POSIX sin/cos — the goal is to understand *how* these
# functions work at the mathematical level.
#
# ## Why Build Trig from Scratch?
#
# Every language ships sin() and cos() in its standard library. Reimplementing
# them reveals:
#   1. How infinite series approximate transcendental functions.
#   2. Why "range reduction" is critical for numerical accuracy.
#   3. What "iterative term computation" means and why it beats naïve power/
#      factorial evaluation.
#
# The techniques here are the same ideas used inside real math libraries
# (just without Chebyshev polynomial approximations and table lookups).
#
# ## The Core Idea: Maclaurin Series
#
# A Maclaurin series expresses a function as an infinite sum evaluated at 0:
#
#     sin(x) = x - x^3/3! + x^5/5! - x^7/7! + ...
#     cos(x) = 1 - x^2/2! + x^4/4! - x^6/6! + ...
#
# Each successive term is smaller than the last (for |x| <= pi), so after
# enough terms the sum converges to the true value.  We use 20 terms, which
# gives double-precision accuracy.
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

our $VERSION = '0.01';

use Exporter 'import';
our @EXPORT_OK = qw(
    sin_approx  cos_approx  tan_approx
    sqrt_approx
    atan_approx atan2_approx
    sin_deg     cos_deg     tan_deg
    degrees_to_radians  radians_to_degrees
);

# ============================================================================
# Constants
# ============================================================================

# PI — the ratio of a circle's circumference to its diameter.
#
# This is one of the most fundamental constants in mathematics.  It appears in
# geometry, trigonometry, calculus, probability (the Gaussian bell curve), and
# even in quantum mechanics.
#
# We define it to the full precision of a Perl NV (IEEE 754 double-precision
# floating-point number), which gives about 15-16 significant decimal digits.
#
# Why not use POSIX::M_PI?  Because this module intentionally avoids depending
# on C library constants — we want to be explicit about every value we use.
our $PI      = 3.141592653589793;
our $TWO_PI  = 2 * $PI;   # 6.283185307179586 — the period of sin and cos

# ============================================================================
# Internal helper: _range_reduce
# ============================================================================
#
# Normalise an angle x (in radians) to the range [-pi, pi].
#
# WHY IS THIS NECESSARY?
#
# The Maclaurin series for sin and cos converges fastest when x is close to
# zero.  For large |x| the series terms start enormous before cancelling,
# causing catastrophic cancellation and precision loss in floating-point.
#
# Since sin and cos repeat every 2*pi (they are "periodic"), we can always
# subtract an integer multiple of 2*pi without changing the function value:
#
#     sin(x) = sin(x - 2*pi*k)   for any integer k
#     cos(x) = cos(x - 2*pi*k)   for any integer k
#
# HOW IT WORKS
#
#  1. Divide x by 2*pi to find how many full cycles it contains.
#  2. Truncate that quotient toward zero (int() in Perl truncates toward zero,
#     unlike POSIX floor which always goes down).
#  3. Subtract those full cycles.
#  4. If the result is still outside [-pi, pi], do one final correction.
#
# The int() truncation is important for symmetry around zero:
#     int( 2.7) =  2   (not 3)
#     int(-2.7) = -2   (not -3, like floor would give)

sub _range_reduce {
    my ($x) = @_;

    # Step 1: Remove full rotations using truncation toward zero.
    my $full_cycles = int( $x / $TWO_PI );
    $x = $x - $TWO_PI * $full_cycles;

    # Step 2: Ensure we are in [-pi, pi].
    # After step 1, x lies in (-2*pi, 2*pi).  One more adjustment suffices.
    if    ( $x >  $PI ) { $x -= $TWO_PI; }
    elsif ( $x < -$PI ) { $x += $TWO_PI; }

    return $x;
}

# ============================================================================
# sin_approx — Sine via Maclaurin Series
# ============================================================================
#
# Compute the sine of x (in radians) without using Perl's built-in sin().
#
# THE MACLAURIN SERIES FOR SINE
#
#     sin(x) = x - x^3/3! + x^5/5! - x^7/7! + x^9/9! - ...
#
# Written compactly with summation notation:
#
#     sin(x) = sum_{n=0}^{inf}  (-1)^n  *  x^(2n+1) / (2n+1)!
#
# Each term uses odd powers of x (1, 3, 5, 7, ...) and alternates in sign.
#
# ITERATIVE TERM COMPUTATION (the clever part)
#
# A naïve implementation would compute x^n and n! separately for each term.
# This is wasteful (large intermediate values) and prone to overflow.
# Instead, compute each term from the previous one:
#
#     term_0 = x
#     term_n = term_{n-1} * (-x^2) / ( (2n) * (2n+1) )
#
# WHY THIS FORMULA?
#
# The ratio of consecutive Maclaurin terms is:
#
#     term_n / term_{n-1}
#       = [ (-1)^n * x^(2n+1) / (2n+1)! ]
#         / [ (-1)^(n-1) * x^(2n-1) / (2n-1)! ]
#       = (-1) * x^2 * (2n-1)! / (2n+1)!
#       = (-1) * x^2 / ( (2n)(2n+1) )
#
# The denominator (2n)(2n+1) grows quadratically, so each term shrinks
# rapidly.  The minus sign creates the alternating pattern.  No factorials
# or powers are computed from scratch.
#
# EXAMPLE: sin(pi/6) = 0.5
#
#     x = 0.5236 (pi/6)
#     term 0: +0.5236
#     term 1: -0.0239  (multiply by -x^2 / (2*3))
#     term 2: +0.0003  (multiply by -x^2 / (4*5))
#     ... sum converges to 0.5000
#
# @param  $x   Angle in radians (or class name if called as method).
# @return      Sine of x (approximate, error < 1e-15 for |x| <= pi).

sub sin_approx {
    # Support both function call (sin_approx($x)) and method call
    # (CodingAdventures::Trig->sin_approx($x)).
    my $x = ( @_ == 2 ) ? $_[1] : $_[0];

    # Step 1: Range reduction — bring x into [-pi, pi].
    $x = _range_reduce($x);

    # Step 2: Initialise the series.
    # The n=0 term of the Maclaurin series for sin is just x.
    my $term = $x;    # current term being added
    my $sum  = $term; # running total

    # Step 3: Accumulate 20 terms.
    # 20 terms is overkill for |x| <= pi (converges in ~12 terms), but
    # costs almost nothing and ensures maximum accuracy.
    for my $n ( 1 .. 20 ) {
        # term_n = term_{n-1} * (-x^2) / ( (2n)(2n+1) )
        my $denom = ( 2 * $n ) * ( 2 * $n + 1 );
        $term = $term * ( -$x * $x ) / $denom;
        $sum  = $sum + $term;
    }

    return $sum;
}

# ============================================================================
# cos_approx — Cosine via Maclaurin Series
# ============================================================================
#
# Compute the cosine of x (in radians) without using Perl's built-in cos().
#
# THE MACLAURIN SERIES FOR COSINE
#
#     cos(x) = 1 - x^2/2! + x^4/4! - x^6/6! + x^8/8! - ...
#
# Written compactly:
#
#     cos(x) = sum_{n=0}^{inf}  (-1)^n  *  x^(2n) / (2n)!
#
# COMPARISON WITH SINE
#
#   sin: odd powers   (1, 3, 5, 7, ...)   starts with x
#   cos: even powers  (0, 2, 4, 6, ...)   starts with 1
#
# The iterative term ratio for cosine is:
#
#     term_n = term_{n-1} * (-x^2) / ( (2n-1)(2n) )
#
# WHY DIFFERENT DENOMINATOR?
#
#   sin denominator: (2n)(2n+1)
#   cos denominator: (2n-1)(2n)
#
# Because the cosine factorial steps are (2n)!/(2(n-1))! = (2n-1)(2n),
# while the sine factorial steps are (2n+1)!/(2n-1)! = (2n)(2n+1).
#
# THE PYTHAGOREAN IDENTITY
#
# For all x:  sin^2(x) + cos^2(x) = 1
#
# This identity is a consequence of the unit circle definition. Our tests
# verify it across many angles as a sanity check on both implementations.
#
# @param  $x   Angle in radians.
# @return      Cosine of x.

sub cos_approx {
    my $x = ( @_ == 2 ) ? $_[1] : $_[0];

    # Step 1: Range reduction.
    $x = _range_reduce($x);

    # Step 2: Initialise the series.
    # The n=0 term of the Maclaurin series for cos is 1.
    my $term = 1.0;
    my $sum  = $term;

    # Step 3: Accumulate 20 terms.
    for my $n ( 1 .. 20 ) {
        # term_n = term_{n-1} * (-x^2) / ( (2n-1)(2n) )
        my $denom = ( 2 * $n - 1 ) * ( 2 * $n );
        $term = $term * ( -$x * $x ) / $denom;
        $sum  = $sum + $term;
    }

    return $sum;
}

# ============================================================================
# tan_approx — Tangent
# ============================================================================
#
# Compute the tangent of x (in radians).
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
# tan(x) is undefined where cos(x) = 0, i.e., at x = pi/2 + k*pi.
# IEEE 754 arithmetic will produce ±Inf when dividing non-zero by zero,
# and NaN when dividing zero by zero.  We do not raise a Perl error —
# the caller receives the IEEE value, matching the behaviour of POSIX tan().
#
# @param  $x   Angle in radians.
# @return      Tangent of x.

sub tan_approx {
    my $x = ( @_ == 2 ) ? $_[1] : $_[0];
    return sin_approx($x) / cos_approx($x);
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
    return 0.0 if $x == 0.0;

    # Initial guess: x itself for x >= 1, else 1.0.
    # For large x, starting at x converges faster than starting at 1.
    # For x in (0, 1), starting at 1.0 avoids dividing by a tiny number.
    my $guess = ( $x >= 1.0 ) ? $x : 1.0;

    # Iterate to convergence (up to 60 steps as a safety cap).
    for my $i ( 1 .. 60 ) {
        my $next = ( $guess + $x / $guess ) / 2.0;

        # Stop when improvement is negligibly small.
        # 1e-15 * $guess handles relative precision for large values.
        # 1e-300 is an absolute floor for subnormal inputs.
        my $improvement = abs( $next - $guess );
        return $next if $improvement < 1e-15 * $guess + 1e-300;

        $guess = $next;
    }

    return $guess;
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
        last if abs($term) < 1e-17;
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

our $HALF_PI = $PI / 2.0;

sub atan_approx {
    my $x = ( @_ == 2 ) ? $_[1] : $_[0];

    return 0.0 if $x == 0.0;

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

# degrees_to_radians — Convert degrees to radians.
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
# MEMORABLE EXAMPLES
#
#     0°    -> 0
#     90°   -> pi/2   (right angle)
#     180°  -> pi     (straight line)
#     360°  -> 2*pi   (full circle)
#
# @param  $deg   Angle in degrees.
# @return        Angle in radians.

sub degrees_to_radians {
    my $deg = ( @_ == 2 ) ? $_[1] : $_[0];
    return $deg * $PI / 180.0;
}

# radians_to_degrees — Convert radians to degrees.
#
# The inverse of degrees_to_radians:
#
#     degrees = radians * (180 / pi)
#
# @param  $rad   Angle in radians.
# @return        Angle in degrees.

sub radians_to_degrees {
    my $rad = ( @_ == 2 ) ? $_[1] : $_[0];
    return $rad * 180.0 / $PI;
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

=head1 NAME

CodingAdventures::Trig - Trigonometric functions from first principles (Maclaurin series)

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

=head1 FUNCTIONS

=over 4

=item B<sin_approx($x)>

Sine of C<$x> in radians, computed via a 20-term Maclaurin series.

=item B<cos_approx($x)>

Cosine of C<$x> in radians.

=item B<tan_approx($x)>

Tangent of C<$x> in radians.  Returns ±Inf at undefined points.

=item B<sqrt_approx($x)>

Square root of C<$x> via Newton's method.  Dies if C<$x < 0>.

=item B<atan_approx($x)>

Arctangent of C<$x> in radians.  Returns a value in C<(-π/2, π/2)>.

=item B<atan2_approx($y, $x)>

Four-quadrant arctangent of C<($y, $x)>.  Returns a value in C<(-π, π]>.

=item B<sin_deg($deg)>

Sine of C<$deg> degrees.

=item B<cos_deg($deg)>

Cosine of C<$deg> degrees.

=item B<tan_deg($deg)>

Tangent of C<$deg> degrees.

=item B<degrees_to_radians($deg)>

Convert degrees to radians: C<$deg * pi / 180>.

=item B<radians_to_degrees($rad)>

Convert radians to degrees: C<$rad * 180 / pi>.

=back

=head1 CONSTANTS

=over 4

=item C<$CodingAdventures::Trig::PI>

3.141592653589793

=item C<$CodingAdventures::Trig::TWO_PI>

6.283185307179586

=back

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
