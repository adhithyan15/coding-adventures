use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::Lexer; 1 }, 'CodingAdventures::Lexer loads' );

# Verify the module exports a version number.
ok(CodingAdventures::Lexer->VERSION, 'has a VERSION');

done_testing;
