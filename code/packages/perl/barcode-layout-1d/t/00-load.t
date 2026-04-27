use strict;
use warnings;
use Test2::V0;

use lib '../paint-instructions/lib';

ok( eval { require CodingAdventures::BarcodeLayout1D; 1 }, 'module loads' );

done_testing;
