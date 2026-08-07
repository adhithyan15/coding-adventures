use strict;
use warnings;
use Test::More;

ok(eval { require CodingAdventures::Trie; 1 }, 'CodingAdventures::Trie loads');
ok(CodingAdventures::Trie->VERSION, 'has a VERSION');

done_testing;
