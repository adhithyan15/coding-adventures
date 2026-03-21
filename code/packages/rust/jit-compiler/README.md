# jit-compiler

Rust scaffolding for the future JIT compiler layer of the computing stack.

## What it does today

This package does **not** yet emit native machine code. Instead, it implements
the first pieces a JIT needs in order to exist at all:

- hot-path execution counting
- a configurable "compile when hot" threshold
- bookkeeping for shell native blocks
- deoptimization hooks for throwing away compiled blocks

That makes this a real package, but still a deliberately incomplete JIT.

## Why this scope

The full JIT compiler spec is intentionally ambitious. Real JITs require:

- profiling
- instruction selection
- register allocation
- platform-specific code generation
- deoptimization
- runtime integration with the VM

This Rust package starts at the front of that pipeline: profiling and block
management. Actual native code emission remains future work.

## Key types

- `JitCompilerConfig` -- target ISA and hot-path threshold
- `TargetIsa` -- symbolic native target (`RiscV`, `Arm`, `X86`)
- `HotPathProfile` -- execution-count snapshot for a bytecode offset
- `NativeBlock` -- shell representation of compiled native code
- `JitCompiler` -- threshold tracking, block installation, and deoptimization

## Example

```rust
use jit_compiler::{JitCompiler, JitCompilerConfig, TargetIsa};

let mut jit = JitCompiler::new(JitCompilerConfig::new(TargetIsa::RiscV, 3));

assert!(!jit.observe_execution(12));
assert!(!jit.observe_execution(12));
assert!(jit.observe_execution(12)); // hot on the third execution

jit.install_shell_block(12, vec!["locals stay integers".to_string()]);
assert!(jit.has_native_block(12));

jit.deoptimize(12);
assert!(!jit.has_native_block(12));
```

## Running tests

```bash
cargo test -p jit-compiler -- --nocapture
```

## Spec

See [05b-jit-compiler.md](../../../specs/05b-jit-compiler.md) for the broader design.
