use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::StarlarkLexer; 1 },
    'CodingAdventures::StarlarkLexer loads' );

ok( CodingAdventures::StarlarkLexer->VERSION, 'has a VERSION' );

done_testing;
