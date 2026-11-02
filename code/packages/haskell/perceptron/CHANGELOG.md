# Changelog

## Unreleased

- `BUILD` runs Cabal with `DEVELOPER_DIR=/Library/Developer/CommandLineTools` on Darwin so GHC 9.4.8 links with the Command Line Tools linker that accepts the CLT SDK's `arm64e.x1` stubs; other platforms are unchanged.

## 0.1.0 - 2026-07-19

- Add a pure sigmoid/BCE perceptron with immutable training results.
- Compose the existing Haskell matrix, loss, and activation packages.
- Validate hyperparameters, feature shapes, label shapes, and finite values.
- Cover AND-gate convergence, column labels, epoch-zero behavior, deterministic
  refitting, prediction guards, and invalid inputs with package-native tests.
