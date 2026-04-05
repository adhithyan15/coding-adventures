# Vigenere Cipher (Go)

A polyalphabetic substitution cipher with full cryptanalysis tools. Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) monorepo.

## What is the Vigenere Cipher?

The Vigenere cipher shifts each letter by a different amount determined by a repeating keyword. It was considered unbreakable for 300 years until Kasiski's attack in 1863.

## Usage

```go
import vigenerecipher "github.com/adhithyan15/coding-adventures/code/packages/go/vigenere-cipher"

// Encryption
ct, _ := vigenerecipher.Encrypt("ATTACKATDAWN", "LEMON") // "LXFOPVEFRNHR"

// Decryption
pt, _ := vigenerecipher.Decrypt("LXFOPVEFRNHR", "LEMON") // "ATTACKATDAWN"

// Automatic cipher breaking (needs ~200+ chars)
key, plaintext, _ := vigenerecipher.BreakCipher(longCiphertext)
```

## Running Tests

```bash
go test ./... -v -cover
```
