# Scytale Cipher (Go)

Ancient Spartan transposition cipher implementation in Go.

## What is the Scytale Cipher?

The Scytale (pronounced "SKIT-ah-lee") is one of the earliest known transposition ciphers, used by the Spartans around 700 BCE. It rearranges character positions using a columnar transposition with a numeric key.

## Usage

```go
import scytalecipher "github.com/adhithyan15/coding-adventures/code/packages/go/scytale-cipher"

ct, err := scytalecipher.Encrypt("HELLO WORLD", 3)
// ct = "HLWLEOODL R "

pt, err := scytalecipher.Decrypt(ct, 3)
// pt = "HELLO WORLD"

results := scytalecipher.BruteForce(ct)
// results = [{Key: 2, Text: "..."}, {Key: 3, Text: "HELLO WORLD"}, ...]
```

## Part of coding-adventures

This package is part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) monorepo.
