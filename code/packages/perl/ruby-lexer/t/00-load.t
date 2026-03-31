use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::RubyLexer; 1 },
    'CodingAdventures::RubyLexer loads' );

ok( CodingAdventures::RubyLexer->VERSION, 'has a VERSION' );

done_testing;
