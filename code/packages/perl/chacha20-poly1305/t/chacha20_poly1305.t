use strict;
use warnings;
use Test2::V0;

use CodingAdventures::ChaCha20Poly1305;

# ============================================================================
# Helper functions
# ============================================================================

# Decode hex string to bytes.
sub h { pack('H*', $_[0]) }

# Encode bytes as hex string.
sub to_hex { unpack('H*', $_[0]) }

# ============================================================================
# ChaCha20 — RFC 8439 Section 2.4.2
# ============================================================================

subtest 'ChaCha20 — RFC 8439 Section 2.4.2 (Sunscreen)' => sub {
    my $key = h(
        '000102030405060708090a0b0c0d0e0f' .
        '101112131415161718191a1b1c1d1e1f'
    );
    my $nonce = h('000000000000004a00000000');
    my $counter = 1;

    my $plaintext =
        "Ladies and Gentlemen of the class of '99: " .
        "If I could offer you only one tip for the future, " .
        "sunscreen would be it.";

    my $expected_ct = h(
        '6e2e359a2568f98041ba0728dd0d6981' .
        'e97e7aec1d4360c20a27afccfd9fae0b' .
        'f91b65c5524733ab8f593dabcd62b357' .
        '1639d624e65152ab8f530c359f0861d8' .
        '07ca0dbf500d6a6156a38e088a22b65e' .
        '52bc514d16ccf806818ce91ab7793736' .
        '5af90bbf74a35be6b40b8eedf2785e42' .
        '874d'
    );

    my $ct = CodingAdventures::ChaCha20Poly1305::chacha20_encrypt(
        $plaintext, $key, $nonce, $counter
    );
    is(to_hex($ct), to_hex($expected_ct), 'RFC 8439 ChaCha20 test vector');
};

subtest 'ChaCha20 — encrypt/decrypt round-trip' => sub {
    my $key = h(
        '000102030405060708090a0b0c0d0e0f' .
        '101112131415161718191a1b1c1d1e1f'
    );
    my $nonce = h('000000000000004a00000000');
    my $plaintext = 'Hello, ChaCha20!';

    my $ct = CodingAdventures::ChaCha20Poly1305::chacha20_encrypt(
        $plaintext, $key, $nonce, 0
    );
    my $recovered = CodingAdventures::ChaCha20Poly1305::chacha20_encrypt(
        $ct, $key, $nonce, 0
    );
    is($recovered, $plaintext, 'round-trip preserves plaintext');
};

subtest 'ChaCha20 — empty plaintext' => sub {
    my $key = h(
        '000102030405060708090a0b0c0d0e0f' .
        '101112131415161718191a1b1c1d1e1f'
    );
    my $nonce = h('000000000000000000000000');
    my $ct = CodingAdventures::ChaCha20Poly1305::chacha20_encrypt(
        '', $key, $nonce, 0
    );
    is($ct, '', 'empty input produces empty output');
};

subtest 'ChaCha20 — single byte' => sub {
    my $key = h(
        '000102030405060708090a0b0c0d0e0f' .
        '101112131415161718191a1b1c1d1e1f'
    );
    my $nonce = h('000000000000000000000000');
    my $ct = CodingAdventures::ChaCha20Poly1305::chacha20_encrypt(
        'X', $key, $nonce, 0
    );
    is(length($ct), 1, 'single byte ciphertext');
    my $pt = CodingAdventures::ChaCha20Poly1305::chacha20_encrypt(
        $ct, $key, $nonce, 0
    );
    is($pt, 'X', 'single byte round-trip');
};

subtest 'ChaCha20 — multi-block (> 64 bytes)' => sub {
    my $key = h(
        '000102030405060708090a0b0c0d0e0f' .
        '101112131415161718191a1b1c1d1e1f'
    );
    my $nonce = h('000000000000000000000000');
    my $plaintext = 'A' x 200;
    my $ct = CodingAdventures::ChaCha20Poly1305::chacha20_encrypt(
        $plaintext, $key, $nonce, 0
    );
    is(length($ct), 200, '200-byte ciphertext');
    my $recovered = CodingAdventures::ChaCha20Poly1305::chacha20_encrypt(
        $ct, $key, $nonce, 0
    );
    is($recovered, $plaintext, 'multi-block round-trip');
};

subtest 'ChaCha20 — rejects invalid key length' => sub {
    like(
        dies { CodingAdventures::ChaCha20Poly1305::chacha20_encrypt(
            'test', 'short', h('000000000000000000000000'), 0
        ) },
        qr/Key must be 32 bytes/,
        'rejects short key'
    );
};

subtest 'ChaCha20 — rejects invalid nonce length' => sub {
    like(
        dies { CodingAdventures::ChaCha20Poly1305::chacha20_encrypt(
            'test',
            h('000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f'),
            'short', 0
        ) },
        qr/Nonce must be 12 bytes/,
        'rejects short nonce'
    );
};

# ============================================================================
# Poly1305 — RFC 8439 Section 2.5.2
# ============================================================================

subtest 'Poly1305 — RFC 8439 Section 2.5.2 (CFRG)' => sub {
    my $key = h(
        '85d6be7857556d337f4452fe42d506a8' .
        '0103808afb0db2fd4abff6af4149f51b'
    );
    my $message = 'Cryptographic Forum Research Group';
    my $expected_tag = h('a8061dc1305136c6c22b8baf0c0127a9');

    my $tag = CodingAdventures::ChaCha20Poly1305::poly1305_mac($message, $key);
    is(to_hex($tag), to_hex($expected_tag), 'RFC 8439 Poly1305 test vector');
};

subtest 'Poly1305 — empty message' => sub {
    my $key = h(
        '00000000000000000000000000000000' .
        '01020304050607080910111213141516'
    );
    my $tag = CodingAdventures::ChaCha20Poly1305::poly1305_mac('', $key);
    is(length($tag), 16, 'tag is 16 bytes');
    # With no blocks processed, tag = (0 + s) mod 2^128 = s
    is(to_hex($tag), '01020304050607080910111213141516', 'empty message tag equals s');
};

subtest 'Poly1305 — rejects invalid key' => sub {
    like(
        dies { CodingAdventures::ChaCha20Poly1305::poly1305_mac('test', 'short') },
        qr/Poly1305 key must be 32 bytes/,
        'rejects short key'
    );
};

subtest 'Poly1305 — single byte' => sub {
    my $key = h(
        '85d6be7857556d337f4452fe42d506a8' .
        '0103808afb0db2fd4abff6af4149f51b'
    );
    my $tag = CodingAdventures::ChaCha20Poly1305::poly1305_mac('A', $key);
    is(length($tag), 16, 'single byte tag is 16 bytes');
};

subtest 'Poly1305 — exactly 16 bytes (one full block)' => sub {
    my $key = h(
        '85d6be7857556d337f4452fe42d506a8' .
        '0103808afb0db2fd4abff6af4149f51b'
    );
    my $tag = CodingAdventures::ChaCha20Poly1305::poly1305_mac('0123456789abcdef', $key);
    is(length($tag), 16, '16-byte message tag is 16 bytes');
};

subtest 'Poly1305 — 17 bytes (two blocks)' => sub {
    my $key = h(
        '85d6be7857556d337f4452fe42d506a8' .
        '0103808afb0db2fd4abff6af4149f51b'
    );
    my $tag = CodingAdventures::ChaCha20Poly1305::poly1305_mac('0123456789abcdefg', $key);
    is(length($tag), 16, '17-byte message tag is 16 bytes');
};

# ============================================================================
# AEAD — RFC 8439 Section 2.8.2
# ============================================================================

subtest 'AEAD — RFC 8439 Section 2.8.2 encryption' => sub {
    my $key = h(
        '808182838485868788898a8b8c8d8e8f' .
        '909192939495969798999a9b9c9d9e9f'
    );
    my $nonce = h('070000004041424344454647');
    my $aad = h('50515253c0c1c2c3c4c5c6c7');
    my $plaintext =
        "Ladies and Gentlemen of the class of '99: " .
        "If I could offer you only one tip for the future, " .
        "sunscreen would be it.";

    my $expected_ct = h(
        'd31a8d34648e60db7b86afbc53ef7ec2' .
        'a4aded51296e08fea9e2b5a736ee62d6' .
        '3dbea45e8ca9671282fafb69da92728b' .
        '1a71de0a9e060b2905d6a5b67ecd3b36' .
        '92ddbd7f2d778b8c9803aee328091b58' .
        'fab324e4fad675945585808b4831d7bc' .
        '3ff4def08e4b7a9de576d26586cec64b' .
        '6116'
    );
    my $expected_tag = h('1ae10b594f09e26a7e902ecbd0600691');

    my ($ct, $tag) = CodingAdventures::ChaCha20Poly1305::aead_encrypt(
        $plaintext, $key, $nonce, $aad
    );
    is(to_hex($ct),  to_hex($expected_ct),  'AEAD ciphertext matches RFC 8439');
    is(to_hex($tag), to_hex($expected_tag), 'AEAD tag matches RFC 8439');
};

subtest 'AEAD — RFC 8439 Section 2.8.2 decryption' => sub {
    my $key = h(
        '808182838485868788898a8b8c8d8e8f' .
        '909192939495969798999a9b9c9d9e9f'
    );
    my $nonce = h('070000004041424344454647');
    my $aad = h('50515253c0c1c2c3c4c5c6c7');
    my $ct = h(
        'd31a8d34648e60db7b86afbc53ef7ec2' .
        'a4aded51296e08fea9e2b5a736ee62d6' .
        '3dbea45e8ca9671282fafb69da92728b' .
        '1a71de0a9e060b2905d6a5b67ecd3b36' .
        '92ddbd7f2d778b8c9803aee328091b58' .
        'fab324e4fad675945585808b4831d7bc' .
        '3ff4def08e4b7a9de576d26586cec64b' .
        '6116'
    );
    my $tag = h('1ae10b594f09e26a7e902ecbd0600691');
    my $expected_pt =
        "Ladies and Gentlemen of the class of '99: " .
        "If I could offer you only one tip for the future, " .
        "sunscreen would be it.";

    my ($pt, $err) = CodingAdventures::ChaCha20Poly1305::aead_decrypt(
        $ct, $key, $nonce, $aad, $tag
    );
    is($err, undef, 'no error');
    is($pt, $expected_pt, 'decrypted plaintext matches');
};

subtest 'AEAD — round-trip' => sub {
    my $key = h(
        '000102030405060708090a0b0c0d0e0f' .
        '101112131415161718191a1b1c1d1e1f'
    );
    my $nonce = h('000000000000000000000000');
    my $aad = 'some metadata';
    my $plaintext = 'secret message!';

    my ($ct, $tag) = CodingAdventures::ChaCha20Poly1305::aead_encrypt(
        $plaintext, $key, $nonce, $aad
    );
    my ($recovered, $err) = CodingAdventures::ChaCha20Poly1305::aead_decrypt(
        $ct, $key, $nonce, $aad, $tag
    );
    is($err, undef, 'no error');
    is($recovered, $plaintext, 'round-trip preserves plaintext');
};

subtest 'AEAD — wrong tag' => sub {
    my $key = h(
        '000102030405060708090a0b0c0d0e0f' .
        '101112131415161718191a1b1c1d1e1f'
    );
    my $nonce = h('000000000000000000000000');
    my ($ct, $_tag) = CodingAdventures::ChaCha20Poly1305::aead_encrypt(
        'secret', $key, $nonce, 'metadata'
    );
    my $bad_tag = "\0" x 16;
    my ($result, $err) = CodingAdventures::ChaCha20Poly1305::aead_decrypt(
        $ct, $key, $nonce, 'metadata', $bad_tag
    );
    is($result, undef, 'returns undef on bad tag');
    is($err, 'authentication failed', 'error message');
};

subtest 'AEAD — tampered ciphertext' => sub {
    my $key = h(
        '000102030405060708090a0b0c0d0e0f' .
        '101112131415161718191a1b1c1d1e1f'
    );
    my $nonce = h('000000000000000000000000');
    my ($ct, $tag) = CodingAdventures::ChaCha20Poly1305::aead_encrypt(
        'secret', $key, $nonce, 'metadata'
    );
    # Flip one bit
    my $tampered = chr(ord(substr($ct, 0, 1)) ^ 1) . substr($ct, 1);
    my ($result, $err) = CodingAdventures::ChaCha20Poly1305::aead_decrypt(
        $tampered, $key, $nonce, 'metadata', $tag
    );
    is($result, undef, 'rejects tampered ciphertext');
    is($err, 'authentication failed', 'error message');
};

subtest 'AEAD — wrong AAD' => sub {
    my $key = h(
        '000102030405060708090a0b0c0d0e0f' .
        '101112131415161718191a1b1c1d1e1f'
    );
    my $nonce = h('000000000000000000000000');
    my ($ct, $tag) = CodingAdventures::ChaCha20Poly1305::aead_encrypt(
        'secret', $key, $nonce, 'correct aad'
    );
    my ($result, $err) = CodingAdventures::ChaCha20Poly1305::aead_decrypt(
        $ct, $key, $nonce, 'wrong aad', $tag
    );
    is($result, undef, 'rejects wrong AAD');
    is($err, 'authentication failed', 'error message');
};

subtest 'AEAD — empty plaintext with AAD' => sub {
    my $key = h(
        '000102030405060708090a0b0c0d0e0f' .
        '101112131415161718191a1b1c1d1e1f'
    );
    my $nonce = h('000000000000000000000000');
    my ($ct, $tag) = CodingAdventures::ChaCha20Poly1305::aead_encrypt(
        '', $key, $nonce, 'authenticate this'
    );
    is($ct, '', 'empty ciphertext');
    is(length($tag), 16, 'tag still 16 bytes');
    my ($recovered, $err) = CodingAdventures::ChaCha20Poly1305::aead_decrypt(
        $ct, $key, $nonce, 'authenticate this', $tag
    );
    is($err, undef, 'no error');
    is($recovered, '', 'recovered empty plaintext');
};

subtest 'AEAD — empty AAD' => sub {
    my $key = h(
        '000102030405060708090a0b0c0d0e0f' .
        '101112131415161718191a1b1c1d1e1f'
    );
    my $nonce = h('000000000000000000000000');
    my ($ct, $tag) = CodingAdventures::ChaCha20Poly1305::aead_encrypt(
        'hello', $key, $nonce, ''
    );
    my ($recovered, $err) = CodingAdventures::ChaCha20Poly1305::aead_decrypt(
        $ct, $key, $nonce, '', $tag
    );
    is($err, undef, 'no error');
    is($recovered, 'hello', 'recovered plaintext');
};

done_testing;
