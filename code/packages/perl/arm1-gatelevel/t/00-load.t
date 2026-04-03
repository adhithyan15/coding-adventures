use strict;
use warnings;
use Test::More;

ok( eval { require CodingAdventures::ARM1Gatelevel; 1 }, 'CodingAdventures::ARM1Gatelevel loads' )
    or diag $@;

ok( CodingAdventures::ARM1Gatelevel->VERSION, 'has a VERSION' );

done_testing();
