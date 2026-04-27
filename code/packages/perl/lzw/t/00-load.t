use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::LZW; 1 }, 'CodingAdventures::LZW loads' );

ok defined $CodingAdventures::LZW::VERSION, 'VERSION is defined';
is $CodingAdventures::LZW::VERSION, '0.1.0', 'VERSION is 0.1.0';

done_testing;
