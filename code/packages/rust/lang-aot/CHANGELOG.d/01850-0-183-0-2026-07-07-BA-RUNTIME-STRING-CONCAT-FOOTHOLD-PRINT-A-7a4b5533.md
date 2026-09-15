## 0.183.0 — 2026-07-07 — BA runtime string CONCAT foothold (`PRINT A$ + B$` over two `INPUT`s)

New matrix cell `10 INPUT A$ / 20 INPUT B$ / 30 PRINT A$ + B$ / 40 END`
(stdin `"OK\n!\n"` → `OK!`). Both `str_concat` operands are read from `INPUT`,
so neither carries any compile-time string metadata — the concatenation can only
happen at run time. Every prior `str_concat` cell had at least one foldable operand;
this is the first proof of `str_concat` over **two genuinely runtime operands**.

- **Proven columns** `[Jvm, Clr, Vm, Jit]` — their `str_concat` is already a runtime
  operation, so no new lowering was needed:
  - **VM/JIT** — a `str` is a tagged `Value::Str`; concat allocates a fresh tagged
    string from the two operands' bytes at run time.
  - **JVM** — the two `str` locals are `java.lang.String` references; `str_concat`
    builds a new `String` from them (operands need no compile-time identity).
  - **CLR** — the same via `System.String::Concat(string, string)`.
- **Deferred to follow-up PRs** `[NativeAot, Llvm, Wasm]` — their `str_concat` is
  currently literal-fold-only (twig-aot folds only when both operands are known
  literals; `iir-to-llvm`'s `lower_str_concat` requires literal values; `iir-to-wasm`
  uses data-segment metadata). Each needs a runtime-operand path: NativeAot/​LLVM
  route to the existing `__twig_str_concat(a, b)` archive symbol; WASM bump-allocates
  and copies both operands' bytes in linear memory. Run-verified locally (JVM/CLR via
  real `javac`/`java` + CoreCLR; VM/JIT in-process).

