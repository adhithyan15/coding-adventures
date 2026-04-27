use v5.40;

# ---------------------------------------------------------------------------
# CodingAdventures::Trig — Trigonometric functions from first principles
# ---------------------------------------------------------------------------

=head1 NAME

CodingAdventures::Trig - Trigonometric functions from first principles

=head1 DESCRIPTION

This module implements sine and cosine using B<Taylor series> (specifically,
Maclaurin series -- Taylor series centered at zero).  No math library is used;
everything is built from addition, multiplication, and division alone.

=head2 Why Taylor series?

Any "smooth" function can be approximated near a point by a polynomial.  The
idea, due to Brook Taylor (1715), is:

    f(x) = f(0) + f'(0)*x + f''(0)*x^2/2! + f'''(0)*x^3/3! + ...

When centered at zero this is called a B<Maclaurin series>.  For sine and
cosine the derivatives cycle through a simple pattern, giving us concrete
formulas we can compute with just arithmetic.

=head2 How accurate is this?

With 20 terms and range reduction to [-pi, pi], we achieve accuracy matching
IEEE 754 double-precision (~15 decimal digits) for all inputs, including very
large ones like sin(1000*pi).

=cut

package CodingAdventures::Trig;
use Exporter 'import';

our @EXPORT_OK = qw(PI sin_taylor cos_taylor radians degrees);


# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

# Pi to full double-precision accuracy.  This is the ratio of a circle's
# circumference to its diameter -- the most important constant in
# trigonometry.  We hard-code it here rather than computing it so that every
# other function in the module can use it without circular imports.

use constant PI => 3.141592653589793;

# Two-pi comes up constantly in range reduction (see below).  A full
# rotation around the unit circle is 2*pi radians, so adding or subtracting
# 2*pi from an angle doesn't change its sine or cosine.

use constant TWO_PI => 2 * PI;


# ---------------------------------------------------------------------------
# Range Reduction
# ---------------------------------------------------------------------------

=head2 Range reduction

The Taylor series converges for I<any> real number, but converges B<faster>
when x is small.  If someone passes in x = 1000*pi, the raw series would
need hundreds of terms to cancel out the enormous intermediate values.  By
first reducing x into [-pi, pi], 20 terms are more than enough.

Since sin and cos are B<periodic> with period 2*pi:

    sin(x) = sin(x - 2*pi*k)    for any integer k

we use Perl's modulo operator to bring x into [0, 2*pi), then shift by -pi
to land in [-pi, pi).

=cut

sub _range_reduce($x) {
    # Step 1: bring x into [0, 2*pi) using the modulo operator.
    # Perl's fmod (%) can return negative values for negative inputs,
    # so we adjust to ensure non-negative results.
    $x = $x - TWO_PI * int($x / TWO_PI);

    # Ensure x is in [0, 2*pi) — handle negative remainders.
    $x += TWO_PI if $x < 0;

    # Step 2: shift from [0, 2*pi) to [-pi, pi).
    # Values greater than pi get wrapped to the negative side.
    $x -= TWO_PI if $x > PI;

    return $x;
}


# ---------------------------------------------------------------------------
# Sine — The Maclaurin Series
# ---------------------------------------------------------------------------

=head2 sin_taylor($x)

Compute the sine of C<$x> (in radians) using the Maclaurin series.

The Maclaurin series for sine:

    sin(x) = x - x^3/3! + x^5/5! - x^7/7! + ...

Written with sigma notation:

             inf
    sin(x) = SUM  (-1)^n * x^(2n+1) / (2n+1)!
             n=0

B<Computing iteratively (the key trick):>

Rather than computing each term from scratch (which would require computing
large factorials and large powers), we compute each term B<from the previous
one>.  The ratio of consecutive terms simplifies to:

    term_{n+1} = term_n * (-x^2) / ((2n+2) * (2n+3))

This is beautiful: each new term is just the old term multiplied by a small
fraction.  No factorials, no large powers -- just one multiply and one
divide per iteration.

=cut

sub sin_taylor($x) {
    # --- Range reduction first ---
    # Bring x into [-pi, pi] so the series converges rapidly.
    $x = _range_reduce($x);

    # --- Series computation ---
    # The first term (n=0) of the Maclaurin series for sin is simply x.
    my $term  = $x;    # current term: (-1)^n * x^(2n+1) / (2n+1)!
    my $total = $x;    # running sum of all terms so far

    # We'll compute 20 terms total (n=0 through n=19).  In practice the
    # series converges well before 20 terms for inputs in [-pi, pi], but
    # extra terms cost almost nothing and guarantee full precision.
    for my $n (1 .. 19) {
        # Compute the multiplier to go from term_{n-1} to term_n:
        #
        #   term_n = term_{n-1} * (-x^2) / ((2n) * (2n+1))
        my $denominator = (2 * $n) * (2 * $n + 1);
        $term = $term * (-$x * $x) / $denominator;
        $total += $term;
    }

    return $total;
}


# ---------------------------------------------------------------------------
# Cosine — The Maclaurin Series
# ---------------------------------------------------------------------------

=head2 cos_taylor($x)

Compute the cosine of C<$x> (in radians) using the Maclaurin series.

The Maclaurin series for cosine:

    cos(x) = 1 - x^2/2! + x^4/4! - x^6/6! + ...

The iterative recurrence:

    term_{n+1} = term_n * (-x^2) / ((2n+1) * (2n+2))

Almost identical to sine -- only the denominator indices shift by one.
This makes sense: cosine uses even powers (0, 2, 4, ...) while sine uses
odd powers (1, 3, 5, ...).

=cut

sub cos_taylor($x) {
    # --- Range reduction first ---
    $x = _range_reduce($x);

    # --- Series computation ---
    # The first term (n=0) of the Maclaurin series for cos is 1.
    my $term  = 1.0;   # current term: (-1)^n * x^(2n) / (2n)!
    my $total = 1.0;   # running sum

    for my $n (1 .. 19) {
        # Going from term at index (n-1) to term at index n:
        #
        #   term_n = term_{n-1} * (-x^2) / ((2n-1) * (2n))
        my $denominator = (2 * $n - 1) * (2 * $n);
        $term = $term * (-$x * $x) / $denominator;
        $total += $term;
    }

    return $total;
}


# ---------------------------------------------------------------------------
# Degree / Radian Conversion
# ---------------------------------------------------------------------------

=head2 radians($deg)

Convert an angle from degrees to radians.

Degrees are a human convenience (360 per full turn, inherited from
Babylonian base-60 arithmetic).  Radians are the I<natural> unit for
angles in mathematics: one radian is the angle subtended by an arc whose
length equals the radius.  A full circle is 2*pi radians.

The conversion is straightforward:

    radians = degrees * (pi / 180)

=cut

sub radians($deg) {
    return $deg * PI / 180;
}


=head2 degrees($rad)

Convert an angle from radians to degrees.

This is simply the inverse of the C<radians()> function:

    degrees = radians * (180 / pi)

=cut

sub degrees($rad) {
    return $rad * 180 / PI;
}


1;  # Perl modules must return a true value

__END__

=head1 SYNOPSIS

    use CodingAdventures::Trig qw(sin_taylor cos_taylor radians degrees PI);

    say sin_taylor(PI / 2);      # 1.0
    say cos_taylor(0);            # 1.0
    say sin_taylor(radians(30));  # 0.5
    say degrees(PI);              # 180.0

=head1 SEE ALSO

L<CodingAdventures::Wave> -- wave physics package that uses this module

=cut
