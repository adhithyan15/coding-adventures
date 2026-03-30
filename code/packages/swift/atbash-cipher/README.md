# AtbashCipher

A Swift implementation of the Atbash cipher, one of the oldest known substitution ciphers. This package is part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) monorepo.

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

Add to your `Package.swift` dependencies:

```swift
dependencies: [
    .package(path: "../atbash-cipher"),
]
```

## Usage

```swift
import AtbashCipher

// Basic encryption
let encrypted = AtbashCipher.encrypt("HELLO")
print(encrypted) // "SVOOL"

// Case is preserved
print(AtbashCipher.encrypt("Hello, World!")) // "Svool, Dliow!"

// Non-alphabetic characters pass through
print(AtbashCipher.encrypt("Test 123!")) // "Gvhg 123!"

// Decryption is the same operation (self-inverse)
print(AtbashCipher.decrypt("SVOOL")) // "HELLO"

// Prove it's self-inverse
let original = "The quick brown fox"
assert(AtbashCipher.encrypt(AtbashCipher.encrypt(original)) == original)
```

## API Reference

### `AtbashCipher.encrypt(_ text: String) -> String`

Encrypt text using the Atbash cipher. Each letter is replaced by its reverse in the alphabet (A<->Z, B<->Y, etc.). Non-alphabetic characters pass through unchanged. Case is preserved.

### `AtbashCipher.decrypt(_ text: String) -> String`

Decrypt text using the Atbash cipher. Because the cipher is self-inverse, this function is identical to `encrypt(_:)`. It exists for API clarity.

## How It Works

The implementation uses Swift's `unicodeScalars` view to process each character as a Unicode scalar value. Each scalar is checked against the ASCII ranges for uppercase (65-90) and lowercase (97-122) letters, and the position is reversed using `25 - position`. The `AtbashCipher` type is declared as an `enum` with no cases (a "caseless enum"), which is Swift's idiomatic way to create a namespace that cannot be instantiated.

## Testing

```bash
swift test
```

The test suite uses XCTest and covers:
- Known encryption pairs (HELLO -> SVOOL, etc.)
- Case preservation for upper and lowercase
- Non-alphabetic character passthrough (digits, punctuation, whitespace)
- Self-inverse property for multiple inputs
- Full alphabet mapping in both directions
- Edge cases: empty string, single characters
- No letter maps to itself
- Decrypt/encrypt equivalence

## License

MIT
