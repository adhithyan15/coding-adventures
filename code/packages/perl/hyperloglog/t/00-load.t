use strict;
use warnings;
use Test::More;

ok(eval { require CodingAdventures::HyperLogLog; 1 }, 'CodingAdventures::HyperLogLog loads');
ok(CodingAdventures::HyperLogLog->VERSION, 'has a VERSION');
ok(CodingAdventures::HyperLogLog->can('new'), 'constructor is available');

done_testing;
