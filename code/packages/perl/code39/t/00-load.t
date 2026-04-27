use strict;
use warnings;
use Test2::V0;

use lib '../paint-instructions/lib';
use lib '../barcode-layout-1d/lib';

ok( eval { require CodingAdventures::Code39; 1 }, 'module loads' );
ok( CodingAdventures::Code39->VERSION, 'has a VERSION' );

done_testing;
