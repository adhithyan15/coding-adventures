use strict;
use warnings;
use Test2::V0;

# Basic smoke test — the module must load cleanly.
ok( eval { require CodingAdventures::LispLexer; 1 }, 'module loads' );
ok( defined $CodingAdventures::LispLexer::VERSION, 'VERSION is defined' );

done_testing;
