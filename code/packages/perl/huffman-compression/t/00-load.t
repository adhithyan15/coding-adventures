use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::HuffmanCompression; 1 },
    'CodingAdventures::HuffmanCompression loads' );

ok defined $CodingAdventures::HuffmanCompression::VERSION, 'VERSION is defined';
is $CodingAdventures::HuffmanCompression::VERSION, '0.1.0', 'VERSION is 0.1.0';

done_testing;
