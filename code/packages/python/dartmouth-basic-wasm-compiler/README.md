# dartmouth-basic-wasm-compiler

Compiles [Dartmouth BASIC](https://en.wikipedia.org/wiki/Dartmouth_BASIC)
programs to [WebAssembly](https://webassembly.org/) and executes them,
capturing standard output.

## Position in the stack

```
dartmouth_basic_parser          — tokenise + parse BASIC source → AST
        ↓
dartmouth_basic_ir_compiler     — lower AST → target-independent IR
        ↓  (char_encoding="ascii")
ir_to_wasm_compiler             — lower IR → WasmModule (WASI preview-1 ABI)
        ↓
wasm_module_encoder             — serialise WasmModule → raw bytes
        ↓
wasm_runtime                    — instantiate WASM binary, bind WASI host, run
```

This package is the WASM counterpart of `dartmouth-basic-ge225-compiler`.
Both share the same parser and IR compiler; they differ only in the backend
that lowers IR to a target binary.

## Usage

```python
from dartmouth_basic_wasm_compiler import run_basic

result = run_basic("""
10 LET S = 0
20 FOR I = 1 TO 100
30 LET S = S + I
40 NEXT I
50 PRINT S
60 END
""")
print(result.output)   # "5050\n"
```

## API

### `run_basic(source, *, max_steps=100_000) → RunResult`

Compile and run a Dartmouth BASIC program through the full WASM pipeline.

- **`source`** — BASIC source text; each line must start with a line number.
- **`max_steps`** — accepted for API parity with the GE-225 runner; not
  enforced (the WASM runtime has no instruction counter).

Returns a `RunResult`.  Raises `BasicError` on parse error, unsupported
feature (GOSUB, DIM, INPUT, arrays, `^` operator), character not in the
ASCII/GE-225 character set, or runtime failure.

### `RunResult`

| Field | Type | Value |
|-------|------|-------|
| `output` | `str` | Standard output from `PRINT` statements |
| `var_values` | `dict[str, int]` | Always `{}` — WASM locals vanish after `_start` returns |
| `steps` | `int` | Always `0` — no instruction counter in the WASM runtime |
| `halt_address` | `int` | Always `0` — halt is a function return, not a fixed address |

### `BasicError`

Raised when the program cannot be compiled or run.  The original exception
is available as `__cause__`.

## Supported BASIC features (V1)

| Feature | Supported |
|---------|-----------|
| `LET` (integer arithmetic: `+` `-` `*` `/`) | ✅ |
| `PRINT` string literals | ✅ |
| `PRINT` numeric expressions | ✅ |
| `PRINT` mixed (e.g. `PRINT "X =", X`) | ✅ |
| `FOR` / `NEXT` (with `STEP`) | ✅ |
| `IF` / `THEN` (all six relational ops) | ✅ |
| `GOTO` | ✅ |
| `REM` (comments) | ✅ |
| `STOP` | ✅ |
| `END` | ✅ |
| `GOSUB` / `RETURN` | ❌ (raises `BasicError`) |
| `DIM` / arrays | ❌ |
| `INPUT` | ❌ |
| `^` exponentiation | ❌ |
| Floating-point | ❌ |

## Character encoding note

The GE-225 typewriter used a proprietary 6-bit character code that is
incompatible with ASCII.  Because WASM's `fd_write` WASI syscall writes raw
bytes to stdout, the IR compiler runs in `char_encoding="ascii"` mode:

- String characters are emitted as their `ord()` values.
- Digits 0-9 are emitted as `ord('0') + digit` (ASCII 48–57).
- Minus sign is emitted as `ord('-')` (ASCII 45).
- Newline is emitted as `ord('\n')` (ASCII 10).

Only characters that exist in both the GE-225 typewriter table and ASCII are
accepted; others raise `BasicError`.

## Development

```bash
cd code/packages/python/dartmouth-basic-wasm-compiler
uv venv
uv pip install -e ".[dev]"
pytest tests/ -v
```
