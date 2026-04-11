use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::LZ78; 1 }, 'CodingAdventures::LZ78 loads' );

ok defined $CodingAdventures::LZ78::VERSION, 'VERSION is defined';
is $CodingAdventures::LZ78::VERSION, '0.1.0', 'VERSION is 0.1.0';

done_testing;
