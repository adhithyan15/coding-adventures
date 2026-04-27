# CodingAdventures.AesModes.CSharp

AES modes of operation helpers for .NET, implemented with the package-local AES block primitive and GHASH.

This package mirrors the repository's AES modes API with PKCS#7 padding, XOR helpers, ECB, CBC, CTR, and GCM. ECB is included for educational parity only; prefer GCM for authenticated encryption.

```csharp
using CodingAdventures.AesModes;

byte[] key = Convert.FromHexString("2b7e151628aed2a6abf7158809cf4f3c");
byte[] iv = new byte[16];
byte[] nonce = new byte[12];

byte[] cbcCiphertext = AesModes.CbcEncrypt(plaintext, key, iv);
byte[] cbcPlaintext = AesModes.CbcDecrypt(cbcCiphertext, key, iv);

var (ciphertext, tag) = AesModes.GcmEncrypt(plaintext, key, nonce, aad);
byte[] verifiedPlaintext = AesModes.GcmDecrypt(ciphertext, key, nonce, aad, tag);
```
