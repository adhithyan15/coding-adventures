# Changelog — mccarthy-lisp-iir-compiler

## v0.1.0 — 2026-06-03 — initial release (L2a)

First IIR lowering for McCarthy 1960 Lisp — the L2a slice of the
McCarthy Lisp plan.

* `compile_source(src, name) -> Result<IIRModule, CompileError>` and
  `compile_forms(&[LispExpr], name)` — lower a top-level form sequence
  into a single `main` function returning the value of the last form
  (empty program → `nil`).
* Supported forms: integer literals, `()`/nil, `QUOTE` (symbols +
  integers + nested lists → cons chains), and the primitives `CONS`,
  `CAR`, `CDR`, `ATOM` (lowered to `not pair?`), and `EQ` (lowered to
  `equal?`, which is identity on atoms — the numeric `=` builtin rejects
  symbols).
* Emits the same IIR opcode + builtin conventions Twig uses
  (`const`/`call_builtin "cons"`/`"car"`/`"cdr"`/`"pair?"`/`"not"`/`"equal?"`,
  `const Var(name)` for symbols, `const 0 : ref<LispyPair>` for nil), so
  the module runs end-to-end on `twig-vm` and feeds every IIR backend.
* `CompileError` reports unsupported forms (`COND`→L2b, `LAMBDA`/`LABEL`→L2c),
  unbound bare symbols, wrong arity, and unknown primitives — with
  actionable messages.

### Tests

* 15 unit tests pin the emitted IIR shape (op sequences, `may_alloc` on
  `cons`, symbol-as-`Var`, nil-as-`ref<LispyPair>`-zero, error paths).
* 16 end-to-end tests run compiled programs on `twig-vm` and assert the
  resulting `LispyValue`: `(CAR '(A B C))` → `A`, `(CDR '(A B C))` →
  `(B C)`, `(CONS 'A 'B)` → `(A . B)`, `(ATOM …)`, `(EQ …)`, nested
  quotes, dotted pairs, and multi-form sequencing.

### Notes

* **Execution VM:** runs on `twig-vm`, not `vm-core` — the latter is
  scalar-only and cannot represent symbols / cons cells.  The plan was
  corrected accordingly.  No `twig-vm` / `lispy-runtime` /
  `lang-runtime-core` source is modified (dev-dependency only), so the
  `twig-vm` Miri obligation does not apply.
