use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::WasmRuntime; 1 }, 'CodingAdventures::WasmRuntime loads' );

# Verify the module exports a version number.
ok(CodingAdventures::WasmRuntime->VERSION, 'has a VERSION');

done_testing;
