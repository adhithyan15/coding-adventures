# Scytale Cipher (Swift)

Ancient Spartan transposition cipher implementation in Swift.

## Usage

```swift
import ScytaleCipher

let ct = try encrypt("HELLO WORLD", key: 3)
// => "HLWLEOODL R "

let pt = try decrypt(ct, key: 3)
// => "HELLO WORLD"

let results = bruteForce(ct)
// => [BruteForceResult(key: 2, text: "..."), BruteForceResult(key: 3, text: "HELLO WORLD"), ...]
```

## Part of coding-adventures

This package is part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) monorepo.
