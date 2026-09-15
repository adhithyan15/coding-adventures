## 0.100.0 — 2026-06-20 — ALGOL 60 reals run on ALL 7 backends — E3 COMPLETE (LANG-FULL E3-native)

The two ALGOL 60 **real** matrix programs now run on the **`NativeAot`** column
too — completing the set: native-AOT + LLVM + WASM + JVM + CLR + VM + JIT. The
direct native backends emit f64 codegen (`aarch64-backend` 0.11.0's `fadd`/`fcmp`,
`x86_64-backend` 0.13.0's SSE2 `addsd`/`ucomisd`). `run_native` compiles for the
host arch, so the `NativeAot` cell is executed on aarch64 locally (Apple Silicon)
and on x86_64 by the Linux-x86 CI runner; `proven_columns_do_not_silently_skip`
guards both.

**Enabler E3 (floating-point) is complete** — every code-gen backend plus the
VM/JIT execute `f64`. ALGOL `real` (AL1) is fully done.

