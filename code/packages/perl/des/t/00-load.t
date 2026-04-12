use strict;
use warnings;
use Test2::V0;

ok(eval { require CodingAdventures::DES; 1 }, 'CodingAdventures::DES loads');

done_testing;
