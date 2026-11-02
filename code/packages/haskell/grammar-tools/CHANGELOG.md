# grammar-tools changelog

- Unreleased: `BUILD` runs Cabal with `DEVELOPER_DIR=/Library/Developer/CommandLineTools` on Darwin so GHC 9.4.8 links with the Command Line Tools linker that accepts the CLT SDK's `arm64e.x1` stubs; other platforms are unchanged.
- `0.1.0`: initial Haskell grammar-tools package with token grammar parsing, parser grammar parsing, cross-validation, and source compilation helpers.
