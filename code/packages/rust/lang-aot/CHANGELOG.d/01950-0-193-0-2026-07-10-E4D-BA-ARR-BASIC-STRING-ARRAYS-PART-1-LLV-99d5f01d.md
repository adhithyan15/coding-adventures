## 0.193.0 — 2026-07-10 — E4d-BA-arr: BASIC string arrays (part 1 — Llvm/Wasm/Vm/Jit)

A new matrix cell proves Dartmouth BASIC string arrays:
`DIM A$(2); A$(0)="O"; A$(1)="K"; PRINT A$(0)+A$(1)` → `OK`. The `OK` (not `OO`/`KK`)
proves the two element slots are distinct and each string handle survives a
store→load round-trip through the aggregate.

Runs on **[Llvm, Wasm, Vm, Jit]**:
- **VM/JIT** hold a tagged `Value::Str` element (already supported).
- **WASM** stores a 4-byte i32 handle per element (`iir-to-wasm` 0.36.0).
- **LLVM** carries a `str` element as an i64 handle — no backend change
  (`llvm_type_for("str")` = `i64`).

`dartmouth-basic-iir-compiler` 0.37.0 lowers `DIM A$` / `A$(i)` to `array<str>`
`alloc_array` / `array_set` / `array_get`.

**Part 2** (a follow-up PR) adds the **NativeAot** (native element-size allowance),
**JVM**, and **CLR** columns — the last two need managed reference-array lowering
(`String[]` / `string[]`; `anewarray` + `aaload`/`aastore` / `newarr` +
`ldelem.ref`/`stelem.ref`), which the numeric E5 arrays don't exercise.

