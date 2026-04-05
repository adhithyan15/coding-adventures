# CR03 — Vigenère Cipher

## Overview

The Vigenère cipher is a polyalphabetic substitution cipher that uses a
keyword to apply different Caesar shifts at each position. Unlike Caesar
(single shift) or Atbash (fixed mapping), each letter in the plaintext is
shifted by a different amount determined by the corresponding letter of
the repeating keyword.

Invented by Giovan Battista Bellaso in 1553 and misattributed to Blaise
de Vigenère, it was considered "le chiffre indéchiffrable" for 300 years
until Friedrich Kasiski published a general method for breaking it in 1863.

## Algorithm

### Encrypt

Given plaintext P and keyword K (both letters only):

1. Repeat K to match the length of alphabetic characters in P
2. For each letter P[i], shift it forward by K[i] positions in the alphabet
3. Non-alphabetic characters pass through unchanged (keyword position
   does NOT advance for non-alpha characters)

```
Plaintext:  A T T A C K A T D A W N
Keyword:    L E M O N L E M O N L E
Shift:      11 4 12 14 13 11 4 12 14 13 11 4
Ciphertext: L X F O P V E F R N H R
```

### Decrypt

Reverse the process: shift each letter backward by the keyword amount.

### Cryptanalysis (Breaking the Cipher)

Breaking Vigenère requires two steps:

**Step 1: Find the key length** using Index of Coincidence (IC):
- For each candidate key length k (2..40):
  - Split ciphertext into k groups (every k-th letter)
  - Calculate IC of each group
  - Average the ICs
- The key length where average IC is closest to English (~0.0667)
  is likely correct (random text has IC ~0.0385)

**Step 2: Find each key letter** using chi-squared:
- For each position in the key (0..k-1):
  - Extract the group of letters at that position
  - Try all 26 possible shifts
  - The shift producing the lowest chi-squared against English
    frequencies is the key letter for that position

## Interface Contract

| Function | Signature | Description |
|----------|-----------|-------------|
| `encrypt` | `(plaintext: str, key: str) -> str` | Vigenère encrypt. Key must be alphabetic. |
| `decrypt` | `(ciphertext: str, key: str) -> str` | Vigenère decrypt. |
| `find_key_length` | `(ciphertext: str, max_length: int = 20) -> int` | Estimate key length via IC analysis. |
| `find_key` | `(ciphertext: str, key_length: int) -> str` | Find key letters via chi-squared analysis. |
| `break_cipher` | `(ciphertext: str) -> (key: str, plaintext: str)` | Full automatic break: find length, then key, then decrypt. |

### Character Handling
- Preserve case: uppercase stays uppercase, lowercase stays lowercase
- Non-alphabetic characters pass through unchanged
- Keyword position only advances on alphabetic characters
- Key must be non-empty and contain only A-Z/a-z

## Worked Example

### Encryption
```
Plaintext:  "ATTACK AT DAWN"
Key:        "LEMON"

Position:   A  T  T  A  C  K  _  A  T  _  D  A  W  N
Key cycle:  L  E  M  O  N  L  _  E  M  _  O  N  L  E
Shift:      11 4  12 14 13 11 _  4  12 _  14 13 11 4
Result:     L  X  F  O  P  V  _  E  F  _  R  N  H  R

Ciphertext: "LXFOPV EF RNHR"
```

### Cryptanalysis
```
Ciphertext (long English text encrypted with key "SECRET"):

Step 1 — IC analysis for key lengths 2..20:
  k=2: avg IC = 0.042  (too low, not English-like)
  k=3: avg IC = 0.045
  k=6: avg IC = 0.065  (close to 0.0667 — likely correct!)

Step 2 — chi-squared for each position with k=6:
  pos 0: best shift = 18 (S)  chi² = 23.4
  pos 1: best shift = 4  (E)  chi² = 18.7
  pos 2: best shift = 2  (C)  chi² = 25.1
  pos 3: best shift = 17 (R)  chi² = 20.3
  pos 4: best shift = 4  (E)  chi² = 19.8
  pos 5: best shift = 19 (T)  chi² = 22.6

Recovered key: "SECRET"
```

## Parity Test Vectors

All 9 languages must produce identical results:

- `encrypt("ATTACKATDAWN", "LEMON")` → `"LXFOPVEFRNHR"`
- `decrypt("LXFOPVEFRNHR", "LEMON")` → `"ATTACKATDAWN"`
- `encrypt("Hello, World!", "key")` → `"Rijvs, Uyvjn!"` (preserve case + punctuation)
- `decrypt(encrypt(text, key), key) == text` for all inputs
- `find_key_length` on sufficiently long text (~200+ chars) encrypted
  with a known key returns the correct length
- `break_cipher` on long English text recovers the original key

## Package Matrix

| Language | Package Directory | Module/Namespace |
|----------|-------------------|------------------|
| Python | `code/packages/python/vigenere-cipher/` | `vigenere_cipher` |
| Go | `code/packages/go/vigenere-cipher/` | `vigenerecipher` |
| Ruby | `code/packages/ruby/vigenere_cipher/` | `CodingAdventures::VigenereCipher` |
| TypeScript | `code/packages/typescript/vigenere-cipher/` | `@coding-adventures/vigenere-cipher` |
| Rust | `code/packages/rust/vigenere-cipher/` | `vigenere_cipher` |
| Elixir | `code/packages/elixir/vigenere_cipher/` | `CodingAdventures.VigenereCipher` |
| Lua | `code/packages/lua/vigenere_cipher/` | `coding_adventures.vigenere_cipher` |
| Perl | `code/packages/perl/vigenere-cipher/` | `CodingAdventures::VigenereCipher` |
| Swift | `code/packages/swift/vigenere-cipher/` | `VigenereCipher` |

**Dependencies:** None for encrypt/decrypt. Cryptanalysis functions may
use ST01 stats (chi_squared, index_of_coincidence, ENGLISH_FREQUENCIES)
or implement the formulas inline for zero-dependency operation.
