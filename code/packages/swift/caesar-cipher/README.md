# CaesarCipher

A Swift implementation of the Caesar cipher, one of the oldest and simplest
encryption techniques in history. This package provides encryption, decryption,
ROT13, brute-force attack, and frequency analysis — all written in literate
programming style with extensive documentation suitable for learners.

Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures)
educational computing stack.

## What is the Caesar Cipher?

The Caesar cipher is a **substitution cipher** where each letter in the
plaintext is replaced by a letter a fixed number of positions further down
the alphabet. It is named after Julius Caesar, who reportedly used it with a
shift of 3 to communicate with his generals during military campaigns.

For example, with a shift of 3:

```
Plain:  A B C D E F G H I J K L M N O P Q R S T U V W X Y Z
Cipher: D E F G H I J K L M N O P Q R S T U V W X Y Z A B C
```

The word `HELLO` becomes `KHOOR`:

- H (position 7) + 3 = K (position 10)
- E (position 4) + 3 = H (position 7)
- L (position 11) + 3 = O (position 14)
- L (position 11) + 3 = O (position 14)
- O (position 14) + 3 = R (position 17)

The cipher wraps around: `X` shifted by 3 becomes `A`, `Y` becomes `B`,
and `Z` becomes `C`.

## Installation

Add this package as a dependency in your `Package.swift`:

```swift
dependencies: [
    .package(path: "../caesar-cipher"),
]
```

Then add `"CaesarCipher"` to your target's dependencies:

```swift
.target(
    name: "YourTarget",
    dependencies: ["CaesarCipher"]
),
```

## Usage

### Encryption and Decryption

The core functions `encrypt` and `decrypt` operate on strings with a given
shift value. Non-alphabetic characters (digits, spaces, punctuation) pass
through unchanged, and the case of each letter is preserved.

```swift
import CaesarCipher

// Basic encryption with shift of 3
let ciphertext = encrypt("HELLO", shift: 3)
// ciphertext == "KHOOR"

// Decryption reverses the process
let plaintext = decrypt("KHOOR", shift: 3)
// plaintext == "HELLO"

// Mixed case and punctuation are handled
let message = encrypt("Hello, World!", shift: 7)
// message == "Olssv, Dvysk!"

// Negative shifts go backward
let shifted = encrypt("abc", shift: -1)
// shifted == "zab"

// Shift of 26 wraps fully (identity)
let same = encrypt("Hello", shift: 26)
// same == "Hello"
```

### ROT13

ROT13 is a special case of the Caesar cipher with a shift of 13. Because 13
is exactly half of 26, applying ROT13 twice returns the original text. This
makes it uniquely self-inverse.

```swift
import CaesarCipher

let encoded = rot13("Hello, World!")
// encoded == "Uryyb, Jbeyq!"

let decoded = rot13(encoded)
// decoded == "Hello, World!"

// ROT13 is its own inverse
assert(rot13(rot13("any text")) == "any text")
```

ROT13 has been widely used in online communities since the early days of
Usenet to hide spoilers, puzzle answers, and mildly offensive jokes. It
provides no real security — it is trivially reversible — but it prevents
accidental reading of content.

### Brute-Force Attack

Since the Caesar cipher only has 26 possible shifts, an attacker can simply
try all of them. The `bruteForce` function returns all 26 possible
decryptions:

```swift
import CaesarCipher

let results = bruteForce("KHOOR")
// results is an array of 26 BruteForceResult values

for result in results {
    print("Shift \(result.shift): \(result.plaintext)")
}
// Shift 0: KHOOR
// Shift 1: JGNNQ
// Shift 2: IFMMP
// Shift 3: HELLO  <-- this one is readable!
// Shift 4: GDKKN
// ...
```

The human analyst inspects the results and picks the one that makes sense.
This demonstrates why the Caesar cipher is not secure: its key space (26
possible keys) is far too small.

### Frequency Analysis

For longer texts, frequency analysis can automatically determine the shift
without human inspection. It works by comparing the frequency of each letter
in the ciphertext against the known frequency distribution of English text.

```swift
import CaesarCipher

let ciphertext = encrypt(
    "the quick brown fox jumps over the lazy dog and the cat sat on the mat",
    shift: 3
)

let result = frequencyAnalysis(ciphertext)
// result.shift == 3
// result.plaintext == "the quick brown fox..."
```

The analysis uses the chi-squared statistic to measure how well each possible
shift matches the expected English letter distribution. The shift that
produces the lowest chi-squared value (best fit) is chosen.

Frequency analysis works best on longer texts (50+ letters). Very short
texts may not have enough statistical signal for reliable detection.

### English Letter Frequencies

The package exports a frequency table that you can use for your own analysis:

```swift
import CaesarCipher

// Access individual frequencies
let eFrequency = englishFrequencies["e"]!  // 0.127 (12.7%)
let zFrequency = englishFrequencies["z"]!  // 0.0007 (0.07%)

// The table covers all 26 lowercase letters
print(englishFrequencies.count)  // 26
```

The top five most frequent letters in English are E (12.7%), T (9.1%),
A (8.2%), O (7.5%), and I (7.0%). The five least frequent are J (0.15%),
X (0.15%), Q (0.10%), Z (0.07%), and K (0.77%).

## API Reference

### Functions

| Function | Description |
|----------|-------------|
| `encrypt(_:shift:)` | Encrypts plaintext by shifting each letter forward |
| `decrypt(_:shift:)` | Decrypts ciphertext by shifting each letter backward |
| `rot13(_:)` | Applies ROT13 encoding (shift of 13, self-inverse) |
| `bruteForce(_:)` | Returns all 26 possible decryptions |
| `frequencyAnalysis(_:)` | Automatically detects shift using letter frequency comparison |

### Types

| Type | Description |
|------|-------------|
| `BruteForceResult` | Holds a shift value and the corresponding decrypted plaintext |

### Constants

| Constant | Description |
|----------|-------------|
| `englishFrequencies` | Dictionary mapping each lowercase letter to its frequency in English text |

## How It Fits in the Stack

This package is part of the coding-adventures project, which builds an
educational computing stack from logic gates up through interpreters and
compilers. The Caesar cipher sits in the "classical algorithms" layer,
demonstrating fundamental concepts in cryptography:

- **Substitution ciphers**: Replacing each symbol with another
- **Modular arithmetic**: The mathematical foundation of the shift operation
- **Key space**: Why 26 possible keys is far too small for security
- **Cryptanalysis**: How frequency analysis breaks simple ciphers
- **Chi-squared statistic**: A practical application of statistics

These concepts form the foundation for understanding more sophisticated
ciphers (Vigenere, Enigma) and modern cryptography (AES, RSA).

## Design Decisions

1. **Free functions, not methods on a type**: The Caesar cipher is a simple
   algorithm, not a stateful object. Free functions like `encrypt(_:shift:)`
   are more idiomatic Swift for this use case and easier to compose.

2. **ASCII letters only**: The classical Caesar cipher operates on the
   26-letter Latin alphabet. Non-ASCII letters (accented characters, Cyrillic,
   etc.) pass through unchanged. A production cipher would need to define
   its own alphabet mapping.

3. **Int shift parameter**: The shift is an unbounded `Int` rather than being
   restricted to 0-25. This is more ergonomic (users don't need to normalize)
   and the modular arithmetic handles all values correctly, including negative
   shifts and values larger than 26.

4. **Sendable conformance**: `BruteForceResult` conforms to `Sendable`,
   making it safe to use in Swift's structured concurrency model.

## Development

```bash
# Run tests
swift test

# Run tests with verbose output
swift test --verbose

# Build only (no tests)
swift build
```

## License

Part of the coding-adventures project.
