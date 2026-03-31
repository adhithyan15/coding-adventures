# CodingAdventures::AtbashCipher

A Perl implementation of the Atbash cipher, one of the oldest known substitution ciphers. This package is part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) monorepo.

## What is the Atbash Cipher?

The Atbash cipher is a monoalphabetic substitution cipher that was originally used with the Hebrew alphabet. The name "Atbash" derives from the first, last, second, and second-to-last letters of the Hebrew alphabet: **A**leph, **T**av, **B**eth, **Sh**in.

The cipher works by replacing each letter with the letter at the opposite end of the alphabet:

```
Plain:  A B C D E F G H I J K L M N O P Q R S T U V W X Y Z
Cipher: Z Y X W V U T S R Q P O N M L K J I H G F E D C B A
```

### The Formula

Given a letter at position `p` (where A=0, B=1, ..., Z=25):

```
encrypted_position = 25 - p
```

For example, 'H' is at position 7: `25 - 7 = 18`, which is 'S'. So 'H' encrypts to 'S'.

### Self-Inverse Property

The most elegant property of the Atbash cipher is that it is **self-inverse**: applying the cipher twice returns the original text. Mathematically:

```
f(f(x)) = 25 - (25 - x) = x
```

This means encryption and decryption are the **same operation**. If you encrypt "HELLO" to get "SVOOL", then encrypting "SVOOL" gives you back "HELLO".

### Historical Context

The Atbash cipher appears in the Hebrew Bible. In the Book of Jeremiah (25:26 and 51:41), the name "Sheshach" is used as an Atbash encoding of "Babel" (Babylon). This is one of the earliest documented uses of cryptography in literature.

While trivially breakable by modern standards (there is only one possible key), the Atbash cipher is a foundational concept in cryptography and serves as an excellent introduction to substitution ciphers.

## Installation

```bash
cpanm CodingAdventures::AtbashCipher
```

Or from source:

```bash
perl Makefile.PL
make
make test
make install
```

## Usage

```perl
use CodingAdventures::AtbashCipher qw(encrypt decrypt);

# Basic encryption
my $encrypted = encrypt("HELLO");
print $encrypted;  # "SVOOL"

# Case is preserved
print encrypt("Hello, World!");  # "Svool, Dliow!"

# Non-alphabetic characters pass through
print encrypt("Test 123!");  # "Gvhg 123!"

# Decryption is the same operation (self-inverse)
print decrypt("SVOOL");  # "HELLO"

# Prove it's self-inverse
my $original = "The quick brown fox";
print encrypt(encrypt($original)) eq $original ? "yes" : "no";  # "yes"
```

## API Reference

### `encrypt($text) -> $encrypted`

Encrypt text using the Atbash cipher. Each letter is replaced by its reverse in the alphabet (A<->Z, B<->Y, etc.). Non-alphabetic characters pass through unchanged. Case is preserved.

### `decrypt($text) -> $decrypted`

Decrypt text using the Atbash cipher. Because the cipher is self-inverse, this function is identical to `encrypt()`. It exists for API clarity.

## How It Works

The Perl implementation uses the `tr///` (transliterate) operator, which is the most idiomatic and efficient way to do character-by-character substitution in Perl. The expression `$result =~ tr/A-Za-z/Z-Az-a/` replaces each letter in the input range with the corresponding letter in the output range, naturally reversing the alphabet.

This is both more readable and more performant than looping through characters individually, as `tr///` is implemented at the C level in Perl's runtime.

## Testing

```bash
prove -l -v t/
```

The test suite uses Test2::V0 and covers:
- Known encryption pairs (HELLO -> SVOOL, etc.)
- Case preservation for upper and lowercase
- Non-alphabetic character passthrough (digits, punctuation, whitespace)
- Self-inverse property for multiple inputs
- Full alphabet mapping in both directions
- Edge cases: empty string, single characters
- No letter maps to itself (verified for all 52 letters)
- Decrypt/encrypt equivalence

## License

MIT
