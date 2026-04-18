# Changelog

## Unreleased

- remove the stray `wasm_runtime` bootstrap from `BUILD`; tests already load the
  runtime directly from source via `package.path`, and the extra bootstrap broke
  clean CI validation because it was not a declared rockspec dependency

## 0.1.0

- add the first Lua Nib-to-WASM orchestration package
- package parse, type-check, IR, validation, and encoding into one API
- cover runtime smoke scenarios for returns, calls, and loops
