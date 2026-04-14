use strict;
use warnings;
use Test2::V0;

use lib '../paint-instructions/lib';
use lib '../barcode-layout-1d/lib';

require CodingAdventures::Itf;
my $pkg = 'CodingAdventures::Itf';

subtest 'normalize_itf' => sub {
  like(
    dies { $pkg->normalize_itf('12345') },
    qr/even number of digits/,
    'rejects odd-length input',
  );
};

subtest 'encode_itf' => sub {
  my $encoded = $pkg->encode_itf('123456');
  is(scalar @$encoded, 3, 'encodes digit pairs');
  is($encoded->[0]{pair}, '12', 'first pair');
};

subtest 'expand_itf_runs' => sub {
  my $runs = $pkg->expand_itf_runs('123456');
  ok((grep { $_->{role} eq 'start' } @$runs) > 0, 'includes start role');
  ok((grep { $_->{role} eq 'stop' } @$runs) > 0, 'includes stop role');
};

subtest 'draw_itf' => sub {
  my $scene = $pkg->draw_itf('123456');
  is($scene->{metadata}{symbology}, 'itf', 'symbology');
  is($scene->{metadata}{pair_count}, 3, 'pair count');
  is($scene->{height}, 120, 'height');
  ok($scene->{width} > 0, 'width');
};

done_testing;
