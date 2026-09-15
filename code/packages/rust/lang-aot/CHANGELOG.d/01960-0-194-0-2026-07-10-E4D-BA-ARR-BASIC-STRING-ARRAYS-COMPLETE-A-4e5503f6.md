## 0.194.0 — 2026-07-10 — E4d-BA-arr: BASIC string arrays COMPLETE (all 7 backends)

The string-array matrix cell (`DIM A$(2); A$(0)="O"; A$(1)="K"; PRINT A$(0)+A$(1)`
→ `OK`) now runs on **all seven backends**, adding NativeAot / JVM / CLR to the
part-1 Llvm / Wasm / VM / JIT:

- **NativeAot** — `x86_64-backend` 0.24.0 + `aarch64-backend` 0.23.0 accept a `str`
  element as an 8-byte handle (`native_array_elem_size`); twig-aot already
  materialises the handle into the slot, so no store/load change.
- **JVM** — `iir-to-jvm-class-file` 0.30.0 lowers `array<str>` to a
  `java.lang.String[]` (`anewarray` + `aaload`/`aastore`) — the backend's first
  reference-element array.
- **CLR** — `iir-to-cil-bytecode` 0.39.0 lowers it to a `System.String[]`
  (`newarr System.String` + `ldelem.ref`/`stelem.ref`). **Run-verified on real
  dotnet.**
- **LLVM fix** — `iir-to-llvm` 0.36.0 `ptrtoint`s a folded str literal to its i64
  handle in `array_set` (the part-1 cell's Llvm column relied on this; it was a
  latent invalid-IR bug).

With this, **E4d-BA-arr is complete** — and the E4-dyn (runtime strings) arc is
fully closed across every frontend payoff.

