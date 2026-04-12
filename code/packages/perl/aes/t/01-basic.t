use strict;
use warnings;
use Test2::V0;

use CodingAdventures::AES;

# Convenience: decode hex string to bytes
sub h { pack('H*', $_[0]) }
sub to_hex { unpack('H*', $_[0]) }

# ============================================================================
# FIPS 197 Appendix B — AES-128
# ============================================================================

subtest 'aes_encrypt_block — AES-128 FIPS 197 Appendix B' => sub {
    my $key   = h('2b7e151628aed2a6abf7158809cf4f3c');
    my $plain = h('3243f6a8885a308d313198a2e0370734');
    my $ct    = h('3925841d02dc09fbdc118597196a0b32');

    is(
        CodingAdventures::AES::aes_encrypt_block($plain, $key),
        $ct,
        'FIPS 197 Appendix B AES-128 encrypt'
    );
};

subtest 'aes_decrypt_block — AES-128 FIPS 197 Appendix B' => sub {
    my $key   = h('2b7e151628aed2a6abf7158809cf4f3c');
    my $plain = h('3243f6a8885a308d313198a2e0370734');
    my $ct    = h('3925841d02dc09fbdc118597196a0b32');

    is(
        CodingAdventures::AES::aes_decrypt_block($ct, $key),
        $plain,
        'FIPS 197 Appendix B AES-128 decrypt'
    );
};

# ============================================================================
# FIPS 197 Appendix C.1 — AES-128 sequential key
# ============================================================================

subtest 'AES-128 Appendix C.1' => sub {
    my $key   = h('000102030405060708090a0b0c0d0e0f');
    my $plain = h('00112233445566778899aabbccddeeff');
    my $ct    = h('69c4e0d86a7b0430d8cdb78070b4c55a');

    is(CodingAdventures::AES::aes_encrypt_block($plain, $key), $ct, 'C.1 encrypt');
    is(CodingAdventures::AES::aes_decrypt_block($ct, $key), $plain, 'C.1 decrypt');
};

# ============================================================================
# FIPS 197 Appendix C.2 — AES-192
# ============================================================================

subtest 'aes_encrypt_block — AES-192 FIPS 197 Appendix C.2' => sub {
    my $key   = h('000102030405060708090a0b0c0d0e0f1011121314151617');
    my $plain = h('00112233445566778899aabbccddeeff');
    my $ct    = h('dda97ca4864cdfe06eaf70a0ec0d7191');

    is(CodingAdventures::AES::aes_encrypt_block($plain, $key), $ct, 'C.2 AES-192 encrypt');
    is(CodingAdventures::AES::aes_decrypt_block($ct, $key), $plain, 'C.2 AES-192 decrypt');
};

# ============================================================================
# FIPS 197 Appendix C.3 — AES-256
# ============================================================================

subtest 'aes_encrypt_block — AES-256 FIPS 197 Appendix C.3' => sub {
    my $key   = h('000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f');
    my $plain = h('00112233445566778899aabbccddeeff');
    my $ct    = h('8ea2b7ca516745bfeafc49904b496089');

    is(CodingAdventures::AES::aes_encrypt_block($plain, $key), $ct, 'C.3 AES-256 encrypt');
    is(CodingAdventures::AES::aes_decrypt_block($ct, $key), $plain, 'C.3 AES-256 decrypt');
};

subtest 'AES-256 SE01 spec vector' => sub {
    my $key   = h('603deb1015ca71be2b73aef0857d77811f352c073b6108d72d9810a30914dff4');
    my $plain = h('6bc1bee22e409f96e93d7e117393172a');
    my $ct    = h('f3eed1bdb5d2a03c064b5a7e3db181f8');

    is(CodingAdventures::AES::aes_encrypt_block($plain, $key), $ct, 'AES-256 SE01 encrypt');
    is(CodingAdventures::AES::aes_decrypt_block($ct, $key), $plain, 'AES-256 SE01 decrypt');
};

# ============================================================================
# S-box properties (accessed via sbox()/inv_sbox() accessors)
# ============================================================================

subtest 'SBOX is a bijection' => sub {
    my @sbox = @{CodingAdventures::AES::sbox()};
    is(scalar @sbox, 256, 'SBOX has 256 elements');
    my %seen;
    $seen{$_}++ for @sbox;
    is(scalar keys %seen, 256, 'SBOX has 256 distinct values');
};

subtest 'INV_SBOX is inverse of SBOX' => sub {
    my @sbox     = @{CodingAdventures::AES::sbox()};
    my @inv_sbox = @{CodingAdventures::AES::inv_sbox()};
    for my $b (0..255) {
        is($inv_sbox[$sbox[$b]], $b, "INV_SBOX[SBOX[$b]] == $b");
    }
};

subtest 'SBOX known values from FIPS 197 Figure 7' => sub {
    my @sbox = @{CodingAdventures::AES::sbox()};
    is($sbox[0x00], 0x63, 'SBOX[0x00] = 0x63');
    is($sbox[0x01], 0x7c, 'SBOX[0x01] = 0x7c');
    is($sbox[0xff], 0x16, 'SBOX[0xff] = 0x16');
    is($sbox[0x53], 0xed, 'SBOX[0x53] = 0xed');
};

subtest 'SBOX has no fixed points' => sub {
    my @sbox = @{CodingAdventures::AES::sbox()};
    for my $b (0..255) {
        isnt($sbox[$b], $b, "SBOX[$b] != $b (no fixed point)");
    }
};

# ============================================================================
# Key schedule
# ============================================================================

subtest 'expand_key — round count' => sub {
    my $rks128 = CodingAdventures::AES::expand_key(h('2b7e151628aed2a6abf7158809cf4f3c'));
    is(scalar @$rks128, 11, 'AES-128 produces 11 round keys');

    my $rks192 = CodingAdventures::AES::expand_key(h('000102030405060708090a0b0c0d0e0f1011121314151617'));
    is(scalar @$rks192, 13, 'AES-192 produces 13 round keys');

    my $rks256 = CodingAdventures::AES::expand_key(h('000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f'));
    is(scalar @$rks256, 15, 'AES-256 produces 15 round keys');
};

# ============================================================================
# Round-trip tests
# ============================================================================

subtest 'round-trip AES-128' => sub {
    my $key = h('fedcba9876543210fedcba9876543210');
    for my $start (0, 32, 64, 128, 192, 224) {
        my $plain = pack('C16', map { ($start + $_) % 256 } 0..15);
        my $ct = CodingAdventures::AES::aes_encrypt_block($plain, $key);
        is(CodingAdventures::AES::aes_decrypt_block($ct, $key), $plain, "round-trip start=$start");
    }
};

subtest 'round-trip AES-192' => sub {
    my $key = h('000102030405060708090a0b0c0d0e0f1011121314151617');
    my $plain = h('deadbeefcafebabe0123456789abcdef');
    my $ct = CodingAdventures::AES::aes_encrypt_block($plain, $key);
    is(CodingAdventures::AES::aes_decrypt_block($ct, $key), $plain, 'AES-192 round-trip');
};

subtest 'round-trip AES-256' => sub {
    my $key = h('000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f');
    my $plain = h('00000000000000000000000000000000');
    my $ct = CodingAdventures::AES::aes_encrypt_block($plain, $key);
    is(CodingAdventures::AES::aes_decrypt_block($ct, $key), $plain, 'AES-256 all-zeros round-trip');
};

# ============================================================================
# Error handling
# ============================================================================

subtest 'invalid block length' => sub {
    my $key = h('2b7e151628aed2a6abf7158809cf4f3c');
    eval { CodingAdventures::AES::aes_encrypt_block(h('0011223344'), $key) };
    like($@, qr/16 bytes/, 'encrypt rejects wrong block size');

    eval { CodingAdventures::AES::aes_decrypt_block(h('001122334455667788990011223344'), $key) };
    like($@, qr/16 bytes/, 'decrypt rejects wrong block size');
};

subtest 'invalid key length' => sub {
    my $block = h('00112233445566778899aabbccddeeff');
    eval { CodingAdventures::AES::aes_encrypt_block($block, h('0102030405060708090a0b0c0d0e')) };
    like($@, qr/key must be/, 'encrypt rejects wrong key size');

    eval { CodingAdventures::AES::aes_decrypt_block($block, h('0102030405060708090a0b0c0d0e')) };
    like($@, qr/key must be/, 'decrypt rejects wrong key size');
};

done_testing;
