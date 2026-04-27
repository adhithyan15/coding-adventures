use strict;
use warnings;
use Test2::V0;

ok(eval { require CodingAdventures::NibIrCompiler; 1 }, 'module loads');

# Verify the module exports a version number.
ok(CodingAdventures::NibIrCompiler->VERSION, 'has a VERSION');

done_testing;
