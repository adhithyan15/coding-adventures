use strict;
use warnings;
use Test2::V0;

ok(eval { require CodingAdventures::IrToWasmValidator; 1 }, 'module loads');

# Verify the module exports a version number.
ok(CodingAdventures::IrToWasmValidator->VERSION, 'has a VERSION');

done_testing;
