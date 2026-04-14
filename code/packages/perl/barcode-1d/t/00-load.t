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
use lib "$FindBin::Bin/../../upc-a/lib";
use lib "$FindBin::Bin/../lib";

ok( eval { require CodingAdventures::Barcode1D; 1 }, 'module loads' );
ok( CodingAdventures::Barcode1D->VERSION, 'has a VERSION' );

done_testing;
