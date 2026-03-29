use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::Md5; 1 }, 'CodingAdventures::Md5 loads' );

# Verify the module exports a version number.
ok(CodingAdventures::Md5->VERSION, 'has a VERSION');

done_testing;
