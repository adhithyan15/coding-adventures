use strict;
use warnings;
use Test::More tests => 2;

BEGIN {
    use_ok('CodingAdventures::HashFunctions');
}

ok($CodingAdventures::HashFunctions::VERSION, 'has a VERSION');
