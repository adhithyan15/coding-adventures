use strict;
use warnings;
use Test::More tests => 2;

BEGIN { use_ok('CodingAdventures::InMemoryDataStore') }
ok(CodingAdventures::InMemoryDataStore->VERSION, 'has a version');
