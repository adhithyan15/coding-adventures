# Scytale Cipher (Go)

Ancient Spartan transposition cipher implementation in Go.

This package follows [CR02](../../../specs/CR02-scytale-cipher.md). Grid cells are Unicode scalar values, uneven ciphertext columns are reconstructed explicitly, and only trailing U+0020 padding is removed. `BruteForce` rejects inputs above 4096 scalars before allocating quadratic candidate output. Production code is deterministic pure computation with no OS capabilities.

## What is the Scytale Cipher?

The Scytale (pronounced "SKIT-ah-lee") is one of the earliest known transposition ciphers, used by the Spartans around 700 BCE. It rearranges character positions using a columnar transposition with a numeric key.

## Usage

```go
import scytalecipher "github.com/adhithyan15/coding-adventures/code/packages/go/scytale-cipher"

ct, err := scytalecipher.Encrypt("HELLO WORLD", 3)
// ct = "HLWLEOODL R "

pt, err := scytalecipher.Decrypt(ct, 3)
// pt = "HELLO WORLD"

results, err := scytalecipher.BruteForce(ct)
// results = [{Key: 2, Text: "..."}, {Key: 3, Text: "HELLO WORLD"}, ...]
```

## Part of coding-adventures

This package is part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) monorepo.
