use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::WasmExecution; 1 }, 'CodingAdventures::WasmExecution loads' );

# Verify the module exports a version number.
ok(CodingAdventures::WasmExecution->VERSION, 'has a VERSION');

done_testing;
