use strict;
use warnings;

BEGIN {
    if (eval { require Test2::V0; 1 }) {
        Test2::V0->import;
    } else {
        require Test::More;
        Test::More->import;
    }
}

ok( eval { require CodingAdventures::UrlParser; 1 }, 'CodingAdventures::UrlParser loads' );

# Verify the module exports a version number.
ok(CodingAdventures::UrlParser->VERSION, 'has a VERSION');

done_testing;
