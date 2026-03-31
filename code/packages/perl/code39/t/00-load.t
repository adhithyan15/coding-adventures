use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::Code39; 1 }, 'module loads' );
ok( CodingAdventures::Code39->VERSION, 'has a VERSION' );

done_testing;
