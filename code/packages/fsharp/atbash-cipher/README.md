# atbash-cipher

F# implementation of the `atbash-cipher` foundation package.

Atbash is the simplest possible fixed substitution cipher: it mirrors the
alphabet so A becomes Z, B becomes Y, and so on. Because that mapping is an
involution, the same code path handles both encryption and decryption.

## API

- `AtbashCipher.encrypt text`
- `AtbashCipher.decrypt text`

## Usage

```fsharp
open CodingAdventures.AtbashCipher

let ciphertext = AtbashCipher.encrypt "HELLO"
let plaintext = AtbashCipher.decrypt ciphertext
```

## Development

```bash
bash BUILD
```
