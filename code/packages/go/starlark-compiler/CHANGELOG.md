# Changelog

## Unreleased

- Make the fixed 0..255 bound explicit before native integer conversion so static analysis can verify the reader on every platform.

- Consume the complete STP01 shared corpus through actual Go provider APIs.
- Add strict bounded test-only JSON transport, semantic corruption and all-four-map/AllOps ownership regression tests.
- Retain the production API and dependency-free provider unchanged.


## 0.1.0

- Initial Go parity package for Starlark compiler opcode metadata.
