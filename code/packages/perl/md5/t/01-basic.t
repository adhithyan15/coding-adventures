use strict;
use warnings;
use Test2::V0;

use CodingAdventures::Md5;

# ---------------------------------------------------------------------------
# Sanity / version
# ---------------------------------------------------------------------------
ok(1, 'module loads');
is(CodingAdventures::Md5->VERSION, '0.01', 'has VERSION 0.01');

# ---------------------------------------------------------------------------
# Known-good MD5 values (RFC 1321 test suite + common test vectors)
# ---------------------------------------------------------------------------

# Test 1: empty string
is(
    CodingAdventures::Md5::hex(""),
    "d41d8cd98f00b204e9800998ecf8427e",
    'MD5("") is correct'
);

# Test 2: "abc"
is(
    CodingAdventures::Md5::hex("abc"),
    "900150983cd24fb0d6963f7d28e17f72",
    'MD5("abc") is correct'
);

# Test 3: "hello"
is(
    CodingAdventures::Md5::hex("hello"),
    "5d41402abc4b2a76b9719d911017c592",
    'MD5("hello") is correct'
);

# Test 4: "message digest" (RFC 1321 test vector)
is(
    CodingAdventures::Md5::hex("message digest"),
    "f96b697d7cb7938d525a2f31aaf161d0",
    'MD5("message digest") is correct'
);

# Test 5: "abcdefghijklmnopqrstuvwxyz" (RFC 1321)
is(
    CodingAdventures::Md5::hex("abcdefghijklmnopqrstuvwxyz"),
    "c3fcd3d76192e4007dfb496cca67e13b",
    'MD5(alphabet) is correct'
);

# Test 6: alphanumeric (RFC 1321)
is(
    CodingAdventures::Md5::hex("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"),
    "d174ab98d277d9f5a5611c2c9f419d9f",
    'MD5(A..Za..z0..9) is correct'
);

# Test 7: pangram
is(
    CodingAdventures::Md5::hex("The quick brown fox jumps over the lazy dog"),
    "9e107d9d372bb6826bd81d3542a419d6",
    'MD5(pangram) is correct'
);

# Test 8: digest() returns arrayref of 16 integers
{
    my $bytes = CodingAdventures::Md5::digest("");
    ok(ref($bytes) eq 'ARRAY', 'digest() returns arrayref');
    is(scalar @$bytes, 16, 'digest() of empty string returns 16 bytes');
    ok((grep { $_ >= 0 && $_ <= 255 } @$bytes) == 16, 'all bytes in range 0-255');
}

# Test 9: digest("hello") has correct first two bytes
{
    my $bytes = CodingAdventures::Md5::digest("hello");
    # MD5("hello") = 5d41402abc4b2a76b9719d911017c592
    is($bytes->[0], 0x5d, 'digest("hello") first byte is 0x5d');
    is($bytes->[1], 0x41, 'digest("hello") second byte is 0x41');
}

# Test 10: OO interface works
{
    my $md5 = CodingAdventures::Md5->new();
    ok($md5, 'new() returns an object');
    is($md5->hex("hello"), "5d41402abc4b2a76b9719d911017c592", 'OO hex() matches functional');
    my $bytes = $md5->digest("hello");
    is(scalar @$bytes, 16, 'OO digest() returns 16 bytes');
}

# Test 11: deterministic
{
    my $h1 = CodingAdventures::Md5::hex("test");
    my $h2 = CodingAdventures::Md5::hex("test");
    is($h1, $h2, 'MD5 is deterministic (same input => same output)');
}

# Test 12: different inputs give different outputs
{
    my $h1 = CodingAdventures::Md5::hex("foo");
    my $h2 = CodingAdventures::Md5::hex("bar");
    isnt($h1, $h2, 'different inputs produce different hashes');
}

# Test 13: output is always exactly 32 lowercase hex characters
{
    for my $s ("", "a", "hello", "x" x 55, "x" x 56, "x" x 64, "x" x 128) {
        my $h = CodingAdventures::Md5::hex($s);
        is(length($h), 32, "hex output length is 32 for input length " . length($s));
        like($h, qr/\A[0-9a-f]{32}\z/, "hex output is lowercase hex for len " . length($s));
    }
}

done_testing;
