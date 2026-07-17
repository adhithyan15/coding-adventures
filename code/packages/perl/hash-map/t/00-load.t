use strict;
use warnings;
use Test::More tests => 2;

BEGIN {
    use_ok('CodingAdventures::HashMap');
}

ok($CodingAdventures::HashMap::VERSION, 'has a VERSION');
