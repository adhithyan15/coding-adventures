# CodingAdventures.AesModes.FSharp

AES modes of operation helpers for .NET, implemented with the package-local AES block primitive and GHASH.

This package mirrors the repository's AES modes API with PKCS#7 padding, XOR helpers, ECB, CBC, CTR, and GCM. ECB is included for educational parity only; prefer GCM for authenticated encryption.

```fsharp
open CodingAdventures.AesModes.FSharp

let key = Convert.FromHexString "2b7e151628aed2a6abf7158809cf4f3c"
let iv = Array.zeroCreate<byte> 16
let nonce = Array.zeroCreate<byte> 12

let cbcCiphertext = AesModes.cbcEncrypt plaintext key iv
let cbcPlaintext = AesModes.cbcDecrypt cbcCiphertext key iv

let ciphertext, tag = AesModes.gcmEncrypt plaintext key nonce aad
let verifiedPlaintext = AesModes.gcmDecrypt ciphertext key nonce aad tag
```
