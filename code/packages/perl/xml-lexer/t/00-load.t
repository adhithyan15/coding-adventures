use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::XmlLexer; 1 },
    'CodingAdventures::XmlLexer loads' );

ok( CodingAdventures::XmlLexer->VERSION, 'has a VERSION' );

done_testing;
