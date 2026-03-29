use strict;
use warnings;
use Test2::V0;

# ---------------------------------------------------------------------------
# Load the module under test.
# ---------------------------------------------------------------------------
ok( eval { require CodingAdventures::Trig; 1 }, 'CodingAdventures::Trig loads' );

use CodingAdventures::Trig qw(
    sin_approx  cos_approx  tan_approx
    sin_deg     cos_deg     tan_deg
    degrees_to_radians  radians_to_degrees
);

# Convenience: floating-point comparison with a given tolerance.
# We use 1e-6 for all trig tests (well within double-precision accuracy).
sub near {
    my ( $got, $expected, $tol ) = @_;
    $tol //= 1e-6;
    return abs( $got - $expected ) < $tol;
}

my $PI = $CodingAdventures::Trig::PI;

# ===========================================================================
# 1. Constants
# ===========================================================================

ok( near( $PI, 3.14159265358979 ), 'PI is approximately 3.14159...' );
ok( near( $CodingAdventures::Trig::TWO_PI, 2 * $PI ), 'TWO_PI = 2 * PI' );

# ===========================================================================
# 2. degrees_to_radians
# ===========================================================================

ok( near( degrees_to_radians(0),   0          ), 'degrees_to_radians(0) = 0' );
ok( near( degrees_to_radians(90),  $PI / 2    ), 'degrees_to_radians(90) = pi/2' );
ok( near( degrees_to_radians(180), $PI        ), 'degrees_to_radians(180) = pi' );
ok( near( degrees_to_radians(360), 2 * $PI    ), 'degrees_to_radians(360) = 2*pi' );
ok( near( degrees_to_radians(45),  $PI / 4    ), 'degrees_to_radians(45) = pi/4' );
ok( near( degrees_to_radians(-90), -$PI / 2   ), 'degrees_to_radians(-90) = -pi/2' );

# ===========================================================================
# 3. radians_to_degrees
# ===========================================================================

ok( near( radians_to_degrees(0),          0    ), 'radians_to_degrees(0) = 0' );
ok( near( radians_to_degrees($PI / 2),   90    ), 'radians_to_degrees(pi/2) = 90' );
ok( near( radians_to_degrees($PI),       180   ), 'radians_to_degrees(pi) = 180' );
ok( near( radians_to_degrees(2 * $PI),   360   ), 'radians_to_degrees(2*pi) = 360' );
ok( near( radians_to_degrees(-$PI / 2), -90    ), 'radians_to_degrees(-pi/2) = -90' );

# Round-trip: convert degrees to radians and back.
ok( near( radians_to_degrees( degrees_to_radians(45) ),  45  ), 'round-trip 45 deg' );
ok( near( radians_to_degrees( degrees_to_radians(123) ), 123 ), 'round-trip 123 deg' );

# ===========================================================================
# 4. sin_approx — well-known values
# ===========================================================================

ok( near( sin_approx(0),          0.0   ), 'sin(0) = 0' );
ok( near( sin_approx($PI / 6),    0.5   ), 'sin(pi/6) = 0.5' );
ok( near( sin_approx($PI / 2),    1.0   ), 'sin(pi/2) = 1' );
ok( near( sin_approx($PI),        0.0   ), 'sin(pi) ~ 0' );
ok( near( sin_approx(3 * $PI / 2), -1.0 ), 'sin(3*pi/2) = -1' );
ok( near( sin_approx(2 * $PI),    0.0   ), 'sin(2*pi) ~ 0' );
ok( near( sin_approx(-$PI / 2),   -1.0  ), 'sin(-pi/2) = -1' );
ok( near( sin_approx($PI / 4), sqrt(2) / 2 ), 'sin(pi/4) = sqrt(2)/2' );

# ===========================================================================
# 5. cos_approx — well-known values
# ===========================================================================

ok( near( cos_approx(0),         1.0   ), 'cos(0) = 1' );
ok( near( cos_approx($PI / 3),   0.5   ), 'cos(pi/3) = 0.5' );
ok( near( cos_approx($PI / 2),   0.0   ), 'cos(pi/2) ~ 0' );
ok( near( cos_approx($PI),      -1.0   ), 'cos(pi) = -1' );
ok( near( cos_approx(2 * $PI),   1.0   ), 'cos(2*pi) = 1' );
ok( near( cos_approx(-$PI),     -1.0   ), 'cos(-pi) = -1' );
ok( near( cos_approx($PI / 4), sqrt(2) / 2 ), 'cos(pi/4) = sqrt(2)/2' );

# ===========================================================================
# 6. Pythagorean Identity: sin^2(x) + cos^2(x) = 1
# ===========================================================================
#
# This identity holds for ALL angles.  Verifying it exercises both sin_approx
# and cos_approx simultaneously.

for my $angle ( 0, $PI/6, $PI/4, $PI/3, $PI/2, $PI, 1.23, 2.71, -0.5, 5.0 ) {
    my $s = sin_approx($angle);
    my $c = cos_approx($angle);
    ok( near( $s*$s + $c*$c, 1.0 ), "sin^2 + cos^2 = 1 at x=$angle" );
}

# ===========================================================================
# 7. tan_approx
# ===========================================================================

ok( near( tan_approx(0),        0.0 ), 'tan(0) = 0' );
ok( near( tan_approx($PI / 4),  1.0 ), 'tan(pi/4) = 1' );
ok( near( tan_approx(-$PI / 4), -1.0), 'tan(-pi/4) = -1' );
ok( near( tan_approx($PI),       0.0 ), 'tan(pi) ~ 0' );

# tan = sin / cos
for my $angle ( 0.1, 0.5, 1.0, 2.0 ) {
    my $expected = sin_approx($angle) / cos_approx($angle);
    ok( near( tan_approx($angle), $expected ), "tan($angle) = sin/cos" );
}

# ===========================================================================
# 8. Degree-input convenience functions
# ===========================================================================

ok( near( sin_deg(0),    0.0 ),  'sin_deg(0) = 0' );
ok( near( sin_deg(30),   0.5 ),  'sin_deg(30) = 0.5' );
ok( near( sin_deg(90),   1.0 ),  'sin_deg(90) = 1' );
ok( near( sin_deg(180),  0.0 ),  'sin_deg(180) ~ 0' );
ok( near( sin_deg(-90), -1.0 ),  'sin_deg(-90) = -1' );

ok( near( cos_deg(0),    1.0 ),  'cos_deg(0) = 1' );
ok( near( cos_deg(60),   0.5 ),  'cos_deg(60) = 0.5' );
ok( near( cos_deg(90),   0.0 ),  'cos_deg(90) ~ 0' );
ok( near( cos_deg(180), -1.0 ),  'cos_deg(180) = -1' );

ok( near( tan_deg(0),   0.0 ), 'tan_deg(0) = 0' );
ok( near( tan_deg(45),  1.0 ), 'tan_deg(45) = 1' );
ok( near( tan_deg(-45),-1.0 ), 'tan_deg(-45) = -1' );

# ===========================================================================
# 9. Large-angle range reduction
# ===========================================================================
#
# sin and cos are periodic: sin(x) = sin(x + 2*pi*k).
# Our range-reduction step must handle large arguments correctly.

ok( near( sin_approx(2 * $PI + $PI / 6), 0.5 ),    'sin(2pi + pi/6) = 0.5  (one full rotation)' );
ok( near( sin_approx(100 * $PI), 0.0 ),             'sin(100*pi) ~ 0  (50 rotations)' );
ok( near( cos_approx(2 * $PI + $PI / 3), 0.5 ),    'cos(2pi + pi/3) = 0.5  (one full rotation)' );
ok( near( cos_approx(-2 * $PI), 1.0 ),              'cos(-2*pi) = 1' );
ok( near( sin_approx(-2 * $PI + $PI / 2), 1.0 ),   'sin(-2pi + pi/2) = 1' );

# ===========================================================================
# 10. Class method interface
# ===========================================================================
#
# All functions can also be called as class methods:
#   CodingAdventures::Trig->sin_approx($x)

ok( near( CodingAdventures::Trig->sin_approx($PI / 6), 0.5 ),
    'class method: sin_approx(pi/6) = 0.5' );
ok( near( CodingAdventures::Trig->cos_approx(0), 1.0 ),
    'class method: cos_approx(0) = 1' );
ok( near( CodingAdventures::Trig->degrees_to_radians(180), $PI ),
    'class method: degrees_to_radians(180) = pi' );
ok( near( CodingAdventures::Trig->sin_deg(30), 0.5 ),
    'class method: sin_deg(30) = 0.5' );

done_testing;
