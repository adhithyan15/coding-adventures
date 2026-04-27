use strict;
use warnings;
use Test2::V0;

use lib '../paint-instructions/lib';
use lib '../barcode-layout-1d/lib';

ok( eval { require CodingAdventures::Codabar; 1 }, 'module loads' );
ok( CodingAdventures::Codabar->VERSION, 'has a VERSION' );

done_testing;
