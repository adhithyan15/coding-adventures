use strict;
use warnings;
use Test::More tests => 2;

BEGIN { use_ok('CodingAdventures::HashSet') }

ok(defined $CodingAdventures::HashSet::VERSION, 'has a VERSION');
