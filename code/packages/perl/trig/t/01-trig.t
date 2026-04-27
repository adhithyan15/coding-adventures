use v5.40;
use Test2::V0;

# ---------------------------------------------------------------------------
# Tests for CodingAdventures::Trig
#
# These tests verify our from-scratch Taylor series implementations of sin
# and cos against known mathematical identities and special values.  We use
# an absolute tolerance of 1e-10 throughout.
# ---------------------------------------------------------------------------

use CodingAdventures::Trig qw(sin_taylor cos_taylor radians degrees PI);

# Tolerance used for all approximate comparisons.
my $TOL = 1e-10;

# Helper: check that a value is approximately equal to an expected value.
sub approx_is($got, $expected, $msg) {
    ok(abs($got - $expected) < $TOL, $msg // "expected $expected, got $got");
}


# ---------------------------------------------------------------------------
# Special values — sin
# ---------------------------------------------------------------------------

subtest 'sin special values' => sub {
    is(sin_taylor(0), 0, 'sin(0) is exactly 0');

    approx_is(sin_taylor(PI / 2), 1.0, 'sin(pi/2) = 1');
    approx_is(sin_taylor(PI), 0.0, 'sin(pi) = 0');
    approx_is(sin_taylor(3 * PI / 2), -1.0, 'sin(3*pi/2) = -1');
    approx_is(sin_taylor(2 * PI), 0.0, 'sin(2*pi) = 0');

    # Common reference angles
    approx_is(sin_taylor(PI / 6), 0.5, 'sin(pi/6) = 0.5');
    approx_is(sin_taylor(PI / 4), 0.7071067811865476, 'sin(pi/4) = sqrt(2)/2');
    approx_is(sin_taylor(PI / 3), 0.8660254037844386, 'sin(pi/3) = sqrt(3)/2');
};


# ---------------------------------------------------------------------------
# Special values — cos
# ---------------------------------------------------------------------------

subtest 'cos special values' => sub {
    is(cos_taylor(0), 1.0, 'cos(0) is exactly 1');

    approx_is(cos_taylor(PI / 2), 0.0, 'cos(pi/2) = 0');
    approx_is(cos_taylor(PI), -1.0, 'cos(pi) = -1');
    approx_is(cos_taylor(3 * PI / 2), 0.0, 'cos(3*pi/2) = 0');
    approx_is(cos_taylor(2 * PI), 1.0, 'cos(2*pi) = 1');

    # Common reference angles
    approx_is(cos_taylor(PI / 6), 0.8660254037844386, 'cos(pi/6) = sqrt(3)/2');
    approx_is(cos_taylor(PI / 4), 0.7071067811865476, 'cos(pi/4) = sqrt(2)/2');
    approx_is(cos_taylor(PI / 3), 0.5, 'cos(pi/3) = 0.5');
};


# ---------------------------------------------------------------------------
# Symmetry properties
# ---------------------------------------------------------------------------

subtest 'sin is odd: sin(-x) = -sin(x)' => sub {
    for my $x (0.5, 1.0, 1.5, 2.0, 2.7, PI / 4, PI / 3) {
        approx_is(sin_taylor(-$x), -sin_taylor($x),
                  "sin(-$x) = -sin($x)");
    }
};

subtest 'cos is even: cos(-x) = cos(x)' => sub {
    for my $x (0.5, 1.0, 1.5, 2.0, 2.7, PI / 4, PI / 3) {
        approx_is(cos_taylor(-$x), cos_taylor($x),
                  "cos(-$x) = cos($x)");
    }
};


# ---------------------------------------------------------------------------
# Pythagorean identity: sin^2(x) + cos^2(x) = 1
# ---------------------------------------------------------------------------

subtest 'Pythagorean identity' => sub {
    my @test_angles = (
        0, PI / 6, PI / 4, PI / 3, PI / 2, PI,
        3 * PI / 2, 2 * PI, -1.0, -2.5, 0.1, 3.0, 5.5,
    );

    for my $x (@test_angles) {
        my $s = sin_taylor($x);
        my $c = cos_taylor($x);
        approx_is($s * $s + $c * $c, 1.0,
                  "sin^2($x) + cos^2($x) = 1");
    }
};


# ---------------------------------------------------------------------------
# Large inputs (tests range reduction)
# ---------------------------------------------------------------------------

subtest 'large inputs' => sub {
    approx_is(sin_taylor(1000 * PI), 0.0, 'sin(1000*pi) = 0');
    approx_is(cos_taylor(1000 * PI), 1.0, 'cos(1000*pi) = 1');

    # sin(100) should satisfy the Pythagorean identity
    my $s = sin_taylor(100);
    my $c = cos_taylor(100);
    approx_is($s * $s + $c * $c, 1.0, 'Pythagorean identity at x=100');

    # sin(-100) = -sin(100)
    approx_is(sin_taylor(-100), -sin_taylor(100), 'sin(-100) = -sin(100)');
};


# ---------------------------------------------------------------------------
# Degree / Radian conversions
# ---------------------------------------------------------------------------

subtest 'degree/radian conversion' => sub {
    approx_is(radians(180), PI, '180 degrees = pi radians');
    approx_is(radians(90), PI / 2, '90 degrees = pi/2 radians');
    approx_is(radians(360), 2 * PI, '360 degrees = 2*pi radians');
    is(radians(0), 0.0, '0 degrees = 0 radians');

    approx_is(degrees(PI), 180.0, 'pi radians = 180 degrees');
    approx_is(degrees(PI / 2), 90.0, 'pi/2 radians = 90 degrees');
    is(degrees(0), 0.0, '0 radians = 0 degrees');
};

subtest 'round-trip conversions' => sub {
    for my $deg (0, 30, 45, 60, 90, 120, 180, 270, 360) {
        approx_is(degrees(radians($deg)), $deg,
                  "degrees(radians($deg)) = $deg");
    }

    for my $rad (0, PI / 6, PI / 4, PI / 3, PI / 2, PI, 2 * PI) {
        approx_is(radians(degrees($rad)), $rad,
                  "radians(degrees($rad)) = $rad");
    }
};


# ---------------------------------------------------------------------------
# Integration: sin and cos with degree input
# ---------------------------------------------------------------------------

subtest 'sin/cos with degree input' => sub {
    approx_is(sin_taylor(radians(30)), 0.5, 'sin(30 degrees) = 0.5');
    approx_is(cos_taylor(radians(60)), 0.5, 'cos(60 degrees) = 0.5');
    approx_is(sin_taylor(radians(45)), 0.7071067811865476,
              'sin(45 degrees) = sqrt(2)/2');
};


done_testing;
