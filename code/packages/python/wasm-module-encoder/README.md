# wasm-module-encoder

Generic WebAssembly 1.0 module encoder.

This package takes a `wasm_types.WasmModule` and produces raw `.wasm` bytes.
It is intentionally generic so it can sit in the middle of a larger pipeline:

```
frontend/backend -> WasmModule -> wasm-module-encoder -> .wasm bytes
```

## Usage

```python
from wasm_module_encoder import encode_module

wasm_bytes = encode_module(module)
```

## Development

```bash
cd code/packages/python/wasm-module-encoder
uv run pytest
```
