use strict;
use warnings;
use Test2::V0;

# Set up library paths for HKDF and all transitive dependencies.
# HKDF -> HMAC -> SHA256, SHA512, MD5, SHA1
use lib '../lib';
use lib '../../hmac/lib';
use lib '../../sha256/lib';
use lib '../../sha512/lib';
use lib '../../md5/lib';
use lib '../../sha1/lib';

use CodingAdventures::HKDF qw(
    hkdf_extract  hkdf_extract_hex
    hkdf_expand   hkdf_expand_hex
    hkdf          hkdf_hex
);

# ============================================================================
# Helpers
# ============================================================================

# Decode hex string to binary.
sub h { pack('H*', $_[0]) }

# Encode binary as hex string.
sub to_hex { unpack('H*', $_[0]) }

# ============================================================================
# RFC 5869 Test Vectors — HKDF-SHA256
# ============================================================================

# Test Case 1: Basic SHA-256 with all parameters present.
# Output length (42) is not a multiple of HashLen (32), testing truncation.
subtest 'RFC 5869 Test Case 1: basic SHA-256' => sub {
    my $ikm  = h('0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b');
    my $salt = h('000102030405060708090a0b0c');
    my $info = h('f0f1f2f3f4f5f6f7f8f9');
    my $expected_prk = '077709362c2e32df0ddc3f0dc47bba6390b6c73bb50f9c3122ec844ad7c2b3e5';
    my $expected_okm = '3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf34007208d5b887185865';

    # Extract
    my $prk = hkdf_extract($salt, $ikm, 'sha256');
    is(to_hex($prk), $expected_prk, 'extract produces correct PRK');

    # Expand
    my $okm = hkdf_expand(h($expected_prk), $info, 42, 'sha256');
    is(to_hex($okm), $expected_okm, 'expand produces correct OKM');

    # Combined
    my $okm2 = hkdf($salt, $ikm, $info, 42, 'sha256');
    is(to_hex($okm2), $expected_okm, 'combined hkdf produces correct OKM');

    # Hex variants
    is(hkdf_extract_hex($salt, $ikm, 'sha256'), $expected_prk, 'extract_hex works');
    is(hkdf_hex($salt, $ikm, $info, 42, 'sha256'), $expected_okm, 'hkdf_hex works');
};

# Test Case 2: Longer inputs — 80-byte IKM, salt, and info.
# L = 82 requires ceil(82/32) = 3 HMAC iterations in the expand phase.
subtest 'RFC 5869 Test Case 2: longer inputs' => sub {
    my $ikm  = h('000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f404142434445464748494a4b4c4d4e4f');
    my $salt = h('606162636465666768696a6b6c6d6e6f707172737475767778797a7b7c7d7e7f808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9fa0a1a2a3a4a5a6a7a8a9aaabacadaeaf');
    my $info = h('b0b1b2b3b4b5b6b7b8b9babbbcbdbebfc0c1c2c3c4c5c6c7c8c9cacbcccdcecfd0d1d2d3d4d5d6d7d8d9dadbdcdddedfe0e1e2e3e4e5e6e7e8e9eaebecedeeeff0f1f2f3f4f5f6f7f8f9fafbfcfdfeff');
    my $expected_prk = '06a6b88c5853361a06104c9ceb35b45cef760014904671014a193f40c15fc244';
    my $expected_okm = 'b11e398dc80327a1c8e7f78c596a49344f012eda2d4efad8a050cc4c19afa97c59045a99cac7827271cb41c65e590e09da3275600c2f09b8367793a9aca3db71cc30c58179ec3e87c14c01d5c1f3434f1d87';

    my $prk = hkdf_extract($salt, $ikm, 'sha256');
    is(to_hex($prk), $expected_prk, 'extract produces correct PRK');

    my $okm = hkdf_expand(h($expected_prk), $info, 82, 'sha256');
    is(to_hex($okm), $expected_okm, 'expand produces correct OKM');

    my $okm2 = hkdf($salt, $ikm, $info, 82, 'sha256');
    is(to_hex($okm2), $expected_okm, 'combined hkdf produces correct OKM');
};

# Test Case 3: Empty salt and empty info.
# When salt is empty, HKDF uses HashLen (32) zero bytes as the HMAC key.
# When info is empty, the expand loop appends only the counter byte.
subtest 'RFC 5869 Test Case 3: empty salt and info' => sub {
    my $ikm  = h('0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b');
    my $salt = '';
    my $info = '';
    my $expected_prk = '19ef24a32c717b167f33a91d6f648bdf96596776afdb6377ac434c1c293ccb04';
    my $expected_okm = '8da4e775a563c18f715f802a063c5a31b8a11f5c5ee1879ec3454e5f3c738d2d9d201395faa4b61a96c8';

    my $prk = hkdf_extract($salt, $ikm, 'sha256');
    is(to_hex($prk), $expected_prk, 'extract produces correct PRK');

    my $okm = hkdf_expand(h($expected_prk), $info, 42, 'sha256');
    is(to_hex($okm), $expected_okm, 'expand produces correct OKM');

    my $okm2 = hkdf($salt, $ikm, $info, 42, 'sha256');
    is(to_hex($okm2), $expected_okm, 'combined hkdf produces correct OKM');
};

# ============================================================================
# Edge Cases
# ============================================================================

subtest 'default hash is sha256' => sub {
    # When hash parameter is omitted, should default to SHA-256.
    my $ikm  = h('0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b');
    my $salt = h('000102030405060708090a0b0c');
    my $info = h('f0f1f2f3f4f5f6f7f8f9');
    my $expected_okm = '3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf34007208d5b887185865';

    my $okm = hkdf($salt, $ikm, $info, 42);
    is(to_hex($okm), $expected_okm, 'default hash produces TC1 OKM');
};

subtest 'expand rejects length <= 0' => sub {
    my $prk = "\x01" x 32;
    like(
        dies { hkdf_expand($prk, '', 0, 'sha256') },
        qr/length must be > 0/,
        'dies on length = 0'
    );
};

subtest 'expand rejects length > 255 * HashLen' => sub {
    my $prk = "\x01" x 32;
    # SHA-256: max = 255 * 32 = 8160
    like(
        dies { hkdf_expand($prk, '', 8161, 'sha256') },
        qr/exceeds maximum/,
        'dies on length = 8161'
    );
};

subtest 'expand allows length = 255 * HashLen exactly' => sub {
    my $prk = "\x01" x 32;
    my $okm = hkdf_expand($prk, '', 8160, 'sha256');
    is(length($okm), 8160, 'produces exactly 8160 bytes');
};

subtest 'expand with length = 1' => sub {
    my $prk = "\x01" x 32;
    my $okm = hkdf_expand($prk, '', 1, 'sha256');
    is(length($okm), 1, 'produces exactly 1 byte');
};

subtest 'expand with length = HashLen' => sub {
    my $prk = "\x01" x 32;
    my $okm = hkdf_expand($prk, 'test', 32, 'sha256');
    is(length($okm), 32, 'produces exactly 32 bytes');
};

subtest 'rejects unsupported hash algorithm' => sub {
    like(
        dies { hkdf_extract('salt', 'ikm', 'md5') },
        qr/unsupported/,
        'dies on md5'
    );
};

subtest 'SHA-512 extract and expand' => sub {
    my $ikm  = h('0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b');
    my $salt = h('000102030405060708090a0b0c');

    my $prk = hkdf_extract($salt, $ikm, 'sha512');
    is(length($prk), 64, 'SHA-512 PRK is 64 bytes');

    my $okm = hkdf_expand($prk, 'info', 64, 'sha512');
    is(length($okm), 64, 'SHA-512 OKM is 64 bytes');
};

subtest 'undef salt treated as empty' => sub {
    my $ikm = h('0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b');
    my $expected_prk = '19ef24a32c717b167f33a91d6f648bdf96596776afdb6377ac434c1c293ccb04';

    my $prk = hkdf_extract(undef, $ikm, 'sha256');
    is(to_hex($prk), $expected_prk, 'undef salt matches TC3 PRK');
};

done_testing;
