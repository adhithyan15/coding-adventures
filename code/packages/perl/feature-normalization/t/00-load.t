use strict;
use warnings;
use Test2::V0;

ok(eval { require CodingAdventures::FeatureNormalization; 1 }, 'loads CodingAdventures::FeatureNormalization')
    or diag $@;

# Verify the module exports a version number.
ok(CodingAdventures::FeatureNormalization->VERSION, 'has a VERSION');

done_testing;
