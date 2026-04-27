use strict;
use warnings;
use Test2::V0;

use lib '../paint-instructions/lib';
use lib '../barcode-layout-1d/lib';

ok( eval { require CodingAdventures::Itf; 1 }, 'module loads' );
ok( CodingAdventures::Itf->VERSION, 'has a VERSION' );

done_testing;
