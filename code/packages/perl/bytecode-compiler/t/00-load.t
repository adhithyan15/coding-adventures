use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::BytecodeCompiler; 1 }, 'CodingAdventures::BytecodeCompiler loads' );

# Verify the module exports a version number.
ok(CodingAdventures::BytecodeCompiler->VERSION, 'has a VERSION');

done_testing;
