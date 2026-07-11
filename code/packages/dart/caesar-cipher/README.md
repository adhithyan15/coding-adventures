# coding_adventures_caesar_cipher

The Caesar cipher — a classic shift substitution cipher — implemented in pure
Dart, together with the two classic attacks that break it.

This is the Dart port of the `caesar-cipher` package that already exists in
Rust, Python, Ruby, Go, and many other languages in the coding-adventures
monorepo. Behaviour is identical across ports (same worked examples, same
frequency table, same round-trip guarantees).

## What it does

| Function | Purpose |
|---|---|
| `encrypt(text, shift)` | Shift each letter forward by `shift`; non-letters unchanged. |
| `decrypt(text, shift)` | Inverse of `encrypt` (shifts backward). |
| `rot13(text)` | Shift-13 special case; its own inverse. |
| `bruteForce(ciphertext)` | All 25 candidate decryptions for a human to scan. |
| `frequencyAnalysis(ciphertext)` | Auto-recover the shift via chi-squared scoring. |
| `englishFrequencies` | The 26-entry English letter-frequency table. |

## How the shift works

Letters are numbered A=0 … Z=25. Encryption maps position `p` to
`(p + shift) mod 26`; decryption to `(p − shift) mod 26`. Case is preserved,
and digits, spaces, punctuation, and any non-ASCII characters pass through
unchanged.

```
shift = 3:   A→D  B→E  …  X→A  Y→B  Z→C
```

Negative and out-of-range shifts are normalised into 0..25, so `shift = -1`
equals `shift = 25`, and `shift = 26` is the identity.

## Usage

```dart
import 'package:coding_adventures_caesar_cipher/coding_adventures_caesar_cipher.dart';

void main() {
  final ct = encrypt('Attack at dawn!', 3);
  print(ct);              // → 'Dwwdfn dw gdzq!'
  print(decrypt(ct, 3));  // → 'Attack at dawn!'
  print(rot13('Hello'));  // → 'Uryyb'  (rot13 again → 'Hello')

  // Break it without the key:
  final r = frequencyAnalysis(encrypt('THE QUICK BROWN FOX', 7));
  print('${r.shift}: ${r.plaintext}'); // → '7: THE QUICK BROWN FOX'
}
```

## Breaking the cipher

The Caesar cipher has only 25 non-trivial keys, so it is trivially broken:

- **Brute force** — `bruteForce` returns all 25 candidate plaintexts (shift 1
  through 25). A human eye picks out the coherent one instantly.
- **Frequency analysis** — `frequencyAnalysis` compares each candidate's letter
  distribution against English using the chi-squared statistic
  `Σ (observedᵢ − expectedᵢ)² / expectedᵢ` and returns the best-fitting shift.
  This works automatically but needs enough text (50+ characters is reliable);
  it assumes English and falls back to shift 1 when there is no letter signal.

## Running the tests

```
dart pub get
dart test
```
