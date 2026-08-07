use strict;
use warnings;
use Test::More tests => 2;

use_ok('CodingAdventures::InMemoryDataStoreEngine');
ok($CodingAdventures::InMemoryDataStoreEngine::VERSION, 'has a VERSION');
