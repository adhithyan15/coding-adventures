# Scytale Cipher (Swift)

Ancient Spartan transposition cipher implementation in Swift.

This package follows [CR02](../../../specs/CR02-scytale-cipher.md). Grid cells are Unicode scalar values rather than grapheme clusters, uneven ciphertext columns are reconstructed explicitly, and only trailing U+0020 padding is removed. `bruteForce` throws before allocating candidate output above 4096 scalars. Production code is deterministic pure computation with no OS capabilities.

## Usage

```swift
import ScytaleCipher

let ct = try encrypt("HELLO WORLD", key: 3)
// => "HLWLEOODL R "

let pt = try decrypt(ct, key: 3)
// => "HELLO WORLD"

let results = try bruteForce(ct)
// => [BruteForceResult(key: 2, text: "..."), BruteForceResult(key: 3, text: "HELLO WORLD"), ...]
```

## Part of coding-adventures

This package is part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) monorepo.
