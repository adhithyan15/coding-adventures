use strict;
use warnings;
use Test2::V0;
use FindBin;

use lib "$FindBin::Bin/../../code39/lib";
use lib "$FindBin::Bin/../../paint-instructions/lib";
use lib "$FindBin::Bin/../../barcode-layout-1d/lib";
use lib "$FindBin::Bin/../../pixel-container/lib";
use lib "$FindBin::Bin/../../paint_vm_metal_native/lib";
use lib "$FindBin::Bin/../../paint_codec_png_native/lib";
use lib "$FindBin::Bin/../lib";

require CodingAdventures::Barcode1D;

subtest 'build_scene' => sub {
  my $scene = CodingAdventures::Barcode1D->build_scene('HELLO-123', 'code39');
  is($scene->{metadata}{symbology}, 'code39', 'symbology');
  ok($scene->{width} > 0, 'width');
  is($scene->{height}, 120, 'height');
};

subtest 'current_backend' => sub {
  my $backend = CodingAdventures::Barcode1D::current_backend();
  ok(!defined($backend) || $backend eq 'metal', 'backend probe');
};

subtest 'render_png' => sub {
  return unless defined CodingAdventures::Barcode1D::current_backend();
  my $png = CodingAdventures::Barcode1D->render_png('HELLO-123', 'code39');
  is(substr($png, 0, 8), "\x89PNG\r\n\x1a\n", 'png signature');
};

done_testing;
