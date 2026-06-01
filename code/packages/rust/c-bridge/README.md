# c-bridge — drive matrix-cpu from any C-FFI-capable language

`c-bridge` is the universal entry point into the Rust matrix-cpu
execution engine for languages that can call C functions.  Same
shape as `matrix-rust-python` and `matrix-rust-napi`, but no
language-specific binding crate — just a stable C ABI.

## The contract

Two C functions, one shared library:

```c
char* matrix_cpu_run_graph(const char* envelope_json,
                           char**       err_out);

void  matrix_cpu_free_string(char* s);
```

- **`matrix_cpu_run_graph`** takes a JSON envelope describing a
  matrix-ir graph plus its hex-encoded input tensors, runs it on
  the CPU executor, and returns a JSON envelope with the hex-encoded
  outputs.  Returns NULL on error; writes the error message into
  `*err_out` (if non-NULL).
- **`matrix_cpu_free_string`** drops a string previously returned
  by `matrix_cpu_run_graph`.  Required because Rust and C may use
  different allocators — you cannot `free()` the returned pointer.

Both functions are safe to call from any thread.  Never panics on
adversarial input.

## Building

```bash
cargo build -p c-bridge --release
# Produces target/release/libmatrix_c_bridge.{so,dylib,dll}
```

## Per-language usage examples

### Ruby (via the `matrix_rust_ruby` gem)

```ruby
require 'matrix_rust_ruby'
out = MatrixRustRuby.run_graph_on_cpu(envelope_json_str)
```

The gem wraps the C ABI through `ruby-bridge`.  See
`code/packages/ruby/matrix_rust_ruby/`.

### Lua (via the `matrix-rust-lua` rock — coming)

```lua
local mlc = require('matrix_rust_lua')
local out = mlc.run_graph_on_cpu(envelope_json_str)
```

### Go (via cgo — coming)

```go
import "github.com/coding-adventures/matrix-rust-go"
out, err := matrixrust.RunGraphOnCpu(envelopeJSON)
```

### Swift (via SwiftPM — coming)

```swift
import MatrixRustSwift
let out = try MatrixRust.runGraphOnCpu(envelopeJSON)
```

### Direct C

```c
#include <stdio.h>
#include <stdlib.h>

extern char* matrix_cpu_run_graph(const char*, char**);
extern void  matrix_cpu_free_string(char*);

int main(void) {
    char* err = NULL;
    char* out = matrix_cpu_run_graph("{\"graph\": ..., \"inputs\": [...]}", &err);
    if (out == NULL) {
        fprintf(stderr, "matrix_cpu error: %s\n", err);
        matrix_cpu_free_string(err);
        return 1;
    }
    printf("output envelope: %s\n", out);
    matrix_cpu_free_string(out);
    return 0;
}
```

Link with `-lmatrix_c_bridge` (or the platform-equivalent).

## The envelope format

Identical to what `matrix-rust-python.run_graph_on_cpu` accepts.
See `code/packages/rust/matrix-ir-json/` for the schema and
`code/packages/python/ml-framework-core/src/ml_framework_core/_rust_backend.py`
for ~30 worked examples of envelope construction (the Python package
builds one envelope per op).

## Why JSON?

Every language has JSON.  Binary formats (FlatBuffers, Cap'n Proto)
would be marginally faster, but force every language binding to
drag in a schema compiler.  JSON keeps the per-language binding
code tiny — typically <300 LOC.

For typical matrix-cpu workloads (matmul, reduction, activation on
≥100k cells), JSON encode/decode is a fraction of one percent of
total call cost.  See `scripts/benchmark_mx10.py` in
`ml-framework-core` for measurements.
