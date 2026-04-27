use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::Argon2i; 1 },
    'CodingAdventures::Argon2i loads' );

ok( CodingAdventures::Argon2i->VERSION, 'has a VERSION' );
ok( defined &CodingAdventures::Argon2i::argon2i,     'argon2i defined' );
ok( defined &CodingAdventures::Argon2i::argon2i_hex, 'argon2i_hex defined' );

done_testing;
