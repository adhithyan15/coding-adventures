# javascript-iir-compiler

The first native JavaScript execution slice. It parses JavaScript into the
existing typed AST, lowers a deliberately small numeric/`console.log` subset
directly into InterpreterIR, and runs it on the Rust `vm-core` interpreter.
It does not use Semantic IR or a host JavaScript runtime to execute programs.

```rust
let output = javascript_iir_compiler::run_source("console.log(1 / 2);")?;
assert_eq!(output, "0.5\n");
```

`cargo run -p javascript-iir-compiler --bin jsvm -- example.js` executes a
file through the same native path.

Only numeric literals, unary `-`, binary `+`, `-`, `*`, `/`, expression
statements, and single-argument `console.log` are accepted. Other syntax is
rejected explicitly. This is a pilot, not a complete JavaScript engine.
Number display outside the pilot's finite `1e-6` to `1e21` range is rejected
until full ECMAScript formatting lands.
Source files are limited to 64 KiB; direct AST compilation has node and depth
limits, and the runner caps instructions and captured output.
See [LANG78](../../../specs/LANG78-native-dynamic-iir-frontends.md).
