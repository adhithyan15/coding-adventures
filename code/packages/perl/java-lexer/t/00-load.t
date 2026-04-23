use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::JavaLexer; 1 },
    'CodingAdventures::JavaLexer loads' );

ok( CodingAdventures::JavaLexer->VERSION, 'has a VERSION' );

done_testing;
