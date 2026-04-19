# nib-ir-compiler

Rust IR compiler for Nib.

This package lowers the convergence-wave Nib subset into the shared compiler
IR used by the Wasm and JVM-style backend work. It emits `_start`, one `_fn_*`
label per source function, function parameters in registers `v2+`, return
values in `v1`, and simple call/return sequences compatible with the Rust
`ir-to-wasm-compiler` ABI.
