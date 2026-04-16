# wasm-execution

Native Java execution helpers for the JVM WASM runtime.

This package currently provides:

- typed WASM values plus `i32`/`i64`/`f32`/`f64` constructors
- linear memory and table helpers
- a small interpreter for the instruction subset used by the runtime tests
