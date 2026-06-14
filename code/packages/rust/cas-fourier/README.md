# cas-fourier

Pure Rust symbolic Fourier transform support for the symbolic IR stack.

This package mirrors the Python `cas-fourier` transform table and is suitable
for the native Rust and WASM pathways. It has no Python runtime or parser
dependency; callers pass symbolic IR nodes directly.

## Exports

- `fourier_transform(f, t, omega)`
- `ifourier_transform(f, omega, t)`
- `fourier_handler` and `ifourier_handler`
- `FOURIER` and `IFOURIER` canonical head names

Unknown forms fall through to unevaluated IR nodes.
