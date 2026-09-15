## 0.8.0 — 2026-06-22 — `abs` standard function (LANG-FULL AL8, PR-1)

ALGOL 60 *standard functions* (§3.2.4) are built into the language rather than
user-declared procedures.  This release adds the first one, **`abs`**.

- `abs(E)` yields the absolute value of `E`, preserving its numeric type
  (`integer`→`integer`, `real`→`real`).  It lowers inline to the value of
  `if E < 0 then -E else E`: a `cmp_lt` against a typed zero, then a
  `jmp_if_false` choosing between a negated (`0 - E`, i.e. `sub`/`fsub`) and a
  pass-through `mov` into a single result slot.  This is the same store-per-branch
  shape the conditional-expression lowering already runs on **all seven backends**
  (native-AOT/LLVM/WASM/JVM/CLR/VM/JIT) — no backend learns anything about `abs`;
  it is compare + branch + subtract in the shared IIR.  `E` is evaluated once.
- **Resolution is name-based and overridable.** A standard function has no
  `proc_sigs` entry, so a call resolves to the built-in only when the name is
  *not* a user-declared procedure — a program that redeclares `procedure abs`
  gets its own version, exactly as the Report permits.
- **No grammar change.** `abs(x)` already parses as a `proc_call`; only the
  IIR-compiler's call lowering changed.
- **Verified by RUNNING:** a new `lang_matrix.rs` cell — `result := abs(0 - 42)`
  ⇒ exit **42** — executes on every backend; plus 9 inline tests (negative /
  positive / zero / composed integer `abs`, negative / positive real `abs`, the
  lowers-to-branches-not-a-call structural check, the user-override case, and the
  wrong-arity rejection).

`sign`/`entier`/`sqrt`/`sin`/`cos`/… follow in later AL8 slices (the
transcendentals need a runtime math library on every backend; the pure-IIR
`abs`/`sign`/`entier` come first).

