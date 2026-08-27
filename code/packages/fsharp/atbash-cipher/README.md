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
## Language-neutral conformance

The test suite executes all six normative `atbash-transform` objects from the
`classical-ciphers-v1` fixture. Generated dependency-free test source pins the
corpus digest and exact case roster; production code does not read the fixture
or gain filesystem or JSON-parser authority.
