use strict;
use warnings;
use Test2::V0;

use lib '../aes/lib';
use CodingAdventures::AESModes;

# Convenience: decode hex string to bytes
sub h { pack('H*', $_[0]) }
sub to_hex { unpack('H*', $_[0]) }

# ============================================================================
# PKCS#7 Padding
# ============================================================================

subtest 'pkcs7_pad — empty input' => sub {
    my $padded = CodingAdventures::AESModes::pkcs7_pad('');
    is(length($padded), 16, 'empty input pads to 16 bytes');
    is(ord(substr($padded, -1)), 16, 'pad value is 16');
};

subtest 'pkcs7_pad — 5 bytes' => sub {
    my $padded = CodingAdventures::AESModes::pkcs7_pad('HELLO');
    is(length($padded), 16, 'pads to 16');
    is(substr($padded, 0, 5), 'HELLO', 'data preserved');
    is(ord(substr($padded, -1)), 11, 'pad value is 11');
};

subtest 'pkcs7_pad — aligned input' => sub {
    my $padded = CodingAdventures::AESModes::pkcs7_pad('A' x 16);
    is(length($padded), 32, 'aligned input gets full padding block');
};

subtest 'pkcs7 round-trip' => sub {
    for my $len (0..48) {
        my $data = 'X' x $len;
        my $result = CodingAdventures::AESModes::pkcs7_unpad(
            CodingAdventures::AESModes::pkcs7_pad($data));
        is($result, $data, "round-trip length $len");
    }
};

subtest 'pkcs7_unpad — rejects invalid padding' => sub {
    like(dies { CodingAdventures::AESModes::pkcs7_unpad("\0" x 16) },
        qr/Invalid PKCS#7 padding/, 'rejects zero pad value');
    like(dies { CodingAdventures::AESModes::pkcs7_unpad('A' x 13 . "\x01\x01\x03") },
        qr/Invalid PKCS#7 padding/, 'rejects inconsistent padding');
};

# ============================================================================
# ECB Mode — NIST SP 800-38A
# ============================================================================

subtest 'ECB encrypt — NIST vector block 1' => sub {
    my $key = h('2b7e151628aed2a6abf7158809cf4f3c');
    my $pt  = h('6bc1bee22e409f96e93d7e117393172a');
    my $ct  = CodingAdventures::AESModes::ecb_encrypt($pt, $key);
    is(length($ct), 32, 'padded to 32 bytes');
    is(to_hex(substr($ct, 0, 16)), '3ad77bb40d7a3660a89ecaf32466ef97', 'first block matches NIST');
};

subtest 'ECB round-trip' => sub {
    my $key = h('2b7e151628aed2a6abf7158809cf4f3c');
    for my $len (0, 1, 15, 16, 17, 31, 32, 100) {
        my $pt = 'Z' x $len;
        my $ct = CodingAdventures::AESModes::ecb_encrypt($pt, $key);
        is(CodingAdventures::AESModes::ecb_decrypt($ct, $key), $pt, "round-trip length $len");
    }
};

subtest 'ECB identical blocks produce identical ciphertext' => sub {
    my $key = h('2b7e151628aed2a6abf7158809cf4f3c');
    my $block = h('6bc1bee22e409f96e93d7e117393172a');
    my $ct = CodingAdventures::AESModes::ecb_encrypt($block . $block, $key);
    is(to_hex(substr($ct, 0, 16)), to_hex(substr($ct, 16, 16)),
        'identical blocks => identical ciphertext (ECB weakness)');
};

# ============================================================================
# CBC Mode — NIST SP 800-38A
# ============================================================================

subtest 'CBC encrypt — NIST vector block 1' => sub {
    my $key = h('2b7e151628aed2a6abf7158809cf4f3c');
    my $iv  = h('000102030405060708090a0b0c0d0e0f');
    my $pt  = h('6bc1bee22e409f96e93d7e117393172a');
    my $ct  = CodingAdventures::AESModes::cbc_encrypt($pt, $key, $iv);
    is(to_hex(substr($ct, 0, 16)), '7649abac8119b246cee98e9b12e9197d',
        'first block matches NIST');
};

subtest 'CBC multi-block encrypt' => sub {
    my $key = h('2b7e151628aed2a6abf7158809cf4f3c');
    my $iv  = h('000102030405060708090a0b0c0d0e0f');
    my $pt  = h('6bc1bee22e409f96e93d7e117393172a'
              . 'ae2d8a571e03ac9c9eb76fac45af8e51'
              . '30c81c46a35ce411e5fbc1191a0a52ef'
              . 'f69f2445df4f9b17ad2b417be66c3710');
    my $ct = CodingAdventures::AESModes::cbc_encrypt($pt, $key, $iv);
    is(to_hex(substr($ct, 0, 16)), '7649abac8119b246cee98e9b12e9197d',
        'block 1 matches');
    my $recovered = CodingAdventures::AESModes::cbc_decrypt($ct, $key, $iv);
    is(to_hex($recovered), to_hex($pt), 'round-trip multi-block');
};

subtest 'CBC round-trip various lengths' => sub {
    my $key = h('2b7e151628aed2a6abf7158809cf4f3c');
    my $iv  = h('000102030405060708090a0b0c0d0e0f');
    for my $len (0, 1, 15, 16, 17, 31, 32, 100) {
        my $pt = 'Q' x $len;
        my $ct = CodingAdventures::AESModes::cbc_encrypt($pt, $key, $iv);
        is(CodingAdventures::AESModes::cbc_decrypt($ct, $key, $iv), $pt,
            "round-trip length $len");
    }
};

subtest 'CBC identical blocks produce different ciphertext' => sub {
    my $key = h('2b7e151628aed2a6abf7158809cf4f3c');
    my $iv  = h('000102030405060708090a0b0c0d0e0f');
    my $block = h('6bc1bee22e409f96e93d7e117393172a');
    my $ct = CodingAdventures::AESModes::cbc_encrypt($block . $block, $key, $iv);
    isnt(to_hex(substr($ct, 0, 16)), to_hex(substr($ct, 16, 16)),
        'identical blocks => different ciphertext (CBC property)');
};

# ============================================================================
# CTR Mode
# ============================================================================

subtest 'CTR round-trip single block' => sub {
    my $key   = h('2b7e151628aed2a6abf7158809cf4f3c');
    my $nonce = h('f0f1f2f3f4f5f6f7f8f9fafb');
    my $pt    = h('6bc1bee22e409f96e93d7e117393172a');
    my $ct    = CodingAdventures::AESModes::ctr_encrypt($pt, $key, $nonce);
    is(length($ct), 16, 'no padding in CTR');
    is(CodingAdventures::AESModes::ctr_decrypt($ct, $key, $nonce), $pt, 'round-trip');
};

subtest 'CTR handles partial blocks' => sub {
    my $key   = h('2b7e151628aed2a6abf7158809cf4f3c');
    my $nonce = h('aabbccddeeff00112233aabb');
    my $pt    = 'Short';
    my $ct    = CodingAdventures::AESModes::ctr_encrypt($pt, $key, $nonce);
    is(length($ct), 5, 'ciphertext same length as plaintext');
    is(CodingAdventures::AESModes::ctr_decrypt($ct, $key, $nonce), $pt, 'round-trip');
};

subtest 'CTR round-trip various lengths' => sub {
    my $key   = h('2b7e151628aed2a6abf7158809cf4f3c');
    my $nonce = h('112233445566778899aabbcc');
    for my $len (0, 1, 15, 16, 17, 31, 32, 100) {
        my $pt = 'C' x $len;
        my $ct = CodingAdventures::AESModes::ctr_encrypt($pt, $key, $nonce);
        is(length($ct), $len, "ciphertext length $len");
        is(CodingAdventures::AESModes::ctr_decrypt($ct, $key, $nonce), $pt,
            "round-trip length $len");
    }
};

# ============================================================================
# GCM Mode — NIST test vectors
# ============================================================================

subtest 'GCM Test Case 2: empty plaintext' => sub {
    my $key = h('00000000000000000000000000000000');
    my $iv  = h('000000000000000000000000');
    my ($ct, $tag) = CodingAdventures::AESModes::gcm_encrypt('', $key, $iv, '');
    is(length($ct), 0, 'empty ciphertext');
    is(to_hex($tag), '58e2fccefa7e3061367f1d57a4e7455a', 'tag matches NIST');
    my ($recovered, $err) = CodingAdventures::AESModes::gcm_decrypt($ct, $key, $iv, '', $tag);
    is($err, undef, 'no error');
    is($recovered, '', 'recovered empty plaintext');
};

subtest 'GCM Test Case 3: 64-byte plaintext, no AAD' => sub {
    my $key = h('feffe9928665731c6d6a8f9467308308');
    my $iv  = h('cafebabefacedbaddecaf888');
    my $pt  = h('d9313225f88406e5a55909c5aff5269a'
              . '86a7a9531534f7da2e4c303d8a318a72'
              . '1c3c0c95956809532fcf0e2449a6b525'
              . 'b16aedf5aa0de657ba637b391aafd255');
    my ($ct, $tag) = CodingAdventures::AESModes::gcm_encrypt($pt, $key, $iv, '');
    is(to_hex($ct),
        '42831ec2217774244b7221b784d0d49c'
      . 'e3aa212f2c02a4e035c17e2329aca12e'
      . '21d514b25466931c7d8f6a5aac84aa05'
      . '1ba30b396a0aac973d58e091473f5985',
        'ciphertext matches NIST');
    is(to_hex($tag), '4d5c2af327cd64a62cf35abd2ba6fab4', 'tag matches NIST');
    my ($recovered, $err) = CodingAdventures::AESModes::gcm_decrypt($ct, $key, $iv, '', $tag);
    is($err, undef, 'no error');
    is(to_hex($recovered), to_hex($pt), 'round-trip');
};

subtest 'GCM Test Case 4: plaintext with AAD' => sub {
    my $key = h('feffe9928665731c6d6a8f9467308308');
    my $iv  = h('cafebabefacedbaddecaf888');
    my $pt  = h('d9313225f88406e5a55909c5aff5269a'
              . '86a7a9531534f7da2e4c303d8a318a72'
              . '1c3c0c95956809532fcf0e2449a6b525'
              . 'b16aedf5aa0de657ba637b39');
    my $aad = h('feedfacedeadbeeffeedfacedeadbeef'
              . 'abaddad2');
    my ($ct, $tag) = CodingAdventures::AESModes::gcm_encrypt($pt, $key, $iv, $aad);
    is(to_hex($ct),
        '42831ec2217774244b7221b784d0d49c'
      . 'e3aa212f2c02a4e035c17e2329aca12e'
      . '21d514b25466931c7d8f6a5aac84aa05'
      . '1ba30b396a0aac973d58e091',
        'ciphertext matches NIST');
    is(to_hex($tag), '5bc94fbc3221a5db94fae95ae7121a47', 'tag matches NIST');
    my ($recovered, $err) = CodingAdventures::AESModes::gcm_decrypt($ct, $key, $iv, $aad, $tag);
    is($err, undef, 'no error');
    is(to_hex($recovered), to_hex($pt), 'round-trip');
};

subtest 'GCM rejects tampered ciphertext' => sub {
    my $key = h('feffe9928665731c6d6a8f9467308308');
    my $iv  = h('cafebabefacedbaddecaf888');
    my $pt  = h('d9313225f88406e5a55909c5aff5269a');
    my ($ct, $tag) = CodingAdventures::AESModes::gcm_encrypt($pt, $key, $iv, '');
    my $tampered = chr(ord(substr($ct, 0, 1)) ^ 1) . substr($ct, 1);
    my ($result, $err) = CodingAdventures::AESModes::gcm_decrypt($tampered, $key, $iv, '', $tag);
    is($result, undef, 'returns undef for tampered ct');
    like($err, qr/mismatch/, 'error message');
};

subtest 'GCM rejects tampered tag' => sub {
    my $key = h('feffe9928665731c6d6a8f9467308308');
    my $iv  = h('cafebabefacedbaddecaf888');
    my $pt  = h('d9313225f88406e5a55909c5aff5269a');
    my ($ct, $tag) = CodingAdventures::AESModes::gcm_encrypt($pt, $key, $iv, '');
    my $bad_tag = chr(ord(substr($tag, 0, 1)) ^ 1) . substr($tag, 1);
    my ($result, $err) = CodingAdventures::AESModes::gcm_decrypt($ct, $key, $iv, '', $bad_tag);
    is($result, undef, 'returns undef for bad tag');
    like($err, qr/mismatch/, 'error message');
};

subtest 'GCM rejects tampered AAD' => sub {
    my $key = h('feffe9928665731c6d6a8f9467308308');
    my $iv  = h('cafebabefacedbaddecaf888');
    my ($ct, $tag) = CodingAdventures::AESModes::gcm_encrypt('test', $key, $iv, 'authentic');
    my ($result, $err) = CodingAdventures::AESModes::gcm_decrypt($ct, $key, $iv, 'tampered', $tag);
    is($result, undef, 'returns undef for tampered AAD');
};

subtest 'GCM round-trip various lengths' => sub {
    my $key = h('feffe9928665731c6d6a8f9467308308');
    my $iv  = h('cafebabefacedbaddecaf888');
    for my $len (1, 15, 16, 17, 31, 32, 100) {
        my $pt = 'G' x $len;
        my ($ct, $tag) = CodingAdventures::AESModes::gcm_encrypt($pt, $key, $iv, 'aad');
        my ($recovered, $err) = CodingAdventures::AESModes::gcm_decrypt($ct, $key, $iv, 'aad', $tag);
        is($err, undef, "no error for length $len");
        is($recovered, $pt, "round-trip length $len");
    }
};

subtest 'GCM empty plaintext with AAD' => sub {
    my $key = h('feffe9928665731c6d6a8f9467308308');
    my $iv  = h('cafebabefacedbaddecaf888');
    my $aad = 'authenticate this';
    my ($ct, $tag) = CodingAdventures::AESModes::gcm_encrypt('', $key, $iv, $aad);
    is(length($ct), 0, 'empty ct');
    my ($recovered, $err) = CodingAdventures::AESModes::gcm_decrypt($ct, $key, $iv, $aad, $tag);
    is($err, undef, 'no error');
    is($recovered, '', 'recovered empty');
};

done_testing;
