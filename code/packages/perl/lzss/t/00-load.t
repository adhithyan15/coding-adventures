use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::LZSS; 1 }, 'CodingAdventures::LZSS loads' );

ok defined $CodingAdventures::LZSS::VERSION, 'VERSION is defined';
is $CodingAdventures::LZSS::VERSION, '0.1.0', 'VERSION is 0.1.0';

done_testing;
