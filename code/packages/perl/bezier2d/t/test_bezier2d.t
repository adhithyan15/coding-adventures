use strict;
use warnings;
use lib '../lib';
use lib '../../trig/lib';
use lib '../../point2d/lib';
use Test2::V0;
use CodingAdventures::Bezier2D qw(new_quad new_cubic);
use CodingAdventures::Point2D qw(new_point);

my $DELTA = 1e-9;
sub approx { abs($_[0] - $_[1]) < $DELTA }
sub pt_approx { approx($_[0]->x, $_[1]->x) && approx($_[0]->y, $_[1]->y) }

my $q = new_quad(new_point(0,0), new_point(1,2), new_point(2,0));

ok pt_approx($q->eval(0), new_point(0,0)), 'quad eval at 0';
ok pt_approx($q->eval(1), new_point(2,0)), 'quad eval at 1';
ok pt_approx($q->eval(0.5), new_point(1,1)), 'quad eval midpoint';

my ($left, $right) = $q->split(0.5);
my $m = $q->eval(0.5);
ok approx($left->p2->x, $m->x), 'split left p2 x';
ok approx($right->p0->x, $m->x), 'split right p0 x';

my $straight = new_quad(new_point(0,0), new_point(1,0), new_point(2,0));
my $pts = $straight->polyline(0.1);
ok scalar(@$pts) == 2, 'straight quad polyline 2 points';

my $bb = $q->bbox;
ok $bb->x <= 0, 'bbox x min';
ok $bb->x + $bb->width >= 2, 'bbox x max';

my $c_elevated = $q->elevate;
for my $t (0, 0.25, 0.5, 0.75, 1) {
    my $qp = $q->eval($t);
    my $cp = $c_elevated->eval($t);
    ok approx($qp->x, $cp->x), "elevate x at t=$t";
    ok approx($qp->y, $cp->y), "elevate y at t=$t";
}

my $c = new_cubic(new_point(0,0), new_point(1,2), new_point(3,2), new_point(4,0));
ok pt_approx($c->eval(0), new_point(0,0)), 'cubic eval at 0';
ok pt_approx($c->eval(1), new_point(4,0)), 'cubic eval at 1';
ok approx($c->eval(0.5)->x, 2), 'cubic symmetric midpoint x';

my ($cl, $cr) = $c->split(0.5);
my $cm = $c->eval(0.5);
ok approx($cl->p3->x, $cm->x), 'cubic split left p3 x';
ok approx($cr->p0->x, $cm->x), 'cubic split right p0 x';

my $cs = new_cubic(new_point(0,0), new_point(1,0), new_point(2,0), new_point(3,0));
$pts = $cs->polyline(0.1);
ok scalar(@$pts) == 2, 'straight cubic polyline 2 points';

$bb = $c->bbox;
for my $i (0..20) {
    my $p = $c->eval($i/20.0);
    ok $p->x >= $bb->x - 1e-6, "cubic bbox x min i=$i";
}

done_testing;
