# caesar-cipher

F# implementation of the `caesar-cipher` foundation package.

This package pairs the classical Caesar cipher with the two attacks that make
it a good teaching cipher: exhaustive brute force and English-frequency
analysis.

## API

- `CaesarCipher.encrypt text shift`
- `CaesarCipher.decrypt text shift`
- `CaesarCipher.rot13 text`
- `CaesarCipher.bruteForce ciphertext`
- `CaesarCipher.frequencyAnalysis ciphertext`
- `CaesarCipher.englishFrequencies`

## Usage

```fsharp
open CodingAdventures.CaesarCipher

let ciphertext = CaesarCipher.encrypt "Hello, World!" 3
let plaintext = CaesarCipher.decrypt ciphertext 3
let guesses = CaesarCipher.bruteForce ciphertext
let shift, decoded = CaesarCipher.frequencyAnalysis ciphertext
```

## Development

```bash
bash BUILD
```
