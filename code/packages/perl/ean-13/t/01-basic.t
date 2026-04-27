use strict;
use warnings;
use Test2::V0;

use lib '../paint-instructions/lib';
use lib '../barcode-layout-1d/lib';

require CodingAdventures::Ean13;
my $pkg = 'CodingAdventures::Ean13';

subtest 'compute_ean_13_check_digit' => sub {
  is($pkg->compute_ean_13_check_digit('400638133393'), 1, 'matches reference check digit');
};

subtest 'normalize_ean_13' => sub {
  is($pkg->normalize_ean_13('400638133393'), '4006381333931', 'appends check digit');
  is($pkg->left_parity_pattern('4006381333931'), 'LGLLGG', 'derives left parity pattern');
};

subtest 'expand_ean_13_runs' => sub {
  my $runs = $pkg->expand_ean_13_runs('4006381333931');
  my $total_modules = 0;
  $total_modules += $_->{modules} for @$runs;
  is($total_modules, 95, 'expands to 95 modules');
};

subtest 'draw_ean_13' => sub {
  my $scene = $pkg->draw_ean_13('400638133393');
  is($scene->{metadata}{symbology}, 'ean-13', 'symbology');
  is($scene->{metadata}{content_modules}, 95, 'module count');
  is($scene->{height}, 120, 'height');
  ok($scene->{width} > 0, 'width');
};

done_testing;
