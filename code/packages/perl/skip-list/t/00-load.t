use strict;
use warnings;
use Test::More;

ok(eval { require CodingAdventures::SkipList; 1 }, 'CodingAdventures::SkipList loads');
ok(CodingAdventures::SkipList->VERSION, 'has a VERSION');
ok(CodingAdventures::SkipList->can('from_entries'), 'constructor is available');

done_testing;
