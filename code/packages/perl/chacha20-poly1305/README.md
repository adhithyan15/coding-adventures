# ChaCha20-Poly1305 (Perl)

ChaCha20-Poly1305 authenticated encryption (RFC 8439) implemented in pure Perl.

## What is ChaCha20-Poly1305?

ChaCha20-Poly1305 is an authenticated encryption scheme combining:

- **ChaCha20**: a stream cipher using only Add, Rotate, XOR (ARX) operations
- **Poly1305**: a one-time message authentication code (MAC)

Together they provide both confidentiality (encryption) and integrity (authentication). Used in TLS 1.3, WireGuard, SSH, and Chrome/Android.

## Usage

```perl
use CodingAdventures::ChaCha20Poly1305;

# ChaCha20 stream cipher
my $ct = CodingAdventures::ChaCha20Poly1305::chacha20_encrypt(
    $plaintext, $key_32, $nonce_12, $counter
);

# Poly1305 MAC
my $tag = CodingAdventures::ChaCha20Poly1305::poly1305_mac($message, $key_32);

# AEAD encrypt
my ($ct, $tag) = CodingAdventures::ChaCha20Poly1305::aead_encrypt(
    $plaintext, $key_32, $nonce_12, $aad
);

# AEAD decrypt (returns undef + error on auth failure)
my ($pt, $err) = CodingAdventures::ChaCha20Poly1305::aead_decrypt(
    $ct, $key_32, $nonce_12, $aad, $tag
);
```

## Running Tests

```sh
prove -l -v t/
```

## Dependencies

- Math::BigInt (core module, no external deps)
