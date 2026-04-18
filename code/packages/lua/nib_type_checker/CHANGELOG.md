# Changelog

## Unreleased

- tighten the package `BUILD` script so the final `luarocks make` runs with
  `--deps-mode=none` after sibling rocks are bootstrapped, matching the
  repository's clean-build CI validator

## 0.1.0

- add the first Lua Nib type checker package
- validate the convergence-wave Nib subset used by the local WASM lane
- return a typed AST wrapper keyed by node identity
