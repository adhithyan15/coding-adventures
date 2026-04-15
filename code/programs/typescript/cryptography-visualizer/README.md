# cryptography-visualizer

`cryptography-visualizer` is an interactive web app for exploring classical substitution ciphers.

The app uses the same layered architecture as other visualizers in the repository:

1. `@coding-adventures/caesar-cipher` provides the Caesar cipher encrypt/decrypt, brute force, and frequency analysis functions.
2. `@coding-adventures/lattice-transpiler` compiles the app's `.lattice` styles into CSS in the browser.
3. The React app renders interactive panels that expose each step of the encryption process.
4. The output panel can copy the current ciphertext to the clipboard for quick reuse.

## What The App Shows

- A text input for any plaintext message
- A cipher selector (Caesar or Atbash)
- A shift slider (1-25) for the Caesar cipher
- The complete 26-letter substitution table for the active cipher
- Step-by-step transformation of each character
- The encrypted ciphertext output
- Frequency analysis comparing ciphertext letter distribution to English (Caesar only)
- Brute force panel showing all 25 possible decryptions (Caesar only)

## Supported Ciphers

### Caesar Cipher

The oldest known substitution cipher. Each letter is shifted forward by a fixed number of positions in the alphabet. The app supports shifts 1 through 25 and includes a ROT13 quick button.

### Atbash Cipher

An ancient Hebrew cipher that reverses the alphabet (A becomes Z, B becomes Y, etc.). It has no key -- the mapping is fixed and self-inverse.

## Development

```bash
bash BUILD
cd code/programs/typescript/cryptography-visualizer
npm run dev
```

## Architecture

The Caesar cipher operations come from the `@coding-adventures/caesar-cipher` package, and the Atbash cipher operations come from `@coding-adventures/atbash-cipher`.
