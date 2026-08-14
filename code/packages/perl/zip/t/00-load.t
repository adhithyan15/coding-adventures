use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::Zip; 1 }, 'CodingAdventures::Zip loads' );

ok defined $CodingAdventures::Zip::VERSION, 'VERSION is defined';
is $CodingAdventures::Zip::VERSION, '0.2.0', 'VERSION is 0.2.0';

done_testing;
