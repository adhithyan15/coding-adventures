# atbash-cipher

C# implementation of the `atbash-cipher` foundation package.

Atbash is the fixed reverse-alphabet substitution cipher: A maps to Z, B maps
to Y, and so on. Because the mapping is self-inverse, encryption and
decryption are the same transformation.

## API

- `AtbashCipher.Encrypt(text)`
- `AtbashCipher.Decrypt(text)`

## Usage

```csharp
using CodingAdventures.AtbashCipher;

var ciphertext = AtbashCipher.Encrypt("HELLO");
var plaintext = AtbashCipher.Decrypt(ciphertext);
```

## Development

```bash
bash BUILD
```
