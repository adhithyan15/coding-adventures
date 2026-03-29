use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::PythonLexer; 1 },
    'CodingAdventures::PythonLexer loads' );

ok( CodingAdventures::PythonLexer->VERSION, 'has a VERSION' );

done_testing;
