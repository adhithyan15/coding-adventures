use strict;
use warnings;
use Test2::V0;

use CodingAdventures::DES;

# Convenience: decode hex string to bytes
sub h { pack('H*', $_[0]) }

# ============================================================================
# DES Encrypt Block
# ============================================================================

subtest 'des_encrypt_block — FIPS known-answer tests' => sub {
    is(
        CodingAdventures::DES::des_encrypt_block(h('0123456789ABCDEF'), h('133457799BBCDFF1')),
        h('85E813540F0AB405'),
        'Stallings/FIPS 46 worked example'
    );

    is(
        CodingAdventures::DES::des_encrypt_block(h('95F8A5E5DD31D900'), h('0101010101010101')),
        h('8000000000000000'),
        'SP 800-20 Table 1 row 0 — plaintext variable'
    );

    is(
        CodingAdventures::DES::des_encrypt_block(h('DD7F121CA5015619'), h('0101010101010101')),
        h('4000000000000000'),
        'SP 800-20 Table 1 row 1'
    );

    is(
        CodingAdventures::DES::des_encrypt_block(h('0000000000000000'), h('8001010101010101')),
        h('95A8D72813DAA94D'),
        'SP 800-20 Table 2 row 0 — key variable'
    );

    is(
        CodingAdventures::DES::des_encrypt_block(h('0000000000000000'), h('4001010101010101')),
        h('0EEC1487DD8C26D5'),
        'SP 800-20 Table 2 row 1'
    );

    is(length(CodingAdventures::DES::des_encrypt_block(h('0000000000000000'), h('0101010101010101'))),
        8, 'encrypt returns 8 bytes');
};

# ============================================================================
# DES Decrypt Block
# ============================================================================

subtest 'des_decrypt_block' => sub {
    is(
        CodingAdventures::DES::des_decrypt_block(h('85E813540F0AB405'), h('133457799BBCDFF1')),
        h('0123456789ABCDEF'),
        'decrypt FIPS vector 1'
    );

    for my $key_hex (qw(133457799BBCDFF0 FFFFFFFFFFFFFFFF 0000000000000000 FEDCBA9876543210)) {
        my $key   = h($key_hex);
        my $plain = h('0123456789ABCDEF');
        my $ct    = CodingAdventures::DES::des_encrypt_block($plain, $key);
        is(CodingAdventures::DES::des_decrypt_block($ct, $key), $plain,
            "round-trip key=$key_hex");
    }
};

# ============================================================================
# expand_key
# ============================================================================

subtest 'expand_key' => sub {
    my $subkeys = CodingAdventures::DES::expand_key(h('0133457799BBCDFF'));
    is(scalar @$subkeys, 16, 'returns 16 subkeys');
    for my $sk (@$subkeys) {
        is(length($sk), 6, 'each subkey is 6 bytes');
    }

    my $sk1 = CodingAdventures::DES::expand_key(h('0133457799BBCDFF'));
    my $sk2 = CodingAdventures::DES::expand_key(h('FEDCBA9876543210'));
    isnt($sk1->[0], $sk2->[0], 'different keys produce different subkeys');

    ok(dies { CodingAdventures::DES::expand_key(h('0102030405')) },
        'raises on wrong key size (5 bytes)');
    ok(dies { CodingAdventures::DES::expand_key(h('010203040506070809')) },
        'raises on wrong key size (9 bytes)');
};

# ============================================================================
# ECB Mode
# ============================================================================

subtest 'des_ecb_encrypt and des_ecb_decrypt' => sub {
    my $KEY = h('0133457799BBCDFF');

    is(length(CodingAdventures::DES::des_ecb_encrypt(h('0123456789ABCDEF'), $KEY)),
        16, '8-byte input → 16 bytes ciphertext');

    is(length(CodingAdventures::DES::des_ecb_encrypt('hello', $KEY)),
        8, 'sub-block → 8 bytes');

    is(length(CodingAdventures::DES::des_ecb_encrypt("\x00" x 16, $KEY)),
        24, '16-byte input → 24 bytes');

    is(length(CodingAdventures::DES::des_ecb_encrypt('', $KEY)),
        8, 'empty → 8 bytes (full padding block)');

    for my $plain ('hello', 'ABCDEFGH', 'The quick brown fox jumps', '') {
        my $ct = CodingAdventures::DES::des_ecb_encrypt($plain, $KEY);
        is(CodingAdventures::DES::des_ecb_decrypt($ct, $KEY), $plain,
            "round-trip: '$plain'");
    }

    ok(dies { CodingAdventures::DES::des_ecb_decrypt('', $KEY) },
        'decrypt raises on empty');
    ok(dies { CodingAdventures::DES::des_ecb_decrypt('1234567', $KEY) },
        'decrypt raises on non-multiple of 8');
};

# ============================================================================
# Invalid Inputs
# ============================================================================

subtest 'invalid inputs' => sub {
    my $KEY = h('0133457799BBCDFF');

    ok(dies { CodingAdventures::DES::des_encrypt_block(h('0102030405060708FF'), $KEY) },
        'encrypt raises on wrong block size (9 bytes)');

    ok(dies { CodingAdventures::DES::des_encrypt_block(h('01020304050607'), $KEY) },
        'encrypt raises on wrong block size (7 bytes)');

    ok(dies { CodingAdventures::DES::des_encrypt_block(h('0102030405060708'), h('01020304')) },
        'encrypt raises on wrong key size');
};

# ============================================================================
# 3DES / TDEA
# ============================================================================

subtest 'tdea_encrypt_block and tdea_decrypt_block' => sub {
    my $K1     = h('0123456789ABCDEF');
    my $K2     = h('23456789ABCDEF01');
    my $K3     = h('456789ABCDEF0123');
    my $PLAIN  = h('6BC1BEE22E409F96');
    my $CIPHER = h('3B6423D418DEFC23');

    is(CodingAdventures::DES::tdea_encrypt_block($PLAIN, $K1, $K2, $K3),
        $CIPHER, 'TDEA encrypt — NIST SP 800-67 EDE vector');

    is(CodingAdventures::DES::tdea_decrypt_block($CIPHER, $K1, $K2, $K3),
        $PLAIN, 'TDEA decrypt');

    # Round-trip
    my $k1 = h('FEDCBA9876543210');
    my $k2 = h('0F1E2D3C4B5A6978');
    my $k3 = h('7869584A3B2C1D0E');
    my $plain = h('0123456789ABCDEF');
    my $ct = CodingAdventures::DES::tdea_encrypt_block($plain, $k1, $k2, $k3);
    is(CodingAdventures::DES::tdea_decrypt_block($ct, $k1, $k2, $k3),
        $plain, 'TDEA round-trip');

    # Backward compatibility: K1=K2=K3 → single DES
    my $key = h('0133457799BBCDFF');
    is(
        CodingAdventures::DES::tdea_encrypt_block($plain, $key, $key, $key),
        CodingAdventures::DES::des_encrypt_block($plain, $key),
        'K1=K2=K3 reduces to single DES'
    );

    is(
        CodingAdventures::DES::tdea_decrypt_block($plain, $key, $key, $key),
        CodingAdventures::DES::des_decrypt_block($plain, $key),
        'K1=K2=K3 decrypt reduces to single DES decrypt'
    );

    # All-same-byte blocks
    for my $val (0x00, 0xFF, 0xA5, 0x5A) {
        my $p = chr($val) x 8;
        my $c = CodingAdventures::DES::tdea_encrypt_block($p, $k1, $k2, $k3);
        is(CodingAdventures::DES::tdea_decrypt_block($c, $k1, $k2, $k3), $p,
            sprintf('TDEA round-trip val=%02X', $val));
    }
};

done_testing;
