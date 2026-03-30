# @coding-adventures/caesar-cipher

A complete TypeScript implementation of the Caesar cipher, including encryption, decryption, ROT13, brute-force attacks, and frequency analysis. Written in literate programming style with extensive documentation aimed at learners.

## What Is the Caesar Cipher?

The Caesar cipher is one of the earliest known encryption techniques, dating back to ancient Rome. Julius Caesar reportedly used a shift of 3 to protect his military communications. The idea is straightforward: replace every letter in a message with the letter that is a fixed number of positions further along in the alphabet. When you reach the end of the alphabet, wrap around to the beginning.

For example, with a shift of 3:
- A becomes D
- B becomes E
- X becomes A (wrapping around)
- Y becomes B
- Z becomes C

This transforms "HELLO" into "KHOOR". To decrypt, you simply shift in the opposite direction.

## Why Study the Caesar Cipher?

The Caesar cipher is trivially breakable by modern standards, but it remains one of the most important ciphers to study for several reasons:

1. **Foundation of cryptography**: It introduces the core concept of substitution ciphers, where each letter is replaced by another according to a fixed rule. Understanding this prepares you for more sophisticated ciphers like Vigenere and Enigma.

2. **Modular arithmetic**: The wrap-around behavior of the Caesar cipher is a perfect introduction to modular arithmetic, which is fundamental to all modern cryptography (RSA, AES, elliptic curves, etc.).

3. **Frequency analysis**: The Caesar cipher was one of the first ciphers broken using statistical analysis. Learning to break it teaches you the principles behind much more advanced cryptanalysis.

4. **Character encoding**: Implementing the cipher requires understanding how computers represent text (ASCII/Unicode code points), which is essential knowledge for any programmer.

## How It Works

### The Alphabet as a Number Line

We assign each letter a position from 0 to 25:

```
A=0  B=1  C=2  D=3  E=4  F=5  G=6  H=7  I=8  J=9  K=10 L=11 M=12
N=13 O=14 P=15 Q=16 R=17 S=18 T=19 U=20 V=21 W=22 X=23 Y=24 Z=25
```

### Encryption Formula

To encrypt a letter at position `p` with shift `s`:

```
encrypted_position = (p + s) mod 26
```

The `mod 26` handles the wrap-around. For example, `Y` (position 24) with shift 3: `(24 + 3) mod 26 = 27 mod 26 = 1`, which is `B`.

### Decryption Formula

Decryption reverses the process:

```
decrypted_position = (p - s) mod 26
```

Or equivalently, decrypt is just encrypt with the negated shift.

### Handling Non-Letters

Spaces, digits, punctuation, and other non-alphabetic characters pass through unchanged. The Caesar cipher historically only operated on letters. This implementation also preserves letter case: uppercase letters remain uppercase, lowercase remain lowercase.

## Installation

This package has zero runtime dependencies.

```bash
npm install @coding-adventures/caesar-cipher
```

## Usage

### Basic Encryption and Decryption

```typescript
import { encrypt, decrypt } from "@coding-adventures/caesar-cipher";

// Encrypt with shift 3 (Caesar's own choice)
const ciphertext = encrypt("HELLO WORLD", 3);
console.log(ciphertext); // "KHOOR ZRUOG"

// Decrypt by providing the same shift
const plaintext = decrypt("KHOOR ZRUOG", 3);
console.log(plaintext); // "HELLO WORLD"

// Case is preserved, non-letters pass through
encrypt("Hello, World! 123", 3); // "Khoor, Zruog! 123"

// Negative shifts work too
encrypt("ABC", -1); // "ZAB"

// Shift of 26 is the identity (full rotation)
encrypt("HELLO", 26); // "HELLO"
```

### ROT13

ROT13 is a special case of the Caesar cipher with shift 13. Because 13 is exactly half of 26, ROT13 is its own inverse: applying it twice returns the original text.

```typescript
import { rot13 } from "@coding-adventures/caesar-cipher";

rot13("HELLO");          // "URYYB"
rot13("URYYB");          // "HELLO"
rot13(rot13("SECRET"));  // "SECRET" (self-inverse)
```

ROT13 was historically used on Usenet forums to hide spoilers and punchlines. A reader who wanted to see the hidden text could simply apply ROT13 again.

### Breaking the Cipher with Brute Force

Since there are only 25 possible non-trivial shifts, you can try them all:

```typescript
import { bruteForce } from "@coding-adventures/caesar-cipher";

const results = bruteForce("KHOOR");
// Returns 25 results, one for each shift 1-25
// results[2] = { shift: 3, plaintext: "HELLO" }

for (const { shift, plaintext } of results) {
  console.log(`Shift ${shift}: ${plaintext}`);
}
```

### Breaking the Cipher with Frequency Analysis

For longer texts, frequency analysis can automatically identify the correct shift by comparing letter frequencies against known English frequencies:

```typescript
import { frequencyAnalysis } from "@coding-adventures/caesar-cipher";

const ciphertext = encrypt(
  "The quick brown fox jumps over the lazy dog",
  7
);

const result = frequencyAnalysis(ciphertext);
console.log(result.shift);     // 7
console.log(result.plaintext); // "The quick brown fox jumps over the lazy dog"
```

The algorithm uses the chi-squared statistic to measure how well the decrypted text's letter distribution matches expected English frequencies. The shift that produces the best match (lowest chi-squared value) is selected as the answer.

Note: Frequency analysis works best on longer texts (50+ characters). For very short messages, the letter distribution may not be statistically representative, and brute force with human inspection is more reliable.

## API Reference

### `encrypt(text: string, shift: number): string`

Encrypts a string by shifting each letter forward by `shift` positions. Non-letters pass through unchanged. Case is preserved.

### `decrypt(text: string, shift: number): string`

Decrypts a Caesar cipher by shifting each letter backward by `shift` positions. Equivalent to `encrypt(text, -shift)`.

### `rot13(text: string): string`

Applies ROT13 encoding (Caesar cipher with shift 13). Self-inverse: `rot13(rot13(x)) === x`.

### `bruteForce(ciphertext: string): BruteForceResult[]`

Returns all 25 possible decryptions (shifts 1-25). Each result contains `{ shift, plaintext }`.

### `frequencyAnalysis(ciphertext: string): { shift: number; plaintext: string }`

Uses chi-squared frequency analysis to automatically determine the most likely shift for English text.

### `ENGLISH_FREQUENCIES: Record<string, number>`

A table of expected letter frequencies in English text, expressed as proportions (0.0 to 1.0).

### `BruteForceResult` (interface)

```typescript
interface BruteForceResult {
  shift: number;     // The shift value (1-25)
  plaintext: string; // The decrypted text using this shift
}
```

## How This Package Fits in the Stack

This package is part of the coding-adventures monorepo, which builds the computing stack from the ground up. The Caesar cipher sits at the intersection of several fundamental concepts:

- **Modular arithmetic** (from the arithmetic package) provides the mathematical foundation
- **Character encoding** (ASCII/Unicode) connects abstract math to real text processing
- **Statistical analysis** (frequency analysis) introduces the idea that natural language has exploitable patterns
- **Algorithm design** (brute force vs. frequency analysis) illustrates the tradeoff between exhaustive search and intelligent heuristics

## Development

```bash
# Install dependencies
npm install

# Run tests
npx vitest run

# Run tests with coverage
npx vitest run --coverage

# Build (compile TypeScript)
npx tsc
```

## License

MIT
