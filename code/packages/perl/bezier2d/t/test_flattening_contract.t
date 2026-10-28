use strict;
use warnings;
use FindBin;
use JSON::PP qw(decode_json);
use POSIX qw(HUGE_VAL isfinite nan);
use lib '../lib';
use lib '../../trig/lib';
use lib '../../point2d/lib';
use Test2::V0;
use CodingAdventures::Bezier2D qw(new_quad new_cubic);
use CodingAdventures::Point2D qw(new_point);

my $fixture_path = "$FindBin::Bin/../../../../specs/fixtures/bezier2d-flattening-v1/cases.json";
open my $fixture_fh, '<:encoding(UTF-8)', $fixture_path or die "cannot read $fixture_path: $!";
local $/;
my $fixture = decode_json(<$fixture_fh>);
close $fixture_fh;

is $fixture->{version}, 1, 'fixture version matches implementation';
is $fixture->{max_depth}, 32, 'fixture depth matches implementation';
is $fixture->{max_subdivisions}, 65_535, 'fixture split budget matches implementation';

sub curve_for {
    my ($case) = @_;
    my @points = map { new_point($_->[0], $_->[1]) } @{$case->{control_points}};
    return $case->{degree} == 2 ? new_quad(@points) : new_cubic(@points);
}

for my $case (@{$fixture->{cases}}) {
    my $curve = curve_for($case);
    if ($case->{expected_disposition} eq 'invalid-tolerance') {
        like dies { $curve->polyline($case->{tolerance}) }, qr/tolerance/, "$case->{id} rejects tolerance";
        next;
    }
    if (($case->{expected_termination} // '') eq 'budget-error') {
        like dies { $curve->polyline($case->{tolerance}) }, qr/budget/, "$case->{id} exhausts budget";
        next;
    }

    my $points = $curve->polyline($case->{tolerance});
    is $points->[0], $curve->p0, "$case->{id} starts at p0";
    my $last = $case->{degree} == 2 ? $curve->p2 : $curve->p3;
    is $points->[-1], $last, "$case->{id} ends at final endpoint";
    ok(!(grep { !isfinite($_->x) || !isfinite($_->y) } @$points),
        "$case->{id} produces finite points");
    $case->{expected_disposition} eq 'chord'
        ? is(scalar(@$points), 2, "$case->{id} emits chord")
        : ok(scalar(@$points) > 2, "$case->{id} subdivides");
}

my $straight = new_quad(new_point(0, 0), new_point(0.5, 0), new_point(1, 0));
for my $tolerance (0, -1, HUGE_VAL, -HUGE_VAL, nan) {
    like dies { $straight->polyline($tolerance) }, qr/tolerance/, 'rejects invalid tolerance';
}

for my $coordinate (HUGE_VAL, -HUGE_VAL, nan) {
    my $curve = new_cubic(
        new_point(0, 0), new_point($coordinate, 1),
        new_point(0, -1), new_point(0, 0),
    );
    like dies { $curve->polyline(0.1) }, qr/control/, 'rejects nonfinite control';
}

my $overflow = new_quad(new_point(1e308, 0), new_point(0, 1), new_point(-1e308, 0));
my $result;
like dies { $result = $overflow->polyline(0.1) }, qr/finite/, 'rejects nonfinite derived geometry';
is $result, undef, 'derived-geometry failure returns no partial result';

my $subnormal = new_quad(new_point(0, 0), new_point(0, 1e-200), new_point(1e-200, 0));
like dies { $subnormal->polyline(5e-324) }, qr/budget/, 'subnormal geometry cannot underflow into a chord';

done_testing;
