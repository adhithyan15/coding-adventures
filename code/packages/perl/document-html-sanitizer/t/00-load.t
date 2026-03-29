use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::DocumentHtmlSanitizer; 1 }, 'CodingAdventures::DocumentHtmlSanitizer loads' );

# Verify the module exports a version number.
ok(CodingAdventures::DocumentHtmlSanitizer->VERSION, 'has a VERSION');

done_testing;
