use strict;
use warnings;
use Test2::V0;

use CodingAdventures::Sha1;

# ---------------------------------------------------------------------------
# Sanity / version
# ---------------------------------------------------------------------------
ok(1, 'module loads');
is(CodingAdventures::Sha1->VERSION, '0.01', 'has VERSION 0.01');

# ---------------------------------------------------------------------------
# Known-good SHA-1 values (FIPS 180-4 examples + NIST test vectors)
# ---------------------------------------------------------------------------

# Test 1: empty string
is(
    CodingAdventures::Sha1::hex(""),
    "da39a3ee5e6b4b0d3255bfef95601890afd80709",
    'SHA1("") is correct'
);

# Test 2: "abc" (FIPS 180-4 §B.1)
is(
    CodingAdventures::Sha1::hex("abc"),
    "a9993e364706816aba3e25717850c26c9cd0d89d",
    'SHA1("abc") is correct'
);

# Test 3: "hello"
is(
    CodingAdventures::Sha1::hex("hello"),
    "aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d",
    'SHA1("hello") is correct'
);

# Test 4: pangram
is(
    CodingAdventures::Sha1::hex("The quick brown fox jumps over the lazy dog"),
    "2fd4e1c67a2d28fced849ee1bb76e7391b93eb12",
    'SHA1(pangram) is correct'
);

# Test 5: "The quick brown fox jumps over the lazy cog" (one letter different)
is(
    CodingAdventures::Sha1::hex("The quick brown fox jumps over the lazy cog"),
    "de9f2c7fd25e1b3afad3e85a0bd17d9b100db4b3",
    'SHA1(pangram-cog variant) is correct'
);

# Test 6: FIPS 180-4 long test vector
#   "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"
is(
    CodingAdventures::Sha1::hex("abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
    "84983e441c3bd26ebaae4aa1f95129e5e54670f1",
    'SHA1(448-bit test vector) is correct'
);

# Test 7: digest() returns arrayref of 20 integers
{
    my $bytes = CodingAdventures::Sha1::digest("");
    ok(ref($bytes) eq 'ARRAY', 'digest() returns arrayref');
    is(scalar @$bytes, 20, 'digest() of empty string returns 20 bytes');
    ok((grep { $_ >= 0 && $_ <= 255 } @$bytes) == 20, 'all 20 bytes in range 0-255');
}

# Test 8: digest("abc") first bytes match known value
{
    my $bytes = CodingAdventures::Sha1::digest("abc");
    # SHA1("abc") = a9993e364706816aba3e25717850c26c9cd0d89d
    # First byte = 0xa9 = 169
    is($bytes->[0], 0xa9, 'digest("abc") first byte is 0xa9');
    is($bytes->[1], 0x99, 'digest("abc") second byte is 0x99');
    is($bytes->[19], 0x9d, 'digest("abc") last byte is 0x9d');
}

# Test 9: OO interface
{
    my $sha1 = CodingAdventures::Sha1->new();
    ok($sha1, 'new() returns an object');
    is($sha1->hex("hello"), "aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d", 'OO hex() matches');
    my $bytes = $sha1->digest("abc");
    is(scalar @$bytes, 20, 'OO digest() returns 20 bytes');
}

# Test 10: deterministic
{
    my $h1 = CodingAdventures::Sha1::hex("determinism");
    my $h2 = CodingAdventures::Sha1::hex("determinism");
    is($h1, $h2, 'SHA1 is deterministic');
}

# Test 11: output always 40 lowercase hex characters
{
    for my $s ("", "a", "abc", "x" x 55, "x" x 56, "x" x 64, "x" x 100) {
        my $h = CodingAdventures::Sha1::hex($s);
        is(length($h), 40, "hex output length is 40 for input len " . length($s));
        like($h, qr/\A[0-9a-f]{40}\z/, "output is lowercase hex for len " . length($s));
    }
}

# Test 12: avalanche — one-bit difference changes the output substantially
{
    my $h1 = CodingAdventures::Sha1::hex("a");
    my $h2 = CodingAdventures::Sha1::hex("b");
    isnt($h1, $h2, 'one character difference gives different hash');
}

done_testing;
