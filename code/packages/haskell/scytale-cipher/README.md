# scytale-cipher

Pure Haskell implementation of the ancient Spartan Scytale transposition
cipher and its small-key-space brute-force attack.

## API

- `encrypt` writes text row-by-row into a keyed grid, pads the final row with
  spaces, and reads the result column-by-column.
- `decrypt` reverses the transposition and removes trailing padding spaces.
- `bruteForce` returns `BruteForceResult` candidates for keys from two through
  half the ciphertext length.

`encrypt` and `decrypt` return `Either String String` so invalid keys are
explicit. Empty text returns `Right ""` for every key, matching the shared
CR02 contract. The library depends only on `base`; tests use Hspec.

## Running the tests

```sh
cabal test all
```
