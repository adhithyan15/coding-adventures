use strict;
use warnings;
use Test::More;

ok(eval { require CodingAdventures::TreeSet; 1 }, 'CodingAdventures::TreeSet loads');
ok(CodingAdventures::TreeSet->VERSION, 'has a VERSION');
ok(CodingAdventures::TreeSet->can('from_values'), 'constructor is available');

done_testing;
