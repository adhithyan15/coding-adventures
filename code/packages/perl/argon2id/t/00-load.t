use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::Argon2id; 1 },
    'CodingAdventures::Argon2id loads' );

ok( CodingAdventures::Argon2id->VERSION, 'has a VERSION' );
ok( defined &CodingAdventures::Argon2id::argon2id,     'argon2id defined' );
ok( defined &CodingAdventures::Argon2id::argon2id_hex, 'argon2id_hex defined' );

done_testing;
