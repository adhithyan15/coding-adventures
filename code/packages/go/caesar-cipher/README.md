# caesar-cipher

A pure-Go implementation of the Caesar cipher — the oldest known substitution cipher — with brute-force decryption and statistical frequency analysis.

This package is part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) monorepo, a ground-up implementation of the computing stack from transistors to operating systems. The Caesar cipher sits at the very beginning of the cryptography track, illustrating foundational concepts in encryption, modular arithmetic, and cryptanalysis before moving on to more complex ciphers like Vigenere, substitution ciphers, and eventually modern symmetric/asymmetric cryptography.

## Historical Background

The Caesar cipher is named after Julius Caesar, who reportedly used it for confidential military correspondence around 58 BCE. According to Suetonius, Caesar used a shift of 3 when writing to his generals — replacing each letter with the letter three positions further in the alphabet. While laughably weak by modern standards, it was effective in an era when most adversaries were illiterate.

The cipher belongs to the family of **monoalphabetic substitution ciphers**: each letter in the plaintext is consistently replaced by a single corresponding letter in the ciphertext. This fixed mapping is both its simplicity and its fatal weakness — it preserves the statistical fingerprint of the original language.

The Caesar cipher remained in military use for centuries. Even as late as the American Civil War, Confederate forces used variants of it. Today it serves primarily as a teaching tool, introducing students to the core concepts of symmetric encryption, key spaces, and cryptanalysis.

## How It Works

### Encryption

The Caesar cipher operates on a simple principle: shift every letter in the plaintext by a fixed number of positions in the alphabet. Non-alphabetic characters (digits, spaces, punctuation) pass through unchanged.

Given a shift of 3:

```
Plaintext alphabet:  A B C D E F G H I J K L M N O P Q R S T U V W X Y Z
Ciphertext alphabet: D E F G H I J K L M N O P Q R S T U V W X Y Z A B C
```

The word "HELLO" encrypts as "KHOOR":

```
H (position  7) + 3 = position 10 → K
E (position  4) + 3 = position  7 → H
L (position 11) + 3 = position 14 → O
L (position 11) + 3 = position 14 → O
O (position 14) + 3 = position 17 → R
```

### The Mathematics: Modular Arithmetic

Mathematically, encryption and decryption are modular addition and subtraction over the integers modulo 26:

```
Encryption: E(x) = (x + shift) mod 26
Decryption: D(x) = (x - shift) mod 26
```

where x is the zero-indexed position of a letter (A=0, B=1, ..., Z=25). The modular arithmetic ensures that the alphabet "wraps around" — shifting Z by 1 gives A, not some character beyond Z.

In Go, the `%` operator can return negative values for negative operands (e.g., `-3 % 26 = -3`), so we use the standard trick of adding 26 before taking the modulus:

```go
shifted := ((position + shift) % 26 + 26) % 26
```

This guarantees a result in the range [0, 25] regardless of whether the shift is positive or negative.

### Decryption

Decryption is simply encryption with the negated shift:

```go
Decrypt(text, shift) = Encrypt(text, -shift)
```

This works because shifting forward by `n` and then backward by `n` returns you to the starting position: `(x + n - n) mod 26 = x`.

### ROT13: A Special Case

ROT13 is the Caesar cipher with shift = 13. Because 13 is exactly half of 26 (the alphabet length), ROT13 is its own inverse: applying it twice returns the original text. This makes it a symmetric involution — you use the same operation for both encryption and decryption.

```
ROT13(ROT13("Hello")) = "Hello"
```

ROT13 was widely used on Usenet newsgroups in the 1980s and 1990s to hide spoilers, punchlines, and potentially offensive content. The reader had to consciously apply ROT13 to reveal the hidden text.

## Breaking the Caesar Cipher

The Caesar cipher is trivially breakable because its key space is minuscule: there are only 25 possible non-trivial shifts (1 through 25). This package provides two attack methods.

### Brute Force

The simplest attack: try all 25 possible shifts and inspect the results. A human can instantly spot which decryption produces readable text. This package's `BruteForce` function returns all 25 candidates as a slice of `BruteForceResult` structs.

### Frequency Analysis

A more elegant approach that works automatically without human inspection. English text has a distinctive letter frequency distribution — E appears about 12.7% of the time, T about 9.1%, A about 8.2%, and so on. When text is Caesar-shifted, these frequencies remain intact but move to different letters.

The `FrequencyAnalysis` function tries each possible shift, computes the letter frequency distribution of the resulting plaintext, and compares it to the expected English distribution using the chi-squared (χ²) goodness-of-fit statistic:

```
χ² = Σ (observed_i - expected_i)² / expected_i
```

The shift producing the lowest χ² value is the best match for English. This technique works reliably on texts of 50 or more characters and is a cornerstone of classical cryptanalysis.

## API Reference

### Types

```go
type BruteForceResult struct {
    Shift     int    // The shift value used (1-25)
    Plaintext string // The decrypted text for this shift
}
```

### Functions

#### `Encrypt(text string, shift int) string`

Encrypts plaintext using the Caesar cipher with the given shift. Each letter is shifted forward by `shift` positions in the alphabet (wrapping from Z to A). Non-alphabetic characters pass through unchanged. Case is preserved.

```go
caesarcipher.Encrypt("Hello, World!", 3)  // → "Khoor, Zruog!"
caesarcipher.Encrypt("xyz", 3)            // → "abc" (wraps around)
caesarcipher.Encrypt("ABC 123", 1)        // → "BCD 123"
```

#### `Decrypt(text string, shift int) string`

Decrypts ciphertext by reversing the Caesar shift. Equivalent to `Encrypt(text, -shift)`.

```go
caesarcipher.Decrypt("Khoor, Zruog!", 3)  // → "Hello, World!"
```

#### `Rot13(text string) string`

Applies ROT13 (Caesar shift of 13). Self-inverse: `Rot13(Rot13(x)) == x`.

```go
caesarcipher.Rot13("Hello")   // → "Uryyb"
caesarcipher.Rot13("Uryyb")   // → "Hello"
```

#### `BruteForce(ciphertext string) []BruteForceResult`

Tries all 25 possible shifts (1 through 25) and returns a slice of results. Useful when you want to manually inspect candidates or when the ciphertext is too short for frequency analysis.

```go
results := caesarcipher.BruteForce("KHOOR")
for _, r := range results {
    fmt.Printf("Shift %2d: %s\n", r.Shift, r.Plaintext)
}
// Shift  1: JGNNQ
// Shift  2: IFMMP
// Shift  3: HELLO  ← correct!
// ...
```

#### `FrequencyAnalysis(ciphertext string) (int, string)`

Automatically detects the most likely shift by comparing letter frequencies against expected English frequencies using the chi-squared statistic. Returns the detected shift and the corresponding plaintext.

```go
shift, plaintext := caesarcipher.FrequencyAnalysis(longCiphertext)
fmt.Printf("Detected shift: %d\nPlaintext: %s\n", shift, plaintext)
```

#### `EnglishFrequencies` (variable)

A `map[rune]float64` containing the expected frequency of each letter (A-Z) in English text. Useful if you want to implement your own frequency analysis variants.

```go
fmt.Printf("E frequency: %.4f\n", caesarcipher.EnglishFrequencies['E'])  // 0.1270
```

## Where It Fits

This package is the first in the cryptography series within coding-adventures:

1. **Caesar Cipher** (this package) — monoalphabetic substitution, modular arithmetic
2. Vigenere Cipher — polyalphabetic substitution, repeated key
3. Substitution Cipher — arbitrary letter mappings, full frequency analysis
4. Transposition Ciphers — rearranging rather than substituting
5. Modern Symmetric Ciphers — AES, DES, block cipher modes
6. Asymmetric Cryptography — RSA, Diffie-Hellman, elliptic curves

Each package builds on concepts introduced in the previous ones. The Caesar cipher introduces the fundamental ideas of keys, encryption, decryption, key spaces, and cryptanalysis that carry through the entire series.

## Running Tests

```bash
# Run all tests with verbose output and coverage
go test ./... -v -cover

# Run benchmarks
go test -bench=. -benchmem

# Using the monorepo build system
bash BUILD
```

## License

Part of the coding-adventures monorepo. See the repository root for license information.
