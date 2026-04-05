use strict;
use warnings;
use Test2::V0;

use CodingAdventures::Sha512;

# ---------------------------------------------------------------------------
# Sanity / version
# ---------------------------------------------------------------------------
ok(1, 'module loads');
is(CodingAdventures::Sha512->VERSION, '0.01', 'has VERSION 0.01');

# ---------------------------------------------------------------------------
# Known-good SHA-512 values (FIPS 180-4 examples + reference vectors)
# ---------------------------------------------------------------------------

# Test 1: empty string
is(
    CodingAdventures::Sha512::hex(""),
    "cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce"
    . "47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e",
    'SHA512("") is correct'
);

# Test 2: "abc" (FIPS 180-4)
is(
    CodingAdventures::Sha512::hex("abc"),
    "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a"
    . "2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f",
    'SHA512("abc") is correct'
);

# Test 3: "hello"
is(
    CodingAdventures::Sha512::hex("hello"),
    "9b71d224bd62f3785d96d46ad3ea3d73319bfbc2890caadae2dff72519673ca7"
    . "2323c3d99ba5c11d7c7acc6e14b8c5da0c4663475c2e5c3adef46f73bcdec043",
    'SHA512("hello") is correct'
);

# Test 4: pangram
is(
    CodingAdventures::Sha512::hex("The quick brown fox jumps over the lazy dog"),
    "07e547d9586f6a73f73fbac0435ed76951218fb7d0c8d788a309d785436bbb64"
    . "2e93a252a954f23912547d1e8a3b5ed6e1bfd7097821233fa0538f3db854fee6",
    'SHA512(pangram) is correct'
);

# Test 5: FIPS 180-4 two-block test vector
is(
    CodingAdventures::Sha512::hex("abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmnhijklmnoijklmnopjklmnopqklmnopqrlmnopqrsmnopqrstnopqrstu"),
    "8e959b75dae313da8cf4f72814fc143f8f7779c6eb9f7fa17299aeadb6889018"
    . "501d289e4900f7e4331b99dec4b5433ac7d329eeb6dd26545e96e55b874be909",
    'SHA512(two-block vector) is correct'
);

# Test 6: single "a"
is(
    CodingAdventures::Sha512::hex("a"),
    "1f40fc92da241694750979ee6cf582f2d5d7d28e18335de05abc54d0560e0f53"
    . "02860c652bf08d560252aa5e74210546f369fbbbce8c12cfc7957b2652fe9a75",
    'SHA512("a") is correct'
);

# Test 7: digest() returns arrayref of 64 integers
{
    my $bytes = CodingAdventures::Sha512::digest("");
    ok(ref($bytes) eq 'ARRAY', 'digest() returns arrayref');
    is(scalar @$bytes, 64, 'digest() of empty string returns 64 bytes');
    ok((grep { $_ >= 0 && $_ <= 255 } @$bytes) == 64, 'all 64 bytes in range 0-255');
}

# Test 8: digest("abc") first bytes match known value
{
    my $bytes = CodingAdventures::Sha512::digest("abc");
    # SHA512("abc") = ddaf35a1...
    is($bytes->[0], 0xdd, 'digest("abc") first byte is 0xdd');
    is($bytes->[1], 0xaf, 'digest("abc") second byte is 0xaf');
    is($bytes->[63], 0x9f, 'digest("abc") last byte is 0x9f');
}

# Test 9: OO interface
{
    my $sha512 = CodingAdventures::Sha512->new();
    ok($sha512, 'new() returns an object');
    is(
        $sha512->hex("hello"),
        "9b71d224bd62f3785d96d46ad3ea3d73319bfbc2890caadae2dff72519673ca7"
        . "2323c3d99ba5c11d7c7acc6e14b8c5da0c4663475c2e5c3adef46f73bcdec043",
        'OO hex() matches'
    );
    my $bytes = $sha512->digest("abc");
    is(scalar @$bytes, 64, 'OO digest() returns 64 bytes');
}

# Test 10: deterministic
{
    my $h1 = CodingAdventures::Sha512::hex("determinism");
    my $h2 = CodingAdventures::Sha512::hex("determinism");
    is($h1, $h2, 'SHA512 is deterministic');
}

# Test 11: output always 128 lowercase hex characters
{
    for my $s ("", "a", "abc", "x" x 111, "x" x 112, "x" x 128, "x" x 200) {
        my $h = CodingAdventures::Sha512::hex($s);
        is(length($h), 128, "hex output length is 128 for input len " . length($s));
        like($h, qr/\A[0-9a-f]{128}\z/, "output is lowercase hex for len " . length($s));
    }
}

# Test 12: avalanche -- one character difference changes the output
{
    my $h1 = CodingAdventures::Sha512::hex("a");
    my $h2 = CodingAdventures::Sha512::hex("b");
    isnt($h1, $h2, 'one character difference gives different hash');
}

done_testing;
