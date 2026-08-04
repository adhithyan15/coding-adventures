use strict;
use warnings;
use Test::More;

ok(eval { require CodingAdventures::FenwickTree; 1 }, 'CodingAdventures::FenwickTree loads');
ok(CodingAdventures::FenwickTree->VERSION, 'has a VERSION');

done_testing;
