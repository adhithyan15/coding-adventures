use strict;
use warnings;
use lib '../../asciidoc/lib';
use Test2::V0;

ok( eval { require CodingAdventures::AsciidocParser; 1 }, 'CodingAdventures::AsciidocParser loads' );

# Verify the module exports a version number.
ok(CodingAdventures::AsciidocParser->VERSION, 'has a VERSION');

done_testing;
