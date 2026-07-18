# CodingAdventures.ChaCha20Poly1305.FSharp

Pure F# implementation of ChaCha20-Poly1305 authenticated encryption from
[RFC 8439](https://www.rfc-editor.org/rfc/rfc8439).

## What It Implements

- the 20-round IETF ChaCha20 block and stream cipher with 256-bit keys;
- Poly1305 one-time authentication over `2^130 - 5`;
- ChaCha20-Poly1305 AEAD with 96-bit nonces and additional authenticated data;
- constant-work tag comparison and authenticate-before-decrypt behavior.

## Usage

```fsharp
open CodingAdventures.ChaCha20Poly1305.FSharp

let encrypted = ChaCha20Poly1305.aeadEncrypt plaintext key32 nonce12 aad
let decrypted =
    ChaCha20Poly1305.aeadDecrypt
        encrypted.Ciphertext key32 nonce12 aad encrypted.Tag
```

The nonce must be unique for each key. Reusing a key/nonce pair repeats both
the ChaCha20 keystream and the one-time Poly1305 key.

## Design Notes

The implementation is self-contained and uses `BigInteger` for readable
Poly1305 field arithmetic. It is intended for learning and conformance work;
production systems should use a carefully audited platform cryptography API.

## Testing

```sh
dotnet test tests/CodingAdventures.ChaCha20Poly1305.Tests/CodingAdventures.ChaCha20Poly1305.Tests.fsproj
```
