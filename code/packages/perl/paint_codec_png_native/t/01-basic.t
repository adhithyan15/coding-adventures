use strict;
use warnings;
use Test2::V0;
use FindBin;

use lib "$FindBin::Bin/../../pixel-container/lib";
use lib "$FindBin::Bin/../lib";

require CodingAdventures::PaintCodecPngNative;

my $png = CodingAdventures::PaintCodecPngNative::encode_rgba8_native(1, 1, pack('C*', 0, 0, 0, 255));
is(substr($png, 0, 8), "\x89PNG\r\n\x1a\n", 'png signature');

done_testing;
