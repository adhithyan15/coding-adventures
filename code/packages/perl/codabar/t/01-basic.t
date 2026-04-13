use strict;
use warnings;
use Test2::V0;

use lib '../paint-instructions/lib';
use lib '../barcode-layout-1d/lib';

require CodingAdventures::Codabar;
my $pkg = 'CodingAdventures::Codabar';

subtest 'normalize_codabar' => sub {
  is($pkg->normalize_codabar('40156'), 'A40156A', 'adds default guards');
  is($pkg->normalize_codabar('B1234D'), 'B1234D', 'preserves explicit guards');
};

subtest 'expand_codabar_runs' => sub {
  my $runs = $pkg->expand_codabar_runs('40156');
  ok((grep { $_->{role} eq 'inter-character-gap' } @$runs) > 0, 'includes inter-character gaps');
};

subtest 'draw_codabar' => sub {
  my $scene = $pkg->draw_codabar('40156');
  is($scene->{metadata}{symbology}, 'codabar', 'symbology');
  is($scene->{metadata}{start}, 'A', 'default start');
  is($scene->{metadata}{stop}, 'A', 'default stop');
  is($scene->{height}, 120, 'height');
  ok($scene->{width} > 0, 'width');
};

done_testing;
