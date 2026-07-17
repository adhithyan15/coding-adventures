use strict;
use warnings;
use Test::More tests => 2;

BEGIN {
    use_ok('CodingAdventures::BloomFilter');
}

ok($CodingAdventures::BloomFilter::VERSION, 'has a VERSION');
