use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::VirtualMachine; 1 }, 'CodingAdventures::VirtualMachine loads' );

# Verify the module exports a version number.
ok(CodingAdventures::VirtualMachine->VERSION, 'has a VERSION');

done_testing;
