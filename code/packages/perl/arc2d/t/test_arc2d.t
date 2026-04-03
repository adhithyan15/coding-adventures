use strict;
use warnings;
use lib '../lib';
use lib '../../trig/lib';
use lib '../../point2d/lib';
use lib '../../bezier2d/lib';
use Test2::V0;
use CodingAdventures::Arc2D qw(new_center_arc new_svg_arc);
use CodingAdventures::Point2D qw(new_point);

my $DELTA = 1e-6;
my $PI    = $CodingAdventures::Trig::PI;
sub approx { abs($_[0] - $_[1]) < $DELTA }
sub pt_approx { approx($_[0]->{x}, $_[1]->{x}) && approx($_[0]->{y}, $_[1]->{y}) }

my $unit_arc = new_center_arc(new_point(0, 0), 1, 1, 0, $PI/2, 0);

ok pt_approx($unit_arc->eval_at(0), new_point(1, 0)), 'eval at 0';
ok pt_approx($unit_arc->eval_at(1), new_point(0, 1)), 'eval at 1';

my $expected = 1.0 / sqrt(2);
my $mid = $unit_arc->eval_at(0.5);
ok approx($mid->{x}, $expected), 'eval midpoint x';
ok approx($mid->{y}, $expected), 'eval midpoint y';

my $full = new_center_arc(new_point(0, 0), 1, 1, 0, 2*$PI, 0);
my $bb = $full->bbox;
ok $bb->{x} <= -0.99, 'full circle bbox x min';
ok $bb->{x} + $bb->{width} >= 0.99, 'full circle bbox x max';

my $curves = $unit_arc->to_cubic_beziers;
ok scalar(@$curves) > 0, 'has bezier curves';
ok pt_approx($curves->[0]->{p0}, new_point(1, 0)), 'first bezier p0';
ok pt_approx($curves->[-1]->{p3}, new_point(0, 1)), 'last bezier p3';

# SvgArc degenerate
my $deg = new_svg_arc(new_point(1, 1), new_point(1, 1), 1, 1, 0, 0, 0);
ok !defined $deg->to_center_arc, 'degenerate same point';

my $zero_r = new_svg_arc(new_point(0, 0), new_point(1, 0), 0, 1, 0, 0, 0);
ok !defined $zero_r->to_center_arc, 'degenerate zero radius';

# Semicircle
my $svg = new_svg_arc(new_point(1, 0), new_point(-1, 0), 1, 1, 0, 0, 1);
my $ca = $svg->to_center_arc;
ok defined $ca, 'svg to center arc defined';
ok approx($ca->center->{x}, 0), 'semicircle center x';
ok approx($ca->center->{y}, 0), 'semicircle center y';
ok pt_approx($ca->eval_at(0), new_point(1, 0)), 'semicircle start';
ok pt_approx($ca->eval_at(1), new_point(-1, 0)), 'semicircle end';

done_testing;
