use strict;
use warnings;
use Test::More;

use_ok('CodingAdventures::DerAsn1');

# Verify the module exports a version number.
ok(CodingAdventures::DerAsn1->VERSION, 'has a VERSION');

done_testing;
