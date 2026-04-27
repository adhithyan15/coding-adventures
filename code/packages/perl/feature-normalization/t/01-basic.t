use strict;
use warnings;
use Test::More;

use CodingAdventures::FeatureNormalization qw(
    fit_standard_scaler transform_standard
    fit_min_max_scaler transform_min_max
);

my $rows = [
    [1000.0, 3.0, 1.0],
    [1500.0, 4.0, 0.0],
    [2000.0, 5.0, 1.0],
];

sub near {
    my ($expected, $actual) = @_;
    return abs($expected - $actual) <= 1e-9;
}

my $standard_scaler = fit_standard_scaler($rows);
ok near(1500.0, $standard_scaler->{means}->[0]), 'standard mean for first column';
ok near(4.0, $standard_scaler->{means}->[1]), 'standard mean for second column';

my $standard = transform_standard($rows, $standard_scaler);
ok near(-1.224744871391589, $standard->[0]->[0]), 'standard transform first row';
ok near(0.0, $standard->[1]->[0]), 'standard transform middle row';
ok near(1.224744871391589, $standard->[2]->[0]), 'standard transform last row';

my $min_max = transform_min_max($rows, fit_min_max_scaler($rows));
is_deeply $min_max, [
    [0.0, 0.0, 1.0],
    [0.5, 0.5, 0.0],
    [1.0, 1.0, 1.0],
], 'min-max maps columns to unit range';

my $constant_rows = [[1.0, 7.0], [2.0, 7.0]];
my $constant_standard = transform_standard($constant_rows, fit_standard_scaler($constant_rows));
my $constant_min_max = transform_min_max($constant_rows, fit_min_max_scaler($constant_rows));
ok near(0.0, $constant_standard->[0]->[1]), 'standard constant column maps to zero';
ok near(0.0, $constant_min_max->[0]->[1]), 'min-max constant column maps to zero';

done_testing;
