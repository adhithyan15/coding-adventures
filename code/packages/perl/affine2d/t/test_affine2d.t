use strict;
use warnings;
use lib '../lib';
use lib '../../trig/lib';
use lib '../../point2d/lib';
use Test2::V0;
use CodingAdventures::Affine2D qw(identity translate rotate scale scale_uniform skew_x rotate_around);
use CodingAdventures::Point2D qw(new_point);

my $DELTA = 1e-9;
sub approx { abs($_[0] - $_[1]) < $DELTA }

my $pt = identity()->apply_to_point(new_point(3, 4));
ok approx($pt->x, 3), 'identity x';
ok approx($pt->y, 4), 'identity y';

$pt = translate(2, 3)->apply_to_point(new_point(1, 1));
ok approx($pt->x, 3), 'translate x';
ok approx($pt->y, 4), 'translate y';

use CodingAdventures::Trig qw();
my $PI = $CodingAdventures::Trig::PI;
$pt = rotate($PI / 2)->apply_to_point(new_point(1, 0));
ok approx($pt->x, 0), 'rotate90 x';
ok approx($pt->y, 1), 'rotate90 y';

$pt = scale(2, 3)->apply_to_point(new_point(1, 1));
ok approx($pt->x, 2), 'scale x';
ok approx($pt->y, 3), 'scale y';

$pt = scale_uniform(5)->apply_to_point(new_point(2, 3));
ok approx($pt->x, 10), 'scale_uniform x';
ok approx($pt->y, 15), 'scale_uniform y';

my $composed = translate(1, 0)->compose(translate(0, 2));
$pt = $composed->apply_to_point(new_point(0, 0));
ok approx($pt->x, 1), 'compose x';
ok approx($pt->y, 2), 'compose y';

ok approx(identity()->determinant, 1), 'det identity';
ok approx(scale(2, 3)->determinant, 6), 'det scale';

my $inv = identity()->invert;
ok defined $inv, 'invert identity defined';
ok $inv->is_identity, 'invert identity is identity';

my $a = translate(3, -5);
$inv = $a->invert;
ok defined $inv, 'invert translate defined';
$pt = $inv->apply_to_point($a->apply_to_point(new_point(1, 2)));
ok approx($pt->x, 1), 'invert translate roundtrip x';
ok approx($pt->y, 2), 'invert translate roundtrip y';

ok !defined scale(0, 1)->invert, 'singular invert is undef';
ok identity()->is_identity, 'is_identity true';
ok !translate(1, 0)->is_identity, 'is_identity false';
ok translate(5, -3)->is_translation_only, 'translation_only true';
ok !rotate(0.1)->is_translation_only, 'translation_only false';

my $arr = identity()->to_array;
ok $arr->[0] == 1 && $arr->[3] == 1, 'to_array identity';

my $v = translate(100, 100)->apply_to_vector(new_point(1, 0));
ok approx($v->x, 1), 'vector excludes translation x';
ok approx($v->y, 0), 'vector excludes translation y';

$pt = rotate_around($PI / 2, 1, 0)->apply_to_point(new_point(1, 0));
ok approx($pt->x, 1), 'rotate_around pivot x';
ok approx($pt->y, 0), 'rotate_around pivot y';

done_testing;
