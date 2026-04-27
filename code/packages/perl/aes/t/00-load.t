use strict;
use warnings;
use Test2::V0;

ok(eval { require CodingAdventures::AES; 1 }, 'CodingAdventures::AES loads');

done_testing;
