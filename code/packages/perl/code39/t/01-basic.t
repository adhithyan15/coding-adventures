use strict;
use warnings;
use Test2::V0;

use lib '../paint-instructions/lib';
use lib '../barcode-layout-1d/lib';

require CodingAdventures::Code39;
my $pkg = 'CodingAdventures::Code39';

subtest 'encode_code39_char' => sub {
  my $enc = $pkg->encode_code39_char('A');
  is($enc->{pattern}, 'WNNNNWNNW', 'pattern for A');
};

subtest 'expand_code39_runs' => sub {
  my $runs = $pkg->expand_code39_runs('A');
  is(scalar @$runs, 29, 'A expands to 29 runs');
  is($runs->[9]{role}, 'inter-character-gap', 'gap role');
  is($runs->[10]{modules}, 3, 'first A bar is wide');
};

subtest 'draw_code39' => sub {
  my $scene = $pkg->draw_code39('A');
  is($scene->{metadata}{symbology}, 'code39', 'symbology');
  is($scene->{height}, 120, 'height');
  ok($scene->{width} > 0, 'width');
};

done_testing;
