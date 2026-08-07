# caesar-cipher

Pure Haskell implementation of the classic Caesar shift cipher and two
classical attacks against it.

## API

- `encrypt`, `decrypt`, and `rot13` transform ASCII letters while preserving
  case and passing every other character through unchanged.
- `bruteForce` returns all 25 non-identity candidate plaintexts.
- `frequencyAnalysis` selects the candidate with the closest chi-squared fit
  to `englishFrequencies`.

The library depends only on `base`. Tests use Hspec.

## Running the tests

```sh
cabal test all
```
