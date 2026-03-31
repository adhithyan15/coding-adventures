use strict;
use warnings;
use Test2::V0;

use CodingAdventures::WasmTypes;

ok(1, 'module loads');
is($CodingAdventures::WasmTypes::VERSION, '0.01', 'has VERSION 0.01');

done_testing;
