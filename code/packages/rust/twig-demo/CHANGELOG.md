# Changelog — `twig-demo`

## [0.1.4] — 2026-05-13

**All typing states produce correct results; demo and commentary updated.**

Following `twig-aot 0.1.5`'s change of the default integer type from `u64` to
`i64`, all three type states (UNTYPED / PARTIAL / FULL) in `run_typing_demo()`
now produce the correct result `0` for every backend including AOT.

### Updated demos

- **`run_typing_demo()`**: UNTYPED variant note changed from "AOT uses u64
  throughout" to "AOT uses i64 default (signed, correct)".  Introductory text
  updated to reflect that all three states are correct.  The "signed/unsigned
  split" explanation was simplified.
- **`aot_type_correctness_demo()`**: Retitled to "both paths use i64 (signed)".
  Both the untyped (default i64) and typed (explicit i64) paths produce
  `abs(-5) = 5 ✅ CORRECT`.
- **`aot_in_process_demo()`**: Labels changed from "Untyped (u64 ops)" /
  "Typed (i64 ops)" to "Untyped (default i64)" / "Typed (explicit i64)".
- **`run_aot_annotated()`**: Docstring and inline comments updated to describe
  the i64 default rather than the old u64 fallback.
- All constant docstrings (`TYPING_UNTYPED`, `TYPING_PARTIAL`, `TYPING_FULL`,
  `ABS_PROGRAM`) updated to describe the i64-default behaviour.

## [0.1.3] — 2026-05-13

**Optional-typing demo: untyped / partially typed / fully typed.**

Added `run_typing_demo()` which runs three variants of the same program through
all six backends and prints a compile/runtime table for each:

### Program

```twig
(define (add-offset x offset) (+ x offset))
(define (clamp-low x) (if (< x 0) 0 x))
(define (process val) (clamp-low (add-offset val -10)))
(process 5)    ; → 0
```

The critical operation is `(< x 0)` in `clamp-low`.  With unsigned u64
semantics, −5 compares as a huge positive number and the branch is never
taken (returns −5, wrong).  With signed i64 semantics, −5 < 0 is true
(returns 0, correct).

### Type states

| State    | Annotations |
|----------|-------------|
| UNTYPED  | none |
| PARTIAL  | `clamp-low` only: `(x : int) -> int` |
| FULL     | all three functions fully annotated |

### Key finding

A single annotation on the function where the comparison lives is
sufficient to fix the AOT backend — `PARTIAL` is enough.  The other five
backends (interpreter, BEAM, WASM, JVM, CLR) are correct in all three
states because they run their own type-inference pass regardless of source
annotations.

### Implementation

- `run_aot_annotated(source)` — new AOT runner that reads `param_refinements`
  from the compiled `IIRModule` and seeds `func.params` type hints from them
  before calling `compile_typed_module_to_arm64_bytes`.  Unannotated params
  default to u64; annotated params use their declared type (e.g. i64 for `int`).
- `run_typing_demo()` — runs all 6 backends for each of the three source
  variants and calls `print_results` (now parameterised on `expected`).
- `print_results` refactored to accept `expected: i64` instead of using the
  module-level `EXPECTED` constant.

## [0.1.2] — 2026-05-13

**Split results table into Compile and Runtime columns.**

The main 6-backend table now shows two separate timing columns:

- **Compile** — time spent turning Twig source into backend-specific
  bytecode or a native binary.  Includes IIR generation, type inference,
  backend lowering, and (for AOT/BEAM/JVM) any subprocess link/assemble step.
- **Runtime** — time spent actually executing the generated code.
  In-process backends (Interpreter, WASM, CLR) show pure interpreter
  throughput in µs.  Subprocess backends (AOT, BEAM, JVM) include OS
  process-launch and runtime-startup cost.

A new `format_time` helper auto-scales: < 1 000 µs → "Xµs",
< 1 000 ms → "Xms", ≥ 1 s → "X.Xs".

Example output:
```
Backend                            Compile     Runtime    Result  Status
────────────────────────────────────────────────────────────────────────
Interpreter (twig-vm)                  1ms       230µs        55  ✅ PASS
AOT (ARM64 native)                   230ms       210ms        55  ✅ PASS
BEAM (Erlang VM)                      45ms        1.8s        55  ✅ PASS
WebAssembly (Rust runtime)             2ms         8µs        55  ✅ PASS
JVM (Java 21)                          8ms       180ms        55  ✅ PASS
CLR (.NET 9)                           3ms        94µs        55  ✅ PASS
```

Each `run_*` function now returns `(compile_us, run_us, Result<i64, String>)`.
For the interpreter, `TwigVM::compile` is timed separately from `twig_vm::run`.

## [0.1.1] — 2026-05-13

**AOT deep-dive section: phase breakdown + in-process execution + type correctness.**

Added `run_aot_demos()` called after the 6-backend table on macOS/ARM64.
Three sub-sections:

### AOT phase breakdown

Times the three phases of `AOT (ARM64 native)` separately to show where
the ~200ms cost comes from:

- **Compile** (`compile_macos_arm64_object`): IIR → ARM64 Mach-O object — ~2ms
- **Link** (`ld` subprocess): object → native executable — ~26ms (the real bottleneck)
- **Exec** (subprocess fork + dyld + fib(10)) — varies by system

### In-process AOT (no ld, no subprocess)

Demonstrates `compile_module_to_arm64_bytes` + `call_arm64_function_in_process`
from `twig-aot 0.1.4`.  Both variants show <1ms execution for `fib(10)`:

- **Untyped (u64 ops)**: standard prep pipeline, params promoted to `u64`.
- **Typed (i64 ops)**: params set to `"i64"`, AOT propagation uses signed types.

Both return `55`.

### Type correctness: abs(-5)

Demonstrates the signed/unsigned comparison gap with
`(define (abs-val x) (if (< x 0) (- 0 x) x)) (abs-val -5)`:

| Backend | Result | Correct? |
|---------|--------|----------|
| Untyped (u64) | -5 | WRONG — `-5` stored as `0xFFFFFFFFFFFFFFFB`, `< 0` unsigned is false |
| Typed (i64)   |  5 | CORRECT — signed comparison sees `-5 < 0` → true → returns `0 - (-5) = 5` |

The typed path works by setting function params to `"i64"` before calling
`compile_typed_module_to_arm64_bytes`, which propagates `i64` to the
comparison instruction and emits `cmp_lt_i64` (signed ARM64 condition).

### Implementation note: `iir_type_checker` removed from typed AOT path

The initial implementation ran `iir_type_checker::infer_and_check` before
`compile_typed_module_to_arm64_bytes`.  This was wrong: the type checker
sets comparison instructions to `"bool"`, which the AOT propagation pass
then skips (already typed), resulting in unsigned `cmp_lt_bool` anyway.

The correct typed AOT pipeline is:
1. `pre_lower_aot_builtins_on_module` — converts `call_builtin "<"` → `cmp_lt`
2. Set params to `"i64"` manually
3. `compile_typed_module_to_arm64_bytes` — propagates from i64 params → `cmp_lt_i64`

The `iir_type_checker` import is still used by the JVM and CLR backends.

## [0.1.0] — 2026-05-13

### Added

Initial release.  End-to-end multi-backend demonstration binary.

- Compiles and runs `(define (fib n) …) (fib 10)` through six backends:
  Interpreter (twig-vm), AOT (ARM64 native), BEAM (Erlang), WebAssembly
  (pure-Rust runtime), JVM (Java class file + `java`), CLR (CIL bytecode +
  built-in simulator).

- All six backends return **55** (`fib(10)`), confirming the shared IIR
  pipeline and all backend lowerers are consistent.

- **JVM backend** (`run_jvm`):
  - Uses the full pre-lower → `infer_and_check` → `fixup_control_flow_types`
    pipeline to resolve `"any"` types before JVM lowering.
  - Hand-generates a `TwigLauncher.class` (Java 21) with the required
    `public static void main(String[])` entry point; uses
    `invokestatic TwigFib.main()J + l2i + System.exit` so the process
    exit code equals `fib(10) % 256 = 55`.
  - `fixup_control_flow_types` has three passes:
    - Pass 0: normalise param types `"any"` → `"i64"`
    - Pass 1: build SSA env from concretely-typed dests
    - Pass 2: fix `"any"` hints on control-flow and arithmetic ops
    - Pass 3: infer `func.return_type` from `ret` instruction src

- **CLR backend** (`run_clr`):
  - Same pre-lowering pipeline as JVM.
  - Multi-method CIL simulator (`run_cil_artifact` / `exec_method`)
    handles the full CIL opcode subset emitted for integer programs
    including two-byte comparison opcodes (`0xFE 0x01` ceq etc.).
  - Fixed `entry_method` lookup to use `entry_label` (not hardcoded
    `methods[0]`) so the correct `main` function is called when the
    module contains multiple methods.

- **AOT backend** (`run_aot`):
  - Calls `twig_aot::compile_file_macos_arm64`; reads the process exit
    code as the result (`fib(10) % 256 = 55`).

- **BEAM / WASM backends**: delegate to `twig_to_beam::compile_twig_to_beam`
  and `twig_to_wasm::compile_twig_to_wasm` respectively; no fixup needed
  as those pipelines handle type normalisation internally.
