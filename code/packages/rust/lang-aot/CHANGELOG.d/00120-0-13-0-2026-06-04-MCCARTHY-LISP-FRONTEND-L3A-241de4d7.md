## 0.13.0 — 2026-06-04 — McCarthy Lisp frontend (L3a)

### What changed

`lang-aot` now drives **McCarthy Lisp** (the 1960 Lisp 1.0) — added the
`Language::McCarthyLisp` variant, wired through `mccarthy-lisp-iir-compiler`.

* `Language::McCarthyLisp` with `--lang` aliases `mccarthy-lisp` /
  `mccarthy` / `mcl` / `lisp`, file-extension detection for `.mcl` and
  `.lisp`, and a `compile_source_to_iir` arm that routes McCarthy source
  through `mccarthy_lisp_iir_compiler::compile_source` to an `IIRModule`.
  Added the `mccarthy-lisp-iir-compiler` path dependency.
* Because the emit/back-end dispatch is language-agnostic once an
  `IIRModule` exists, McCarthy automatically reaches every existing
  `--emit` target.  **Scalar** McCarthy programs run end-to-end on the
  native AOT pipeline today (`42` → executable exits 42, exactly like the
  Nib smoke test).
* **Scope (L3a).** This wires the frontend and proves the scalar path.
  Programs that return a **symbol or cons** (e.g. `(CAR '(A B C))` → `A`,
  `(CONS 'A 'B)`) currently get a clean `AotError::BackendRefused` from the
  native backend — lowering the `lispy-runtime` value model (symbol
  interning, heap cons cells) into each backend is **L3b**, tracked
  separately.  CLI help marks McCarthy as "full IIR; scalar programs run on
  every AOT target (symbol/cons backend support: WIP)".
* Tests: 3 new unit tests (parse/Display round-trip for the McCarthy
  aliases; `.mcl`/`.lisp` extension detection; `compile_source_to_iir`
  yields a valid `main`-entry module for a spread of McCarthy programs incl.
  the symbol/cons worked example; a frontend lex error surfaces as
  `FrontendError`) + a native end-to-end smoke test (`42` → exit 42,
  Linux/Windows-gated like the other languages).

