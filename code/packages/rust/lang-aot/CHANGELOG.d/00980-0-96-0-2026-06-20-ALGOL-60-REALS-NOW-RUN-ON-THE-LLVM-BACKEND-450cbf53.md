## 0.96.0 — 2026-06-20 — ALGOL 60 reals now run on the LLVM backend too (LANG-FULL E3-codegen-slots, LLVM)

The two ALGOL 60 **real** matrix programs (real `*`+`=` → exit 42, real `/`+`<`
→ exit 1) now run on the **LLVM** column in addition to VM/JIT — `iir-to-llvm`
0.13.0 gives an `f64` variable a `double` stack slot, renders f64 constants in
LLVM's exact hex form, and zexts a float comparison's boolean result to `i64`.
The `proven_columns_do_not_silently_skip` guard confirms clang actually executes
the programs (no silent skip).

Remaining E3 backends: wasm + jvm need the same slot fix (E3-codegen-slots);
native (x86_64/aarch64) + CLR need FP emission (E3-native / E3-clr).

