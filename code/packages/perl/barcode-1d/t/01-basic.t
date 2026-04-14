use strict;
use warnings;
use Test2::V0;
use FindBin;

use lib "$FindBin::Bin/../../codabar/lib";
use lib "$FindBin::Bin/../../code128/lib";
use lib "$FindBin::Bin/../../code39/lib";
use lib "$FindBin::Bin/../../ean-13/lib";
use lib "$FindBin::Bin/../../itf/lib";
use lib "$FindBin::Bin/../../paint-instructions/lib";
use lib "$FindBin::Bin/../../barcode-layout-1d/lib";
use lib "$FindBin::Bin/../../pixel-container/lib";
use lib "$FindBin::Bin/../../paint_vm_metal_native/lib";
use lib "$FindBin::Bin/../../paint_codec_png_native/lib";
use lib "$FindBin::Bin/../../upc-a/lib";
use lib "$FindBin::Bin/../lib";

require CodingAdventures::Barcode1D;

subtest 'build_scene' => sub {
  my $scene = CodingAdventures::Barcode1D->build_scene('HELLO-123', 'code39');
  is($scene->{metadata}{symbology}, 'code39', 'symbology');
  ok($scene->{width} > 0, 'width');
  is($scene->{height}, 120, 'height');
};

subtest 'build_scene_additional_symbologies' => sub {
  my %cases = (
    codabar => ['40156', 'codabar'],
    code128 => ['Code 128', 'code128'],
    'ean-13' => ['400638133393', 'ean-13'],
    itf => ['123456', 'itf'],
    'upc-a' => ['03600029145', 'upc-a'],
  );

  for my $symbology (sort keys %cases) {
    my ($input, $expected) = @{ $cases{$symbology} };
    my $scene = CodingAdventures::Barcode1D->build_scene($input, $symbology);
    is($scene->{metadata}{symbology}, $expected, "$symbology symbology tag");
    ok($scene->{width} > 0, "$symbology width");
  }
};

subtest 'current_backend' => sub {
  my $backend = CodingAdventures::Barcode1D::current_backend();
  ok(!defined($backend) || $backend eq 'metal', 'backend probe');
};

subtest 'render_png' => sub {
  my $backend = CodingAdventures::Barcode1D::current_backend();
  if (defined $backend) {
    my $png = CodingAdventures::Barcode1D->render_png('HELLO-123', 'code39');
    is(substr($png, 0, 8), "\x89PNG\r\n\x1a\n", 'png signature');
    return;
  }

  like(
    dies { CodingAdventures::Barcode1D->render_png('HELLO-123', 'code39') },
    qr/no native Paint VM is available for this host/,
    'returns a clean backend-unavailable error when no native backend exists',
  );
};

done_testing;
