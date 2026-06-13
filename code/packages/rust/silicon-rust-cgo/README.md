# silicon-rust-cgo

Rust cdylib that exports the silicon simulation stack as a plain C ABI for
consumption by Go's CGo runtime.

## How it fits in the stack

```
Go caller
  ↓ import "silicon_rust_go"
silicon_rust_go  (Go package, CGo)
  ↓ import "C" → silicon_cgo.h
silicon-rust-cgo  (this crate)
  ↓ Rust function calls
device-physics   mosfet-models   fab-process-simulation
```

## Build

```bash
cargo build -p silicon-rust-cgo --release
```

Produces `target/release/libsilicon_rust_cgo.{so,dylib,dll}`.

## C API

All symbols are declared in `include/silicon_cgo.h`.  See that file for the
full API reference and calling conventions.

### Calling conventions

* **Infallible functions** — return `double` directly.
* **Fallible functions** — return `int` (0 = success, -1 = error).  On error,
  a nul-terminated UTF-8 message is written into `err[err_cap]`.
* **String-returning functions** — write the nul-terminated wire string into
  `out[out_cap]`.  4096 bytes is sufficient for any realistic process flow.

## Wire format

A `CrossSection` travels across the boundary as a pipe-separated string:

```
""                              empty
"Si:500.0"                      bare substrate
"SiO2:4.8|Si:500.0"            gate oxide on silicon
"Poly:50.0|SiO2:4.8|Si:500.0" poly on oxide on silicon
```

Material names containing `|` or `:` are rejected by `silicon_deposit`,
`silicon_etch`, and `silicon_implant`.

## Safety

All `unsafe` blocks in this crate are bounded:
- Input C strings are read via `CStr::from_ptr` (requires nul-terminated input).
- Output buffers are written via `ptr::copy_nonoverlapping` with an explicit
  length bound (`n = bytes.len().min(cap - 1)`).
- The `fill_mos_result` region copy is bounded to 31 bytes.

No undefined symbols are exported; all dependencies are statically linked.

## Testing

```bash
cargo test -p silicon-rust-cgo
```

Unit tests cover the pure-Rust wire format helpers and name validation.
Integration tests (FFI round-trip) run from the Go package.
