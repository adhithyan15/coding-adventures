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

ok( eval { require CodingAdventures::TcpClient; 1 }, 'CodingAdventures::TcpClient loads' );

# Verify the module exports a version number.
ok(CodingAdventures::TcpClient->VERSION, 'has a VERSION');

done_testing;
