use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::HuffmanTree; 1 }, 'CodingAdventures::HuffmanTree loads' );

ok defined $CodingAdventures::HuffmanTree::VERSION, 'VERSION is defined';
is $CodingAdventures::HuffmanTree::VERSION, '0.1.0', 'VERSION is 0.1.0';

done_testing;
