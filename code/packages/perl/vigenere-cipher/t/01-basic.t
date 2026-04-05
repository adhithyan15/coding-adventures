use Test2::V0;
use CodingAdventures::VigenereCipher qw(encrypt decrypt find_key_length find_key break_cipher);

# A long English text for cryptanalysis testing. Needs 200+ characters so the
# Index of Coincidence has enough statistical data.
my $LONG_TEXT = "The Vigenere cipher was long considered unbreakable and was known as "
    . "le chiffre indechiffrable for three hundred years until Friedrich "
    . "Kasiski published a general method of cryptanalysis in eighteen "
    . "sixty three which exploits the repeating nature of the keyword to "
    . "determine the key length and then uses frequency analysis on each "
    . "group of letters encrypted with the same key letter to recover the "
    . "original plaintext message without knowing the secret keyword at all "
    . "this technique works because each group of letters encrypted with the "
    . "same key letter forms a simple caesar cipher which can be broken by "
    . "comparing the frequency distribution of letters against the expected "
    . "frequencies found in normal english language text passages and "
    . "selecting the shift value that produces the closest match";

# --- Encryption Tests ---

subtest "encrypt" => sub {
    is(encrypt("ATTACKATDAWN", "LEMON"), "LXFOPVEFRNHR", "parity vector: ATTACKATDAWN/LEMON");
    is(encrypt("Hello, World!", "key"), "Rijvs, Uyvjn!", "parity vector: mixed case + punct");
    is(encrypt("A-T-T", "LEM"), "L-X-F", "non-alpha passes through");
    is(encrypt("ABBA", "AB"), "ACBB", "key wraps around");
    is(encrypt("A", "B"), "B", "single character");
    is(encrypt("ATTACKATDAWN", "lemon"), "LXFOPVEFRNHR", "lowercase key");
    is(encrypt("attackatdawn", "LEMON"), "lxfopvefrnhr", "uppercase key lowercase text");
    is(encrypt("", "key"), "", "empty plaintext");

    like(
        dies { encrypt("hello", "") },
        qr/Key must be/,
        "empty key raises"
    );

    like(
        dies { encrypt("hello", "key1") },
        qr/Key must be/,
        "non-alpha key raises"
    );
};

# --- Decryption Tests ---

subtest "decrypt" => sub {
    is(decrypt("LXFOPVEFRNHR", "LEMON"), "ATTACKATDAWN", "parity vector");
    is(decrypt("Rijvs, Uyvjn!", "key"), "Hello, World!", "mixed case + punct");
    is(decrypt("L-X-F", "LEM"), "A-T-T", "non-alpha preserved");
    is(decrypt("B", "B"), "A", "single character");
    is(decrypt("", "key"), "", "empty ciphertext");

    like(
        dies { decrypt("hello", "") },
        qr/Key must be/,
        "empty key raises"
    );
};

# --- Round Trip Tests ---

subtest "round_trip" => sub {
    my $text = "ATTACKATDAWN";
    is(decrypt(encrypt($text, "LEMON"), "LEMON"), $text, "uppercase round trip");

    $text = "Hello, World! This is a test of the Vigenere cipher.";
    is(decrypt(encrypt($text, "secret"), "secret"), $text, "mixed case round trip");

    for my $k ("A", "KEY", "LONGER", "VERYLONGKEYWORD") {
        my $ct = encrypt($LONG_TEXT, $k);
        my $pt = decrypt($ct, $k);
        is($pt, $LONG_TEXT, "round trip with key=$k");
    }
};

# --- Key Length Detection ---

subtest "find_key_length" => sub {
    my $ct = encrypt($LONG_TEXT, "SECRET");
    my $detected = find_key_length($ct);
    is($detected % 6, 0, "detects key length 6 or multiple (got $detected)");

    $ct = encrypt($LONG_TEXT, "KEY");
    $detected = find_key_length($ct);
    is($detected % 3, 0, "detects key length 3 or multiple (got $detected)");

    $ct = encrypt($LONG_TEXT, "SECRET");
    $detected = find_key_length($ct, 4);
    ok($detected >= 1 && $detected <= 4, "respects max_length=4 (got $detected)");
};

# --- Key Finding ---

subtest "find_key" => sub {
    my $ct = encrypt($LONG_TEXT, "SECRET");
    is(find_key($ct, 6), "SECRET", "finds SECRET with length 6");

    $ct = encrypt($LONG_TEXT, "KEY");
    is(find_key($ct, 3), "KEY", "finds KEY with length 3");
};

# --- Full Break ---

subtest "break_cipher" => sub {
    my $ct = encrypt($LONG_TEXT, "SECRET");
    my ($key, $pt) = break_cipher($ct);
    # Key may be repeated (IC can find multiples of true length)
    is($pt, $LONG_TEXT, "recovers plaintext for SECRET");
    is(length($key) % 6, 0, "key length is multiple of 6 (got " . length($key) . ")");

    $ct = encrypt($LONG_TEXT, "KEY");
    ($key, $pt) = break_cipher($ct);
    is($pt, $LONG_TEXT, "recovers plaintext for KEY");
    is(length($key) % 3, 0, "key length is multiple of 3 (got " . length($key) . ")");
};

# --- Edge Cases ---

subtest "edge_cases" => sub {
    my $text = "Hello, World!";
    is(encrypt($text, "A"), $text, "key A is identity");
    is(encrypt("A", "Z"), "Z", "key Z shifts A to Z");
    is(encrypt("B", "Z"), "A", "key Z shifts B to A");

    $text = "Test 123 !@# end";
    is(decrypt(encrypt($text, "KEY"), "KEY"), $text, "numbers and symbols round trip");

    # Key does not advance on non-alpha
    is(encrypt("A B", "AB"), "A C", "key skips spaces");
};

done_testing;
