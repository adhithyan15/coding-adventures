use strict;
use warnings;
use Test2::V0;

use CodingAdventures::SHA256;

# ---------------------------------------------------------------------------
# Sanity / version
# ---------------------------------------------------------------------------
ok(1, 'module loads');
is(CodingAdventures::SHA256->VERSION, '0.01', 'has VERSION 0.01');

# ---------------------------------------------------------------------------
# NIST FIPS 180-4 test vectors (one-shot)
# ---------------------------------------------------------------------------

# Test 1: empty string
is(
    CodingAdventures::SHA256::sha256_hex(""),
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    'SHA256("") is correct'
);

# Test 2: "abc" (FIPS 180-4 example)
is(
    CodingAdventures::SHA256::sha256_hex("abc"),
    "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
    'SHA256("abc") is correct'
);

# Test 3: 448-bit test vector
is(
    CodingAdventures::SHA256::sha256_hex("abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
    "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1",
    'SHA256(448-bit test vector) is correct'
);

# Test 4: "hello"
is(
    CodingAdventures::SHA256::sha256_hex("hello"),
    "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824",
    'SHA256("hello") is correct'
);

# Test 5: pangram
is(
    CodingAdventures::SHA256::sha256_hex("The quick brown fox jumps over the lazy dog"),
    "d7a8fbb307d7809469ca9abcb0082e4f8d5651e46d3cdb762d02d0bf37c9e592",
    'SHA256(pangram) is correct'
);

# Test 6: single 'a'
is(
    CodingAdventures::SHA256::sha256_hex("a"),
    "ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb",
    'SHA256("a") is correct'
);

# ---------------------------------------------------------------------------
# sha256() returns arrayref of 32 integers
# ---------------------------------------------------------------------------
{
    my $bytes = CodingAdventures::SHA256::sha256("");
    ok(ref($bytes) eq 'ARRAY', 'sha256() returns arrayref');
    is(scalar @$bytes, 32, 'sha256() of empty string returns 32 bytes');
    ok((grep { $_ >= 0 && $_ <= 255 } @$bytes) == 32, 'all 32 bytes in range 0-255');
}

# Test first bytes of sha256("abc")
{
    my $bytes = CodingAdventures::SHA256::sha256("abc");
    # ba7816bf...
    is($bytes->[0], 0xba, 'sha256("abc") first byte is 0xba');
    is($bytes->[1], 0x78, 'sha256("abc") second byte is 0x78');
    is($bytes->[31], 0xad, 'sha256("abc") last byte is 0xad');
}

# ---------------------------------------------------------------------------
# Block boundary tests
# ---------------------------------------------------------------------------
{
    is(
        CodingAdventures::SHA256::sha256_hex("a" x 55),
        "9f4390f8d30c2dd92ec9f095b65e2b9ae9b0a925a5258e241c9f1e910f734318",
        '55 bytes (padding fits in one block)'
    );
    is(
        CodingAdventures::SHA256::sha256_hex("a" x 56),
        "b35439a4ac6f0948b6d6f9e3c6af0f5f590ce20f1bde7090ef7970686ec6738a",
        '56 bytes (padding overflows to second block)'
    );
    is(
        CodingAdventures::SHA256::sha256_hex("a" x 64),
        "ffe054fe7ae0cb6dc65c3af9b61d5209f439851db43d0ba5997337df154668eb",
        '64 bytes (exact one block before padding)'
    );
}

# ---------------------------------------------------------------------------
# OO interface (one-shot)
# ---------------------------------------------------------------------------
{
    my $sha256 = CodingAdventures::SHA256->new();
    ok($sha256, 'new() returns an object');
}

# ---------------------------------------------------------------------------
# Determinism
# ---------------------------------------------------------------------------
{
    my $h1 = CodingAdventures::SHA256::sha256_hex("determinism");
    my $h2 = CodingAdventures::SHA256::sha256_hex("determinism");
    is($h1, $h2, 'SHA256 is deterministic');
}

# ---------------------------------------------------------------------------
# Output format
# ---------------------------------------------------------------------------
{
    for my $s ("", "a", "abc", "x" x 55, "x" x 56, "x" x 64, "x" x 100) {
        my $h = CodingAdventures::SHA256::sha256_hex($s);
        is(length($h), 64, "hex output length is 64 for input len " . length($s));
        like($h, qr/\A[0-9a-f]{64}\z/, "output is lowercase hex for len " . length($s));
    }
}

# ---------------------------------------------------------------------------
# Avalanche
# ---------------------------------------------------------------------------
{
    my $h1 = CodingAdventures::SHA256::sha256_hex("a");
    my $h2 = CodingAdventures::SHA256::sha256_hex("b");
    isnt($h1, $h2, 'one character difference gives different hash');
}

# ---------------------------------------------------------------------------
# Streaming hasher
# ---------------------------------------------------------------------------

# Single update equals one-shot
{
    my $h = CodingAdventures::SHA256->new();
    $h->update("abc");
    is($h->hex_digest(), CodingAdventures::SHA256::sha256_hex("abc"),
        'streaming single update equals one-shot');
}

# Split at byte boundary
{
    my $h = CodingAdventures::SHA256->new();
    $h->update("ab");
    $h->update("c");
    is($h->hex_digest(),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        'streaming split at byte boundary');
}

# Split at block boundary
{
    my $h = CodingAdventures::SHA256->new();
    $h->update("a" x 64);
    $h->update("a" x 64);
    is($h->hex_digest(), CodingAdventures::SHA256::sha256_hex("a" x 128),
        'streaming split at block boundary');
}

# Many tiny updates
{
    my $msg = "abcdefghijklmnopqrstuvwxyz";
    my $h = CodingAdventures::SHA256->new();
    for my $ch ( split //, $msg ) {
        $h->update($ch);
    }
    is($h->hex_digest(), CodingAdventures::SHA256::sha256_hex($msg),
        'streaming many tiny updates');
}

# Digest is non-destructive
{
    my $h = CodingAdventures::SHA256->new();
    $h->update("abc");
    my $d1 = $h->hex_digest();
    my $d2 = $h->hex_digest();
    is($d1, $d2, 'digest is non-destructive');
}

# Continue after digest
{
    my $h = CodingAdventures::SHA256->new();
    $h->update("abc");
    $h->hex_digest();  # call and discard
    $h->update("def");
    is($h->hex_digest(), CodingAdventures::SHA256::sha256_hex("abcdef"),
        'can continue after digest');
}

# Empty streaming
{
    my $h = CodingAdventures::SHA256->new();
    is($h->hex_digest(), CodingAdventures::SHA256::sha256_hex(""),
        'empty streaming matches empty one-shot');
}

# Copy is independent
{
    my $original = CodingAdventures::SHA256->new();
    $original->update("abc");
    my $copied = $original->copy();
    $copied->update("def");

    is($original->hex_digest(), CodingAdventures::SHA256::sha256_hex("abc"),
        'original unchanged after copy update');
    is($copied->hex_digest(), CodingAdventures::SHA256::sha256_hex("abcdef"),
        'copy has additional data');
}

# Digest returns 32 bytes
{
    my $h = CodingAdventures::SHA256->new();
    $h->update("test");
    my $d = $h->digest();
    is(scalar @$d, 32, 'streaming digest returns 32 bytes');
}

# Update is chainable
{
    my $h = CodingAdventures::SHA256->new();
    my $ret = $h->update("abc");
    is($ret, $h, 'update returns self (chainable)');
}

# Large streaming input
{
    my $data = "X" x 1000;
    my $h = CodingAdventures::SHA256->new();
    for ( 1 .. 10 ) {
        $h->update("X" x 100);
    }
    is($h->hex_digest(), CodingAdventures::SHA256::sha256_hex($data),
        'large streaming input matches one-shot');
}

done_testing;
