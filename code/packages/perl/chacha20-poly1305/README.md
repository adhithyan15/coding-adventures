# CodingAdventures::ChaCha20Poly1305

Pure-Perl implementation of ChaCha20-Poly1305 Authenticated Encryption with
Associated Data (AEAD) as specified in **RFC 8439**.

## What It Is

ChaCha20-Poly1305 is a modern AEAD cipher combining:

- **ChaCha20** — a stream cipher based on the ARX (Add-Rotate-XOR) design,
  producing a 512-bit keystream block from a 256-bit key, 32-bit counter, and
  96-bit nonce. Fast in software with no lookup tables; immune to cache-timing
  attacks.

- **Poly1305** — a one-time MAC operating in GF(2^130 - 5). Produces a 128-bit
  authentication tag that detects any tampering with the ciphertext or additional
  authenticated data (AAD).

Together they provide confidentiality, integrity, and authenticity — the three
pillars of authenticated encryption.

## Where It Fits in the Stack

This package is a standalone cryptographic primitive. It depends only on
`Math::BigInt` (Perl core) for Poly1305's 130-bit arithmetic. It implements the
same algorithm as:

- `code/packages/python/chacha20-poly1305/`
- `code/packages/typescript/chacha20-poly1305/`
- etc.

The spec lives at `code/specs/chacha20-poly1305.md`.

## Usage

```perl
use CodingAdventures::ChaCha20Poly1305;

my $C     = 'CodingAdventures::ChaCha20Poly1305';
my $key   = "\x80" x 32;   # 32-byte key — use a CSPRNG in production!
my $nonce = "\x07" x 12;   # 12-byte nonce — never reuse with the same key!
my $aad   = "version=1;type=msg";  # Authenticated but not encrypted

# Encrypt
my ($ciphertext, $tag) = $C->aead_encrypt("Hello, world!", $key, $nonce, $aad);

# Decrypt (dies if tag is invalid)
my $plaintext = $C->aead_decrypt($ciphertext, $key, $nonce, $aad, $tag);
# => "Hello, world!"

# Low-level: ChaCha20 stream cipher only
my $keystream_block = $C->chacha20_block($key, 1, $nonce);  # 64 bytes
my $encrypted = $C->chacha20_encrypt("plaintext", $key, $nonce);

# Low-level: Poly1305 MAC only
my $one_time_key = "\x85\xd6" . ("\x00" x 30);
my $tag = $C->poly1305_mac("message", $one_time_key);  # 16 bytes
```

## Security Notes

- **Never reuse a (key, nonce) pair.** Poly1305 is a one-time MAC; nonce reuse
  allows an attacker to recover both messages and potentially the key.
- **Use a CSPRNG for keys.** Keys must be 32 bytes of cryptographically random data.
- **Verify before decrypting.** `aead_decrypt` enforces this — it always verifies
  the tag before returning plaintext and uses constant-time comparison.
- **AAD is authenticated but not encrypted.** Use it for headers, metadata, or
  framing data that the recipient needs to read before decrypting.

## API

### `aead_encrypt($class, $plaintext, $key, $nonce, $aad)`

Encrypt and authenticate. Returns `($ciphertext, $tag)`.

- `$plaintext` — bytes to encrypt (any length, including empty)
- `$key`       — 32-byte key
- `$nonce`     — 12-byte nonce
- `$aad`       — additional authenticated data (may be empty string)

### `aead_decrypt($class, $ciphertext, $key, $nonce, $aad, $tag)`

Verify and decrypt. Returns `$plaintext`. Dies with
`"Authentication tag mismatch\n"` if the tag is invalid.

### `chacha20_block($class, $key, $counter, $nonce)`

Generate one 64-byte ChaCha20 keystream block. Used internally; exposed for
testing and educational purposes.

### `chacha20_encrypt($class, $plaintext, $key, $nonce, $counter)`

Encrypt or decrypt with ChaCha20 stream cipher. `$counter` defaults to 1.
Since ChaCha20 is symmetric (XOR), this function encrypts and decrypts.

### `poly1305_mac($class, $message, $key)`

Compute a 16-byte Poly1305 authentication tag.

## Test Vectors

The implementation is verified against all official RFC 8439 test vectors:

- §2.1.2: ChaCha20 block function
- §2.4.2: ChaCha20 stream encryption (Sunscreen message)
- §2.5.2: Poly1305 MAC ("Cryptographic Forum Research Group")
- §2.6.2: Poly1305 key generation from ChaCha20 block
- §2.8.2: Full AEAD construction

## Running Tests

```bash
cpanm --with-test --installdeps --quiet .
prove -l -v t/
```

Or directly:

```bash
perl -Ilib t/chacha20_poly1305.t
```

## Reference

- RFC 8439: [ChaCha20 and Poly1305 for IETF Protocols](https://www.rfc-editor.org/rfc/rfc8439)
- Bernstein, D. J. (2008). "ChaCha, a variant of Salsa20"
