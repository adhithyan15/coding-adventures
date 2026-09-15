## 0.49.0 — 2026-06-11 — McCarthy **cons/car/cdr** on real CoreCLR (CLR-real C2)

`compile_source_to_cil_text` now compiles cons programs (via `iir-to-cil-bytecode`
0.12.0). The `ilasm`/`dotnet` e2e harness is extracted to `tests/clr_support/mod.rs`
(shared by all CLR-real test files) with a robust `find_ilasm` that searches every
`*ilasm*` NuGet package — the binary ships only in the `runtime.<rid>.*` pack, not
the ref-only `microsoft.netcore.ilasm`, so picking the first match was fragile. New
e2e `tests/clr_real_cons.rs`: `(CAR (CONS 7 9))`→7, `(CDR …)`→9, nested→2 on **real
CoreCLR**; `clr_real_scalar.rs` refactored onto the shared harness.

