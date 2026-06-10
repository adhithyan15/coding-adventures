# Changelog — `lang-aot`

## 0.29.0 — 2026-06-10 — McCarthy → **CLR ATOM/EQ/COND** on the simulator (LANG77 / W7)

The CLR backend now runs McCarthy's primitive predicates (F3–F5). The same
managed structural pipeline `compile_source_to_cil_artifact` already runs (no
driver change) emits `pair?`/`not`/`equal?`/`jmp_if`; `iir-to-cil-bytecode` 0.9.0
+ `clr-simulator` 0.3.0 (new `isinst`/`xor` + ref-aware compares) execute them.
New `tests/cil_predicates.rs`: `(ATOM 7)`→1, `(ATOM (CONS 1 2))`→0, `(EQ 7 7)`→1,
`(EQ 7 8)`→0, `(COND ((ATOM 7) 100) ((ATOM 8) 200))`→100, and fall-through→200.

## 0.28.0 — 2026-06-10 — McCarthy → **CLR cons** on the simulator (LANG77 / W6b)

`compile_source_to_cil_artifact` now runs the **managed value-model pipeline**
(the same `lower_heap_builtins` + `intern_symbols_structural` +
`lower_lisp_repr_structural` the wasm/JVM paths use), so McCarthy **cons** runs on
the CLR: `(CAR (CONS 7 9))` → 7, `(CDR (CONS 7 9))` → 9, nested cons too, on the
object-capable in-repo `clr-simulator` (W6b-1). The structural passes emit
backend-agnostic `box`/`unbox`/`alloc`/`field_*`; `iir-to-cil-bytecode` 0.8.0
lowers them to `box [int32]`/`unbox.any` + `object[]` cells (where wasm uses
`i31ref`/`$LispyPair` and the JVM `Integer`/`Object[]`). New `tests/cil_cons.rs`.

## 0.27.0 — 2026-06-10 — McCarthy → **BEAM (Erlang VM)** run-foundation (scalar) (LANG77 / W9a)

Adds `compile_source_to_beam` — the **fourth** managed `--emit` target and the
first on the **Erlang VM**. Source → IIR → `concretize_scalar_any_for_beam`
(scalar `any` → `i64`; the BEAM has native arbitrary-precision integers) →
`iir-to-beam` → `encode_beam` (a `.beam` module exporting `main/0`). **Scalar
McCarthy programs emit a `.beam` that RUNS** — verified by running it on a real
`erl` (OTP 28): `42`→42, `0`→0, `7`→7; Twig `42` too. Adds
`LangAotError::BeamBackendError`. BEAM uses the native **Erlang-terms** value
model (integers/atoms/list cells), not the structural uniform-reference model of
WASM/JVM/CLR — so its cons/symbol/lambda lowering (W9+) is its own.

## 0.26.0 — 2026-06-09 — McCarthy → **CLR (CIL)** run-foundation (scalar) (LANG77 / W6a)

Adds `compile_source_to_cil_artifact` — the **third** managed `--emit` target
(after WASM and JVM). Source → IIR → `concretize_scalar_any_for_cil` (scalar
`any`/`i64` → CLR `i32`) → `iir-to-cil-bytecode`. **Scalar McCarthy programs emit
CIL that RUNS** — verified by running the entry method's IL on the in-repo
`clr-simulator` (zero external `dotnet`, mirroring how the JVM path uses
`jvm-simulator`): `42`→42, `0`→0, `7`→7; Twig `42` too. Adds
`LangAotError::ClrBackendError`. The cons/symbol/lambda uniform-`object` value
model (the CLR replication of the shared structural passes, reusing the JVM
strict-backend fixes) is W6b+.

## 0.25.0 — 2026-06-09 — McCarthy **`LAMBDA`/`LABEL`/recursion** on the JVM — **JVM complete** (LANG77 / W5b, F7)

`compile_source_to_jvm` now runs McCarthy functions on a real `java`:
`((LAMBDA (X) X) 5)`→5, multi-arg lambdas, `(CAR ((LAMBDA (X) (CONS X X)) 7))`→7,
and a **recursive `LABEL`** walking a list to its atom→99. The win is in
`iir-builtin-lowering` 0.13.0 (lisp-`call` results typed `ref<any>` + reference
funnels) — the JVM backend already lowered `Object`-param/return methods +
`invokestatic`. **With this the JVM backend is McCarthy-complete (F1–F7)** — the
second managed backend done after WASM. New `tests/jvm_lambda.rs`.

## 0.24.0 — 2026-06-09 — McCarthy **symbols** on the JVM (LANG77 / W5a, F6)

`compile_source_to_jvm` now produces working classes for McCarthy **symbols**:
`(EQ 'X 'X)` → 1, `(EQ 'X 'Y)` → 0, `(QUOTE X)` → its interned id, all run on a
real `java`. No lang-aot code change — the win is in `iir-to-jvm-class-file`
0.10.0, which fixed the large-`int` constant `ldc` path (a symbol id lives in the
`2²⁹` reserved range, too big for `bipush`/`sipush`; the old backend emitted an
invalid `ldc 0` that crashed the JVM). New `tests/jvm_symbols.rs` round-trips it
on a real JVM.

## 0.23.0 — 2026-06-09 — McCarthy → **JVM `ATOM`/`EQ`/`COND`** on a real JVM (LANG77 / W4)

The JVM backend now lowers the lisp predicates (`pair?`/`not`/`equal?`), so
McCarthy `ATOM`, `EQ`, and `COND` run on a real `java`: `(ATOM 5)`→1,
`(ATOM (CONS 1 2))`→0, `(EQ 5 5)`→1, `(EQ 5 6)`→0, `(COND ((EQ 1 1) 7) (5 9))`→7.
Same shared structural pass as wasm — only the per-builtin JVM lowering is new
(`instanceof Object[]` / `ixor` / `checkcast`+`intValue`+`if_icmpeq`). The
real-`java` test harness (`tests/jvm_predicates.rs`) is now **descriptor-aware**:
a predicate result is `int` (`()I`), a COND selecting an integer atom is `long`
(`()J`) — it picks the matching `println` overload. (Symbols — F6 — are W5: their
interned ids land in a high range that needs `ldc`, handled separately.)

## 0.22.0 — 2026-06-09 — McCarthy → **JVM cons** on a real JVM (LANG77 / W3b)

`compile_source_to_jvm` now runs the **managed value-model pipeline** (the same
`lower_heap_builtins` + `intern_symbols_structural` + `lower_lisp_repr_structural`
the wasm path uses), so McCarthy **cons** runs on the JVM: `(CAR (CONS 7 9))` →
7, `(CDR (CONS 7 9))` → 9, nested cons too. The structural passes emit
backend-agnostic `box`/`unbox`/`alloc`/`field_*`; `iir-to-jvm-class-file` lowers
them to `Integer.valueOf`/`intValue` + `Object[]` cells (where wasm uses
`i31ref`/`$LispyPair`) — the reusable primitive a future lisp inherits. Adds
`compile_source_to_jvm_class` (returns the `JvmClassFile` pre-serialization, so a
caller can inject a `main` launcher). Verified by **running on a real `java`**
(Temurin 21; the cons cells are `Object[]` the in-repo `jvm-simulator` can't
execute) — see `tests/jvm_cons.rs`.

## 0.21.0 — 2026-06-09 — McCarthy → **JVM** run-foundation (scalar) (LANG77 / W3a)

Adds `compile_source_to_jvm` / `compile_file_to_jvm` — the second *managed*
`--emit` target. Source → IIR → `concretize_scalar_any_for_jvm` (scalar `any`/
`i64` → JVM `i32`) → `iir-to-jvm-class-file` → a serialized `.class`. **Scalar
McCarthy programs emit a class that RUNS** — verified end-to-end by parsing the
emitted bytes and running the entry method on the in-repo `jvm-simulator` (zero
external `java`, mirroring how the wasm path uses `wasm-runtime`): `42` → 42,
`0` → 0, `7` → 7; Twig `42` too. The cons/symbol/lambda uniform-`Object` value
model (the JVM replication of the WASM structural passes) is W3b+.

## 0.20.0 — 2026-06-09 — McCarthy → WebAssembly, **`LAMBDA`/`LABEL`/recursion** (LANG77 / W2)

`compile_source_to_wasm` now runs McCarthy functions: `LAMBDA` application,
multi-argument lambdas, and recursive `LABEL`. The structural pass makes the
call boundary uniform-anyref (params anyref, call args boxed, lambda returns
anyref), so `((LAMBDA (X) X) 5)` → 5, `(CDR ((LAMBDA (X Y) (CONS X Y)) 3 4))` →
4, and a recursive `LABEL` walks a list to its atom. `concretize_scalar_any_for_wasm`
skips functions with lisp params. **With this the WASM backend is McCarthy-complete
(F1–F7): cons, ATOM, EQ, COND, symbols, and lambda/label/recursion.** Twig/scalar
programs are unaffected (regression-tested).

## 0.19.0 — 2026-06-09 — McCarthy → WebAssembly, **symbols** (LANG77 / W1)

`compile_source_to_wasm` now runs `intern_symbols_structural` (before the repr
pass), so McCarthy **symbols** (`QUOTE` / `'A`) work: each distinct symbol is a
distinct interned value (boxed as `i31ref`), so `(EQ 'A 'A)` → T, `(EQ 'A 'B)` →
nil, `(EQ 'A 5)` → nil. Symbols flow through cons cells and `COND` guards. With
this, the WASM backend runs the full McCarthy core **plus symbols** (F1–F6);
integer/cons/scalar programs are unaffected.

## 0.18.0 — 2026-06-09 — McCarthy → WebAssembly, **`COND`** (LANG77 / L3b-3a-4d)

`compile_source_to_wasm` now compiles McCarthy's `COND` conditional with correct
lisp-truthiness. The structural pass wraps a lisp-value clause guard with
`not(is_null(...))`, so an integer atom (even `0`) is true and only `nil` is
false; predicate guards (`pair?`/`EQ`) test directly. The control flow already
lowered. Verified end-to-end: `(COND ((ATOM 5) 7) (5 9))` → 7,
`(COND ((ATOM (CONS 1 2)) 7) (5 9))` → 9, `(COND (0 7) (5 9))` → 7 (0 truthy!),
`(COND ((ATOM (CONS 1 2)) 7))` → nil (exit 0). **This completes the McCarthy
core — cons, ATOM/pair?, EQ, and COND — on the wasm backend.**

## 0.17.0 — 2026-06-08 — McCarthy → WebAssembly, **`EQ`/`equal?`** (LANG77 / L3b-3a-4c)

`compile_source_to_wasm` now compiles McCarthy's `EQ` (atom equality): the atoms
are boxed as `i31ref` by the structural pass, and `iir-to-wasm` lowers `equal?`
to unbox-both + `i32.eq`. **`(EQ 5 5)` → 1, `(EQ 5 6)` → 0**, and the compared
values may be computed (`(EQ (CAR (CONS 3 4)) 3)` → 1). Atom equality only
(McCarthy `eq`); deep structural `equal` over cons cells is later.

## 0.16.0 — 2026-06-08 — McCarthy → WebAssembly, **`ATOM`/`pair?`** (LANG77 / L3b-3a-4b)

`compile_source_to_wasm` now compiles McCarthy's `pair?` / `ATOM` predicate. The
structural representation pass boxes the predicate's integer atom as an `i31ref`
and concretises the boolean result to `i32`; `iir-to-wasm` lowers `pair?` to
`ref.test $LispyPair` and the lisp `not` to `i32.eqz`. So `ATOM x` =
`not(pair? x)` runs: **`(ATOM 5)` → 1, `(ATOM (CONS 1 2))` → 0**. Cons and scalar
programs (McCarthy and Twig) are unaffected (regression-tested).

## 0.15.0 — 2026-06-08 — McCarthy → WebAssembly, **cons** (LANG77 / L3b-3a-3c)

`compile_source_to_wasm` now compiles McCarthy **cons** programs — not just
scalars — to a runnable WasmGC module. The pipeline gains the structural
representation pass between the heap lowering and the scalar concretizer:

```
lower_heap_builtins            cons/car/cdr → alloc/field_store/field_load
lower_lisp_repr_structural     box atoms → i31ref, unbox the entry result   ← new
concretize_scalar_any_for_wasm any → i64 for the remaining pure-scalar fns
```

The two representation passes partition the module's functions (heap-using vs
pure-scalar), so every value ends up concretely typed. **`(CAR (CONS 7 9))`
emits a `.wasm` that runs to `7`** on the in-repo `wasm-runtime`; `CDR` and
nested cons work too. The previous "cons is cleanly unsupported" test is
replaced by these end-to-end runs. Scalar McCarthy and Twig programs are
unaffected (regression-tested).

## 0.14.0 — 2026-06-05 — McCarthy → WebAssembly, scalar (LANG77 / L3b-3a-2)

Adds `compile_source_to_wasm` / `compile_file_to_wasm` — the first of the
modern *managed* `--emit` targets. The pipeline runs the **structural** heap
lowering (`iir_builtin_lowering::lower_heap_builtins`) then
`iir-to-wasm`'s WasmGC backend + `encode_module`.

**Scope: scalar programs.** The managed backends are *typed* and reject the
polymorphic `"any"` lisp value (a `LispyValue`), so a new
`concretize_scalar_any_for_wasm` pass retypes `"any"`→`"i64"` for any function
with **no heap/reference ops** (every value there is a machine integer). Cons/
symbol programs need the boxed-`anyref` value model — a follow-up slice — and
fail cleanly with a `WasmBackendError` for now.

**Verified end-to-end, zero-external-dep:** the new tests *run* the emitted
module on the in-repo `wasm-runtime` (a dev-dependency) and assert the result —
McCarthy `42` → a `.wasm` whose `main` returns `i64 42`; a Twig `42` runs the
same path (reusability); a cons program is a clean error. New
`WasmBackendError` variant.

## 0.13.2 — 2026-06-04 — McCarthy symbols e2e (L3b-2c-3)

Adds Linux/Windows end-to-end smoke tests for native McCarthy symbols — the
worked example `(CAR '(A B C))` → `A`, observed via `EQ` + `COND`:
`(COND ((EQ (CAR (QUOTE (A B C))) (QUOTE A)) 7) ((QUOTE T) 9))` → exits 7 and
the `(QUOTE B)` variant → exits 9. Test-only; no library change.

## 0.13.1 — 2026-06-04 — McCarthy ATOM/EQ + COND e2e (L3b-2c-2)

Adds Linux/Windows end-to-end smoke tests exercising McCarthy `ATOM`/`COND`
through the native pipeline: `(COND ((ATOM 5) 7) (5 9))` → exits 7 and
`(COND ((ATOM (CONS 1 2)) 7) (5 9))` → exits 9 (ATOM of an int is true; ATOM
of a pair is false). Test-only; no library change.

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

## 0.12.0 — 2026-06-03 — Phase 7 (FINAL lane) of historical-arch backend migration

### What changed

`--emit=riscv32` now routes through `aot_core::infer` +
`aot_core::specialise` + `riscv_backend::compile` (the new
`Backend` trait implementation) instead of `iir_to_riscv`
(deprecated as of v0.4.0).

Same pattern as the previous five migration phases for GE-225
(Phase 3), Intel 4004 (Phase 4), ARMv7 (Phase 5), and Intel 8008
(Phase 6).

### Migration complete

With Phase 7 landed, every historical-arch lane now consumes typed
CIR via the `Backend` trait.  The historical migration is **done**.

See `code/specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md` for the full
end-state summary.

### Dependencies

* Removed: `iir-to-riscv` (deprecated; lang-aot no longer pulls
  it in).
* Added: `riscv-encoder`, `riscv-backend`.

### Test surface

* The existing `end_to_end_basic_print_emits_riscv32_bin_via_lang_aot`
  test now exercises the new CIR-via-Backend path.  BASIC
  `PRINT 42` lowers `call_builtin print_i64`, which the v0.1.0
  `riscv-backend` doesn't yet cover — the test treats that as an
  expected gap and skips with `eprintln!`, identical to its
  behaviour during Phases 5 and 6.

## Unreleased — A5++++++++ — **Dartmouth BASIC end-to-end through GE-225**

### The historical round-trip

The **GE-225** (1959) was the General Electric mainframe at
Dartmouth College where **John Kemeny and Thomas Kurtz designed
Dartmouth BASIC in 1964**.  BASIC was *born* on this silicon.

As of this release, the LANG VM lang-aot driver can compile
Dartmouth BASIC source through:

```text
.bas → dartmouth-basic-iir-compiler → IIR → iir-to-ge225 → 20-bit GE-225 words → .bin
```

Sixty-two years after Kemeny and Kurtz first wrote BASIC programs
on the GE-225, BASIC source round-trips back to the silicon it was
designed for.  This is the milestone moment for the GE-225 lane.

### Added — BASIC GE-225 end-to-end smoke tests

Three new tests in `tests/end_to_end_smoke.rs` exercise the
full BASIC → IIR → GE-225 .bin pipeline:

1. `end_to_end_basic_let_a_5_emits_ge225_bin_via_lang_aot` —
   the simplest BASIC program (`10 LET A = 5\n20 END`) compiles
   to a non-empty word-aligned .bin containing at least one
   `LDA` and at least one `HLT`.  Confirms the trivial case
   round-trips.
2. `end_to_end_basic_let_a_1_plus_2_exercises_add_via_lang_aot` —
   `10 LET A = 1 + 2\n20 END` exercises the GE-225 ADD opcode
   (0x04) inside the emitted byte stream.
3. `end_to_end_basic_print_documents_call_builtin_gap` —
   `10 LET A = 5\n20 PRINT A\n30 END` documents the
   `call_builtin` lowering gap (currently rejected with
   `UnsupportedOp`); a future iir-to-ge225 increment that adds
   `call_builtin` lowering will automatically activate the test.

Tests tolerate "lowering gap" errors so the cascade keeps
progressing as BASIC frontend and GE-225 backend both add ops
over time.

### Known BASIC ⇄ GE-225 gaps

| BASIC IIR op | iir-to-ge225 v0.7.0 status |
|--------------|----------------------------|
| `const`, `mov`, `add`, `cmp_le`, `jmp`, `jmp_if_true`, `jmp_if_false`, `label`, `ret` | ✓ supported |
| `call_builtin` (PRINT, etc.) | ✗ deferred |
| `neg` (unary minus) | ✗ deferred |

No version bump on iir-to-ge225 — this is a pure wiring/test
release of lang-aot that consumes iir-to-ge225 v0.7.0 unchanged.

## 0.7.0 — 2026-06-02 (A1+++ — `--emit=riscv32` + iir-to-riscv wiring)

### Added — `--emit=riscv32` flag and `compile_file_to_riscv32_bin` API

Wires `iir-to-riscv` (v0.3.3) into the lang-aot driver.  Source files
for every supported language (Twig, Nib, Brainfuck, BASIC, Oct) can
now be lowered to a flat `.bin` of little-endian 32-bit RV32I
instruction words via:

```text
lang-aot path/to/input.bas --emit=riscv32 [-o out.bin]
```

Aliases accepted for the value: `riscv32` (canonical), `rv32`, `bin`.

When `-o` is omitted, the default output is the input with the
extension replaced by `.bin` (matching the conventional flat ELF-less
RV32I name downstream simulators / `qemu-riscv32` expect).

#### Downstream consumers

* [`riscv-simulator`](../riscv-simulator) — load + execute in-process.
* `qemu-riscv32 -kernel out.bin` — host-side simulation.
* Physical flash loader on a SiFive / ESP32-C3 / RISC-V board.

#### Wire format

Each emitted word is written as **little-endian** bytes per the
RISC-V spec (Volume I §1.4): bit `[7:0]` of the word goes to the
lowest-address byte.

#### Why cross-platform (no host gating)

The native-executable pipelines (`compile_file_to_{linux,windows,macos}_executable`)
are `cfg`-gated because they invoke the host linker.  RV32I `.bin`
emission is **pure byte output** — `compile_file_to_riscv32_bin` runs
on any host.  Downstream loading / running is the caller's job.

#### Public API added

* `pub fn compile_file_to_riscv32_bin(src: &Path, out: &Path,
   language: Language) -> Result<(), LangAotError>`
* `LangAotError::RiscvBackendError(String)` — wraps human-readable
  errors surfaced by `iir-to-riscv`.

#### CLI flag reference

```text
--emit=<MODE>     What to emit:
                    native           → host executable (default)
                    llvm-ir          → textual LLVM IR (.ll)
                    riscv32 | rv32 | bin
                                     → flat RV32I .bin
```

#### Tests added (28 total, was 27)

* `end_to_end_basic_print_emits_riscv32_bin_via_lang_aot` —
  cross-platform e2e: BASIC `PRINT 42` → `.bin`.  Asserts:
  non-empty, 4-byte aligned, last 4 bytes = `0x67 0x80 0x00 0x00`
  (canonical `ret` little-endian).  Tolerates not-yet-covered op
  gaps via the same skip pattern as the LLVM e2e test.

## 0.6.0 — 2026-06-01 (LLVM04 — `--emit=llvm-ir` + iir-to-llvm wiring)

### Added — `--emit=llvm-ir` flag and `compile_file_to_llvm_ir` API

Wires `iir-to-llvm` (v0.4.0) into the lang-aot driver.  Source files for
every supported language (Twig, Nib, Brainfuck, BASIC, Oct) can now be
lowered to textual LLVM IR (`.ll`) via:

```text
lang-aot path/to/input.bas --emit=llvm-ir [-o out.ll]
```

When `-o` is omitted the default output is the input with the extension
replaced by `.ll` (matching what downstream `llc` / `opt` expect).
Accepted aliases for the value: `llvm-ir` (canonical), `llvm`, `ll`.

#### Why cross-platform (no host gating)

The native-executable pipelines (`compile_file_to_{linux,windows,macos}_executable`)
are `cfg`-gated because they invoke the host linker.  LLVM IR emission
is **pure string output** — `compile_file_to_llvm_ir` is therefore
cross-platform and runs on any host.  Downstream `llc` / `opt`
invocations are the caller's job.

#### Public API surface added

* `pub fn compile_file_to_llvm_ir(src: &Path, out: &Path, language: Language)
   -> Result<(), LangAotError>`
* `LangAotError::LlvmBackendError(String)` — wraps human-readable errors
  surfaced by `iir-to-llvm`'s lowerer.

#### Tests added (27 total, was 25)

* `end_to_end_twig_emits_llvm_ir_via_lang_aot`
* `end_to_end_basic_print_emits_llvm_ir_with_print_extern`

Both cross-platform.  Tolerate unsupported-op / unsupported-type errors
from `iir-to-llvm` as "expected gaps" (a future LLVM05+ will broaden
coverage).  The BASIC test asserts on the `@__print_i64` extern shape.

## 0.5.0 — 2026-05-30 (AOT05 — BASIC + Oct smoke parity with Nib)

### Added — 6 new end-to-end smoke tests (BASIC + Oct)

Brings BASIC and Oct's lang-aot smoke coverage from 2 tests each to
5 tests each, matching Nib's breadth.  Closes task #32 from the
multi-language tooling parity work.

#### Oct — 3 new tests (was 2)

- `end_to_end_oct_if_else_exits_zero` — `if x == 0 { x = 1; } else
  { x = 2; }` compiles, links, runs successfully.  Exercises typed
  `cmp_eq` + `jmp_if_false` + `mov` + `jmp` + `label` through native
  codegen.
- `end_to_end_oct_while_loop_exits_zero` — `while n < 10 { n = n + 1; }`
  compiles and runs to completion.  Exercises backward `jmp` (the
  AOT chain's branch-distance encoding) and the typed `cmp_lt`+`add`
  loop body.
- `end_to_end_oct_cross_fn_chain_exits_zero` — `add_one(add_one(8))`
  chains two cross-fn calls through the typed-argument reloc path.

#### BASIC — 3 new tests (was 2)

- `end_to_end_basic_arith_chain_prints_42` — `A + B + C` printed.
  Exercises multiple typed `add` ops through the AOT pipeline.
- `end_to_end_basic_if_then_prints_1` — `IF A > 5 THEN 100` takes
  the then branch, prints 1.  Exercises typed `cmp_gt` +
  `jmp_if_*` with line-label resolution.
- `end_to_end_basic_goto_prints_1` — `GOTO 100` skips the
  assignment on line 30, prints A's original value.  Exercises
  forward unconditional branch resolution.

### Coverage parity

| Language | Smoke tests (before) | Smoke tests (now) |
|---|---|---|
| Twig | 1 | 1 |
| Nib | 5 | 5 |
| Brainfuck | 1 | 1 |
| **BASIC** | **2** | **5** |
| **Oct** | **2** | **5** |

### Tests

All 17 smoke tests pass on the local host platform.  Each test is
gated to its host OS (`#[cfg(target_os = ...)]`) so CI runners only
execute the tests appropriate to their platform.

## 0.4.0 — 2026-05-20 (OCT02 phase 4 — Oct end-to-end on LANG VM)

Oct programs now compile end-to-end via `oct-iir-compiler` (OCT02 phase 3,
PR #3878).  Closes the final phase of the OCT02 four-phase plan — every
language in the LANG74 roadmap (Twig, Nib, Brainfuck, Dartmouth BASIC,
Oct) now ships through the shared LANG VM AOT chain.

**Dispatch wiring.**  `compile_source_to_iir`'s `Language::Oct` arm now
calls `oct_iir_compiler::compile_source` and surfaces frontend errors
(`Unsupported8008Intrinsic`, `Type`, `Parse`) through
`LangAotError::FrontendError`.  The `UnsupportedLanguage` arm is no
longer reachable for any built-in `Language` variant — kept in the
enum so adding a new variant remains a one-arm change.

**End-to-end smoke tests** on both Windows + Linux:

- `end_to_end_oct_minimal_main_exits_zero`: `fn main() { let x: u8 = 42; }`
  compiles + links + runs + exits with the synthesised i64-return code 0.
- `end_to_end_oct_user_fn_call_succeeds`: program with `fn double(a: u8) -> u8 { return a + a; }` and `fn main() { let x: u8 = double(21); }` exercises the cross-function `call` reloc.

Verified locally on Windows.

**Lib test updates.**  `oct_returns_clean_unsupported_error` →
`oct_compiles_to_iir`; new `oct_8008_intrinsic_reports_frontend_error`
confirms the rejection path still surfaces a clean error.

## 0.3.0 — 2026-05-20 (PL05 — Dartmouth BASIC end-to-end on LANG VM)

Dartmouth BASIC programs now compile end-to-end via the new
`dartmouth-basic-iir-compiler` crate.  `lang-aot foo.bas` produces a
native executable on Linux, Windows, and macOS — the same chain Twig,
Nib, and Brainfuck use.

**Wiring.**  The `Language::DartmouthBasic` arm in
`compile_source_to_iir` now calls
`dartmouth_basic_iir_compiler::compile_source` instead of returning
`UnsupportedLanguage`.  No other changes to the lang-aot surface —
the existing `compile_file_to_*_executable` entry points handle
BASIC transparently.

**V1 BASIC coverage.**  Integer-only programs with LET / PRINT /
INPUT / IF / GOTO / FOR / NEXT / END / REM.  GOSUB/RETURN, READ/
DATA, DIM/arrays, and DEF are deferred.  See
[`dartmouth-basic-iir-compiler/CHANGELOG.md`](../dartmouth-basic-iir-compiler/CHANGELOG.md)
for the full table.

**End-to-end smoke tests:**

- `end_to_end_basic_print_42_via_lang_aot` — `10 PRINT 42 / 20 END`
  exits cleanly and writes exactly `"42\n"`.
- `end_to_end_basic_for_loop_prints_1_2_3` — `FOR I = 1 TO 3 / PRINT
  I / NEXT I / END` writes exactly `"1\n2\n3\n"`.

Verified locally on Windows.

**Lib-test renamed.**  `dartmouth_basic_returns_clean_unsupported_error`
is gone; `dartmouth_basic_compiles_to_iir` asserts the new success
path.

## 0.2.0 — 2026-05-20 (BF07 — Brainfuck end-to-end on LANG VM)

Brainfuck programs now compile all the way to a native executable via
`lang-aot foo.bf`.

**New BF lowering pass.**  `lower_brainfuck_for_aot(&mut IIRModule)`
runs after `brainfuck_iir_compiler::compile_source` returns and
rewrites the BF-shaped IIR into a LANG76-shaped one without modifying
the frontend (so existing consumers — `vm-core`, `jit-core`,
`iir-to-wasm` — keep working unchanged):

- Prepends `const __bf_tape_size = 30000` + `alloc_bytes
  __bf_tape_size -> __bf_tape` to `main`.
- Rewrites `load_mem v, ptr` → `load_byte __bf_tape, ptr -> v`.
- Rewrites `store_mem ptr, v` → `store_byte __bf_tape, ptr, v`.
- Replaces the trailing `ret_void` with `const __bf_ret = 0; ret
  __bf_ret`, changing `main`'s return type from `void` to `i64` so
  the LANG VM AOT chain's entry-point convention (exit code = main's
  return value) is satisfied.

**End-to-end smoke test:** `end_to_end_brainfuck_prints_a_via_lang_aot`
on both Windows + Linux compiles `++++++++[>++++++++<-]>+.` (canonical
"print 'A'") through `lang-aot` and asserts stdout is exactly `"A"`.
This exercises every mechanic LANG75 + LANG76 deliver: pointer shift,
cell mutation, nested loops, the 30000-byte tape, and putchar.
Verified locally on Windows.

**Lib test:** `brainfuck_lowering_inserts_tape_and_byte_ops` asserts
the lowering pass produces the expected IIR shape (alloc_bytes
preamble, no leftover load_mem/store_mem, ret/i64 epilogue) without
needing the linker.

## 0.1.0 — 2026-05-20

Initial release.  Multi-language AOT driver that routes Twig, Nib, and
Brainfuck source through the shared LANG VM chain (frontend → IIR →
x86_64-backend / aarch64-backend → object → system linker → native
executable).

### What's wired

| Language | Extensions | Frontend |
|---|---|---|
| Twig | `.twig` | `twig-ir-compiler` |
| Nib  | `.nib`  | `nib-iir-compiler` |
| Brainfuck | `.bf`, `.b` | `brainfuck-iir-compiler` (IIR-emission works; AOT backend doesn't lower BF ops yet) |
| Dartmouth BASIC | `.bas`, `.basic` | placeholder — returns `UnsupportedLanguage` with guidance |
| Oct | `.oct` | placeholder — returns `UnsupportedLanguage` with guidance |

### API

- `Language` enum with `parse(&str)` and `Display`.
- `detect_language_from_path(&Path) -> Option<Language>` — by extension.
- `compile_source_to_iir(language, source, module_name) -> Result<IIRModule, LangAotError>`
  — frontend dispatch.
- `compile_file_to_{linux, windows, macos}_executable(src, out, lang)`
  — full pipeline, cfg-gated to the matching host (same host-targets-
  host policy as `twig-aot`).
- `LangAotError` with `UnsupportedLanguage { language, guidance }`,
  `FrontendError`, `AotError`, `Io` variants.

### Companion change in `twig-aot`

`twig-aot` exposes three new public functions:

- `compile_module_to_linux_executable(&IIRModule, &Path)` (Linux host).
- `compile_module_to_windows_executable(&IIRModule, &Path)` (Windows host).
- `compile_module_to_macos_executable(&IIRModule, &Path)` (Unix host).

…and three new public link helpers:

- `link_linux_x86_64_executable(obj_bytes, stem, out)`.
- `link_windows_x86_64_executable(obj_bytes, stem, out)`.
- `link_macos_arm64_executable(obj_bytes, stem, out)`.

The existing `compile_file_*` functions now delegate to these so the
link logic is shared between source-file input and module input.

### Tests

- 7 lib tests cover language parsing, extension detection, and the
  unsupported-language error paths.
- 3 end-to-end smoke tests (`tests/end_to_end_smoke.rs`) gated to
  the host's OS:
  - `end_to_end_twig_returns_42_via_lang_aot`
  - `end_to_end_nib_returns_42_via_lang_aot`
  - `end_to_end_nib_arithmetic_via_lang_aot` (`30+12`, `if 1==1`,
    `if 1==2`)

All tests pass on Windows x86-64 host.  CI will additionally verify
on `ubuntu-latest` and `macos-latest`.

### Known limitations

- **Host-targets-host only.** Same as `twig-aot` V1.
- **No `--target` / `--emit-object` CLI flags.** Coming in a follow-up.
- **Brainfuck end-to-end gap.** Frontend produces correct IIR, but the
  x86_64-backend and aarch64-backend don't lower BF-specific ops
  (`load_mem`, `putchar`, etc.).  Wiring is correct; backend extension
  is a separate piece of work.
- **Dartmouth BASIC and Oct stubs.** They surface
  `UnsupportedLanguage` errors with one-line guidance on what's needed
  to unblock each.
