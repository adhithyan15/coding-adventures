use strict;
use warnings;
use Test::More;

ok(eval { require CodingAdventures::RespProtocol; 1 }, 'CodingAdventures::RespProtocol loads');
ok(CodingAdventures::RespProtocol->VERSION, 'has a VERSION');

done_testing;
