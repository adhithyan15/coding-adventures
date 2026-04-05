use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::WasmValidator; 1 }, 'CodingAdventures::WasmValidator loads' );

# Verify the module exports a version number.
ok(CodingAdventures::WasmValidator->VERSION, 'has a VERSION');

done_testing;
