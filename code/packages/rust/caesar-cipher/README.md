# caesar-cipher

Caesar cipher -- the oldest substitution cipher, with brute-force and frequency analysis.

## What is a Caesar cipher?

The Caesar cipher is one of the simplest and oldest encryption techniques in recorded history.  It is named after Julius Caesar, who reportedly used it in his private correspondence around 58 BC.  The idea is elegant in its simplicity: every letter in your message is replaced by the letter a fixed number of positions further along in the alphabet.

For example, with a shift of 3:

- A becomes D
- B becomes E
- C becomes F
- ...
- X becomes A (wrapping around the end)
- Y becomes B
- Z becomes C

The full substitution table for shift 3 looks like this:

```text
Plain:    A B C D E F G H I J K L M N O P Q R S T U V W X Y Z
Cipher:   D E F G H I J K L M N O P Q R S T U V W X Y Z A B C
```

So the word "HELLO" becomes "KHOOR":

```text
H -> K  (position 7,  7+3 = 10 -> K)
E -> H  (position 4,  4+3 =  7 -> H)
L -> O  (position 11, 11+3 = 14 -> O)
L -> O  (position 11, 11+3 = 14 -> O)
O -> R  (position 14, 14+3 = 17 -> R)
```

To decrypt, we shift each letter backwards by the same amount: "KHOOR" with shift 3 in reverse gives back "HELLO".

## The mathematics

If we assign each letter a number (A=0, B=1, ..., Z=25), then encryption and decryption are simple modular arithmetic operations:

```text
Encrypt:  E(x) = (x + shift) mod 26
Decrypt:  D(x) = (x - shift) mod 26  =  (x + 26 - shift) mod 26
```

The `mod 26` operation is what makes the alphabet "wrap around" -- when we go past Z, we come back to A.  This is the same principle behind a clock: 10 o'clock plus 5 hours is 3 o'clock (because (10 + 5) mod 12 = 3).

### Handling negative shifts

A negative shift moves letters backwards.  Shift -1 turns B into A and A into Z.  Internally, we normalise negative shifts using the formula:

```text
normalised = ((shift % 26) + 26) % 26
```

This converts any integer shift into the range 0..25.  For example:
- shift = -1 becomes 25
- shift = -27 becomes 25 (same as -1)
- shift = 29 becomes 3

### What about non-letters?

Digits, spaces, punctuation, and any other non-alphabetic characters pass through the cipher unchanged.  Only ASCII letters A-Z and a-z are transformed.  The case of each letter is preserved: an uppercase input produces an uppercase output.

## ROT13 -- a special case

ROT13 is the Caesar cipher with shift 13.  It has a delightful mathematical property: because 13 + 13 = 26 (the size of the alphabet), applying ROT13 twice returns you to the original text.  In other words, ROT13 is its own inverse:

```text
rot13(rot13("Hello")) = "Hello"
```

ROT13 was historically popular on Usenet newsgroups in the 1980s and 1990s for hiding spoilers and punchlines.  It provides no real cryptographic security -- its purpose is light obfuscation, a "do not read" wrapper for text that the reader can trivially decode if they choose to.

## Breaking the cipher

The Caesar cipher is extremely weak by modern standards.  With only 25 possible non-trivial keys (shifts 1 through 25), it can be broken in two ways:

### Brute force

Simply try all 25 shifts and look for the one that produces readable text.  This crate's `brute_force` function returns all 25 candidates, letting the caller (human or program) pick the correct one.

For example, given the ciphertext "KHOOR":

```text
Shift  1: JGNNQ
Shift  2: IFMMP
Shift  3: HELLO   <-- this one looks like English!
Shift  4: GDKKN
...
Shift 25: LIPPS
```

### Frequency analysis

A more sophisticated approach compares the letter frequencies in the ciphertext against the known frequency distribution of English text.  In a large sample of English:

- E is the most common letter (~12.7%)
- T is second (~9.1%)
- A is third (~8.2%)
- Z is the rarest (~0.07%)

The Caesar cipher preserves these relative frequencies -- it just shifts them.  If we decrypt with the correct key, the resulting letter distribution should closely match the expected English distribution.

We measure "closeness" using the chi-squared statistic:

```text
chi2 = SUM_i (observed_i - expected_i)^2 / expected_i
```

where `observed_i` is the count of letter i in the candidate plaintext and `expected_i` is the count predicted by the English frequency table scaled to the text length.  A lower chi-squared value means a better fit.

The `frequency_analysis` function tries all 25 shifts, computes chi-squared for each, and returns the shift with the lowest score.  This works well for texts of 50+ characters; shorter texts may not have enough statistical signal for reliable detection.

## Usage

Add the crate to your project:

```toml
[dependencies]
caesar-cipher = { path = "../caesar-cipher" }
```

### Encrypting and decrypting

```rust
use caesar_cipher::cipher;

// Encrypt with shift 3
let ciphertext = cipher::encrypt("Hello, World!", 3);
assert_eq!(ciphertext, "Khoor, Zruog!");

// Decrypt reverses the operation
let plaintext = cipher::decrypt(&ciphertext, 3);
assert_eq!(plaintext, "Hello, World!");

// ROT13 is its own inverse
let encoded = cipher::rot13("Spoiler alert");
let decoded = cipher::rot13(&encoded);
assert_eq!(decoded, "Spoiler alert");
```

### Breaking a cipher with brute force

```rust
use caesar_cipher::analysis;

let ciphertext = "KHOOR ZRUOG";
let results = analysis::brute_force(ciphertext);

// Inspect all 25 candidates
for result in &results {
    println!("Shift {:2}: {}", result.shift, result.plaintext);
}
// Shift 3 will show "HELLO WORLD"
```

### Automatic detection with frequency analysis

```rust
use caesar_cipher::analysis;

let ciphertext = "WKH TXLFN EURZQ IRA MXPSV RYHU WKH ODCB GRJ";
let (shift, plaintext) = analysis::frequency_analysis(ciphertext);
println!("Detected shift: {}", shift);       // 3
println!("Decoded text: {}", plaintext);     // THE QUICK BROWN FOX JUMPS OVER THE LAZY DOG
```

## API reference

### `cipher` module

| Function | Signature | Description |
|----------|-----------|-------------|
| `encrypt` | `(text: &str, shift: i32) -> String` | Shift each letter forward, preserving case |
| `decrypt` | `(text: &str, shift: i32) -> String` | Reverse an encryption (shift backward) |
| `rot13` | `(text: &str) -> String` | Encrypt with shift 13 (self-inverse) |

### `analysis` module

| Item | Type | Description |
|------|------|-------------|
| `ENGLISH_FREQUENCIES` | `[f64; 26]` | Expected letter frequencies for English (A..Z) |
| `BruteForceResult` | struct | Pairs a candidate shift with its resulting plaintext |
| `brute_force` | `(ciphertext: &str) -> Vec<BruteForceResult>` | Try all 25 shifts |
| `frequency_analysis` | `(ciphertext: &str) -> (i32, String)` | Chi-squared best-fit detection |

## Historical context

The Caesar cipher belongs to the family of **monoalphabetic substitution ciphers** -- ciphers where each letter in the plaintext maps to exactly one letter in the ciphertext.  While trivially breakable today, it represents an important conceptual milestone: the idea that a message can be transformed using a secret key and later recovered by someone who knows that key.

More complex substitution ciphers (like the Vigenere cipher, which uses a repeating keyword to vary the shift for each letter) and polyalphabetic ciphers built on this foundation, eventually leading to mechanical cipher machines like Enigma and, ultimately, to the mathematical encryption systems we use today.

## Part of coding-adventures

This crate is part of the **coding-adventures** monorepo, a ground-up implementation of the computing stack from transistors to operating systems.  It lives in the cryptography layer, demonstrating fundamental cipher concepts before building up to more sophisticated encryption algorithms.

## Development

```bash
# Run all tests
cargo test -p caesar-cipher

# Run tests with output
cargo test -p caesar-cipher -- --nocapture

# Build the package
bash BUILD
```
