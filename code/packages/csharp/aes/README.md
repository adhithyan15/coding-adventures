# CodingAdventures.Aes.CSharp

AES single-block helpers for .NET.

This package mirrors the repository's low-level AES block API. It encrypts or decrypts exactly one 16-byte block with a 16-, 24-, or 32-byte key.

```csharp
using CodingAdventures.Aes;

byte[] ciphertext = AesBlock.EncryptBlock(plaintextBlock, key);
byte[] plaintext = AesBlock.DecryptBlock(ciphertext, key);
```
