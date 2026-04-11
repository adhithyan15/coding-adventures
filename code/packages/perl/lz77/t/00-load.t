use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::LZ77; 1 }, 'CodingAdventures::LZ77 loads' );

ok defined $CodingAdventures::LZ77::VERSION, 'VERSION is defined';
is $CodingAdventures::LZ77::VERSION, '0.1.0', 'VERSION is 0.1.0';

done_testing;
