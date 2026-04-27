use strict;
use warnings;
use Test2::V0;

use lib '../paint-instructions/lib';
use lib '../barcode-layout-1d/lib';

require CodingAdventures::UpcA;
my $pkg = 'CodingAdventures::UpcA';

subtest 'compute_upc_a_check_digit' => sub {
  is($pkg->compute_upc_a_check_digit('03600029145'), 2, 'matches reference check digit');
};

subtest 'normalize_upc_a' => sub {
  is($pkg->normalize_upc_a('03600029145'), '036000291452', 'appends check digit');
};

subtest 'expand_upc_a_runs' => sub {
  my $runs = $pkg->expand_upc_a_runs('036000291452');
  my $total_modules = 0;
  $total_modules += $_->{modules} for @$runs;
  is($total_modules, 95, 'expands to 95 modules');
};

subtest 'draw_upc_a' => sub {
  my $scene = $pkg->draw_upc_a('03600029145');
  is($scene->{metadata}{symbology}, 'upc-a', 'symbology');
  is($scene->{metadata}{content_modules}, 95, 'module count');
  is($scene->{height}, 120, 'height');
  ok($scene->{width} > 0, 'width');
};

done_testing;
