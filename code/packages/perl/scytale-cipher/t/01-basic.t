use Test2::V0;
use CodingAdventures::ScytaleCipher qw(encrypt decrypt brute_force);

# --- Encryption tests ---

subtest "encrypt" => sub {
    is(encrypt("HELLO WORLD", 3), "HLWLEOODL R ", "HELLO WORLD key=3");
    is(encrypt("ABCDEF", 2), "ACEBDF", "ABCDEF key=2");
    is(encrypt("ABCDEF", 3), "ADBECF", "ABCDEF key=3");
    is(encrypt("ABCD", 4), "ABCD", "key equals text length");
    is(encrypt("", 2), "", "empty string");

    like(
        dies { encrypt("HELLO", 1) },
        qr/Key must be >= 2/,
        "key < 2 raises"
    );

    like(
        dies { encrypt("HI", 3) },
        qr/Key must be <= text length/,
        "key > length raises"
    );
};

# --- Decryption tests ---

subtest "decrypt" => sub {
    is(decrypt("HLWLEOODL R ", 3), "HELLO WORLD", "HELLO WORLD key=3");
    is(decrypt("ACEBDF", 2), "ABCDEF", "ACEBDF key=2");
    is(decrypt("", 2), "", "empty string");

    like(
        dies { decrypt("HELLO", 0) },
        qr/Key must be >= 2/,
        "key < 2 raises"
    );
};

# --- Round trip tests ---

subtest "round trip" => sub {
    my $text = "HELLO WORLD";
    is(decrypt(encrypt($text, 3), 3), $text, "HELLO WORLD round trip");

    $text = "The quick brown fox jumps over the lazy dog!";
    my $n = length($text);
    for my $key (2 .. int($n / 2)) {
        my $ct = encrypt($text, $key);
        my $pt = decrypt($ct, $key);
        is($pt, $text, "round trip key=$key");
    }
};

# --- Brute force tests ---

subtest "brute_force" => sub {
    my $original = "HELLO WORLD";
    my $ct = encrypt($original, 3);
    my @results = brute_force($ct);

    my ($found) = grep { $_->{key} == 3 } @results;
    ok($found, "found key=3 in brute force results");
    is($found->{text}, $original, "correct decryption for key=3");

    my @results2 = brute_force("ABCDEFGHIJ");
    is(scalar(@results2), 4, "4 results for 10-char text");
    is($results2[0]{key}, 2, "first key is 2");
    is($results2[3]{key}, 5, "last key is 5");

    my @empty = brute_force("AB");
    is(scalar(@empty), 0, "short text returns empty");
};

# --- Padding tests ---

subtest "padding" => sub {
    my $ct = encrypt("HELLO", 3);
    is(decrypt($ct, 3), "HELLO", "padding stripped on decrypt");

    $ct = encrypt("ABCDEF", 2);
    is(length($ct), 6, "no padding when evenly divisible");
};

done_testing;
