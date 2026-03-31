use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::VerilogLexer; 1 },
    'CodingAdventures::VerilogLexer loads' );

ok( CodingAdventures::VerilogLexer->VERSION, 'has a VERSION' );

done_testing;
