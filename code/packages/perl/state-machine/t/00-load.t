use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::StateMachine; 1 }, 'CodingAdventures::StateMachine loads' );

# Verify the module exports a version number.
ok(CodingAdventures::StateMachine->VERSION, 'has a VERSION');

done_testing;
