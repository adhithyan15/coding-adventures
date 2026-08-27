# Vigenere Cipher (Swift)

## CR03 conformance

This implementation follows [CR03](../../../specs/CR03-vigenere-cipher.md): keys and cipher transforms use ASCII letters only, analysis ignores non-ASCII letters, and non-ASCII Unicode scalars pass through without advancing the key. Analysis accepts at most 8,192 Unicode scalars and key-length bounds up to 40, uses the smallest candidate within 90% of the best index-of-coincidence score, preserves the exact requested recovered-key length, and resolves score ties toward the smallest shift.

A Swift implementation of the Vigenere polyalphabetic substitution cipher with full cryptanalysis support (key length detection via Index of Coincidence and key recovery via chi-squared analysis).

Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) project.

## Usage

```swift
import VigenereCipher

// Encrypt
let ct = try encrypt("ATTACKATDAWN", key: "LEMON")  // "LXFOPVEFRNHR"

// Decrypt
let pt = try decrypt("LXFOPVEFRNHR", key: "LEMON")  // "ATTACKATDAWN"

// Cryptanalysis (requires 200+ character ciphertext)
let keyLen = try findKeyLength(longCiphertext)
let key = try findKey(longCiphertext, keyLength: keyLen)
let (recoveredKey, plaintext) = try breakCipher(longCiphertext)
```

## API

| Function | Description |
|----------|-------------|
| `encrypt(_:key:)` | Encrypt using Vigenere cipher |
| `decrypt(_:key:)` | Decrypt using Vigenere cipher |
| `findKeyLength(_:maxLength:)` | Estimate key length via IC analysis |
| `findKey(_:keyLength:)` | Find key via chi-squared analysis |
| `breakCipher(_:)` | Full automatic break (returns key + plaintext) |

## Testing

```bash
swift test --verbose
```
