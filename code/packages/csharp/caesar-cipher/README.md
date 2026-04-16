# caesar-cipher

C# implementation of the `caesar-cipher` foundation package.

This package includes both sides of the classic Caesar cipher story: the
encryption helpers themselves and the brute-force / frequency-analysis tools
that show why the cipher is insecure.

## API

- `CaesarCipher.Encrypt(text, shift)`
- `CaesarCipher.Decrypt(text, shift)`
- `CaesarCipher.Rot13(text)`
- `CaesarCipher.BruteForce(ciphertext)`
- `CaesarCipher.FrequencyAnalysis(ciphertext)`
- `CaesarCipher.EnglishFrequencies`

## Usage

```csharp
using CodingAdventures.CaesarCipher;

var ciphertext = CaesarCipher.Encrypt("Hello, World!", 3);
var plaintext = CaesarCipher.Decrypt(ciphertext, 3);
var guesses = CaesarCipher.BruteForce(ciphertext);
var (shift, decoded) = CaesarCipher.FrequencyAnalysis(ciphertext);
```

## Development

```bash
bash BUILD
```
