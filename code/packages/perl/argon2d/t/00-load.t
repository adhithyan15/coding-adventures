use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::Argon2d; 1 },
    'CodingAdventures::Argon2d loads' );

ok( CodingAdventures::Argon2d->VERSION, 'has a VERSION' );
ok( defined &CodingAdventures::Argon2d::argon2d,     'argon2d defined' );
ok( defined &CodingAdventures::Argon2d::argon2d_hex, 'argon2d_hex defined' );

done_testing;
