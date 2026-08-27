# vigenere-cipher

## CR03 conformance

This implementation follows [CR03](../../../specs/CR03-vigenere-cipher.md): keys and cipher transforms use ASCII letters only, analysis ignores non-ASCII letters, and non-ASCII Unicode scalars pass through without advancing the key. Analysis accepts at most 8,192 Unicode scalars and key-length bounds up to 40, uses the smallest candidate within 90% of the best index-of-coincidence score, preserves the exact requested recovered-key length, and resolves score ties toward the smallest shift.

The native suite executes all 26 language-neutral `classical-ciphers-v1` Vigenere cases through a generated dependency-free consumer; `generate_vigenere_fixture_consumers.py --check` prevents corpus, expected-object, and established-lane drift.

Vigenere cipher -- polyalphabetic substitution cipher with full cryptanalysis.

## What is the Vigenere Cipher?

The Vigenere cipher (1553) applies a repeating keyword to shift each letter by a different amount. It was considered unbreakable for 300 years until Kasiski (1863) and Friedman (1920s) developed statistical attacks.

## API

```rust
use vigenere_cipher::{encrypt, decrypt, find_key_length, find_key, break_cipher};

// Encrypt and decrypt
let ct = encrypt("ATTACKATDAWN", "LEMON").unwrap();  // "LXFOPVEFRNHR"
let pt = decrypt("LXFOPVEFRNHR", "LEMON").unwrap();  // "ATTACKATDAWN"

// Cryptanalysis (requires ~200+ chars of English)
let key_len = find_key_length(&ciphertext, 20);
let key = find_key(&ciphertext, key_len);
let result = break_cipher(&ciphertext);  // result.key, result.plaintext
```

## How It Works

- **encrypt/decrypt**: Shift each letter forward/backward by the key letter's position (A=0..Z=25). Non-alpha passes through unchanged.
- **find_key_length**: IC analysis to detect periodicity in the ciphertext.
- **find_key**: Chi-squared analysis to recover each key letter.
- **break_cipher**: Combines find_key_length + find_key + decrypt.

## Part of coding-adventures

This is CR03 in the cryptography layer. See `code/specs/CR03-vigenere-cipher.md`.
