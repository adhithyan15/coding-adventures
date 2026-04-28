use strict;
use warnings;
use Test2::V0;
use lib 'lib';
use CodingAdventures::ChaCha20Poly1305;

# Convenience: decode hex string to binary bytes
sub h { pack('H*', shift) }

my $C = 'CodingAdventures::ChaCha20Poly1305';

# ============================================================================
# ChaCha20 Block Function — RFC 8439 §2.1.2
# ============================================================================
# The test vector from RFC 8439 §2.1.2 verifies that the core block function
# (20 rounds of quarter-rounds) produces the correct 64-byte output.
#
# Test input:
#   key    = 0x00..0x1f (bytes 0..31)
#   nonce  = 0x00000009 0x0000004a 0x00000000 (3 LE words)
#   counter= 1

subtest 'ChaCha20 block function — RFC 8439 §2.1.2' => sub {
    my $key   = pack('C32', 0..31);
    my $nonce = h('000000090000004a00000000');
    my $out   = $C->chacha20_block($key, 1, $nonce);

    is(
        $out,
        h('10f1e7e4d13b5915500fdd1fa32071c4'
         .'c7d1f4c733c068030422aa9ac3d46c4e'
         .'d2826446079faa0914c2d705d98b02a2'
         .'b5129cd1de164eb9cbd083e8a2503c4e'),
        'ChaCha20 block matches RFC 8439 §2.1.2'
    );
    is(length($out), 64, 'Block output is exactly 64 bytes');
};

# ============================================================================
# ChaCha20 Block — different counter values produce different outputs
# ============================================================================

subtest 'ChaCha20 block — counter changes output' => sub {
    my $key   = pack('C32', 0..31);
    my $nonce = h('000000090000004a00000000');

    my $block0 = $C->chacha20_block($key, 0, $nonce);
    my $block1 = $C->chacha20_block($key, 1, $nonce);
    my $block2 = $C->chacha20_block($key, 2, $nonce);

    isnt($block0, $block1, 'Counter 0 != counter 1');
    isnt($block1, $block2, 'Counter 1 != counter 2');
    isnt($block0, $block2, 'Counter 0 != counter 2');
};

# ============================================================================
# ChaCha20 Encrypt — RFC 8439 §2.4.2
# ============================================================================
# RFC 8439 §2.4.2 provides a test vector for encrypting the Sunscreen message.

subtest 'ChaCha20 encrypt — RFC 8439 §2.4.2 Sunscreen' => sub {
    my $key   = h('000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f');
    my $nonce = h('000000000000004a00000000');
    my $pt    = 'Ladies and Gentlemen of the class of \'99: If I could offer you only one tip for the future, sunscreen would be it.';
    my $ct_hex = '6e2e359a2568f98041ba0728dd0d6981'
               . 'e97e7aec1d4360c20a27afccfd9fae0b'
               . 'f91b65c5524733ab8f593dabcd62b357'
               . '1639d624e65152ab8f530c359f0861d8'
               . '07ca0dbf500d6a6156a38e088a22b65e'
               . '52bc514d16ccf806818ce91ab7793736'
               . '5af90bbf74a35be6b40b8eedf2785e42'
               . '874d';

    my $ct = $C->chacha20_encrypt($pt, $key, $nonce, 1);
    is($ct, h($ct_hex), 'ChaCha20 encrypt matches RFC §2.4.2');
    is(length($ct), length($pt), 'Ciphertext length equals plaintext length');

    # Decrypt is identical (XOR again with same keystream)
    my $decrypted = $C->chacha20_encrypt($ct, $key, $nonce, 1);
    is($decrypted, $pt, 'ChaCha20 decrypt round-trip');
};

# ============================================================================
# ChaCha20 Encrypt — empty input
# ============================================================================

subtest 'ChaCha20 encrypt — empty input' => sub {
    my $key   = pack('C32', 0..31);
    my $nonce = "\x00" x 12;
    is($C->chacha20_encrypt('', $key, $nonce), '', 'Empty plaintext → empty ciphertext');
};

# ============================================================================
# ChaCha20 Encrypt — multi-block input (>64 bytes)
# ============================================================================

subtest 'ChaCha20 encrypt — multi-block' => sub {
    my $key   = pack('C32', 0..31);
    my $nonce = "\x00" x 12;
    my $pt    = 'A' x 200;

    my $ct = $C->chacha20_encrypt($pt, $key, $nonce);
    is(length($ct), 200, 'Multi-block ciphertext correct length');
    isnt($ct, $pt, 'Multi-block ciphertext differs from plaintext');

    my $rt = $C->chacha20_encrypt($ct, $key, $nonce);
    is($rt, $pt, 'Multi-block round-trip');
};

# ============================================================================
# Poly1305 MAC — RFC 8439 §2.5.2
# ============================================================================
# This is the official RFC test vector for Poly1305.
# key = 85d6be7857556d337f4452fe42d506a80103808afb0db2fd4abff6af4149f51b
# msg = "Cryptographic Forum Research Group"
# tag = a8061dc1305136c6c22b8baf0c0127a9

subtest 'Poly1305 MAC — RFC 8439 §2.5.2' => sub {
    my $key = h('85d6be7857556d337f4452fe42d506a8'
              . '0103808afb0db2fd4abff6af4149f51b');
    my $msg = 'Cryptographic Forum Research Group';

    my $tag = $C->poly1305_mac($msg, $key);
    is($tag, h('a8061dc1305136c6c22b8baf0c0127a9'), 'Poly1305 tag matches RFC §2.5.2');
    is(length($tag), 16, 'Tag is always 16 bytes');
};

# ============================================================================
# Poly1305 MAC — empty message
# ============================================================================

subtest 'Poly1305 MAC — empty message' => sub {
    my $key = h('85d6be7857556d337f4452fe42d506a8'
              . '0103808afb0db2fd4abff6af4149f51b');

    # Empty message: acc stays 0, tag = s mod 2^128
    my $tag = $C->poly1305_mac('', $key);
    is(length($tag), 16, 'Empty message tag is 16 bytes');
};

# ============================================================================
# Poly1305 MAC — different messages produce different tags
# ============================================================================

subtest 'Poly1305 MAC — sensitivity' => sub {
    my $key = h('85d6be7857556d337f4452fe42d506a8'
              . '0103808afb0db2fd4abff6af4149f51b');

    my $tag1 = $C->poly1305_mac('Hello', $key);
    my $tag2 = $C->poly1305_mac('hello', $key);
    my $tag3 = $C->poly1305_mac('Hello!', $key);

    isnt($tag1, $tag2, 'Case change produces different tag');
    isnt($tag1, $tag3, 'Length change produces different tag');
};

# ============================================================================
# AEAD Encrypt+Decrypt — RFC 8439 §2.8.2
# ============================================================================
# The full AEAD test vector from RFC 8439 §2.8.2.
# This is the canonical test of the complete construction.

subtest 'AEAD — RFC 8439 §2.8.2' => sub {
    my $key   = h('808182838485868788898a8b8c8d8e8f'
                . '909192939495969798999a9b9c9d9e9f');
    my $nonce = h('070000004041424344454647');
    my $aad   = h('50515253c0c1c2c3c4c5c6c7');
    my $pt    = "Ladies and Gentlemen of the class of '99: "
              . "If I could offer you only one tip for the future, "
              . "sunscreen would be it.";

    my ($ct, $tag) = $C->aead_encrypt($pt, $key, $nonce, $aad);

    # Expected ciphertext from RFC §2.8.2
    my $expected_ct = h(
        'd31a8d34648e60db7b86afbc53ef7ec2'
      . 'a4aded51296e08fea9e2b5a736ee62d6'
      . '3dbea45e8ca9671282fafb69da92728b'
      . '1a71de0a9e060b2905d6a5b67ecd3b36'
      . '92ddbd7f2d778b8c9803aee328091b58'
      . 'fab324e4fad675945585808b4831d7bc'
      . '3ff4def08e4b7a9de576d26586cec64b'
      . '6116'
    );

    # Expected tag from RFC §2.8.2
    is($ct,  $expected_ct,                          'AEAD ciphertext matches RFC §2.8.2');
    is($tag, h('1ae10b594f09e26a7e902ecbd0600691'), 'AEAD tag matches RFC §2.8.2');

    # Decrypt and verify round-trip
    my $decrypted = $C->aead_decrypt($ct, $key, $nonce, $aad, $tag);
    is($decrypted, $pt, 'AEAD round-trip produces original plaintext');
};

# ============================================================================
# AEAD — bad tag rejected
# ============================================================================

subtest 'AEAD — bad tag is rejected' => sub {
    my $key   = h('808182838485868788898a8b8c8d8e8f'
                . '909192939495969798999a9b9c9d9e9f');
    my $nonce = h('070000004041424344454647');
    my $aad   = h('50515253c0c1c2c3c4c5c6c7');
    my $pt    = 'Hello, world!';

    my ($ct, $tag) = $C->aead_encrypt($pt, $key, $nonce, $aad);

    # Flip the first bit of the tag
    my $bad_tag = $tag;
    substr($bad_tag, 0, 1) = chr(ord(substr($tag, 0, 1)) ^ 1);

    like(
        dies { $C->aead_decrypt($ct, $key, $nonce, $aad, $bad_tag) },
        qr/Authentication tag mismatch/,
        'Bad tag causes die with correct message'
    );

    # Flip the last bit of the tag
    my $bad_tag2 = $tag;
    substr($bad_tag2, 15, 1) = chr(ord(substr($tag, 15, 1)) ^ 0x80);

    like(
        dies { $C->aead_decrypt($ct, $key, $nonce, $aad, $bad_tag2) },
        qr/Authentication tag mismatch/,
        'Bad tag (last byte) causes die'
    );
};

# ============================================================================
# AEAD — tampered ciphertext rejected
# ============================================================================

subtest 'AEAD — tampered ciphertext is rejected' => sub {
    my $key   = h('808182838485868788898a8b8c8d8e8f'
                . '909192939495969798999a9b9c9d9e9f');
    my $nonce = h('070000004041424344454647');
    my $aad   = '';
    my $pt    = 'Sensitive data';

    my ($ct, $tag) = $C->aead_encrypt($pt, $key, $nonce, $aad);

    # Flip a bit in the middle of the ciphertext
    my $bad_ct = $ct;
    substr($bad_ct, length($ct) / 2, 1) = chr(ord(substr($ct, length($ct) / 2, 1)) ^ 0x42);

    like(
        dies { $C->aead_decrypt($bad_ct, $key, $nonce, $aad, $tag) },
        qr/Authentication tag mismatch/,
        'Tampered ciphertext causes authentication failure'
    );
};

# ============================================================================
# AEAD — tampered AAD rejected
# ============================================================================

subtest 'AEAD — tampered AAD is rejected' => sub {
    my $key   = h('808182838485868788898a8b8c8d8e8f'
                . '909192939495969798999a9b9c9d9e9f');
    my $nonce = h('070000004041424344454647');
    my $aad   = 'original-header';
    my $pt    = 'payload';

    my ($ct, $tag) = $C->aead_encrypt($pt, $key, $nonce, $aad);

    like(
        dies { $C->aead_decrypt($ct, $key, $nonce, 'tampered-header', $tag) },
        qr/Authentication tag mismatch/,
        'Tampered AAD causes authentication failure'
    );
};

# ============================================================================
# AEAD — empty plaintext round-trip
# ============================================================================

subtest 'AEAD — empty plaintext' => sub {
    my $key   = h('808182838485868788898a8b8c8d8e8f'
                . '909192939495969798999a9b9c9d9e9f');
    my $nonce = h('070000004041424344454647');

    my ($ct, $tag) = $C->aead_encrypt('', $key, $nonce, '');
    is(length($ct), 0, 'Empty plaintext → empty ciphertext');
    is(length($tag), 16, 'Tag is 16 bytes even for empty plaintext');

    my $pt = $C->aead_decrypt($ct, $key, $nonce, '', $tag);
    is($pt, '', 'Empty plaintext round-trip');
};

# ============================================================================
# AEAD — multi-block plaintext (>64 bytes)
# ============================================================================

subtest 'AEAD — multi-block plaintext round-trip' => sub {
    my $key   = h('808182838485868788898a8b8c8d8e8f'
                . '909192939495969798999a9b9c9d9e9f');
    my $nonce = h('070000004041424344454647');
    my $aad   = 'some-associated-data';
    my $pt    = 'x' x 200;  # 200 bytes = 4 blocks (64+64+64+8)

    my ($ct, $tag) = $C->aead_encrypt($pt, $key, $nonce, $aad);
    is(length($ct), 200, 'Multi-block ciphertext correct length');

    my $decrypted = $C->aead_decrypt($ct, $key, $nonce, $aad, $tag);
    is($decrypted, $pt, 'Multi-block round-trip');
};

# ============================================================================
# AEAD — AAD without plaintext
# ============================================================================

subtest 'AEAD — AAD only (empty plaintext)' => sub {
    my $key   = h('808182838485868788898a8b8c8d8e8f'
                . '909192939495969798999a9b9c9d9e9f');
    my $nonce = h('070000004041424344454647');
    my $aad   = 'authenticate-this-header-but-dont-encrypt-it';

    my ($ct, $tag) = $C->aead_encrypt('', $key, $nonce, $aad);
    is($C->aead_decrypt($ct, $key, $nonce, $aad, $tag), '', 'AAD-only round-trip');
};

# ============================================================================
# AEAD — determinism: same inputs → same outputs
# ============================================================================

subtest 'AEAD — determinism' => sub {
    my $key   = h('808182838485868788898a8b8c8d8e8f'
                . '909192939495969798999a9b9c9d9e9f');
    my $nonce = h('070000004041424344454647');
    my $aad   = 'header';
    my $pt    = 'payload';

    my ($ct1, $tag1) = $C->aead_encrypt($pt, $key, $nonce, $aad);
    my ($ct2, $tag2) = $C->aead_encrypt($pt, $key, $nonce, $aad);

    is($ct1,  $ct2,  'Same inputs → same ciphertext');
    is($tag1, $tag2, 'Same inputs → same tag');
};

# ============================================================================
# AEAD — single-byte plaintext
# ============================================================================

subtest 'AEAD — single-byte plaintext' => sub {
    my $key   = h('808182838485868788898a8b8c8d8e8f'
                . '909192939495969798999a9b9c9d9e9f');
    my $nonce = h('070000004041424344454647');

    for my $byte (0x00, 0x01, 0x7f, 0x80, 0xff) {
        my $pt = chr($byte);
        my ($ct, $tag) = $C->aead_encrypt($pt, $key, $nonce, '');
        is(length($ct), 1, sprintf('Single byte 0x%02x → 1-byte ciphertext', $byte));
        is($C->aead_decrypt($ct, $key, $nonce, '', $tag), $pt,
           sprintf('Single byte 0x%02x round-trip', $byte));
    }
};

# ============================================================================
# ChaCha20 — known keystream test (RFC 8439 §2.4.1 counter=0 block)
# ============================================================================
# The Poly1305 key generation uses counter=0. This test verifies the block
# at counter=0 produces the expected first 32 bytes (Poly1305 key material).

subtest 'ChaCha20 block — counter=0 (Poly1305 key generation)' => sub {
    my $key   = h('808182838485868788898a8b8c8d8e8f'
                . '909192939495969798999a9b9c9d9e9f');
    my $nonce = h('070000004041424344454647');

    my $block = $C->chacha20_block($key, 0, $nonce);
    is(length($block), 64, 'Block at counter=0 is 64 bytes');

    # The first 32 bytes should be the Poly1305 key from RFC §2.6.2
    my $poly_key = substr($block, 0, 32);
    is($poly_key,
       h('7bac2b252db447af09b67a55a4e95584'
        .'0ae1d6731075d9eb2a9375783ed553ff'),
       'Counter-0 block first 32 bytes = Poly1305 key per RFC §2.6.2');
};

done_testing;
