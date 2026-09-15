## 0.209.0 - 2026-07-13 — E6d-4: Twig symbols / quote on the code-gen backends

Two matrix cells prove Twig **symbols** (quote literals + `equal?` identity):

- `(equal? 'a 'a)` → #t → exit 1  (same symbol)
- `(equal? 'a 'b)` → #f → exit 0  (distinct symbols)

on `[NativeAot, Llvm, Wasm, Jvm, Clr]`. A Twig quote literal (`'a` / `(quote a)`)
now lowers to `const Var(name) : symbol` — the interned-const form McCarthy Lisp
emits (twig-ir-compiler 0.43.0) — instead of the runtime `make_symbol` string path
(which needs data-section string emission the code-gen backends lack). This rides
the already-wired `intern_symbols` (native) / `intern_symbols_structural` (managed)
passes: each distinct name gets one module-wide id in a reserved high range, so a
symbol never collides with an integer atom and `equal?` on symbols is bit-equality
— no new value type, no backend change. twig-vm is unaffected (its `const Var(name)`
dispatch already interns text to a symbol). Run-verified locally on WASM
(in-process) + real dotnet CLR; native/LLVM/JVM via CI. (Runtime symbol *creation*
and `symbol->string` name recovery on the code-gen backends remain deferred — they
need the `make_symbol` data-section path.)

