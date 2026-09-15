## 0.60.0 — 2026-06-11 — Dartmouth BASIC joins the WASM column (LANG-MATRIX LM-W BASIC)

`lang_matrix.rs` greens **Dartmouth BASIC on WASM** (the `DartmouthBasic` program now
lists `Wasm`), completing the WASM column for every non-Brainfuck language. BASIC's
`PRINT` lowers to a wasm import `env.__print_i64 : (i64) -> ()` — the wasm sibling of the
LLVM column's `@__print_i64` C runtime. The `run_wasm` runner now installs a tiny
`PrintHost` (a test-local `wasm_execution::HostInterface`) that resolves that single
import to a `PrintFunc` capturing each printed `i64` into a shared buffer; the runner
joins the captured values as the program's stdout. The expression languages import no
host functions, so the host is never consulted for them and their behaviour is unchanged
(`main`'s i64 result is still read as the exit code). Verified by RUNNING: BASIC
`10 PRINT 42` → stdout `42` on the in-process `wasm-runtime`. New dev-deps `wasm-execution`
+ `wasm-types` (the host-interface + value types, both in-repo — wasm verification stays
zero-external-dep). WASM column now green for Twig / Nib / Oct / ALGOL 60 / BASIC; only
Brainfuck (tape ops) remains. **16 proven matrix cells.**

