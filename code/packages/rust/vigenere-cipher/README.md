# vigenere-cipher

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
