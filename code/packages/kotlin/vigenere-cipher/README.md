# vigenere-cipher — Kotlin

## CR03 conformance

This implementation follows [CR03](../../../specs/CR03-vigenere-cipher.md): keys and cipher transforms use ASCII letters only, analysis ignores non-ASCII letters, and non-ASCII Unicode scalars pass through without advancing the key. Analysis accepts at most 8,192 Unicode scalars and key-length bounds up to 40, uses the smallest candidate within 90% of the best index-of-coincidence score, preserves the exact requested recovered-key length, and resolves score ties toward the smallest shift.

The native suite executes all 26 language-neutral `classical-ciphers-v1` Vigenere cases through a generated dependency-free consumer; `generate_vigenere_fixture_consumers.py --check` prevents corpus, expected-object, and established-lane drift.

The Vigenère cipher: the "unbreakable" polyalphabetic substitution cipher that
stumped cryptanalysts for three centuries. Also includes a full automatic
ciphertext-only attack using the Index of Coincidence and chi-squared analysis.

## Usage

```kotlin
import com.codingadventures.vigenerecipher.VigenereCipher

VigenereCipher.encrypt("ATTACKATDAWN", "LEMON")  // → "LXFOPVEFRNHR"
VigenereCipher.decrypt("LXFOPVEFRNHR", "LEMON")  // → "ATTACKATDAWN"

// Case-insensitive key, preserves input case, non-alpha passes through
VigenereCipher.encrypt("Hello, World!", "key")    // → "Rijvs, Uyvjn!"

// Fully automatic ciphertext-only attack (needs ≥ 200 alphabetic chars)
val result = VigenereCipher.breakCipher(ciphertext)
println("Key: ${result.key}")        // → "LEMON"
println("Plaintext: ${result.plaintext}")

// Step-by-step cryptanalysis
val keyLen = VigenereCipher.findKeyLength(ciphertext)   // → 5
val key    = VigenereCipher.findKey(ciphertext, keyLen) // → "LEMON"
```

## How it works

### Encryption

The keyword repeats to match the plaintext length. Each alphabetic character
is shifted by the corresponding key letter (A=0, B=1, …, Z=25). Non-alpha
characters are copied unchanged and do **not** advance the key position.

```
keyword:    L  E  M  O  N  L  E  M  O  N  L  E
plaintext:  A  T  T  A  C  K  A  T  D  A  W  N
shifts:    11  4 12 14 13 11  4 12 14 13 11  4
ciphertext: L  X  F  O  P  V  E  F  R  N  H  R
```

Formula:
- Encrypt: `C[i] = (P[i] + K[i mod len(K)]) mod 26`
- Decrypt: `P[i] = (C[i] - K[i mod len(K)] + 26) mod 26`

### Breaking it (Kasiski / Babbage method)

1. **Key length** — Index of Coincidence. The IC of a random text is ≈ 0.038;
   for English text ≈ 0.065. When the ciphertext is split into `L` groups
   (positions 0, L, 2L, …; 1, L+1, …; etc.) and `L` equals the key length,
   each group is a Caesar cipher and its IC is close to 0.065.

2. **Key letters** — Chi-squared frequency analysis. Each group is treated as
   a Caesar-cipher ciphertext; the shift minimising chi-squared against English
   letter frequencies gives the key letter for that position.

3. **Exact requested length** — Key recovery returns exactly the requested
   number of characters. It does not shorten repeated periods.

## Running Tests

```bash
gradle test
```

The suite covers encryption, decryption, roundtrip, non-alpha handling, input
validation, IC key-length detection, chi-squared key recovery, CR03 edge cases,
and the full end-to-end break.

## Part of the Coding Adventures series

Kotlin counterpart to the Python, Rust, Go, TypeScript, and Java
implementations.
