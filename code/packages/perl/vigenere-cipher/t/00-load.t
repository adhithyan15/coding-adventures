use Test2::V0;
use CodingAdventures::VigenereCipher qw(encrypt decrypt find_key_length find_key break_cipher);

ok(1, "Module loaded successfully");
is($CodingAdventures::VigenereCipher::VERSION, "0.1.0", "Version is correct");

done_testing;
