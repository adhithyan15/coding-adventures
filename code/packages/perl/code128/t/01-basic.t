use strict;
use warnings;
use Test2::V0;

use lib '../paint-instructions/lib';
use lib '../barcode-layout-1d/lib';

require CodingAdventures::Code128;
my $pkg = 'CodingAdventures::Code128';

subtest 'compute_code128_checksum' => sub {
  my @values = map { $pkg->value_for_code128_b_char($_) } split //, 'Code 128';
  is($pkg->compute_code128_checksum(\@values), 64, 'matches reference checksum');
};

subtest 'encode_code128_b' => sub {
  my $encoded = $pkg->encode_code128_b('Code 128');
  is($encoded->[0]{role}, 'start', 'start marker');
  is($encoded->[-2]{role}, 'check', 'checksum marker');
  is($encoded->[-1]{role}, 'stop', 'stop marker');
};

subtest 'draw_code128' => sub {
  my $scene = $pkg->draw_code128('Code 128');
  is($scene->{metadata}{symbology}, 'code128', 'symbology');
  is($scene->{metadata}{code_set}, 'B', 'code set');
  is($scene->{height}, 120, 'height');
  ok($scene->{width} > 0, 'width');
};

done_testing;
