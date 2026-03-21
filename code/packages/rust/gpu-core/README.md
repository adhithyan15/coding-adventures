# gpu-core (Rust)

A pluggable, educational GPU processing element simulator. This is a Rust port of the Python `gpu-core` package.

## What is this?

A GPU core is the smallest independently programmable compute unit on a GPU. Unlike complex CPU cores (branch predictors, out-of-order execution, speculation), GPU cores are simple in-order processors. GPUs achieve performance through massive parallelism: thousands of simple cores running simultaneously.

This crate simulates a single GPU core with:
- **Pluggable ISA** — swap instruction sets to simulate any vendor (NVIDIA PTX, AMD GCN, Intel Xe, ARM Mali)
- **FP Register File** — configurable floating-point registers (1-256, FP32/FP16/BF16)
- **Local Memory** — byte-addressable scratchpad with float load/store
- **Execution Tracing** — every instruction's journey is recorded for learning

## Architecture

```text
+-------------------------------------------+
|              GPU Core                      |
|                                            |
|  +---------+    +-----------------+        |
|  | Program |---→|   Fetch         |        |
|  | Memory  |    |   instruction   |        |
|  +---------+    |   at PC         |        |
|                 +-------+---------+        |
|                         |                  |
|                 +-------v---------+        |
|  +-----------+  |   ISA.execute() |        |
|  | Register  |<-|   (pluggable!)  |-->Trace|
|  | File      |->|                 |        |
|  +-----------+  +-------+---------+        |
|                         |                  |
|  +-----------+  +-------v---------+        |
|  |  Local   |<--|  Update PC      |        |
|  |  Memory  |   +-----------------+        |
|  +-----------+                             |
+-------------------------------------------+
```

## The 16 Opcodes

| Category   | Opcodes                              |
|-----------|--------------------------------------|
| Arithmetic | `Fadd`, `Fsub`, `Fmul`, `Ffma`, `Fneg`, `Fabs` |
| Memory     | `Load`, `Store`                      |
| Data Move  | `Mov`, `Limm`                        |
| Control    | `Beq`, `Blt`, `Bne`, `Jmp`, `Nop`, `Halt` |

## Quick Start

```rust
use gpu_core::{GPUCore, GenericISA};
use gpu_core::opcodes::{limm, fadd, halt};

let mut core = GPUCore::new(Box::new(GenericISA));
core.load_program(vec![
    limm(0, 3.0),   // R0 = 3.0
    limm(1, 4.0),   // R1 = 4.0
    fadd(2, 0, 1),  // R2 = R0 + R1 = 7.0
    halt(),
]);

let traces = core.run(1000).unwrap();
assert_eq!(core.registers.read_float(2), 7.0);

for trace in &traces {
    println!("{}", trace.format());
}
```

## How it fits in the stack

This crate depends on `fp-arithmetic` for IEEE 754 floating-point operations. It sits at the processing element layer of the accelerator computing stack:

```text
Application (Python/CUDA)
    |
Compiler / Runtime
    |
Warp Scheduler
    |
>>> GPU Core (this crate) <<<
    |
FP Arithmetic (fp-arithmetic crate)
    |
Logic Gates (logic-gates crate)
```

## Testing

```bash
cd code/packages/rust
cargo test -p gpu-core
cargo clippy -p gpu-core
```

## License

Educational / MIT
