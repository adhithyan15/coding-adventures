use strict;
use warnings;
use Test2::V0;

use CodingAdventures::WasmSimulator;

ok(1, 'module loaded');
is($CodingAdventures::WasmSimulator::VERSION, '0.01', 'VERSION is 0.01');

done_testing;
