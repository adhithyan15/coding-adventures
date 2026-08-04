# atbash-cipher

Pure Haskell implementation of the classical Atbash substitution cipher.

## API

- `encrypt` mirrors ASCII letters across their alphabet (`A` to `Z`, `B` to
  `Y`, and so on), preserving case and passing every other character through.
- `decrypt` performs the same transformation because Atbash is its own inverse.

The library depends only on `base`. Tests use Hspec.

## Running the tests

```sh
cabal test all
```
