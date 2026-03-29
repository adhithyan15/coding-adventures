use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::WasmLeb128; 1 }, 'CodingAdventures::WasmLeb128 loads' );

# Verify the module exports a version number.
ok(CodingAdventures::WasmLeb128->VERSION, 'has a VERSION');

done_testing;
