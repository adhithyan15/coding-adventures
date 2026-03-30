# CR00 — Caesar Cipher

## Overview

The Caesar cipher is the oldest known substitution cipher, used by Julius Caesar
around 58 BCE to protect military correspondence. Each letter in the plaintext is
shifted by a fixed number of positions in the alphabet. For example, with a shift
of 3, A becomes D, B becomes E, and so on. When the shift reaches the end of the
alphabet, it wraps around: X becomes A, Y becomes B, Z becomes C.

This is the first package in the **CR** (Cryptography) series. The series will
build from ancient ciphers to modern cryptographic algorithms, each package
increasing in complexity while building on concepts from the previous one.

### Why Start Here?

The Caesar cipher is the ideal entry point for learning cryptography because:

1. **It is trivially simple.** The core algorithm is one line of arithmetic.
2. **It introduces key concepts.** Substitution, keys, encryption, decryption,
   and the encrypt/decrypt duality all appear in their simplest form.
3. **It is trivially breakable.** There are only 25 possible keys, which lets us
   introduce brute-force attacks and frequency analysis — the two foundational
   cryptanalysis techniques.
4. **ROT13 is a famous special case.** Shift-13 is self-inverse (encrypting twice
   returns the original), which introduces the idea of involutions.

### Historical Context

Caesar reportedly used a shift of 3 for his personal correspondence. Suetonius
describes it in *Life of Julius Caesar*: each letter was replaced by the letter
three positions further in the alphabet. The cipher was considered adequate
because most of Caesar's enemies were illiterate.

The cipher remained in use for centuries after Caesar. As late as the 15th
century, it appeared in military and diplomatic communication. It was only
superseded when Al-Kindi's frequency analysis technique (circa 850 CE) made
simple substitution ciphers trivially breakable — which we demonstrate in the
`frequency_analysis` function.

## How It Works

### Encryption

Given a plaintext letter and a shift value:

```
encrypted_char = (char_position + shift) mod 26
```

Where `char_position` is 0-indexed (A=0, B=1, ..., Z=25).

**Worked example: encrypt "HELLO" with shift 3**

```
H (7)  + 3 = 10 → K
E (4)  + 3 =  7 → H
L (11) + 3 = 14 → O
L (11) + 3 = 14 → O
O (14) + 3 = 17 → R
```

Result: `KHOOR`

### Decryption

Decryption is encryption with the negated shift:

```
decrypted_char = (char_position - shift) mod 26
```

Or equivalently, decrypt is encrypt with shift `(26 - shift)`.

**Worked example: decrypt "KHOOR" with shift 3**

```
K (10) - 3 =  7 → H
H (7)  - 3 =  4 → E
O (14) - 3 = 11 → L
O (14) - 3 = 11 → L
R (17) - 3 = 14 → O
```

Result: `HELLO`

### The Full Substitution Table (shift=3)

```
Plain:  A B C D E F G H I J K L M N O P Q R S T U V W X Y Z
Cipher: D E F G H I J K L M N O P Q R S T U V W X Y Z A B C
```

### ROT13 — The Self-Inverse Special Case

ROT13 uses shift=13. Since the English alphabet has 26 letters, shifting by 13
twice returns to the original:

```
encrypt(encrypt(text, 13), 13) == text
```

This makes ROT13 its own inverse — the same function encrypts and decrypts.
ROT13 was historically used on Usenet to hide spoilers and punchlines.

## Cryptanalysis

### Brute Force

Since there are only 25 possible non-identity shifts (1 through 25), an attacker
can simply try all of them and inspect the results. This is the simplest
cryptanalytic attack and demonstrates why the Caesar cipher is completely insecure.

### Frequency Analysis

In English text, letters appear with characteristic frequencies:

```
E: 12.7%   T: 9.1%   A: 8.2%   O: 7.5%   I: 7.0%
N: 6.7%    S: 6.3%   H: 6.1%   R: 6.0%   D: 4.3%
L: 4.0%    C: 2.8%   U: 2.8%   M: 2.4%   W: 2.4%
F: 2.2%    G: 2.0%   Y: 2.0%   P: 1.9%   B: 1.5%
V: 1.0%    K: 0.8%   J: 0.2%   X: 0.2%   Q: 0.1%
Z: 0.1%
```

If we count the frequency of each letter in the ciphertext and compare it to the
expected English distribution, the shift that produces the best match is likely
the correct key. We use the chi-squared statistic to score each candidate shift:

```
chi_squared = Σ (observed_count - expected_count)² / expected_count
```

The shift with the lowest chi-squared score is the best guess.

## Interface Contract

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `encrypt` | `(text: string, shift: int) → string` | Encrypt plaintext using the given shift |
| `decrypt` | `(text: string, shift: int) → string` | Decrypt ciphertext using the given shift |
| `rot13` | `(text: string) → string` | Apply ROT13 (encrypt with shift=13) |
| `brute_force` | `(ciphertext: string) → list[(int, string)]` | Try all 25 shifts, return (shift, plaintext) pairs |
| `frequency_analysis` | `(ciphertext: string) → (int, string)` | Guess the shift using English letter frequencies |

### Character Handling Rules

1. **Alphabetic characters** (A-Z, a-z) are shifted. Case is preserved:
   - `encrypt("Hello", 3)` → `"Khoor"` (H→K, e→h, l→o, l→o, o→r)
2. **Non-alphabetic characters** (digits, punctuation, spaces) pass through unchanged:
   - `encrypt("Hello, World! 123", 3)` → `"Khoor, Zruog! 123"`
3. **Empty strings** return empty strings.
4. **Shift wrapping**: shift values are taken modulo 26. A shift of 26 is the same as 0.
   Negative shifts are supported: `encrypt(text, -3)` == `decrypt(text, 3)`.

### Brute Force Output

Returns a list of 25 tuples `(shift, decrypted_text)` for shifts 1 through 25.
Shift 0 is excluded because it produces the input unchanged.

### Frequency Analysis Output

Returns a tuple `(best_shift, decrypted_text)` where `best_shift` is the shift
value (0-25) that produces plaintext with the closest match to English letter
frequencies. For very short texts (fewer than ~20 characters), the result may be
unreliable due to insufficient statistical data.

## Package Matrix

| Language | Package Directory | Module/Namespace |
|----------|-------------------|------------------|
| Python | `code/packages/python/caesar-cipher/` | `caesar_cipher` |
| Go | `code/packages/go/caesar-cipher/` | `caesarcipher` |
| Ruby | `code/packages/ruby/caesar_cipher/` | `CodingAdventures::CaesarCipher` |
| TypeScript | `code/packages/typescript/caesar-cipher/` | `@coding-adventures/caesar-cipher` |
| Rust | `code/packages/rust/caesar-cipher/` | `caesar_cipher` |
| Elixir | `code/packages/elixir/caesar_cipher/` | `CodingAdventures.CaesarCipher` |
| Lua | `code/packages/lua/caesar-cipher/` | `coding_adventures.caesar_cipher` |
| Perl | `code/packages/perl/caesar-cipher/` | `CodingAdventures::CaesarCipher` |
| Swift | `code/packages/swift/caesar-cipher/` | `CaesarCipher` |

**Dependencies:** None. This is a standalone foundation package.

## Planned CR Series Roadmap

| Spec | Package | Concepts Introduced |
|------|---------|---------------------|
| CR00 | Caesar Cipher | Substitution, brute force, frequency analysis |
| CR01 | Vigenere Cipher | Polyalphabetic substitution, Kasiski examination |
| CR02 | Enigma Machine | Rotors, plugboard, mechanical encryption |
| CR03 | One-Time Pad | Perfect secrecy, XOR, key management |
| CR04 | DES | Feistel networks, S-boxes, block ciphers |
| CR05 | AES | Substitution-permutation networks, Galois fields |
| CR06 | Diffie-Hellman | Key exchange, discrete logarithm problem |
| CR07 | RSA | Public-key cryptography, prime factorization |
| CR08 | Elliptic Curve | ECC, point multiplication, smaller keys |
| CR09 | SHA-256 | Hash functions, collision resistance, Merkle-Damgård |
| CR10 | TLS Handshake | Protocol composition, hybrid encryption |
