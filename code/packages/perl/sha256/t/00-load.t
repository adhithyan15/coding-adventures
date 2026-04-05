use strict;
use warnings;
use Test2::V0;

ok(eval { require CodingAdventures::SHA256; 1 }, 'CodingAdventures::SHA256 loads');

done_testing;
