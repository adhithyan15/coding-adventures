# coding-adventures-atbash-cipher

A Lua implementation of the Atbash cipher, one of the oldest known substitution ciphers. This package is part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) monorepo.

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

Install via LuaRocks:

```bash
luarocks install coding-adventures-atbash-cipher
```

## Usage

```lua
local atbash = require("coding_adventures.atbash_cipher")

-- Basic encryption
print(atbash.encrypt("HELLO"))  --> "SVOOL"

-- Case is preserved
print(atbash.encrypt("Hello, World!"))  --> "Svool, Dliow!"

-- Non-alphabetic characters pass through
print(atbash.encrypt("Test 123!"))  --> "Gvhg 123!"

-- Decryption is the same operation (self-inverse)
print(atbash.decrypt("SVOOL"))  --> "HELLO"

-- Prove it's self-inverse
local original = "The quick brown fox"
assert(atbash.encrypt(atbash.encrypt(original)) == original)
```

## API Reference

### `M.encrypt(text) -> string`

Encrypt text using the Atbash cipher. Each letter is replaced by its reverse in the alphabet (A<->Z, B<->Y, etc.). Non-alphabetic characters pass through unchanged. Case is preserved.

### `M.decrypt(text) -> string`

Decrypt text using the Atbash cipher. Because the cipher is self-inverse, this function is identical to `encrypt()`. It exists for API clarity.

## How It Works

The implementation processes each byte in the string using `string.byte()`, applies the Atbash substitution, and converts back with `string.char()`. Each byte is checked against the ASCII ranges for uppercase (65-90) and lowercase (97-122) letters.

## Testing

```bash
cd tests && busted . --verbose --pattern=test_
```

The test suite uses busted and covers:
- Known encryption pairs (HELLO -> SVOOL, etc.)
- Case preservation for upper and lowercase
- Non-alphabetic character passthrough
- Self-inverse property for multiple inputs
- Full alphabet mapping in both directions
- Edge cases: empty string, single characters
- No letter maps to itself
- Decrypt/encrypt equivalence

## License

MIT
