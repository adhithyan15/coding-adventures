use strict;
use warnings;
use lib '../lib';
use lib '../../trig/lib';
use Test2::V0;
use CodingAdventures::Point2D qw(new_point new_rect);

my $DELTA = 1e-9;

sub approx { abs($_[0] - $_[1]) < $DELTA }

# Point tests
my $p = new_point(1, 2)->add(new_point(3, 4));
ok approx($p->x, 4), 'add x';
ok approx($p->y, 6), 'add y';

$p = new_point(5, 3)->subtract(new_point(2, 1));
ok approx($p->x, 3), 'subtract x';
ok approx($p->y, 2), 'subtract y';

$p = new_point(2, 3)->scale(2);
ok approx($p->x, 4), 'scale x';
ok approx($p->y, 6), 'scale y';

$p = new_point(1, -2)->negate;
ok approx($p->x, -1), 'negate x';
ok approx($p->y,  2), 'negate y';

ok approx(new_point(1, 2)->dot(new_point(3, 4)), 11), 'dot';
ok approx(new_point(1, 2)->cross(new_point(3, 4)), -2), 'cross';
ok approx(new_point(3, 4)->magnitude, 5), 'magnitude';
ok approx(new_point(3, 4)->magnitude_squared, 25), 'magnitude_squared';

my $n = new_point(3, 4)->normalize;
ok approx($n->magnitude, 1), 'normalize';

my $zero = new_point(0, 0)->normalize;
ok approx($zero->x, 0), 'normalize zero x';

ok approx(new_point(0, 0)->distance(new_point(3, 4)), 5), 'distance';

$p = new_point(0, 0)->lerp(new_point(10, 0), 0.5);
ok approx($p->x, 5), 'lerp mid';

$p = new_point(1, 0)->perpendicular;
ok approx($p->x, 0), 'perp x';
ok approx($p->y, 1), 'perp y';

use CodingAdventures::Trig qw(sin_approx cos_approx);
my $pi_over_4 = 3.141592653589793 / 4;
ok approx(new_point(1, 1)->angle, $pi_over_4), 'angle';

# Rect tests
my $r = new_rect(0, 0, 10, 10);
ok $r->contains_point(new_point(5, 5)), 'contains inside';
ok !$r->contains_point(new_point(10, 5)), 'contains outside';

my $r2 = new_rect(5, 5, 10, 10);
my $u = $r->union($r2);
ok approx($u->x, 0), 'union x';
ok approx($u->width, 15), 'union width';

my $i = $r->intersection($r2);
ok defined $i, 'intersection exists';
ok approx($i->x, 5), 'intersection x';
ok approx($i->width, 5), 'intersection width';

ok !defined $r->intersection(new_rect(20, 20, 5, 5)), 'disjoint';

my $e = $r->expand_by(2);
ok approx($e->x, -2), 'expand x';
ok approx($e->width, 14), 'expand width';

done_testing;
