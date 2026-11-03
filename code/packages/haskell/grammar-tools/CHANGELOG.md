# grammar-tools changelog

- Unreleased: the Darwin BUILD now runs Cabal with the Command Line Tools
  developer dir (`DEVELOPER_DIR=/Library/Developer/CommandLineTools`), so GHC
  9.4.8's CLT SDK sysroot gets the matching linker on macOS CI with CLT 27.
- `0.1.0`: initial Haskell grammar-tools package with token grammar parsing, parser grammar parsing, cross-validation, and source compilation helpers.
