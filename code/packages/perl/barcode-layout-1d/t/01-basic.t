use strict;
use warnings;
use Test2::V0;

use lib '../paint-instructions/lib';

require CodingAdventures::BarcodeLayout1D;
my $pkg = 'CodingAdventures::BarcodeLayout1D';

my $runs = $pkg->runs_from_binary_pattern('111001');
is($runs->[0]{color}, 'bar', 'first run color');
is($runs->[0]{modules}, 3, 'first run modules');

my $width_runs = $pkg->runs_from_width_pattern('WNW', ['bar', 'space', 'bar'],
    source_char => 'A', source_index => 0);
my $scene = $pkg->layout_barcode_1d($width_runs);
is($scene->{width}, 27 * 4, 'scene width');
is($scene->{height}, 120, 'scene height');

done_testing;
