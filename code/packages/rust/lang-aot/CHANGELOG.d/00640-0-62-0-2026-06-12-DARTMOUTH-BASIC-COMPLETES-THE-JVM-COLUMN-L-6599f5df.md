## 0.62.0 — 2026-06-12 — Dartmouth BASIC completes the JVM column (LANG-MATRIX LM-J BASIC)

`lang_matrix.rs` greens **Dartmouth BASIC on real `java`**, completing the JVM column
for every non-Brainfuck language. BASIC's `PRINT` lowers to `invokestatic
env/BasicRuntime.println(J)V`, so `run_jvm` now:

* compiles a tiny `env.BasicRuntime` host class (`println(long)` → `System.out`) with
  `javac` onto the classpath — the JVM sibling of the wasm `PrintHost` / LLVM print
  runtime — only for I/O programs (the expression languages still run a standalone
  `Main.class`);
* reads the entry method's **real** return descriptor and, for an I/O program, injects
  a **discard** launcher (`invokestatic main; pop`/`pop2; return`) rather than the
  print launcher, since BASIC writes its own output as a side effect; then captures
  `System.out`.

**Backend fix found by running on real `java`:** a printing function must keep the
**wide i64** value model. `print_i64` lowers to `lload val; invokestatic
env/BasicRuntime.println(J)V`, so concretizing the value to `i32` made it `istore`d as
an `int` but `lload`ed as a `long` — which `java` rejects with `VerifyError: Accessing
value from uninitialized register pair`. `concretize_scalar_any_for_jvm` now skips any
function that calls `print_i64` (exactly as it already skips lisp/heap functions), so
BASIC's entry stays `()J` and the printed value round-trips as a `long`. The 32-bit
in-repo `jvm-simulator` never exercises BASIC, so real `java` is the only thing that
runs this path.

Verified by RUNNING (OpenJDK 21): BASIC `10 PRINT 42` → `System.out` `42`. **21 proven
matrix cells** (was 20); the expression-language JVM cells and the `jvm_emit`,
`conformance`, `wasm_emit` suites all still green. JVM column complete except the
deferred Brainfuck (tape ops).

