# CodingAdventures.Aes.FSharp

AES single-block helpers for .NET, implemented directly from FIPS 197.

This package mirrors the repository's low-level AES block API. It encrypts or decrypts exactly one 16-byte block with a 16-, 24-, or 32-byte key.

```fsharp
open CodingAdventures.Aes.FSharp

let ciphertext = AesBlock.encryptBlock plaintextBlock key
let plaintext = AesBlock.decryptBlock ciphertext key
```
