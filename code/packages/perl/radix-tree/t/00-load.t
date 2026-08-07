use strict;
use warnings;
use Test::More;

ok(eval { require CodingAdventures::RadixTree; 1 }, 'CodingAdventures::RadixTree loads');
ok(CodingAdventures::RadixTree->VERSION, 'has a VERSION');

done_testing;
