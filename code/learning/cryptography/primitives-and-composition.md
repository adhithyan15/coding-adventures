<!-- learning-concepts: aes, aes-modes, blake2b, sha1, sha256, sha512, md5, hmac, hkdf, pbkdf2, scrypt, argon2d, argon2i, argon2id, chacha20-poly1305, ed25519, x25519, caesar-cipher, atbash-cipher, scytale-cipher, vigenere-cipher -->

# Cryptographic Primitives and Composition

Cryptographic code is easiest to misunderstand at the seams. A block cipher,
hash, key derivation function, signature scheme, and key-agreement scheme solve
different problems. Secure systems compose them under a protocol that defines
keys, nonces, associated data, serialization, and failure behavior.

The repository implementations are for learning. Do not substitute them for a
reviewed cryptographic library in production.

## Historical Ciphers

Caesar, Atbash, Scytale, and Vigenere make useful transformations visible, but
they provide no modern security. Their patterns survive statistical analysis
and their key spaces are small. They are stepping stones for learning about
keys and reversible transformations.

## Hash Functions

A cryptographic hash maps an arbitrary message to a fixed-size digest.
Important properties include resistance to finding a preimage, a second
preimage, or any collision.

MD5 and SHA-1 are historically important but collision-broken. They remain
useful for compatibility exercises, not new security designs. SHA-256,
SHA-512, and BLAKE2b are modern hash choices, subject to protocol requirements.

A plain hash does not authenticate a message because anyone can recompute it.
HMAC combines a secret key with a hash using a construction designed for
message authentication.

## Symmetric Encryption

AES transforms fixed-size blocks under a secret key. A mode defines how to
handle messages larger than one block and what additional guarantees exist.
ECB exposes repeated-block patterns and should not be used for general message
encryption.

Authenticated encryption binds confidentiality and integrity. ChaCha20-
Poly1305 combines a stream cipher with an authenticator. The receiver must
verify the tag before releasing plaintext.

Nonce reuse can be catastrophic. A nonce need not always be secret, but its
uniqueness requirements are part of the algorithm and protocol.

## Passwords Into Keys

Passwords have low and uneven entropy, so a fast hash is the wrong tool.
PBKDF2 repeats a pseudorandom function to increase attacker cost. Scrypt and
Argon2 additionally demand memory, making parallel guessing more expensive.

Argon2d, Argon2i, and Argon2id differ in how memory is accessed and the threats
they prioritize. New systems commonly prefer Argon2id with parameters chosen
for their deployment environment.

A salt is public and unique; it prevents identical passwords from sharing the
same stored result. It is not a replacement for cost parameters.

## Key Derivation

HKDF extracts a strong pseudorandom key from input key material, then expands
it into one or more context-bound keys. Its context information should identify
the protocol purpose so keys for different jobs are not accidentally reused.

## Public-Key Operations

X25519 performs key agreement: two parties combine a private key with the
other party's public key to derive shared material. That result should normally
flow through a key derivation function.

Ed25519 creates and verifies digital signatures. A signature authenticates a
message under a public key; it does not encrypt the message.

Key agreement alone does not prove who the peer is. A protocol adds
authentication, transcript binding, and downgrade protection.

## Composition Checklist

Before trusting a cryptographic flow, identify:

1. the exact security property required
2. how keys are generated, separated, stored, rotated, and destroyed
3. nonce and salt rules
4. which bytes are authenticated, including metadata
5. the canonical serialization
6. constant-time handling of secrets and authentication results
7. whether plaintext is withheld until authentication succeeds
8. test vectors and negative tests for corrupted inputs

The primitive is only one line in that checklist. Most of the security story
lives in the protocol around it.
