use Test2::V0;
use CodingAdventures::ScytaleCipher qw(encrypt decrypt brute_force);

ok(1, "Module loaded successfully");
is($CodingAdventures::ScytaleCipher::VERSION, "0.1.0", "Version is correct");

done_testing;
