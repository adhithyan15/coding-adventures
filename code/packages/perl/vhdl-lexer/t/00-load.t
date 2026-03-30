use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::VhdlLexer; 1 },
    'CodingAdventures::VhdlLexer loads' );

ok( CodingAdventures::VhdlLexer->VERSION, 'has a VERSION' );

done_testing;
