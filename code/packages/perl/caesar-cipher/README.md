# CodingAdventures::CaesarCipher

A Perl implementation of the Caesar cipher with brute-force and frequency analysis, written in literate programming style. This is the first Perl package in the coding-adventures monorepo.

## What is the Caesar Cipher?

The Caesar cipher is one of the oldest known encryption techniques, dating back to approximately 58 BCE. Julius Caesar used it to protect sensitive military correspondence during the Gallic Wars. The method is elegantly simple: each letter in the plaintext message is replaced by a letter a fixed number of positions further down the alphabet.

For example, with a shift of 3, the letter A becomes D, B becomes E, C becomes F, and so on. When you reach the end of the alphabet, it wraps around: X becomes A, Y becomes B, Z becomes C. This wrapping is handled mathematically using modular arithmetic -- the same math that makes clock arithmetic work (after 12 comes 1, not 13).

The cipher preserves the case of each letter and leaves non-alphabetic characters (spaces, digits, punctuation) completely unchanged. This means the structure of the message is visible even in ciphertext, which is one of the reasons the cipher is so easy to break.

## How It Fits in the Stack

This package belongs to the cryptography layer of the coding-adventures educational computing stack. It demonstrates fundamental concepts that appear throughout computer science and security:

- **Modular arithmetic**: The foundation of modern cryptography, from RSA to elliptic curves
- **Frequency analysis**: A statistical technique used in everything from codebreaking to natural language processing
- **Brute-force search**: The simplest form of cryptanalysis, illustrating why key space size matters
- **Substitution ciphers**: The ancestor of modern symmetric encryption algorithms

Understanding why the Caesar cipher is trivially breakable helps build intuition for what makes modern encryption strong: large key spaces, diffusion, and confusion.

## Installation

This package uses standard Perl module conventions. To install dependencies:

```bash
cpanm --installdeps .
```

Or if you are working within the coding-adventures monorepo, the BUILD file handles this automatically.

## Usage

### Encrypting a Message

The `encrypt` function takes a plaintext string and a shift value (an integer). It shifts each letter forward by that many positions in the alphabet:

```perl
use CodingAdventures::CaesarCipher;

my $ciphertext = CodingAdventures::CaesarCipher::encrypt("Hello, World!", 3);
# Result: "Khoor, Zruog!"

# Uppercase letters shift within A-Z, lowercase within a-z
my $mixed = CodingAdventures::CaesarCipher::encrypt("Attack at Dawn", 5);
# Result: "Fyyfhp fy Ifbs"
```

The shift is normalized modulo 26, so a shift of 29 is the same as a shift of 3. Negative shifts are also supported and shift letters backward.

### Decrypting a Message

Decryption reverses the encryption by shifting in the opposite direction. You need to know the original shift value:

```perl
my $plaintext = CodingAdventures::CaesarCipher::decrypt("Khoor, Zruog!", 3);
# Result: "Hello, World!"

# Decryption is encryption with the negated shift
# decrypt(text, 3) is equivalent to encrypt(text, -3)
```

Encrypt and decrypt are perfect inverses -- for any text and shift:

```perl
my $original = "Any text at all! 123";
my $shift = 17;
my $encrypted = CodingAdventures::CaesarCipher::encrypt($original, $shift);
my $decrypted = CodingAdventures::CaesarCipher::decrypt($encrypted, $shift);
# $decrypted eq $original  -- always true
```

### ROT13

ROT13 is a special case of the Caesar cipher with a shift of exactly 13. Because the English alphabet has 26 letters, applying ROT13 twice returns the original text -- it is its own inverse:

```perl
my $hidden = CodingAdventures::CaesarCipher::rot13("spoiler alert");
# Result: "fcbvyre nyreg"

my $revealed = CodingAdventures::CaesarCipher::rot13($hidden);
# Result: "spoiler alert"  -- back to the original!
```

ROT13 was historically popular on Usenet forums for hiding spoilers and joke punchlines. Readers who wanted to see the hidden text could apply ROT13 themselves.

### Brute-Force Attack

Since the Caesar cipher has only 25 meaningful keys (shifts 1 through 25), an attacker can simply try all of them:

```perl
my @results = CodingAdventures::CaesarCipher::brute_force("Khoor");

for my $result (@results) {
    printf "Shift %2d: %s\n", $result->{shift}, $result->{text};
}

# Output:
# Shift  1: Jgnnq
# Shift  2: Ifmmp
# Shift  3: Hello    <-- this one is readable!
# Shift  4: Gdkkn
# ...
# Shift 25: Lipps
```

The function returns a list of 25 hash references, each containing the shift value and the resulting decrypted text. A human or program can then identify which result is readable English.

### Frequency Analysis

For longer texts, frequency analysis provides a more automated approach. The function analyzes the letter frequency distribution of the ciphertext and compares it to known English letter frequencies using the chi-squared statistic:

```perl
my $long_ciphertext = CodingAdventures::CaesarCipher::encrypt(
    "the quick brown fox jumps over the lazy dog and "
    . "the five boxing wizards jump quickly",
    7
);

my $detected_shift = CodingAdventures::CaesarCipher::frequency_analysis($long_ciphertext);
# Result: 7  -- correctly identifies the shift!
```

Frequency analysis works because English text has a distinctive letter distribution: E appears about 12.7% of the time, T about 9.1%, and Z less than 0.1%. The Caesar cipher shifts this distribution but does not flatten it, making it detectable.

The chi-squared statistic measures how far the observed distribution is from the expected one. For each possible shift, we compute the chi-squared score, and the shift with the lowest score (closest match to English) wins.

Limitations of frequency analysis:

- It requires enough text for statistical patterns to emerge (at least 50 characters of letters works well; shorter texts may give incorrect results)
- It assumes the plaintext is English. Other languages have different frequency distributions
- Texts with unusual letter distributions (e.g., lipograms that avoid the letter E) may confuse the analysis

## How the Math Works

The Caesar cipher operates in the mathematical group Z/26Z (integers modulo 26). Each letter maps to a number: A=0, B=1, ..., Z=25. Encryption and decryption are:

```
encrypt(p, s) = (p + s) mod 26
decrypt(c, s) = (c - s) mod 26
```

In Perl, we use `ord()` to convert a character to its ASCII code, perform the arithmetic, and then use `chr()` to convert back. The ASCII offset for uppercase A is 65 and for lowercase a is 97:

```perl
# Encrypting a single uppercase character:
my $position = ord($char) - ord('A');        # 0-25
my $shifted  = ($position + $shift) % 26;    # still 0-25
my $result   = chr($shifted + ord('A'));      # back to A-Z
```

The modulo operator ensures wrapping: shifting Z (position 25) by 1 gives (25+1) % 26 = 0, which maps back to A.

## Running Tests

The test suite covers module loading, encryption/decryption round-trips, edge cases, and cryptanalysis:

```bash
# Using prove (Perl's test runner)
prove -l -v t/

# Or via the BUILD file
bash BUILD
```

The tests are organized into three files:

- `t/00-load.t` -- verifies the module loads and has a version
- `t/01-cipher.t` -- tests encrypt, decrypt, and rot13 with various inputs
- `t/02-analysis.t` -- tests brute force and frequency analysis

## Why is the Caesar Cipher Insecure?

The Caesar cipher has only 25 possible keys. Even without a computer, a person can try all 25 shifts in a few minutes. Modern encryption algorithms like AES have key spaces of 2^128 or 2^256 possible keys -- numbers so large that brute force is physically impossible even with all the computing power in the universe.

Beyond key space size, the Caesar cipher suffers from another fatal flaw: it preserves the frequency distribution of the plaintext. Every occurrence of E in the plaintext maps to the same letter in the ciphertext. Modern ciphers use techniques like diffusion (spreading the influence of each plaintext bit across many ciphertext bits) and confusion (making the relationship between the key and ciphertext complex) to resist frequency analysis.

Despite its weaknesses, the Caesar cipher remains an excellent teaching tool. It introduces the core concepts of encryption, decryption, key space, and cryptanalysis that underpin all of modern cryptography.

## License

MIT
