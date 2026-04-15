use strict;
use warnings;
use Test2::V0;
use FindBin;

use lib "$FindBin::Bin/../../paint-instructions/lib";
use lib "$FindBin::Bin/../../pixel-container/lib";
use lib "$FindBin::Bin/../lib";

require CodingAdventures::PaintVmMetalNative;

ok(1, 'PaintVmMetalNative module loads');

done_testing;
