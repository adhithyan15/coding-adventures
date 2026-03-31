use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::TomlLexer; 1 },
    'CodingAdventures::TomlLexer loads' );

ok( CodingAdventures::TomlLexer->VERSION, 'has a VERSION' );

done_testing;
