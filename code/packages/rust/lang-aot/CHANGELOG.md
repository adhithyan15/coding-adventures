# Changelog — `lang-aot`

## 0.200.0 - 2026-07-11 (DVAL01-2: rename IIR builtin names lispy_* -> dyn_* + passes)

DVAL01-2: the lang-aot wiring, `jit_lisp.rs`, and the LLVM/JIT/metacircular/
conformance integration tests move to the `dyn_*` IIR builtin names and the
renamed `dyn_repr`/`dyn_repr_structural` passes. Verified: the McCarthy-lisp
cells stay green across VM/JIT/LLVM/native (native now emits the correct
`__dyn_*` runtime symbols); cross-backend agreement is preserved. Pure rename
of the builtin-name surface + the native emit fix (see aarch64/x86_64-backend).

## 0.199.0 - 2026-07-11 (DVAL01-1c: rename Rust crate lispy-runtime -> dynval-runtime)

DVAL01-1c: the Rust golden-reference crate `lispy-runtime` is renamed to `dynval-runtime`, completing the crate-level de-lisp of the generic dynamic-value substrate (spec DVAL01 section 3.2). lang-aot's `Cargo.toml` dependency and every `use lispy_runtime` / `lispy_runtime::` import across `src/jit_lisp.rs` and the LLVM/metacircular/conformance integration tests move to `dynval_runtime`; the local test-helper `lispy_runtime_c()` (which returns the path to the now-`dynval_runtime.c` runtime) becomes `dynval_runtime_c()`. Pure rename -- no ABI, tag-layout, or behaviour change. The public type names (`LispyValue` etc.) and the IIR `lispy_*` builtin names are renamed by DVAL01-2, not here. Verified by the cross-backend matrix staying green on all five code-gen backends.

## 0.198.0 - 2026-07-11 (DVAL01-1b: rename C runtime file lispy_runtime.c -> dynval_runtime.c)

DVAL01-1b: the shared C runtime file is renamed `lispy_runtime.c` -> `dynval_runtime.c` (and the golden test `lispy_runtime_golden.rs` -> `dynval_runtime_golden.rs`), continuing the de-lisp of the generic dynamic-value substrate (spec DVAL01). Pure file/path rename -- no symbol, ABI, or behaviour change; the link/build path strings that reference the runtime are updated to match. The `lispy-runtime` Rust crate rename follows in DVAL01-1c.

## 0.197.0 - 2026-07-11 (DVAL01-1a: dynamic-value runtime ABI __twig_lispy_* -> __dyn_*)

De-lisp the tagged dynamic-value runtime ABI: every `__twig_lispy_*` C symbol (box_int/unbox_int/cons/car/cdr/pair_p/equal/not/nil/make_symbol/truthy/to_exit_code/tag_*) is renamed to the language-neutral `__dyn_*` (per spec DVAL01). Pure rename -- the 3-bit tag layout, encodings, and runtime behaviour are byte-for-byte unchanged, so any dynamic frontend (not just lisp) can target the same primitives. The GC ABI (`__twig_gc_*`) is untouched.

## 0.196.0 — 2026-07-11 — LANG-FULL E6d-2a: dynamic integer arithmetic over `any` (structural backends)

Wires the new `iir_builtin_lowering::lower_dynamic_arith` pass into every managed + native lisp lowering pipeline (after `lower_heap_builtins`), and adds a matrix cell `(+ (car (cons 41 0)) 1)` → 42 on the **structural** code-gen backends `[Wasm, Jvm, Clr]` (whose `i31ref`/`Integer`/boxed-int32 box/unbox now width-adapt to i64). NativeAot + LLVM (NaN-box tagged-i64 world) follow in E6d-2b. Run-verified: WASM in-process, JVM via a reflective launcher (both → 42); CLR via CI.

## 0.195.0 — 2026-07-10 — E6d-1: Twig dynamic cons/car/cdr on the code-gen backends

First slice of **E6 layer 2** (general dynamic dispatch — spec
`code/specs/lang-full-e6-dispatch.md`). Two matrix cells prove Twig's first
genuinely *dynamic* value runs on the code-gen backends:

- `(car (cons 42 0))` → 42
- `(car (cdr (cons 1 (cons 42 0))))` → 42 (nested, multi-cell pointer chasing)

on `[NativeAot, Llvm, Wasm, Jvm, Clr]`. **Zero production code** — the surveyed
fact held: the shared `iir-builtin-lowering` heap passes (`lower_heap_builtins` →
`lower_lisp_repr_structural`), which run for *every* source language, already
lower Twig's `call_builtin "cons"/"car"/"cdr"` to the `alloc`/`field_load`/
`field_store` heap-object family over the uniform boxed `ref<any>` value that
McCarthy Lisp already runs on all five code-gen backends (WASM `anyref` +
`$LispyPair`, JVM `Object[]`, CLR `object[]`, LLVM tagged-i64 + `__twig_lispy_*`
runtime, native). Run-verified locally on **WASM** (in-process) and **real dotnet
CLR**; native/LLVM/JVM via CI.

The generic `Vm`/`Jit` columns run `vm-core` typed IIR (no `ref<any>`/`alloc`), so
dynamic Twig cells list the 5 code-gen columns; `twig-vm` is the off-matrix
interpreter reference. Turns the stale `TW3 ☐` (cons core) into a guardrailed
proof. Remaining E6 layer-2 slices: dynamic arithmetic, list ops, symbols,
records/unions, closures-on-WASM, dynamic globals.

## 0.194.0 — 2026-07-10 — E4d-BA-arr: BASIC string arrays COMPLETE (all 7 backends)

The string-array matrix cell (`DIM A$(2); A$(0)="O"; A$(1)="K"; PRINT A$(0)+A$(1)`
→ `OK`) now runs on **all seven backends**, adding NativeAot / JVM / CLR to the
part-1 Llvm / Wasm / VM / JIT:

- **NativeAot** — `x86_64-backend` 0.24.0 + `aarch64-backend` 0.23.0 accept a `str`
  element as an 8-byte handle (`native_array_elem_size`); twig-aot already
  materialises the handle into the slot, so no store/load change.
- **JVM** — `iir-to-jvm-class-file` 0.30.0 lowers `array<str>` to a
  `java.lang.String[]` (`anewarray` + `aaload`/`aastore`) — the backend's first
  reference-element array.
- **CLR** — `iir-to-cil-bytecode` 0.39.0 lowers it to a `System.String[]`
  (`newarr System.String` + `ldelem.ref`/`stelem.ref`). **Run-verified on real
  dotnet.**
- **LLVM fix** — `iir-to-llvm` 0.36.0 `ptrtoint`s a folded str literal to its i64
  handle in `array_set` (the part-1 cell's Llvm column relied on this; it was a
  latent invalid-IR bug).

With this, **E4d-BA-arr is complete** — and the E4-dyn (runtime strings) arc is
fully closed across every frontend payoff.

## 0.193.0 — 2026-07-10 — E4d-BA-arr: BASIC string arrays (part 1 — Llvm/Wasm/Vm/Jit)

A new matrix cell proves Dartmouth BASIC string arrays:
`DIM A$(2); A$(0)="O"; A$(1)="K"; PRINT A$(0)+A$(1)` → `OK`. The `OK` (not `OO`/`KK`)
proves the two element slots are distinct and each string handle survives a
store→load round-trip through the aggregate.

Runs on **[Llvm, Wasm, Vm, Jit]**:
- **VM/JIT** hold a tagged `Value::Str` element (already supported).
- **WASM** stores a 4-byte i32 handle per element (`iir-to-wasm` 0.36.0).
- **LLVM** carries a `str` element as an i64 handle — no backend change
  (`llvm_type_for("str")` = `i64`).

`dartmouth-basic-iir-compiler` 0.37.0 lowers `DIM A$` / `A$(i)` to `array<str>`
`alloc_array` / `array_set` / `array_get`.

**Part 2** (a follow-up PR) adds the **NativeAot** (native element-size allowance),
**JVM**, and **CLR** columns — the last two need managed reference-array lowering
(`String[]` / `string[]`; `anewarray` + `aaload`/`aastore` / `newarr` +
`ldelem.ref`/`stelem.ref`), which the numeric E5 arrays don't exercise.

## 0.192.0 — 2026-07-08 — ALGOL string value-parameters on NativeAot+LLVM+WASM → **matrix 100% (159/159 on all 7 backends)**

The two ALGOL `string`-value-parameter cells (`echo('HELLO')` with an inline-literal
actual, and `say(msg)` with a named outer-block string slot as the actual — each
`value s; string s; print(s)`) gain their **NativeAot**, **LLVM**, and **WASM** columns.
No backend change was needed: the E4-dyn runtime-string work (E4d-2b LLVM / E4d-3b WASM /
E4d-4 native) already gave `print_str` a runtime path that reads the `[len][bytes]` block
header at run time for any string lacking a compile-time-length entry — which is exactly
a string parameter. The cells' exclusion comments predated that work; run-verified here
(native via a real executable + stdout capture, LLVM via clang, WASM via `wasm-runtime`).

**With this every cell in the conformance matrix (159/159) runs on all 7 backends** —
NativeAot, LLVM, WASM, JVM, CLR, VM, JIT.

## 0.191.0 — 2026-07-08 — ALGOL `for`-loop integer-array now proven on NativeAot + WASM (all 7 backends)

The ALGOL sum-of-squares `for`-loop-over-integer-array cell gains its **NativeAot** and
**WASM** columns, reaching all 7 backends. No backend change was needed: the `for`-loop
lowers to the same generic `alloc_array`/`array_get`/`array_set` + integer relation/
branch ops the straight-line E5 array cell already proved on both backends — the loop is
a pure control-flow composition over ops every backend lowers. Run-verified (WASM via the
in-repo `wasm-runtime`; NativeAot via a real executable in `run_native`).

## 0.190.0 — 2026-07-08 — lang-full tail CLOSED: the Twig `string=?` cell now runs on WASM (all 3 on all 7 backends)

The last remaining lang-full string-tail cell — `string=?` over two runtime string
handles — gains its **WASM** column via `iir-to-wasm` 0.35.0's in-module `$__str_eq`
helper. With this, all 3 Twig/McCarthy-lisp runtime-string cells
(`substring`-result, `let*`-`str_concat`-result, `string=?`) run on **all 7 backends**
— the lang-full string tail is complete.

## 0.189.0 — 2026-07-08 — lang-full tail: 2 of the 3 Twig concat/substring-across-call cells now run on WASM

The `substring`-result and `let*`-`str_concat`-result cells gain their **WASM** column
(`iir-to-wasm` 0.34.0's folded-result runtime-block promotion), so they now run on **all
7 backends**. Both previously produced exit `72` on WASM (`'H'` — the callee read the
first data byte as the string length) and were excluded from the WASM column.

The third cell (`string=?` over two runtime string params) still lacks WASM: `str_eq`
on WASM has no runtime path yet (it requires both operands be direct `str_const` locals).
That is the final lang-full string-tail item, to follow in a separate PR.

## 0.188.0 — 2026-07-08 — lang-full tail: the 3 Twig `str_concat`/`substring`/`string=?`-across-call cells now run on LLVM

The last 3 Twig/McCarthy-lisp string cells — passing a `str_concat`/`substring` RESULT
across a call, or comparing two runtime strings with `string=?` — gain their **LLVM**
column: `[NativeAot, Llvm, Jvm, Clr, Vm, Jit]` (6/7; WASM is the final follow-up).

- The `substring`/`let*`-concat cells' folded results are `@__twig_str` globals already
  `ptrtoint`'d by `lower_call` (0.34.0).
- The `string=?` cell needed `iir-to-llvm` 0.35.0's new runtime `str_eq` path (calls
  `@__twig_str_eq`). The `run_llvm` test harness gains a malloc-free `__twig_str_eq`
  shim + links it when the `.ll` references the symbol.

With this, the only lang-full string-tail work left is these 3 cells' **WASM** column
(promoting a folded `str_concat`/`str_slice` result to a runtime block).

## 0.187.0 — 2026-07-07 — lang-full tail: Twig string literals crossing a call now run on ALL 7 backends

The 4 Twig/McCarthy-lisp cells that pass a string LITERAL across a function boundary
(`(define (strlen s) (string-length s)) (strlen "HELLO")` and variants) gain their
**LLVM** column, reaching **all 7 backends** `[NativeAot, Llvm, Wasm, Jvm, Clr, Vm,
Jit]` (they were 6/7 after 0.186.0's WASM fix). `iir-to-llvm` 0.34.0 `ptrtoint`s a
`str_const` literal's global pointer to an i64 handle at the call site, so the callee's
runtime `str_len` reads the length header. These 4 cells are now fully at all 7 backends.

The remaining 3 cells — those passing a `str_concat`/`substring` RESULT (not a
`str_const`) across a call — are the last of the string tail.

## 0.186.0 — 2026-07-07 — lang-full tail: Twig string literals crossing a call now run on WASM

Four Twig/McCarthy-lisp string cells that pass a string LITERAL across a function
boundary (`(define (strlen s) (string-length s)) (strlen "HELLO")` and variants —
direct arg, top-level `define`, `let` binding) gain their **WASM** column, going from
5 → 6 backends `[NativeAot, Wasm, Jvm, Clr, Vm, Jit]`. They previously returned 72
(=`'H'`) on WASM because the callee read the raw literal's bytes as a length; fixed by
`iir-to-wasm` 0.33.0 promoting a `str_const` literal used as a call argument to a
length-prefixed runtime block. (Bumps the Cargo version to 0.186.0, reconciling it with
the 0.185.0 CHANGELOG entry left by #7770.)

The remaining lang-full string tail — the same cells' **LLVM** column, and the cells
that pass a `str_concat`/`substring` *result* across a call — are follow-ups.

## 0.185.0 — 2026-07-07 — BA runtime string CONCAT: WASM column — `PRINT A$ + B$` on ALL SEVEN backends

The final column. The `PRINT A$ + B$` runtime-concat matrix cell (stdin `"OK\n!\n"`
→ `OK!`) now runs on **all seven backends** `[NativeAot, Llvm, Wasm, Jvm, Clr, Vm,
Jit]`.

- **WASM** (`iir-to-wasm` 0.32.0): the concat is built entirely in wasm — bump-allocate
  a `[i32 len][bytes]` block, write the header, and splice both operands' bytes with two
  `memory.copy` instructions (no scratch locals; each length re-read via `i32.load`).
- **Executor** (`wasm-execution` 0.5.0): gained a `0xFC` bulk-memory decoder + a
  `memory.copy` handler backed by `LinearMemory::copy` (bounds-checked, overlap-safe).
- Matrix cell backends → `[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit]`; both guard tests
  pass. WASM run-verified in-process locally.

This completes the E4-dyn BASIC runtime string-concatenation arc across all three PRs
(Jvm/Clr/Vm/Jit foothold → NativeAot/LLVM → WASM).

## 0.184.0 — 2026-07-07 — BA runtime string CONCAT: static columns (NativeAot + LLVM)

The `PRINT A$ + B$` runtime-concat matrix cell (stdin `"OK\n!\n"` → `OK!`) gains its
two **static** columns, so it now runs on `[NativeAot, Llvm, Jvm, Clr, Vm, Jit]` —
6 of 7 backends (WASM remains). Both static backends carried a `str_concat` as a
literal fold only; each now routes a non-foldable concat to the runtime helper
`__twig_str_concat(a, b)`:

- **NativeAot** (twig-aot 0.30.0): runtime `str_concat` → `call_builtin "str_concat"`;
  aarch64 0.22.0 / x86_64 0.23.0 add `str_concat` to `V1_BUILTINS` (same shape as
  `str_eq`, no new codegen). aarch64 run-verified locally; x86_64 on CI.
- **LLVM** (iir-to-llvm 0.33.0): runtime `str_concat` → `call i64 @__twig_str_concat`
  + declare guard; literal fold retained. Via clang.
- **Harness**: `PRINT_RUNTIME_C` gains a malloc-backed `__twig_str_concat` (reads both
  `[i64 len][bytes]` headers, joins), and `run_llvm`'s link condition now also fires
  when the `.ll` references `@__twig_str_concat`.

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

## 0.182.0 — 2026-07-07 — BA string INPUT: WASM column — `INPUT A$` on ALL SEVEN backends

The final column. The `INPUT A$` matrix cell now runs on **all seven backends**
`[NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit]` (stdin `"OK"` → `OK`).

- **WASM** (`iir-to-wasm` 0.31.0): `call_builtin "input_str"` bump-allocates a
  `[i32 len][256 bytes]` block and calls `env.__input_str(block, max) -> ()`.
- **Harness** (`run_wasm`): new `InputStrFunc` host resolves that import — it drains
  one line from the shared stdin buffer and writes the `[i32 len][bytes]` block into
  wasm linear memory (`store_i32` header + `store_i32_8` bytes), capped at `max`.
  Registered next to `InputI64Func`/`PrintStrFunc`. Run-verified in-process locally.

This completes the E4-dyn BASIC string `INPUT A$` arc across all four PRs (VM/JIT →
JVM/CLR → native/LLVM → WASM).

## 0.181.0 — 2026-07-07 — BA string INPUT: static columns (NativeAot + LLVM)

The `INPUT A$` matrix cell (`10 INPUT A$ / 20 PRINT A$ / 30 END`, stdin `"OK"` →
`OK`) gains its two **static** columns, so it now runs on `[NativeAot, Llvm, Jvm,
Clr, Vm, Jit]` — 6 of 7 backends (WASM remains). On the static backends a `str`
is an i64 handle to a `[i64 len][bytes]` heap block, so both columns gain a host
primitive that BUILDS such a block from the input line:

- **NativeAot** (`twig-aot` 0.29.0): `__twig_input_str()` in `twig_runtime.c`;
  the aarch64 (0.21.0) / x86_64 (0.22.0) tables add it as a 0-arg/returns-i64
  `V1_BUILTINS` entry (no codegen change). Run-verified locally on aarch64.
- **LLVM** (`iir-to-llvm` 0.32.0): `call_builtin "input_str"` → `call i64
  @__twig_input_str()`; also fixes a latent declare-guard bug that omitted the
  input helpers. The harness `run_llvm` C shim (`PRINT_RUNTIME_C`) gains a
  self-contained `__twig_input_str` (malloc-backed) and links it when the IR
  references it. Run-verified locally via real `clang`.

WASM (`env.__input_str` writing the block into linear memory) is the last column.

## 0.180.0 — 2026-07-06 — BA string INPUT: managed columns (JVM + CLR)

The `INPUT A$` matrix cell (`10 INPUT A$ / 20 PRINT A$ / 30 END`, stdin `"OK"` →
`OK`) gains its two **managed** columns, so it now runs on `[Jvm, Clr, Vm, Jit]`.
Neither needed a new value model — `str` is already a host string object on both —
only a read-a-line host primitive returning that native string, mirroring numeric
`input_i64`'s `readLong` / `ReadLine`+`Int32.Parse`.

- **JVM** (`iir-to-jvm-class-file` 0.29.0): the validator now accepts a `str`
  result on `call_builtin`/`mov`; `input_str` lowers to `invokestatic
  env/BasicRuntime.readLine()Ljava/lang/String;` + `astore`. The harness
  `BASIC_RUNTIME_JAVA` gains a `readLine()` method (reads to newline/EOF, returns
  the line). Run-verified locally via real `javac`/`java`.
- **CLR** (`iir-to-cil-bytecode` 0.38.0): `input_str` lowers to `call string
  System.Console::ReadLine()` (no `Int32.Parse`), stored into the `System.String`
  local. Verified on the CLR column via real `ilasm`/`dotnet` in CI.

Wiring the static columns (native/LLVM `__twig_input_str` returning a `[i64
len][bytes]` heap handle; WASM `env.__input_str` writing into linear memory) is
the next slice of this arc.

## 0.179.0 — 2026-07-06 — BA string INPUT: `INPUT A$` reads a runtime string (VM/JIT)

A new Dartmouth BASIC matrix cell — `10 INPUT A$ / 20 PRINT A$ / 30 END` — proves
that `INPUT A$` reads a whole stdin line **as the string value itself**, not as a
number to parse (`INPUT X`) nor as a selector between compile-time literals (the
E4-dyn foothold's `INPUT N`).  `A$` holds bytes that never appear in the program
source, so the compiler cannot fold it: this is the first matrix proof of a
runtime string that **originates at the input boundary**.

- Frontend: `dartmouth-basic-iir-compiler` 0.36.0 lowers `INPUT A$` to
  `call_builtin "input_str"` (a `str`-typed sibling of `input_i64`) + `mov` into
  the string slot; `PRINT A$` consumes it via the shared E4 `print_str` op.
- Harness (`tests/lang_matrix.rs`): a new `input_str` closure — registered on the
  VM, the JIT's interpreter-fallback VM tier, and the `GenericCirJit` backend —
  reads a line from the shared stdin buffer via a new `drain_stdin_line` helper
  and returns a tagged `vm_core::Value::Str`.  The cell lists `[Vm, Jit]`; its
  stdin `"OK\n"` is registered in `program_stdin`.  Stdin `"OK"` → `A$ = "OK"` →
  prints `OK`.  Wiring the four subprocess/WASM columns' host read-a-line
  primitive (`__twig_input_str` / `env.__input_str` / `readLine` /
  `Console.ReadLine`) is the next slice of this arc.

## 0.178.0 — 2026-07-04 — E4d-JVM/CLR: ALGOL string procedure now runs on ALL SEVEN backends

The ALGOL string-procedure matrix cell (`print(pick(1))` where `pick` returns a
runtime, branch-selected string) gains the final two columns — **`Jvm`** and
**`Clr`** — so it now runs on **all seven backends** (NativeAot, Llvm, Wasm, Jvm,
Clr, Vm, Jit), all agreeing on `"HI"`. A `string procedure` — the first E4-dyn
*frontend* feature — is now fully portable.

- **JVM** (`iir-to-jvm-class-file` 0.28.0): a `str` value is already a
  `java.lang.String`; the validator now accepts `str` on `call`/`ret` (the same
  fix as WASM E4d-3b). Verified on real `java`.
- **CLR**: already compiled and ran the string procedure (a `str` is a
  `System.String`, and its validator/lowering already accepted `str` call/ret) —
  it only needed adding to the cell. Verified via the CLR simulator.

Both matrix guard tests pass.

## 0.177.0 — 2026-07-04 — E4d-3b: ALGOL string procedure now runs on Wasm

The ALGOL string-procedure matrix cell (`print(pick(1))` where `pick` returns a
runtime, branch-selected string) gains the **`Wasm`** column, verified
end-to-end in-process via the `wasm-runtime` interpreter. This is the payoff of
`iir-to-wasm` 0.30.0, which reads the string length from the `[i32 len][bytes]`
block header at run time for any string lacking a compile-time literal entry (a
call result / return value / parameter), not only a promoted slot — so a
*returned* runtime string prints correctly. The cell now runs on **NativeAot,
Llvm, Wasm, VM, JIT**; only JVM/CLR remain (the same runtime-return change on
those backends). Both matrix guard tests pass.

## 0.176.0 — 2026-07-03 — E4d-2b: ALGOL string procedure now runs on Llvm

The ALGOL string-procedure matrix cell (`print(pick(1))` where `pick` returns a
runtime, branch-selected string) gains the **`Llvm`** column, verified
end-to-end via `clang`. This is the payoff of `iir-to-llvm` 0.31.0, which carries
a `str` value as an i64 handle across function boundaries (a `str` parameter /
return / call result) and reads the length header at run time for any string
without a compile-time length — so a *returned* runtime string prints correctly,
not just a branch-selected local. The cell now runs on **NativeAot, Llvm, VM,
JIT**; WASM and JVM/CLR remain (E4d-3b + JVM/CLR analogs of this change). Both
matrix guard tests pass.

## 0.175.0 — 2026-07-03 — E4d-AL: ALGOL string procedures (E4-dyn frontend payoff)

Added a matrix cell for the first E4-dyn *frontend* payoff — ALGOL 60
`string procedure`s (algol-iir-compiler 0.28.0):

```algol
begin string procedure pick(n); value n; integer n;
    if n > 0 then pick := 'HI' else pick := 'LO';
print(pick(1)) end
```

This proves the full chain — string-procedure declaration + call + runtime-string
**return value** + print — end-to-end on the backends that already carry a runtime
string arriving as a *call result*: **NativeAot** and **VM/JIT**. (The E4-dyn
foothold only ever printed a branch-selected *local*; a string return value is a
new path.) The LLVM/WASM/JVM/CLR columns take their runtime-string path only for
promoted-slot operands, so a string return value on those backends is the E4d-2b
(LLVM) / E4d-3b (WASM) / JVM+CLR follow-up that will extend this cell to all seven.

Bringing this up surfaced and fixed a latent native miscompile (twig-aot 0.28.0):
`strip_dead_aot_string_allocs` dropped all but the last buffer of a multi-block
string alias, so a not-last branch printed `""`. Both matrix guard tests pass.

## 0.174.0 — 2026-07-03 — E4-dyn E4d-4: foothold runtime string now runs on NativeAot (all 7 backends)

The E4-dyn runtime branch-selected-string foothold cell (`INPUT N` picks
`A$ = "HI"`/`"LO"`) gains the **`NativeAot`** column — completing the E4-dyn
backend ladder. The runtime string now runs on **all seven backends**
(NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit), all agreeing on `"HI"`. aarch64 is
run-verified locally on this Apple Silicon host; x86_64 is verified on the Linux
CI runner through the same matrix cell.

This is the payoff of `twig-aot` 0.27.0: on native, `str_const` already builds a
`[i64 len][bytes]` heap buffer and stores its address in the variable's stack
slot, and `print_str` already has a runtime length-read path (`field_load` the
header). The E4d-4 fix simply keeps a branch-selected string out of `twig-aot`'s
compile-time literal map so `print_str` takes that runtime path instead of folding
one branch's static length — one change covering both the aarch64 and x86_64
backends. `matrix_every_proven_cell_agrees` + `proven_columns_do_not_silently_skip`
pass with `NativeAot` added.

## 0.173.0 — 2026-07-03 — E4-dyn E4d-3: foothold runtime string now runs on Wasm

The E4-dyn runtime branch-selected-string foothold cell (`INPUT N` picks
`A$ = "HI"`/`"LO"`) gains the **`Wasm`** column, verified end-to-end in-process
via the `wasm-runtime` interpreter and its `env.__print_str` host. This is the
payoff of `iir-to-wasm` 0.29.0, which promotes a runtime (cross-basic-block)
string to an i32 **handle** = the offset of a length-prefixed block
`[i32 len][bytes]` in linear memory, and reads the length back with `i32.load`
in `print_str`. The cell now proves the runtime string on **6 backends**
(Llvm, Wasm, Jvm, Clr, Vm, Jit); only `NativeAot` remains, added by E4d-4.

## 0.172.0 — 2026-07-03 — E4-dyn E4d-2: foothold runtime string now runs on Llvm

The E4-dyn runtime branch-selected-string foothold cell (`INPUT N` picks
`A$ = "HI"`/`"LO"`) gains the **`Llvm`** column, verified end-to-end via real
`clang`. This is the payoff of `iir-to-llvm` 0.30.0, which lowers a runtime
(cross-basic-block) string as an `i64`-handle slot and reads its length from the
block header at run time in `print_str`. The cell now proves the runtime string
on **5 backends** (Llvm, Jvm, Clr, Vm, Jit); `Wasm`/`NativeAot` follow in
E4d-3/E4d-4.

## 0.171.0 — 2026-07-03 — E4-dyn foothold: first runtime (non-foldable) string matrix cell

First matrix proof of a **runtime** string (`code/specs/lang-full-e4-dyn-strings.md`):

```basic
10 INPUT N
20 IF N > 0 THEN 50
30 LET A$ = "LO"
40 GOTO 60
50 LET A$ = "HI"
60 PRINT A$
70 END
```

Because `N` is read at run time, *which* literal `A$` holds at line 60 is **not
known at compile time** — `A$` is a genuinely dynamic string, unlike every prior
E4 cell where the compiler folds the string to a constant. Stdin `1` → `N=1>0` →
`A$="HI"` → prints `HI`.

Tagged on the **four already-dynamic backends** (`Vm`, `Jit`, `Jvm`, `Clr`),
which carry a reassigned-across-branches string slot natively (tagged value /
`java.lang.String` / `System.String`). The four static backends
(`NativeAot`/`Llvm`/`Wasm`) fold strings to compile-time constants and cannot yet
represent this value; the E4-dyn backend PRs (E4d-2 LLVM → E4d-3 WASM → E4d-4
native) extend **this cell's `backends` list** column-by-column as each lands its
runtime heap-string lowering on the E4d-1 `__twig_str_*` helpers.

This establishes the shared matrix cell the backend PRs prove themselves against
— resolving the ordering constraint that a static-backend runtime-string lowering
can't be matrix-proven until a frontend emits a non-foldable string.

## 0.170.0 — 2026-07-02 — AL-pow: ALGOL 60 exponentiation operator (all 7 backends)

**New matrix proof cell** (`lang_matrix.rs`): ALGOL 60's `↑` exponentiation
operator (spelled `^`) running on all 7 backends (NativeAot, Llvm, Wasm, Jvm,
Clr, Vm, Jit).

```algol60
begin integer result; result := 10 + 2 ^ 5 end
```

A nonnegative integer-literal exponent unrolls to repeated integer
multiplication and keeps the base's type, so `2 ^ 5` is the *integer* 32 —
exactly `mul`/`imul` the backends already run (no new IIR op). `↑` binds tighter
than `*`/`+`, so `10 + 2 ^ 5` = `10 + 32` = 42. The `real ↑ real` shape reuses
the `f64_pow` op BASIC's BA-pow already proved. Requires `algol-iir-compiler`
0.27.0. Exit 42.

## 0.169.0 — 2026-07-02 — AL-multidim-bounds: ALGOL 60 2D array with arbitrary/negative lower bounds (all 7 backends)

**New matrix proof cell** (`lang_matrix.rs`): ALGOL 60 2D integer array with a
**negative** lower bound on one axis and a non-zero one on the other, running on
all 7 backends (NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit).

```algol60
begin integer array M[0-1:1, 2:3]; integer result;
  M[0-1, 2] := 40; M[1, 3] := 2;
  result := M[0-1, 2] + M[1, 3] end
```

`M[-1:1, 2:3]` has sizes `(3, 2)`, strides `[2, 1]`, so `M[i,j]` folds to
`(i−(−1))*2 + (j−2)`.  `M[-1,2]` is flat 0, `M[1,3]` is flat 5 (the last of 6
cells — proving both the per-dimension `sub−lower` subtraction and the row
stride).  The negative bound is written `0-1` since ALGOL number literals are
unsigned.  No new IIR op, no backend change.  Requires `algol-iir-compiler`
0.26.0.  Exit 42.

## 0.168.0 — 2026-07-02 — BA-DIM-2D: Dartmouth BASIC 2D DIM array matrix proof cell (all 7 backends)

**New matrix proof cell** (`lang_matrix.rs`): Dartmouth BASIC two-dimensional
array running on all 7 backends (NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit).

```basic
10 DIM A(1,2)
20 LET A(0,0) = 40
30 LET A(1,2) = 2
40 PRINT A(0,0) + A(1,2)
50 END
```

`DIM A(1,2)` is a 2×3 matrix (0-based inclusive) lowered to a single flat
`alloc_array` of 6 `f64` elements; `A(i,j)` folds through the row-major strides
`[3,1]` to the flat index `i*3 + j`. Stores `A(0,0)=40` (flat 0) and `A(1,2)=2`
(flat 5, the last cell — proving the row stride), then prints their sum ⇒ `42`.
No new IIR op, no backend change — same E5 array substrate as the BA3 1-D cell.
Requires `dartmouth-basic-iir-compiler` 0.35.0 / `-parser` 0.3.0.

## 0.167.0 — 2026-07-02 — AL-multidim-3D: ALGOL 60 3D integer array matrix proof cell (all 7 backends)

**New matrix proof cell** (`lang_matrix.rs`): ALGOL 60 3D integer array running
on all 7 backends (NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit) — proves the
multidim lowering is genuinely N-dimensional, not hardcoded to 2-D.

```algol60
begin integer array M[1:2, 1:2, 1:2]; integer result;
  M[1, 1, 1] := 6; M[2, 1, 1] := 16; M[1, 2, 1] := 20;
  result := M[1, 1, 1] + M[2, 1, 1] + M[1, 2, 1] end
```

For `M[1:2, 1:2, 1:2]` the strides are `stride[0]=4, stride[1]=2, stride[2]=1`,
so `M[i,j,k]` → flat `(i−1)*4 + (j−1)*2 + (k−1)`. Three corner cells with
distinct flat indices (0, 4, 2) are stored to exercise every stride, then summed
= 42. Still only `alloc_array`/`array_set`/`array_get` with flat indices — no
backend change. Requires `algol-iir-compiler` 0.25.0. Exit 42.

## 0.166.0 — 2026-07-02 — AL-multidim-real: ALGOL 60 2D **real** array matrix proof cell (all 7 backends)

**New matrix proof cell** (`lang_matrix.rs`): ALGOL 60 2D **real** (f64) array
running on all 7 backends (NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit).

```algol60
begin real array M[1:2, 1:2]; real sum; integer result;
  M[1, 1] := 10.25; M[1, 2] := 10.25;
  M[2, 1] := 10.75; M[2, 2] := 10.75;
  sum := M[1, 1] + M[1, 2] + M[2, 1] + M[2, 2];
  result := entier(sum) end
```

Same multidim flat-index lowering as the 0.165.0 integer cell, but the element
type is `real`: the fractional doubles ride the E5 8-byte slots, are summed with
the f64 `add` path to 42.0, then floored to an integer exit code via the E8
`entier` (`real_to_int_floor`) conversion.  Proves f64 multidim elements — the
follow-up flagged when AL-multidim first landed — with no backend change.
Exit 42.  Requires `algol-iir-compiler` 0.24.0.

## 0.165.0 — 2026-07-01 — AL-multidim: ALGOL 60 2D array matrix proof cell (all 7 backends)

**New matrix proof cell** (`lang_matrix.rs`): ALGOL 60 2D integer array
running on all 7 backends (NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit).

```algol60
begin integer array M[1:2, 1:2]; integer result;
  M[1, 1] := 10; M[1, 2] := 20; M[2, 1] := 5; M[2, 2] := 7;
  result := M[1, 1] + M[1, 2] + M[2, 1] + M[2, 2] end
```

The `algol-iir-compiler` (v0.23.0) lowers `M[i, j]` to the row-major flat
index `(i − 1)*2 + (j − 1)` using `alloc_array`/`array_set`/`array_get` with
precomputed flat indices.  No backend change is needed for Llvm/Wasm/Jvm/Clr/Vm/Jit
— the IIR is identical to E5 1D arrays.  NativeAot on aarch64 required the
`aarch64-backend` v0.20.0 large-frame support (the 2D program produces 72
variable slots = 576 bytes, exceeding the old 504-byte limit).  Exit 42.

## 0.164.0 — 2026-07-01 — LANG-STR-RT: 7 NativeAot string-parameter matrix cells unlocked

Seven `lang_matrix.rs` test cells that were previously excluded from `NativeAot`
(with `// NativeAot: str_len/str_eq on function params not yet wired`) now pass
on NativeAot.  The change adds `NativeAot` to the `backends` array for each of
these `Prog` entries:

- `(define (strlen (s : str)) (string-length s)) (strlen "HELLO")`
- `(define (print_hi (s : str)) (print_str s)) (print_hi "HELLO\n")`
- `(define (same? (a : str) (b : str)) (string=? a b)) (same? "X" "X")`
- `(define (diff? (a : str) (b : str)) (string=? a b)) (diff? "A" "B")`
- `(define (concat_len (a : str) (b : str)) ...) (concat_len "HE" "LLO")`
- `(define (greet (name : str)) ...) (greet "World")`
- `(define (initial (s : str)) (string-ref s 0)) (initial "ABC")`

The unlock is enabled by the LANG-STR-RT buffer layout (8-byte length prefix
at offset 0, data at offset 8+), implemented in `twig-aot` 0.24.0, combined
with the `str_eq` V1 builtin in `aarch64-backend` 0.18.0 / `x86_64-backend`
0.20.0.

## 0.163.0 — 2026-06-30 — BA-INPUT: JVM `INPUT X` end-to-end (concretize + test matrix)

**`concretize_scalar_any_for_jvm`** (`src/lib.rs`): The function that narrows
`i64` type-hints to `i32` for programs that don't need the JVM wide `long` model
previously only exempted programs that call `print_i64` from narrowing.  BASIC
`INPUT X` programs call `input_i64` but not `print_i64`, so they were incorrectly
narrowed to `i32` — causing a JVM VerifyError when `readLong()J` tried to store a
`long` into an `int` slot.  Added `WIDE_I64_BUILTINS = ["print_i64", "input_i64"]`
and updated both the module-level `module_prints` check and the per-function
`uses_wide_builtin` guard to use this list.

**`putchar` lowering** (`iir-to-jvm-class-file`): In wide i64 mode, the value
being written via `putchar(I)V` lives in a Long slot.  The lowering previously
always emitted `iload` (int load), causing a VerifyError when the slot contained a
`long`.  Now emits `lload; l2i` when `val_type == JvmType::Long`.

**`emit_lconst_cp`** (`iir-to-jvm-class-file`): Added proper Long CP-entry helper
for integer literals outside the i16 range stored into `JvmType::Long` slots (e.g.
`100000` in `__basic_print_real`).  Fixed the `"const"` and `"return"` IIR cases to
call it.

**Test matrix** (`tests/lang_matrix.rs`): Added two new Dartmouth BASIC INPUT
`Prog` entries:
- `10 INPUT X\n20 PRINT X\n30 END` with stdin `42` → stdout `"42"` (single INPUT)
- `10 INPUT A\n20 INPUT B\n30 PRINT A+B\n40 END` with stdin `10\n32` → `"42"` (two INPUTs)

`matrix_every_proven_cell_agrees` and `proven_columns_do_not_silently_skip` both
pass across all 7 backends.

## 0.162.0 — 2026-06-30 — boolean backend correctness: WASM/CLR/LLVM fixes + matrix proof

Completed the `proven_columns_do_not_silently_skip` matrix test across all 7
backends (NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit).

**lang_matrix.rs**:
- Added ALGOL compound boolean program (`s = 'ALPHA' and s != 'OMEGA'`) as a
  matrix Prog cell on all 7 backends — unblocked by WASM boolean and/or
  width-coherence fix and CLR Operand::Bool fix.
- Restored Llvm + Wasm to cells that were previously regressing (ALGOL `say`
  and string-echo programs).
- Improved assertion messages to include `p.src` on CLR failures for easier
  debugging.

**end_to_end_smoke.rs**:
- Updated `end_to_end_basic_print_emits_llvm_ir_with_print_extern` to assert
  `@__basic_print_real` instead of `@__print_i64` — the BASIC PRINT path
  changed in the BA2 overhaul (integer literals now route through the real
  formatter).

## 0.161.0 — 2026-06-29 — ALGOL real-valued procedures (LANG-FULL AL13-real-proc)

One new matrix `Prog` cell proving ALGOL 60 `real procedure` declarations on all 7
backends (NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit):

- `real procedure scale(x); value x; real x; scale := x * 6.0; entier(scale(7.0))`
  — scale(7.0) = 42.0; entier(42.0) = 42 → Exit(42)

The call path (`real procedure` → `call` IIR instruction returning f64 → `entier`
standard function → `real_to_int_floor` → integer exit code) was already wired in
all 7 backends; this cell is the first explicit end-to-end matrix proof of it.

No new backend code was needed.  The `algol-iir-compiler` 0.21.0 bump adds the
corresponding VM-level unit test (`real_procedure_runs`).

856 total proven cells pass.

## 0.160.0 — 2026-06-29 — ALGOL recursive procedures, goto, nested block shadowing (LANG-FULL AL12)

Three new matrix `Prog` cells proving advanced ALGOL 60 control-flow and scoping
features on all 7 backends (NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit):

- **Recursive procedure** — `fact(3)` using `n * fact(n - 1)` base case `n < 2`.
  Proves that backends correctly lower recursive `call`/`ret` pairs with separate
  stack frames.  Result = 6.  Exit 6.
- **`goto` loop** — accumulates `x := x + 7` until `x >= 42` via an unconditional
  label + conditional `goto`.  x: 7 → 14 → 21 → 28 → 35 → 42.  Exit 42.
- **Nested block variable shadowing** — three nested `begin … end` blocks each
  declare their own `integer x` and `boolean flag`, exercising the lexical scoping
  rules of ALGOL 60 §2.7.  Inner reads correct (innermost) binding; outer reads
  survive exit of inner block.  Trace: result = 31 + 10 + 1 = 42.  Exit 42.

855 total proven cells pass.

## 0.159.0 — 2026-06-29 — ALGOL 60 for-while + single-value + for-list (LANG-FULL AL11)

Three new matrix `Prog` cells proved on all 7 backends (NativeAot, Llvm, Wasm,
Jvm, Clr, Vm, Jit):

- **`for … while`**: `for i := i + 6 while i <= 36 do result := i + 6` — starts
  at 0, advances by 6 each iteration until i > 36; the body captures the
  stepped value; last assigned result = 42 on iteration where i=36.
  The `emit_for_while` IIR lowering emits a label → expression code →
  `jmp_if_false` → body → `jmp` skeleton (same as `step-until`).

- **Single-value for element**: `for i := 2 do result := 40 + i` — executes
  the body exactly once with i=2 → result=42.  The `emit_for_value` path
  emits a single `mov` + body block with no loop at all.

- **Multi-element for list**: `for i := 1 step 1 until 3, 10, i + 1 while i < 13`
  sequences three for-element kinds in one head: `step-until` (i=1,2,3),
  a literal value (i=10), and `while` (i=11,12).  Sum 1+2+3+10+11+12=39,
  then `result := result + 3` → 42.  Multi-element lowering chains the
  exit-of-one / init-of-next blocks sequentially.

852 total proven cells pass.

## 0.158.0 — 2026-06-29 — BASIC `^` general exponentiation on 7 backends (LANG-FULL BA-pow)

Added `PowFunc` WASM host function (`env.__pow(f64, f64) -> f64`, uses
`f64::powf`) and registered it in `PrintHost::resolve_function`.  Added proof
cell: `PRINT 4 ^ 0.5` → `Stdout("2")` on all 7 backends (NativeAot, Llvm,
Wasm, Jvm, Clr, Vm, Jit).  `pow(4.0, 0.5) = 2.0` exactly; the `2` output
(no decimal point) comes from `__basic_print_real`'s existing integer-valued
real formatting.

849 total proven cells pass.
## 0.157.0 — 2026-06-29 — ALGOL 60 boolean variables + BASIC FOR STEP (LANG-FULL AL10 + BA-step)

Four new matrix `Prog` cells proved on all 7 backends (NativeAot, Llvm, Wasm,
Jvm, Clr, Vm, Jit):

### ALGOL 60 — boolean variables (AL10)

- `boolean b; b := true; if b then result := 42` — declared boolean variable
  used directly as an `if` condition; proves `bool`-typed `const`/`mov` + named
  variable condition reach every backend's branch op without a spurious coerce.
- `boolean b; b := false; if not b then result := 42` — unary `not` operator;
  LLVM `xor i1 %b, 1`, WASM `i32.eqz`, JVM/CLR integer NOT, VM `not` dispatch.
- `boolean a, b; a := true; b := false; if a and (not b) then result := 42` —
  two-operand `and` wired after a `not`-inverted sub-expression; proves compound
  boolean algebra end-to-end on all 7 backends.

### Dartmouth BASIC — FOR STEP > 1 (BA-step)

- `FOR I = 1 TO 5 STEP 2` + `LET S = S + I` + `NEXT I` → S = 1+3+5 = 9;
  `PRINT S` → `"9"`.  The `_for_<n>_step` IIR slot holds the compile-time `2`
  constant; NEXT adds it before re-testing `I <= 5`.  Distinguishes from the
  existing default-STEP-1 cell (which sums to 15), proving the STEP clause is
  live not dead on all 7 backends.

848 total proven cells pass.

## 0.156.0 — 2026-06-29 — ALGOL 60 real array + real procedure on 7 backends (LANG-FULL AL9)

Added two matrix proof cells for ALGOL 60 real-typed features:

- **AL9-a (real array)**: `real array A[1:3]` with f64 element stores/loads;
  `A[1]:=40.0; A[3]:=2.0; entier(A[1]+A[3])` ⇒ exit 42.  Exercises non-contiguous
  f64 slots and ALGOL's lower-bound subtraction on array access.
- **AL9-b (real procedure)**: `real procedure square(x); value x; real x;` with
  a real value parameter and real return; `entier(square(6.5))` = `entier(42.25)`
  ⇒ exit 42.  Exercises the `(f64) → f64` call/ret pathway on all 7 backends.

No new IIR ops or backend changes — both cells run on the existing E5 array substrate
(array_set/array_get with f64 type_hint) and the existing call/ret mechanism.

844 total proven cells pass.

## 0.155.0 — 2026-06-29 — ALGOL 60 transcendental functions `sin`/`cos`/`ln`/`exp` (LANG-FULL AL8-trig)

Four new matrix `Prog` cells proving ALGOL 60's §3.2.4 transcendental standard
functions on all 7 backends (NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit):

- `cos(0.0)` → `entier(1.0) + 41` = 42
- `exp(0.0)` → `entier(1.0) + 41` = 42
- `sin(0.0)` → `entier(0.0 + 42.0)` = 42
- `ln(1.0)`  → `entier(0.0 + 42.0)` = 42

Each uses an exact IEEE-754 input/output (no rounding error) to verify correctness
portably.  Backend mappings: WASM `env.__sin/cos/ln/exp` host imports (resolved by
`PrintHost` to Rust `f64::*`); LLVM `@llvm.sin/cos/log/exp.f64` intrinsics;
JVM `Math.sin/cos/log/exp`; CLR `System.Math.Sin/Cos/Log/Exp`; native aarch64/x86_64
`BL` / `call rel32` to libm `sin/cos/log/exp`; VM/JIT `f64_*` dispatch handlers.

Four new WASM host functions added to `PrintHost`: `SinFunc`, `CosFunc`, `LnFunc`,
`ExpFunc` — each stateless, `f64 → f64`, resolved as `env.__sin/cos/ln/exp`.

842 total proven cells pass.

## 0.154.0 — 2026-06-28 — ALGOL string procedure parameters on all 7 backends (LANG-FULL AL4-str-params)

Two new matrix `Prog` cells proving ALGOL 60 string-typed value parameters on all
7 backends (NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit):

- `integer procedure echo(s); value s; string s; print(s); echo('HELLO')` → stdout `HELLO`
  (string literal as actual argument)
- `string msg; … msg := 'HI'; say(msg)` → stdout `HI`
  (named string variable as actual argument)

No new backend code — `str`-typed function parameters were already proven by Twig
(TW4). The only change is in `algol-iir-compiler` 0.18.0: `specifier_scalar_type`
now accepts `"string"` specifiers, and `compile_procedure` adds string parameter
slots to `literal_string_slots` so `print(s)` works inside the body.

838 total proven cells pass.

## 0.153.0 — 2026-06-28 — BASIC built-in functions `SQR`/`INT`/`ABS`/`SGN` (LANG-FULL BA-builtins)

Four new matrix `Prog` cells proving Dartmouth BASIC's built-in math functions
on all 7 backends (NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit):

- `PRINT SQR(49)` → `7` (`f64_sqrt` IIR op, hardware sqrt everywhere)
- `PRINT INT(3.7)` → `3` (`real_to_int_floor` + `int_to_real`, E8 ops)
- `PRINT ABS(-42)` → `42` (inline conditional, store-per-branch)
- `PRINT SGN(-5)` → `-1` (inline 3-way conditional, result is f64)

818 total proven cells pass.

## 0.152.0 — 2026-06-28 — ALGOL `sqrt` on all seven backends (LANG-FULL AL8-sqrt)

The matrix now proves `sqrt(49.0) = 7` — the ALGOL 60 §3.2.4 `sqrt` standard
function — on **all 7 backends** (NativeAot / LLVM / WASM / JVM / CLR / VM / JIT).
The proof program is `begin real r; integer result; r := sqrt(49.0); result :=
entier(r) end` → exit 7.  `sqrt` lowers through the new `f64_sqrt` IIR op to
hardware sqrt on every backend: WASM `f64.sqrt` (0x9F), LLVM `@llvm.sqrt.f64`,
JVM `Math.sqrt`, CLR `System.Math::Sqrt`, aarch64 `FSQRT`, x86_64 `SQRTSD`,
VM/JIT `f64::sqrt()`.

## 0.151.0 — 2026-06-28 — ALGOL string predicates on all seven backends (LANG-FULL AL4)

The matrix now proves ALGOL 60 literal-backed scalar string equality and
ordering branches through the shared E4 `str_eq` / `str_cmp` ops:

```algol
begin string s; s := 'ALPHA';
  if (s = 'ALPHA' and s != 'OMEGA') and
     (s < 'BETA' and 'BETA' > s) then print('OK') else print('BAD')
end
```

Expected stdout is `OK` on native-AOT + LLVM + WASM + JVM + CLR + VM + JIT.
`algol-iir-compiler` 0.16.0 lowers the string predicate results to typed zero
comparisons before reusing the standard ALGOL conditional branch.

## 0.150.0 — 2026-06-28 — BASIC lexical string ordering on all seven backends (LANG-FULL BA4)

The matrix now proves Dartmouth BASIC `$` string ordering branches through the
shared E4 `str_cmp` op:

```basic
10 LET A$ = "ALPHA"
20 IF A$ < "BETA" THEN 40
30 END
40 IF "BETA" > A$ THEN 60
50 END
60 PRINT "OK"
```

Expected stdout is `OK` on native-AOT + LLVM + WASM + JVM + CLR + VM + JIT.
`dartmouth-basic-iir-compiler` 0.31.0 lowers the string ordering result to
typed zero comparisons before reusing the standard BASIC line-control branch.

## 0.149.0 — 2026-06-28 — Twig lexical string ordering on all seven backends (LANG-FULL E4)

The matrix now proves nested `string<?` and `string>?` predicates via the shared
`str_cmp` op on native-AOT + LLVM + WASM + JVM + CLR + VM + JIT.

## 0.148.0 — 2026-06-28 — Twig substring feeds string indexing on all seven backends (LANG-FULL E4)

The matrix now proves a typed Twig lexical string can be sliced and then byte
indexed:

```scheme
(let ((s "ABCDE")) (string-ref (substring s 1 4) 1))
```

Expected exit code is `67` (`C` in `BCD`) on native-AOT + LLVM + WASM + JVM +
CLR + VM + JIT. `twig-ir-compiler` 0.33.0 lowers `substring` to the shared
`str_slice` op, and the static backends either fold the derived literal slice
or map it to their managed string substring primitive.

## 0.147.0 — 2026-06-28 — Twig string length computes string indexing on all seven backends (LANG-FULL E4)

The matrix now proves a typed Twig lexical string length can feed integer
arithmetic and then byte indexing:

```scheme
(let ((s "ABCDE")) (string-ref s (- (string-length s) 1)))
```

Expected exit code is `69` (`E` in `ABCDE`) on native-AOT + LLVM + WASM + JVM +
CLR + VM + JIT. `twig-ir-compiler` 0.32.0 lowers the local string to E4
`str_const`, computes the index with E4 `str_len` plus typed `sub`, and consumes
that computed register with E4 `str_index`; `twig-aot` 0.20.0 folds the same
metadata chain before direct native lowering, and `iir-to-llvm` 0.22.0 does the
same for the LLVM column.

## 0.146.0 — 2026-06-28 — Twig string concat feeds string indexing on all seven backends (LANG-FULL E4)

The matrix now proves a typed Twig lexical string append whose result feeds
byte indexing:

```scheme
(let ((a "AB") (b "CDE") (i 3)) (string-ref (string-append a b) i))
```

Expected exit code is `68` (`D` in `ABCDE`) on native-AOT + LLVM + WASM + JVM +
CLR + VM + JIT. `twig-ir-compiler` 0.31.0 lowers the local strings to E4
`str_const`, appends them with E4 `str_concat`, and consumes the temporary
directly with E4 `str_index`.

## 0.145.0 — 2026-06-28 — BASIC variable-variable string IF equality runs on all seven backends (LANG-FULL BA4)

The matrix now proves a BASIC string concat expression whose two operands are
both scalar string variables and whose result feeds the standard `=` branch:

```basic
10 LET A$ = "O"
20 LET B$ = "K"
30 IF A$ + B$ = "OK" THEN 60
40 PRINT "BAD"
50 END
60 PRINT "OK"
70 END
```

Expected stdout is `OK` on native-AOT + LLVM + WASM + JVM + CLR + VM + JIT.
`dartmouth-basic-iir-compiler` 0.30.0 lowers the expression to E4
`str_concat`, compares it with E4 `str_eq`, and branches on true equality.

## 0.144.0 — 2026-06-27 — BASIC variable-variable string IF inequality runs on all seven backends (LANG-FULL BA4)

The matrix now proves a BASIC string concat expression whose two operands are
both scalar string variables and whose result feeds the standard `<>` branch:

```basic
10 LET A$ = "O"
20 LET B$ = "K"
30 IF A$ + B$ <> "NO" THEN 60
40 PRINT "BAD"
50 END
60 PRINT "OK"
70 END
```

Expected stdout is `OK` on native-AOT + LLVM + WASM + JVM + CLR + VM + JIT.
`dartmouth-basic-iir-compiler` 0.29.0 lowers the expression to E4
`str_concat`, compares it with E4 `str_eq`, and branches on false equality.

## 0.143.0 — 2026-06-27 — BASIC variable-variable string PRINT concat runs on all seven backends (LANG-FULL BA4)

The matrix now proves a BASIC string concat expression whose two operands are
both scalar string variables and whose result is consumed directly by `PRINT`:

```basic
10 LET A$ = "O"
20 LET B$ = "K"
30 PRINT A$ + B$
```

Expected stdout is `OK` on native-AOT + LLVM + WASM + JVM + CLR + VM + JIT.
`dartmouth-basic-iir-compiler` 0.28.0 lowers the expression to E4
`str_concat` and feeds the temporary result directly to `print_str`.

## 0.142.0 — 2026-06-27 — BASIC chained string concat runs on all seven backends (LANG-FULL BA4)

The matrix now proves a left-associative BASIC string concat chain:

```basic
10 LET A$ = "A"
20 LET B$ = A$ + "B" + "C"
30 PRINT B$
```

Expected stdout is `ABC` on native-AOT + LLVM + WASM + JVM + CLR + VM + JIT.
`dartmouth-basic-iir-compiler` 0.27.0 lowers the chain to repeated E4
`str_concat` instructions and stores the final value in the target scalar
string slot.

## 0.141.0 — 2026-06-27 — BASIC literal exponentiation runs on all seven backends (LANG-FULL BA-^)

The matrix now proves the safe BASIC `^` subset:

```basic
10 PRINT 6 ^ 2 + 6
20 END
```

Expected stdout is `42` on native-AOT + LLVM + WASM + JVM + CLR + VM + JIT.
`dartmouth-basic-iir-compiler` 0.26.0 lowers integer-valued literal exponents
through repeated `f64` multiplication, avoiding a backend math runtime.

## 0.140.0 — 2026-06-27 — Oct logical NOT runs on all seven backends (LANG-FULL O-!)

The matrix now proves unary logical NOT for Oct:

```oct
fn main() { if !(1 == 2) { out(1, 42); } else { out(1, 0); } }
```

Expected stdout is `42` on native-AOT + LLVM + WASM + JVM + CLR + VM + JIT.
`oct-iir-compiler` 0.9.0 lowers `!` through the shared truthiness branch
contract, materialising a clean 0/1 bool instead of reusing bitwise `not`.

## 0.139.0 — 2026-06-27 — Nib const/static expression folding runs on all seven backends (LANG-FULL N10)

The matrix now proves a folded Nib const expression feeding a folded static
initializer:

```nib
const BASE: u8 = 6 * 7;
static counter: u8 = BASE + 0;
fn main() -> u8 { return counter; }
```

Expected exit code is `42` on native-AOT + LLVM + WASM + JVM + CLR + VM + JIT.
`nib-type-checker` 0.6.0 types the initializer expressions, and
`nib-iir-compiler` 0.19.0 folds them before runtime code is emitted.

## 0.138.0 — 2026-06-27 — Nib logical NOT runs on all seven backends (LANG-FULL N9)

The matrix now proves unary logical NOT:

```nib
fn main() -> u8 { if !(1 == 2) { return 42; } return 0; }
```

Expected exit code is `42` on native-AOT + LLVM + WASM + JVM + CLR + VM + JIT.
`nib-type-checker` 0.5.0 types `!` as `bool`, and `nib-iir-compiler` 0.18.0
lowers it through the shared truthiness branch contract.

## 0.137.0 — 2026-06-27 — Nib static globals run on all seven backends (LANG-FULL N8)

The matrix now proves module-scoped mutable Nib statics through the shared E6
global substrate:

```nib
static counter: u8 = 40;
fn bump(step: u8) -> u8 { counter = counter + step; return counter; }
fn main() -> u8 { let a: u8 = bump(1); let b: u8 = bump(1); return counter; }
```

Expected exit code is `42` on native-AOT + LLVM + WASM + JVM + CLR + VM + JIT.
`nib-type-checker` 0.4.0 makes static names visible across functions, while
`nib-iir-compiler` 0.17.0 seeds `main` with `global_store` and lowers reads/writes
to `global_load`/`global_store`.

## 0.136.0 — 2026-06-27 — Twig local string equality branches run on all seven backends (LANG-FULL E4/TW4)

The Twig matrix now proves lexical string locals can feed E4 equality before
control flow:

```scheme
(let ((s "OK") (t "OK")) (if (string=? s t) 42 0))
```

Expected exit code is `42` on native-AOT + LLVM + WASM + JVM + CLR + VM + JIT.
The frontend emits local `str_const` slots, `str_eq`, and the existing branch
shape instead of dynamic `string=?`.

## 0.135.0 — 2026-06-27 — Twig `let*` string locals run on all seven backends (LANG-FULL E4/TW4)

The Twig matrix now proves the sequential lexical binding form can feed E4
string ops:

```scheme
(let* ((s "HELLO")) (string-length s))
```

Expected exit code is `5` on native-AOT + LLVM + WASM + JVM + CLR + VM + JIT.
This backs the existing `twig-ir-compiler` unit proof with an all-backend run:
`let*` materializes a typed `str_const` local slot, then `str_len` consumes it
without falling back to dynamic `string-length`.

## 0.134.0 — 2026-06-27 — ALGOL multi-argument string output runs on all seven backends (LANG-FULL AL4/E4)

The ALGOL matrix now proves multiple literal-backed scalar string actuals in one
`output` statement:

```algol
begin string s, t; s := 'O'; t := 'K'; output(s, t) end
```

Expected stdout is `OK` on native-AOT + LLVM + WASM + JVM + CLR + VM + JIT.
The compiler emits two ordered E4 `print_str` calls and avoids dynamic procedure
dispatch, keeping the proof inside the current AL4 immutable scalar string
foothold.

## 0.133.0 — 2026-06-27 — BASIC comma-separated string PRINT runs on all seven backends (LANG-FULL BA4/E4)

The BASIC matrix now proves BA2 comma separators compose with BA4/E4 string
items:

```basic
10 LET A$ = "O"
20 LET B$ = "K"
30 PRINT A$, B$
```

Expected stdout is `O K` on native-AOT + LLVM + WASM + JVM + CLR + VM + JIT.
The compiler emits the existing comma separator as `putchar(' ')` between two
ordered `print_str` calls, so string items remain on the shared E4 output path
and do not route through numeric formatting helpers.

## 0.132.0 — 2026-06-27 — BASIC multi-item string PRINT runs on all seven backends (LANG-FULL BA4/E4)

The BASIC matrix now proves two scalar string slots in one `PRINT` statement:

```basic
10 LET A$ = "O"
20 LET B$ = "K"
30 PRINT A$; B$
```

The row expects stdout `OK` on native-AOT + LLVM + WASM + JVM + CLR + VM + JIT,
proving ordered repeated `print_str` calls for `;`-separated string items.

## 0.131.0 — 2026-06-27 — ALGOL string copy snapshots run on all seven backends (LANG-FULL AL4/E4)

The ALGOL matrix now proves scalar string copy snapshot semantics:

```algol
begin string s, t; s := 'OK'; t := s; s := 'NO'; print(t) end
```

The row expects stdout `OK` on native-AOT + LLVM + WASM + JVM + CLR + VM + JIT,
proving a later source-slot `str_const` does not change the copied target slot.

## 0.130.0 — 2026-06-27 — BASIC copied string equality runs on all seven backends (LANG-FULL BA4/E4)

The BASIC matrix now proves variable-to-variable string equality after a scalar
string copy:

```basic
10 LET A$ = "OK"
20 LET B$ = A$
30 IF B$ = A$ THEN 60
40 PRINT "BAD"
50 END
60 PRINT "OK"
```

The row expects stdout `OK` on native-AOT + LLVM + WASM + JVM + CLR + VM + JIT,
proving copied string slots can feed E4 `str_eq` before BASIC line-control
branching.

## 0.129.0 — 2026-06-27 — Twig lexical string concat runs on all seven backends (LANG-FULL E4/TW4)

The Twig matrix now proves lexical string locals can feed E4 concat:

```scheme
(let ((a "AB") (b "CDE")) (string-length (string-append a b)))
```

The row expects exit `5` on native-AOT + LLVM + WASM + JVM + CLR + VM + JIT,
proving local non-literal string operands can flow through `str_concat` and
`str_len` without falling back to dynamic builtins.

## 0.128.0 — 2026-06-27 — ALGOL scalar string copy runs on all seven backends (LANG-FULL AL4/E4)

The ALGOL matrix now proves literal-backed scalar string copies:

```algol
begin string s, t; s := 'OK'; t := s; print(t) end
```

The row expects stdout `OK` on native-AOT + LLVM + WASM + JVM + CLR + VM + JIT,
proving the AL4 frontend can copy a literal-backed string slot through E4
`str_concat` with an empty suffix before `print_str` consumes the target.

## 0.127.0 — 2026-06-27 — BASIC variable-backed concat assignment runs on all seven backends (LANG-FULL BA4/E4)

The BASIC matrix now proves a string expression assignment whose left operand is
a scalar string variable:

```basic
10 LET A$ = "O"
20 LET B$ = A$ + "K"
30 PRINT B$
40 END
```

The row expects stdout `OK` on native-AOT + LLVM + WASM + JVM + CLR + VM + JIT,
proving E4 `str_concat` can store directly into another BASIC scalar string
slot before `print_str` consumes it.

## 0.126.0 — 2026-06-27 — BASIC string expressions in IF run on all seven backends (LANG-FULL BA4/E4)

The BASIC matrix now proves string expressions can feed line-control
comparisons:

```basic
10 LET A$ = "O"
20 IF A$ + "K" = "OK" THEN 50
30 PRINT "BAD"
40 END
50 PRINT "OK"
60 END
```

The row expects stdout `OK` on native-AOT + LLVM + WASM + JVM + CLR + VM + JIT,
proving E4 `str_concat` can feed `str_eq` before the existing `jmp_if_true`
branch.

## 0.125.0 — 2026-06-27 — BASIC string expressions in PRINT run on all seven backends (LANG-FULL BA4/E4)

The BASIC matrix now proves `PRINT` can consume a temporary E4 string expression
result directly:

```basic
10 LET A$ = "O"
20 PRINT A$ + "K"
30 END
```

The row expects stdout `OK` on native-AOT + LLVM + WASM + JVM + CLR + VM + JIT,
proving BA4 string-expression output without routing the concat through `LET`.

## 0.124.0 — 2026-06-27 — BASIC scalar string copy runs on all seven backends (LANG-FULL BA4/E4)

The BASIC matrix now proves literal-backed string-to-string assignment:

```basic
10 LET A$ = "OK"
20 LET B$ = A$
30 PRINT B$
40 END
```

The frontend lowers the copy as E4 `str_concat` with an empty suffix, so the
row expects stdout `OK` on native-AOT + LLVM + WASM + JVM + CLR + VM + JIT
without adding a dedicated string-copy opcode.

## 0.123.0 — 2026-06-27 — BASIC literal string concatenation runs on all seven backends (LANG-FULL BA4/E4)

The BASIC matrix now proves a source-level string expression over E4
`str_concat`:

```basic
10 LET A$ = "O" + "K"
20 PRINT A$
30 END
```

The row expects stdout `OK` on native-AOT + LLVM + WASM + JVM + CLR + VM + JIT.
The frontend materializes both literals, writes the concatenation into the safe
`A$` string slot, and then prints that slot. Non-literal copies and dynamic
byte-string storage remain follow-up BA4/E4 work. The LLVM leg is backed by
`iir-to-llvm` 0.21.0, which binds derived literal concat constants so
`str_concat` can feed `print_str`.

## 0.122.0 — 2026-06-27 — BASIC string inequality drives control flow (LANG-FULL BA4/E4)

The BASIC matrix now proves the other standard equality-family string branch:

```basic
10 LET A$ = "N"
20 IF A$ <> "Y" THEN 50
30 PRINT "BAD"
40 END
50 PRINT "OK"
60 END
```

The frontend lowers `<>` by reusing E4 `str_eq` and branching with
`jmp_if_false`, so stdout `OK` is produced on native-AOT + LLVM + WASM + JVM +
CLR + VM + JIT without adding a bespoke `str_ne` opcode.

## 0.121.0 — 2026-06-27 — BASIC literal string reassignment runs on all seven backends (LANG-FULL BA4/E4)

The BASIC matrix now proves that a scalar string variable can be assigned more
than once from literals and that the latest literal is the one printed:

```basic
10 LET A$ = "NO"
20 LET A$ = "OK"
30 PRINT A$
40 END
```

The row expects stdout `OK` on native-AOT + LLVM + WASM + JVM + CLR + VM + JIT.
This stays inside the literal-backed E4 subset by re-emitting `str_const` into
the same safe backend-facing slot; non-literal copies, string arrays, string
`INPUT`, and dynamic byte-string storage remain follow-up BA4/E4 work.

## 0.120.0 — 2026-06-27 — ALGOL `output` string alias runs on all seven backends (LANG-FULL AL4/E4)

The ALGOL 60 matrix now executes:

```algol
begin string s; s := 'OK'; output(s) end
```

on **native-AOT + LLVM + WASM + JVM + CLR + VM + JIT**, producing stdout `OK`.

This is the same literal-backed scalar string slot as the `print(s)` proof, but
through the implementation-defined `output` spelling. It closes the current AL4
output alias proof without widening into dynamic string values.

## 0.119.0 — 2026-06-27 — BASIC string equality drives control flow (LANG-FULL BA4/E4)

The Dartmouth BASIC matrix now executes:

```basic
10 LET A$ = "Y"
20 IF A$ = "Y" THEN 50
30 PRINT "BAD"
40 END
50 PRINT "OK"
60 END
```

on **native-AOT + LLVM + WASM + JVM + CLR + VM + JIT**, producing stdout `OK`.

This proves the BA4 string-variable slice beyond printing: `dartmouth-basic-iir-compiler`
lowers `A$ = "Y"` in `IF` to shared E4 `str_eq`, and the resulting boolean feeds
the existing line-control branch machinery on every backend.

## 0.118.0 — 2026-06-27 — ALGOL string variables run on all seven backends (LANG-FULL AL4/E4)

The ALGOL 60 matrix now executes:

```algol
begin string s; s := 'HI'; print(s) end
```

on **native-AOT + LLVM + WASM + JVM + CLR + VM + JIT**, producing stdout `HI`.

`algol-iir-compiler` 0.12.0 lowers literal assignments to scalar string slots as
direct E4 `str_const` producers and allows `print(s)` only for those
literal-backed slots. Dynamic string values, string copies, captured/`own`
strings, string arrays, and string parameters remain follow-up AL4 work.

## 0.117.0 — 2026-06-27 — ALGOL literal string output runs on all seven backends (LANG-FULL AL4/E4)

The ALGOL 60 matrix now executes:

```algol
begin print('HI') end
```

on **native-AOT + LLVM + WASM + JVM + CLR + VM + JIT**, producing stdout `HI`.

`algol-iir-compiler` 0.11.0 recognises undeclared statement-position
`print`/`output` calls as standard output procedures and lowers string literal
actuals to shared E4 `str_const` + `print_str`. Full ALGOL string variables,
string arrays, and non-literal string expressions remain follow-up AL4 work.

## 0.116.0 — 2026-06-27 — BASIC string variables run on all seven backends (LANG-FULL BA4/E4)

The Dartmouth BASIC matrix now executes:

```basic
10 LET A$ = "HI"
20 PRINT A$
30 END
```

on **native-AOT + LLVM + WASM + JVM + CLR + VM + JIT**, producing stdout `HI`.

`coding-adventures-dartmouth-basic-lexer` 0.2.0 tokenizes `$`-suffixed names as
single `NAME` tokens, `coding-adventures-dartmouth-basic-parser` 0.2.0 accepts
`STRING` primaries, and `dartmouth-basic-iir-compiler` 0.15.0 lowers
literal-backed string variables to safe E4 `str_const` slots consumed by
`print_str`.

## 0.115.0 — 2026-06-27 — Lexical Twig string locals run on all seven backends (LANG-FULL E4)

The Twig matrix now executes `(let ((s "ABC") (i 2)) (string-ref s i))` on
**native-AOT + LLVM + WASM + JVM + CLR + VM + JIT**, returning exit code `67`
everywhere.

`twig-ir-compiler` 0.28.0 materializes lexical `let`/`let*` string literal
bindings as typed `str_const` registers and lets known local string/index
registers feed the E4 string-op lowering. `twig-aot` 0.19.0 carries literal
metadata through the local integer `mov` so the native column folds the same
`str_index` as the other static backends. This closes the local string-slot proof
for the current literal/named-value E4 foothold without claiming
captured/reassigned strings or broader dynamic byte-string values.

## 0.114.0 — 2026-06-27 — Twig string index OOB traps on all seven backends (LANG-FULL E4)

The Twig matrix now treats runtime traps as a first-class expected result via
`Expect::Trap`, and executes `(string-ref "ABC" 3)` on **native-AOT + LLVM +
WASM + JVM + CLR + VM + JIT**. Every backend fails closed instead of returning a
byte or silently skipping the cell.

This closes the E4 `str_index` out-of-bounds proof for the current literal/named
string foothold: WASM, VM, and JIT use their existing bounds checks; JVM/CLR use
their managed string index exceptions; and native AOT plus LLVM now lower a
compile-known literal OOB index to a runtime trap path instead of rejecting during
lowering.

## 0.113.0 — 2026-06-27 — Named Twig string values run on all seven backends (LANG-FULL E4)

The Twig matrix now executes three named string value programs on **native-AOT +
LLVM + WASM + JVM + CLR + VM + JIT**:

- `(define a "AB") (define b "CDE") (string-length (string-append a b))`
  returns exit code `5`.
- `(define s "HELLO") (if (string=? s "HELLO") 42 0)` returns exit code `42`.
- `(define s "ABC") (string-ref s 2)` returns exit code `67`.

`twig-ir-compiler` 0.27.0 keeps non-escaping top-level string defines in `main`
as typed `str_const` registers, so the existing E4 backend support can consume
named string values without dynamic `global_set`/`global_get` or `call_builtin`
paths. Reassignable string variables, captured strings, `let` string slots, and
the out-of-bounds `str_index` trap proof remain follow-up E4 slices.

## 0.112.0 — 2026-06-27 — Twig literal string index runs on all seven backends (LANG-FULL E4)

The Twig matrix now executes `(string-ref "ABC" 1)` on **native-AOT + LLVM +
WASM + JVM + CLR + VM + JIT**, returning exit code `66` everywhere.

`twig-ir-compiler` 0.26.0 lowers direct-literal `string-ref` to typed
`str_const` + integer `const` + `str_index`. Native AOT and LLVM fold from
literal metadata, WASM emits a guarded `i32.load8_u` from its string data
segment, JVM calls `String.charAt(I)`, and CLR calls
`String::get_Chars(int32)`. This extends the all-backend E4 foothold to
in-bounds ASCII indexing without claiming dynamic string variables or the
out-of-bounds trap matrix proof yet.

## 0.111.0 — 2026-06-27 — Twig literal string metadata runs on all seven backends (LANG-FULL E4)

The Twig matrix now executes `(string-length "HELLO")`,
`(string=? "HELLO" "HELLO")`, and
`(string-length (string-append "AB" "CDE"))` on **native-AOT + LLVM + WASM +
JVM + CLR + VM + JIT**, returning exit codes `5`, `1`, and `5` everywhere.

`twig-ir-compiler` 0.25.0 lowers literal `string-length`, `string=?`, and
`string-append`-feeding-`string-length` to typed `str_const` +
`str_len`/`str_eq`/`str_concat`. The code-gen backends each keep the slice
literal-only: native AOT folds to constants, LLVM/WASM read literal metadata,
JVM calls `String.length()` / `String.equals(Object)` / `String.concat(String)`,
and CLR calls `String::get_Length()` / `String::Equals(string,string)` /
`String::Concat(string,string)`.

This extends the all-backend E4 foothold beyond output without claiming full
string algebra: `str_index`, non-literal `str_concat`/`str_eq`, and string
variables remain follow-up work.

## 0.110.0 — 2026-06-27 — BASIC string literal PRINT runs on all seven backends (LANG-FULL E4 / BA4)

The Dartmouth BASIC `PRINT "HELLO"` matrix row now runs on **native-AOT + LLVM + WASM + JVM + CLR + VM + JIT**.
`twig-aot` 0.15.0 lowers native `str_const` + `print_str` to the existing heap-byte
runtime path (`alloc_bytes`, `store_byte`, `call_builtin "print_string"`), so the
native executable links `__twig_print_string` from the existing AOT runtime archive.

This completes the all-backend literal-output proof. Richer byte-string ops
(`str_len`, `str_index`, `str_concat`, `str_eq`) and string variables remain
follow-up E4/BA4 work.

## 0.109.0 — 2026-06-27 — BASIC string literal PRINT reaches the LLVM column (LANG-FULL E4 / BA4)

The Dartmouth BASIC `PRINT "HELLO"` matrix row now runs on **LLVM + WASM + JVM + CLR + VM + JIT**.
`iir-to-llvm` 0.17.0 emits each printable-ASCII literal as a private unmanaged
constant `{ i64 len, [len x i8] bytes }`, then lowers `print_str` to
`@__print_str(payload,len)`. The LLVM matrix runner compiles the same generic C
print runtime used for `__print_i64`, adding a `fwrite`-based `__print_str`.

This remains the literal-output shape only; `str_len`, `str_index`, `str_concat`,
and `str_eq` fail closed until the byte-string runtime lands. Native string
lowering remains the last backend column for this BASIC string-print row.

## 0.108.0 — 2026-06-27 — BASIC string literal PRINT reaches the WASM column (LANG-FULL E4 / BA4)

The Dartmouth BASIC `PRINT "HELLO"` matrix row now runs on **WASM + JVM + CLR + VM + JIT**.
This extends the managed/static literal-output foothold to the in-repo WASM runtime:
`iir-to-wasm` stores printable ASCII string literals in a linear-memory data segment,
materialises the `str` value as an `i32` pointer, and lowers `print_str` to the host
import `env.__print_str(ptr,len)`. The existing BASIC newline still flows through
`putchar`.

This remains the literal-output shape only; `str_len`, `str_index`, `str_concat`,
and `str_eq` fail closed until the byte-string runtime lands. Native and LLVM
string lowering remain follow-up slices.

## 0.107.0 — 2026-06-27 — BASIC string literal PRINT reaches the JVM column (LANG-FULL E4 / BA4)

The Dartmouth BASIC `PRINT "HELLO"` matrix row now runs on **JVM + CLR + VM + JIT**.
This extends the 0.106.0 CLR foothold to a second managed code-gen backend:
the JVM backend maps `str_const` to `ldc` + `CONSTANT_String` and `print_str` to
`PrintStream.print(String)`. The existing BASIC newline still flows through
`putchar`.

This is still the literal-output shape only; `str_len`, `str_index`, `str_concat`,
and `str_eq` fail closed until the managed backends own byte-string semantics.
WASM/native/LLVM string lowering remain follow-up slices.

## 0.106.0 — 2026-06-27 — BASIC string literal PRINT reaches the CLR column (LANG-FULL E4 / BA4)

The Dartmouth BASIC `PRINT "HELLO"` matrix row now runs on **CLR + VM + JIT**.
This extends the E4/BA4 source-language proof beyond the interpreter columns:
the CLR textual `.il` backend maps `str_const` to `ldstr` and `print_str` to
`Console.Write(string)`, then the existing BASIC newline still flows through
`putchar`.

This is not full E4 backend coverage yet. CLR supports the literal-output shape
only; `str_len`, `str_index`, `str_concat`, and `str_eq` still fail closed until
the managed backend owns byte-string semantics. JVM/WASM/native/LLVM string
lowering remain follow-up slices.

## 0.105.0 — 2026-06-26 — BA1-WASM: GOSUB/RETURN on all 7 backends (dispatch-loop fix)

**LANG-FULL BA1** — both executed Dartmouth BASIC `GOSUB`/`RETURN` programs
now run on all **7** backends (native-AOT + LLVM + **WASM** + JVM + CLR + VM + JIT):

- `10 GOSUB 100 / 20 PRINT 1; / 30 GOSUB 100 / 40 END / 100 PRINT 9; / 110 RETURN`
  ⇒ stdout `919` — the same `RETURN` resumes at two different `GOSUB` sites,
  proved on WASM for the first time.
- `10 GOSUB 100 / 20 END / 100 PRINT 8; / 110 GOSUB 200 / 120 PRINT 6; / 130 RETURN /
  200 PRINT 7; / 210 RETURN` ⇒ stdout `876` — nested depth-2 LIFO discipline.

The fix is in `iir-to-wasm` 0.18.0: when the last basic block contains
`jmp_if_true`/`jmp_if_false`, a sentinel empty block is appended so the dispatch
chain is in the second-to-last block, where the loop-restart depth formula works
correctly.  The WASM `StackUnderflow` trap is eliminated.

## 0.104.0 — 2026-06-26 — BASIC `GOSUB`/`RETURN` matrix proof + BASIC-putchar harness fix

- **LANG-FULL BA1** — two executed Dartmouth BASIC `GOSUB`/`RETURN` programs in
  `tests/lang_matrix.rs`: `GOSUB 100` twice sharing one `RETURN` ⇒ stdout `919`
  (proves the same `RETURN` resumes at the dynamically most-recent `GOSUB`), and a
  nested `GOSUB` ⇒ `876` (LIFO across depth > 1). Run on six backends
  (native/LLVM/JVM/CLR/VM/JIT); Wasm excluded pending the BA1-WASM iir-to-wasm
  dispatch-loop fix (the computed-`goto` is irreducible → runtime `StackUnderflow`).
- **Test-harness fix (BASIC putchar print model).** BA2 switched BASIC `PRINT` to
  the `putchar` builtin, but the per-backend host wiring still assumed `print_i64`,
  leaving origin/main's BASIC matrix red on a fresh build: JVM compiled
  `env.BasicRuntime` (println) instead of `env.BFRuntime` (putchar) → empty output;
  VM/JIT/Wasm didn't `.trim()` the putchar byte stream → `"42\n"` vs `"42"`. Fixed
  `run_jvm`/`run_vm`/`run_jit`/`run_wasm` so every backend runs BASIC's putchar
  output (same harness fix as the focused PR). Both matrix guards pass on all 7
  backends for the existing BASIC cells.

## 0.103.0 — 2026-06-26 — Dartmouth BASIC multi-item `PRINT` on all 7 backends (LANG-FULL BA2)

`tests/lang_matrix.rs` gains two executed Dartmouth BASIC programs proving
**same-line multi-item `PRINT`** across native-AOT + LLVM + WASM + JVM + CLR +
VM + JIT:

- `10 PRINT 0 - 12; 34` ⇒ stdout **`-1234`** — two items joined tightly by `;`
  on one line, with multi-digit rendering (`12`, `34`) and a negative sign. The
  old per-item `print_i64` lowering appended a newline, so this would have been
  `-12⏎34`; BA2's character-level model (`dartmouth-basic-iir-compiler` 0.9.0)
  renders digits via the universal `putchar` builtin so items share a line.
- `10 PRINT 5, 6` ⇒ stdout **`5 6`** — the `,` separator inserts a space (vs
  `;`'s tight join).

Both reuse only ops the matrix already runs everywhere (`call`, integer
arithmetic, `cmp_*`, `putchar`), so BA2 added **zero** backend changes. The
existing single-item BASIC cells now route through `putchar` as well and still
pass unchanged. Bumped to 0.103.0.

## 0.102.0 — 2026-06-23 — ALGOL `entier` runs on ALL 7 backends — E8 COMPLETE (LANG-FULL E8 PR-7)

`tests/lang_matrix.rs` gains an executed ALGOL `entier` program —
`begin real r; integer result; r := 0.0 - 2.7; result := 45 + entier(r) end`
⇒ exit **42** — across native-AOT + LLVM + WASM + JVM + CLR + VM + JIT. `entier(E)`
(ALGOL 60 §3.2.5) is the largest integer not greater than the *real* `E` — floor,
rounding toward −∞ — and lowers to a single E8 `real_to_int_floor` IIR op
(`algol-iir-compiler` 0.10.0), so every backend emits its native floor-then-convert
(LLVM `@llvm.floor.f64`+`fptosi`, WASM `f64.floor`+`i32.trunc_f64_s`, JVM
`Math.floor`+`d2i`, CLR `Math::Floor`+`conv.ovf.i4`, native aarch64 `frintm`+`fcvtzs`,
native x86_64 `roundsd …,1`+`cvttsd2si`). The program builds a **negative** real so
the result distinguishes floor from trunc: `45 + entier(−2.7)` = `45 + (−3)` = 42
(trunc would give 43). `run_native` compiles for the host arch, so the `NativeAot`
cell is executed on aarch64 locally (Apple Silicon). This is the proof that **closes
the E8 numeric-conversions arc** — `int_to_real`/`real_to_int_trunc`/`real_to_int_floor`
now run end-to-end on all seven backends.

## 0.101.0 — 2026-06-21 — ALGOL arrays run on the JVM (LANG-FULL E5 PR-3)

`concretize_scalar_any_for_jvm` now narrows an `array<i64>` handle to
`array<i32>` in lockstep with the existing scalar `i64`→`i32` narrowing. Without
this the JVM backend would build a `long[]` (from `alloc_array`'s `array<i64>`)
but emit `iaload`/`iastore` (from the now-`i32` element hints) — a `long[]` with
`iaload` is a real-`java` `VerifyError`. Aligning the element type makes the
whole array `int[]` and lets the ALGOL sum-of-squares array `Prog` run on the
**JVM** column (`lang_matrix.rs`, exit 55) alongside VM + JIT. Pairs with
`iir-to-jvm-class-file` 0.16.0 (native `int[]`/`long[]`/`double[]` lowering).

## 0.100.0 — 2026-06-20 — ALGOL 60 reals run on ALL 7 backends — E3 COMPLETE (LANG-FULL E3-native)

The two ALGOL 60 **real** matrix programs now run on the **`NativeAot`** column
too — completing the set: native-AOT + LLVM + WASM + JVM + CLR + VM + JIT. The
direct native backends emit f64 codegen (`aarch64-backend` 0.11.0's `fadd`/`fcmp`,
`x86_64-backend` 0.13.0's SSE2 `addsd`/`ucomisd`). `run_native` compiles for the
host arch, so the `NativeAot` cell is executed on aarch64 locally (Apple Silicon)
and on x86_64 by the Linux-x86 CI runner; `proven_columns_do_not_silently_skip`
guards both.

**Enabler E3 (floating-point) is complete** — every code-gen backend plus the
VM/JIT execute `f64`. ALGOL `real` (AL1) is fully done.

## 0.99.0 — 2026-06-20 — ALGOL 60 reals now run on the CLR too — 6/7 backends (LANG-FULL E3-clr)

The two ALGOL 60 **real** matrix programs now also run on the **CLR** column
(LLVM + WASM + JVM + CLR + VM + JIT — **6 of 7 backends**). `iir-to-cil-bytecode`
0.22.0's textual `.il` emitter lowers `f64` to `float64` locals + `ldc.r8`
constants (CIL's `add`/`mul`/`ceq`/`clt` are stack-type-overloaded, so no opcode
change), with the comparison result forced to `int32`. The
`proven_columns_do_not_silently_skip` guard confirms real `ilasm` + `dotnet`
execute the programs.

Only **E3-native** (the x86_64/aarch64 direct native backends) remains before
ALGOL reals run on all 7.

## 0.98.0 — 2026-06-20 — ALGOL 60 reals now run on the JVM too — E3-codegen-slots complete (LANG-FULL E3-codegen-slots, JVM)

The two ALGOL 60 **real** matrix programs now also run on the **JVM** column
(LLVM + WASM + JVM + VM + JIT — 5 of 7 backends). `iir-to-jvm-class-file` 0.15.0
fixes the two remaining f64 gaps: non-0/1 double constants now get a real
`CONSTANT_Double` pool entry (`ldc2_w <idx>` instead of the invalid `#0`
placeholder), and f64 comparisons emit `dcmpl`/`dcmpg` + a unary branch instead
of falling through to the integer `if_icmp` path (which mis-read a two-slot
double as an int → VerifyError). The `proven_columns_do_not_silently_skip`
guard confirms real `java` executes the programs.

**This completes E3-codegen-slots** (LLVM + WASM + JVM all run reals). Remaining
E3: the two direct native backends (x86_64/aarch64 — E3-native) and CLR
(E3-clr) still reject `Operand::Float`.

## 0.97.0 — 2026-06-20 — ALGOL 60 reals now run on the WASM backend too (LANG-FULL E3-codegen-slots, WASM)

The two ALGOL 60 **real** matrix programs now also run on the **WASM** column
(LLVM + WASM + VM + JIT). WASM needed **no backend change**: `iir-to-wasm`'s
typed-local model already carries an `f64` variable in an `F64` local and
selects `f64.mul`/`f64.eq`/`f64.lt` from the `f64` type_hint — the
uniform-`i64`-slot problem that LLVM had simply doesn't exist there. The
`proven_columns_do_not_silently_skip` guard confirms `wasm-runtime` actually
executes the programs. (`iir-to-wasm` 0.15.1 adds op-selection regression
tests.) Remaining E3: jvm (slot fix), native + CLR (FP emission).

## 0.96.0 — 2026-06-20 — ALGOL 60 reals now run on the LLVM backend too (LANG-FULL E3-codegen-slots, LLVM)

The two ALGOL 60 **real** matrix programs (real `*`+`=` → exit 42, real `/`+`<`
→ exit 1) now run on the **LLVM** column in addition to VM/JIT — `iir-to-llvm`
0.13.0 gives an `f64` variable a `double` stack slot, renders f64 constants in
LLVM's exact hex form, and zexts a float comparison's boolean result to `i64`.
The `proven_columns_do_not_silently_skip` guard confirms clang actually executes
the programs (no silent skip).

Remaining E3 backends: wasm + jvm need the same slot fix (E3-codegen-slots);
native (x86_64/aarch64) + CLR need FP emission (E3-native / E3-clr).

## 0.95.0 — 2026-06-20 — ALGOL 60 real (f64) programs run on VM/JIT (LANG-FULL AL1 / E3 phase 1)

`tests/lang_matrix.rs` adds two ALGOL 60 **real** programs, executed on the VM
and JIT:

- `r := 2.5 * 2.0; if r = 5.0 then result := 42 …` → exit **42** (real multiply
  + `f64` equality), and
- `r := 7.0 / 2.0; if r < 4.0 then result := 1 …` → exit **1** (real division +
  `f64` ordered comparison).

These RUN end-to-end real arithmetic (`algol-iir-compiler` 0.4.0 lowers `real`
→ IIR `f64`; `vm-core` 0.6.0 executes the `f64` ops). The comparison fold yields
an integer exit code, so no float *printing* is needed to verify.

**Scope.** The proofs run on the VM + JIT only — they carry a tagged float value
model. The five code-gen backends are deferred E3 follow-ups: `iir-to-{llvm,
wasm,jvm}` model every variable slot as a uniform `i64` so an `f64` variable
can't be stored (E3-codegen-slots), and `iir-to-cil-bytecode` / the x86_64 +
aarch64 native backends reject `Operand::Float` (E3-clr / E3-native). See
`code/specs/LANG-FULL-IMPLEMENTATION.md`.

## 0.94.0 — 2026-06-20 — BASIC `DEF FN` runs cross-backend; JVM scalar concretization is module-consistent (LANG-FULL BA5)

### Added — executed matrix proof for BASIC user-defined functions

`tests/lang_matrix.rs` adds `DEF FNS(X) = X * X : PRINT FNS(7)` → stdout `49`
on **all 7 backends** (native/LLVM/WASM/JVM/CLR/VM/JIT). This RUNS a real
cross-function `call` combined with `print_i64`, proving a same-module function
call resolves and executes on the code-gen backends, not just the VM/JIT.
(`dartmouth-basic-iir-compiler` 0.6.0 lowers `DEF FN`.)

### Fixed — `concretize_scalar_any_for_jvm` decided per-function, breaking cross-function calls

The JVM scalar-concretization pass narrows a scalar function's `i64` values to
`i32` (the in-repo `jvm-simulator` is 32-bit) **unless** the function prints —
a printing function keeps the wide i64 model because `println(J)V` needs a
`long`. But this was a **per-function** decision, and a `call` couples two
functions' value models: the caller pushes the argument and consumes the result
at the callee's declared width. The BA5 program lowers to a printing `main`
(kept i64) plus a non-printing helper `FNS` (narrowed to `(I)I`), so `main`
`invokestatic`'d `FNS` with a `long` argument and `lstore`'d an `int` result —
real `java` rejected the mismatch with a `VerifyError` and the program printed
nothing.

Concretization is now a **whole-module** decision: if **any** function in the
module prints, every scalar function stays at i64, keeping all cross-function
call signatures consistent. A module with no printing function (Nib/Twig/ALGOL,
which return an exit code) still concretizes to i32 uniformly — unchanged. Only
printing multi-function modules are affected.

## 0.93.0 — 2026-06-17 — Brainfuck cat `,[.,]` runs cross-backend (LANG-FULL B1-eof)

`tests/lang_matrix.rs` adds the canonical Brainfuck **cat** — `,[.,]` (read a byte and,
while non-zero, print it and read the next) — with input `"Hi"` → stdout `"Hi"`, on **all
7 backends** (native/LLVM/WASM/JVM/CLR/VM/JIT).

This closes the EOF-convention divergence deferred from B1-stdin (#6006). The `,+.`/`,.,.`
stdin programs read exactly their input and never hit EOF; cat reads *past* the input, so
it exercises the convention directly. Backends disagreed — JVM/VM/JIT returned `0` at EOF,
but libc `getchar`/`Console.Read`/the wasm host returned `-1` → the u8 cell wrapped to
`255` → cat looped forever there. `brainfuck-iir-compiler` 0.4.0 now clamps a negative `,`
result to `0` in the shared IIR (read at i64, `cmp_lt 0` + branch, store u8), so EOF is `0`
on every backend and cat halts. No per-backend change.

## 0.92.0 — 2026-06-16 — Oct bitwise `~` + u8 wrap run cross-backend (LANG-FULL O2)

`tests/lang_matrix.rs` gains two executed Oct programs proving u8 width semantics run on
**all 7 backends** (native/LLVM/WASM/JVM/CLR/VM/JIT):

- `out(1, ~0)` → `255`. Oct's only integer type is `u8`, so `~0` flips 8 bits → `255`
  (`-1 & 0xFF`), not the i64 all-ones. (An unmasked `~0` would print `-1`.)
- `out(1, 200 + 100)` → `44`. The grammar specifies Oct addition wraps mod-256; `300` wraps
  to `44`. Distinct from the existing `100 + 100 = 200` Oct program, which does not overflow
  — this one proves the wrap fires.

Driven by `oct-iir-compiler` 0.7.0 (emits the `u8` hint on arithmetic/bitwise/`~`; Oct has a
single integer width, so every integer op is u8) and `iir-to-jvm-class-file` 0.14.0 (Oct's
`out` programs keep the JVM long model, so the narrow mask had to become `i2l; land` over the
long result — the int `iand` was unverifiable and returned empty on java). Completes Oct O2.

## 0.91.0 — 2026-06-16 — Nib bitwise `~` runs cross-backend (LANG-FULL N3)

`tests/lang_matrix.rs` gains two executed `~` programs proving unary bitwise NOT runs
on **all 7 backends** (native/LLVM/WASM/JVM/CLR/VM/JIT):

- `~0u8` — `let x: u8 = ~0; if x == 255 { return 1; }` → exit `1`. `~0` flips all bits;
  masked to u8 it is `255`. The `== 255` guard distinguishes the masked complement from
  an unmasked `not 0` (`-1`, which would give exit `0`).
- `~15u4` — `let x: u4 = ~15; if x == 0 { return 1; }` → exit `1`. On a nibble `~15 = 0`;
  proves the mask is *width-correct* (a u8/i64 mask would leave `0xF0`/`-16`, not `0`).

Driven by `nib-iir-compiler` 0.16.0 (lowers `~` → IIR `not` with the narrow width — it had
been silently dropped) and `iir-to-cil-bytecode` 0.21.0 (adds the unary `not` arm to its
textual `.il` emitter — the last backend that couldn't assemble `~` on CoreCLR). Completes
Nib N3 (`& | ^ ~`).

## 0.90.0 — 2026-06-16 — Brainfuck reads real stdin cross-backend (LANG-FULL B1-stdin)

The matrix proved every backend can *write* output (`.`); two new programs prove every
backend can *read* input (`,`):

- `,+.` — read one byte from real stdin, `+` (increment), print: input `"A"` (65) →
  output `"B"` (66). The output depends on **both** the input and a computation on it —
  not a constant, not a bare echo.
- `,.,.` — read a byte and print it, twice: input `"Hi"` → output `"Hi"`. Proves
  *repeated* reads advance through the input stream (the second `,` sees `'i'`, not `'H'`).

Both run on **all 7 backends** (native/LLVM/WASM/JVM/CLR/VM/JIT — verified locally with
every toolchain present). Wiring (test-harness only — every backend already *compiled*
`,`→`getchar`; it had simply never been fed input):

- **native / LLVM / JVM / CLR** read from their real process stdin (libc `getchar`,
  `System.in`, `Console.Read`). New `output_with_stdin` helper spawns the child with a
  piped stdin, writes the program's input, and closes the pipe (→ EOF).
- **WASM / VM / JIT** are in-process: their `getchar` host now drains a per-program byte
  buffer seeded from `program_stdin` (empty for every non-stdin program, so the first
  read is EOF — unchanged behaviour).

Both programs read **exactly** the bytes supplied and never read past EOF, so they
terminate identically on every backend **regardless** of the divergent `getchar`-EOF
convention (JVM/VM/JIT return `0`; libc/`Console.Read`/wasm return `-1` → the u8 cell
wraps to 255). The classic cat `,[.,]` would loop forever on the `-1` backends, so
normalising EOF across backends is tracked as a separate item. No backend/frontend crate
changed.

## 0.89.0 — 2026-06-16 — Nib `+%` wrapping / `+?` saturating add cross-backend (LANG-FULL N7)

Four new matrix programs prove Nib's N7 operators run on all 7 backends: `200u8 +% 100`
→ wraps to `44`, `200u8 +? 100` → saturates to `255`, `15u4 +? 1` → clamps to `15`,
and `3 +? 4` → `7` (no clamp). Comparison-based so they distinguish saturate/wrap from
the unclamped sum. (nib-iir-compiler 0.15.0.)

## 0.88.0 — 2026-06-16 — Oct `&&`/`||` short-circuit run on the JVM (LANG-FULL, BA-JVM-1 follow-through)

The two Oct short-circuit matrix programs (`&&` → `9`, `||` → `7`) now include the
**JVM** backend: `iir-to-jvm-class-file` 0.13.3 makes a `mov` bridge int↔long when
the dest slot width differs (Oct keeps the i64 value model, so a bool comparison
result mov'd into a long accumulator needs `i2l`). With this, **every matrix
program runs on all 7 backends**.

## 0.87.0 — 2026-06-16 — BASIC `IF`/`FOR` run on the JVM (LANG-FULL BA-JVM-1)

The two Dartmouth BASIC control-flow matrix programs (the `FOR` sum → `15` and
the `IF` branch → `7`) now include the **JVM** backend: `iir-to-jvm-class-file`
0.13.2 fixes the comparison-dest slot typing that made a branch-after-a-loop over
BASIC's i64 value model fail JVM verification (`uninitialized register pair`).
Both run on real `java` now; the matrix proves it cross-backend.

## 0.86.0 — 2026-06-15 — Nib u8 wrap runs cross-backend (LANG-FULL E2 / N6)

`tests/lang_matrix.rs` gains two new **Nib integer-wrap** programs, both run
across native / LLVM / WASM / JVM / CLR / VM / JIT:

**Wrap proof** (LANG-FULL N6 — u8 wrap semantics):
```nib
fn main() -> u8 { let x: u8 = 200 + 100; if x == 44 { return 1; } return 0; }
```
→ exit **1**. `200 + 100 = 300` wraps mod-256 to **44** in every backend;
comparing the in-register value (not just the exit-code low byte) proves the
wrap happened *before* the comparison.  Exercises the full E2 stack:
`nib-type-checker` 0.3.0 annotates the `add_expr` node as `u8` (bidirectional
context flows from the `let x: u8` declaration); `nib-iir-compiler` 0.14.0
emits `add` with `type_hint = "u8"`; each backend masks — vm-core and jit-core
by `result & 0xFF`, iir-to-wasm by `i32.and 255`, iir-to-jvm/cil by `and`,
iir-to-llvm by `and i64 ..., 255`, native-AOT by `and X0, X0, #0xFF` /
`and rax, 0xFF`.

**Magnitude regression guard** (confirms bidirectional typing):
```nib
fn main() -> u8 { return 6 * 7; }
```
→ exit **42**. Without `nib-type-checker` 0.3.0, `6` and `7` would infer as
`u4` (magnitude ≤ 15), the backend would mask `42 & 0xF = 10`, and the test
would fail.  Passing proves literals in a `u8` return context adopt `u8`, and
`6 * 7 = 42` is within the u8 range so no wrap occurs.

### Crates updated

- `nib-type-checker` 0.3.0 — bidirectional typing
- `nib-iir-compiler` 0.14.0 — narrow type_hints + unary `~` lowering
- `aot-core` 0.2.2 — `u4` added to CIR type pipeline

## 0.85.0 — 2026-06-14 — Twig top-level value `define` runs cross-backend (LANG-FULL TW2)

`tests/lang_matrix.rs` gains an executed **top-level value `define`** program:

```scheme
(define x 40) (define y 2) (+ x y)
```

⇒ exit **42**, running across native / LLVM / WASM / JVM / CLR / VM / JIT.
Previously a value `define` lowered to `call_builtin "global_set"` (and reads to
`global_get`), `type_hint = "any"` — rejected by every code-gen backend
validator, so top-level constants ran only on the VM. `twig-ir-compiler`
(0.24.0) adds a small escape analysis: a value `define` not captured by any
lambda is read only from `main`, so its statically-typed value stays in a `main`
register and reads return it directly — no `call_builtin` survives. A
lambda-captured value (or a forward reference) still uses the host global table,
unchanged.

## 0.84.0 — 2026-06-14 — Twig variadic arithmetic runs cross-backend (LANG-FULL TW1)

`tests/lang_matrix.rs` gains an executed **n-ary Twig arithmetic** program:

```scheme
(+ 10 20 12)
```

⇒ exit **42**, running across native / LLVM / WASM / JVM / CLR / VM / JIT.
`twig-ir-compiler` (0.23.0) folds an all-`i64` arithmetic call (`+`/`-`/`*`/`/`)
into a left-associated chain of typed binary CIR ops (`r1 = add 10,20; r2 = add
r1,12`). Before TW1 only the binary `(+ a b)` form lowered to a typed `add`;
three-or-more-argument calls fell back to `call_builtin "+"` (`type_hint =
"any"`), which every code-gen backend validator rejects — so this is the first
variadic Twig arithmetic to run anywhere but the dynamic VM/JIT path.

## 0.83.0 — 2026-06-14 — ALGOL switches run cross-backend (LANG-FULL AL5)

`tests/lang_matrix.rs` gains an executed ALGOL **switch / computed goto**:

```algol
begin integer result; switch s := a1, a2, a3; integer i; i := 3;
  goto s[i];
  a1: result := 1; goto done;
  a2: result := 2; goto done;
  a3: result := 49; done:
end
```

⇒ exit **49**, running across native / LLVM / WASM / JVM / CLR / VM / JIT.
`algol-iir-compiler` (0.3.0) lowers `goto s[i]` to a 1-based `index == k ? jmp Lk`
chain over the portable `jmp`/`jmp_if_false`/`label` subset.

The switch's index test (`i == k`) is the **first ALGOL comparison ever run on a
code-gen backend**, and it surfaced a latent width bug: ALGOL emitted `cmp_*`
with a `bool` type_hint, so LLVM compared two `i64` operands at 1-bit `i1`
(`3 == 1` → `1 == 1` → true → wrong target) and emitted invalid IR `clang`
rejected outright — the matrix cell failed to run at all. Fixed in
`algol-iir-compiler` 0.3.0 by comparing at the **operand** width (`i64`), the
same fix the BASIC BA0 work applied. `s[3]` is the chosen index precisely
because an i1 compare mis-selects the first arm, so the cell proves the fix.

## 0.82.0 — 2026-06-14 — ALGOL typed procedures run cross-backend (LANG-FULL AL3)

`tests/lang_matrix.rs` gains the first **multi-function ALGOL** program: a typed
procedure with a value parameter,

```algol
begin integer result;
  integer procedure sq(x); value x; integer x; sq := x * x;
  result := sq(7)
end
```

⇒ exit **49**, running across native / LLVM / WASM / JVM / CLR / VM / JIT. The
`algol-iir-compiler` (0.2.0) lowers the procedure to a sibling
`IIRFunction sq(x: i64) -> i64` and the call to an IIR `call`; every backend
already resolves a same-module `call` by name, so procedures are "just functions
+ calls" in the shared IIR — no backend learned anything about ALGOL.

Running this across all backends **surfaced a real `jit-core` bug**: the
`CIROptimizer`'s constant propagation never invalidated a register's known
constant when the register was reassigned, so a procedure's result slot
(`const sq=0; …; mov sq=t; ret sq`) propagated the dead `0` and the JIT returned
`0` instead of `49` while every other backend agreed on `49`. Fixed in `jit-core`
0.4.1 (reassignment-kills + block-boundary-clears). The executed matrix is
exactly the guardrail that caught it.

## 0.81.0 — 2026-06-14 — real Brainfuck programs run cross-backend (LANG-FULL B1)

`tests/lang_matrix.rs` gains two executed Brainfuck programs beyond the existing
1-loop "print A": a **nested-loop** multiply-by-repeated-addition program
(`++++++++>+++++++++<[>[->+>+<<]>>[-<<+>>]<<<-]>>.-------.`) that computes 8 × 9 = 72
then adjusts to 65 → stdout **"HA"**, and a two-sequential-loop program → **"OK"**.
Both run across native / LLVM / WASM / JVM / CLR / VM / JIT. This proves the backends
lower **nested loops + multi-cell pointer movement + multiple `putchar`s**, not just a
single loop printing one char — converting the biggest Brainfuck smoke-test gap (only
the trivial "A" ran cross-backend) into real coverage. Test-harness only; Brainfuck's
semantics were already complete, so no `brainfuck-iir-compiler` change.

## 0.80.0 — 2026-06-13 — Oct `&&` / `||` short-circuit, proven by running (LANG-FULL O1)

`tests/lang_matrix.rs` gains two executed Oct programs that PROVE short-circuit via
a side-effecting function call in the right operand — `if 1 == 2 && side() == 1 { … }
else { out(1, 9) }` where `side()` prints 5 → stdout **"9"** (the old eager code
printed "5","9"), and the `||` analogue → **"7"** — across native / LLVM / WASM /
CLR / VM / JIT (JVM excluded: branch + print, the BA-JVM-1 follow-up). Backed by
`oct-iir-compiler` 0.6.0 (short-circuit lowering + the i64 function-return fix the
proof's `side() -> u8` helper exposed). No `lang-aot/src` change.

## 0.79.0 — 2026-06-13 — Oct gains observable output via `out` (LANG-FULL O-OUT)

`tests/lang_matrix.rs` gains two executed Oct programs that print to **stdout** —
`out(1, 200)` → `200` and `out(1, 100 + 100)` → `200` (arithmetic proven observably)
— across native / LLVM / WASM / JVM / CLR / VM / JIT. This is Oct's first checkable
output: until now Oct's void `main` always exited 0, so no Oct result could be
verified by running. Backed by `oct-iir-compiler` 0.5.0, which lowers the 8008 `out`
intrinsic to `call_builtin "print_i64"`. Unblocks verification of the Oct value-level
items. No `lang-aot/src` change.

## 0.78.0 — 2026-06-13 — BASIC control flow runs on the code-gen backends (LANG-FULL BA0)

`tests/lang_matrix.rs` gains two executed Dartmouth BASIC programs exercising real
control flow — a `FOR`/`NEXT` accumulator (`FOR I = 1 TO 5: S = S + I` → prints
**15**) and an `IF A > 5 THEN 100` jump (→ prints **7**) — across native / LLVM /
WASM / CLR / VM / JIT. Until now BASIC loops/conditionals executed only on the
VM/JIT. Backed by `dartmouth-basic-iir-compiler` 0.5.0, which fixes the comparison
`type_hint` (`bool` → `i64` operand width) that had LLVM comparing at a 1-bit `i1`
width. JVM is excluded for these two programs pending BA-JVM-1 (a StackMapTable
follow-up for branch + `print_i64`). No `lang-aot/src` change.

## 0.77.0 — 2026-06-13 — Nib `const` declarations executed on every backend (LANG-FULL N5)

`tests/lang_matrix.rs` gains two executed Nib programs using module-scoped
`const`s — `const N: u8 = 42; … return N;` → 42 and `const A = 30; const B = 12;
… A + B` → 42 — across native/LLVM/WASM/JVM/CLR/VM/JIT. Backed by
`nib-iir-compiler` 0.13.0 (a const reference folds to its literal). Frontend-only;
no backend change. No `lang-aot/src` change.

## 0.76.0 — 2026-06-13 — Nib `&&` / `||` short-circuit executed on every backend (LANG-FULL N4)

`tests/lang_matrix.rs` gains three executed Nib programs proving short-circuit
`&&`/`||` across native/LLVM/WASM/JVM/CLR/VM/JIT: `1 == 2 && 84 / 0 == 0` → 7 and
`1 == 1 || 84 / 0 == 0` → 7 (the divide-by-zero right operand is positive proof it
was never evaluated — if it were, the program would trap), plus a `&&` true-path
program → 1. Backed by `nib-iir-compiler` 0.12.0 (`compile_short_circuit`). Frontend-
only — no backend change. No `lang-aot/src` change.

## 0.75.0 — 2026-06-13 — Nib bitwise `& | ^` executed on every backend (LANG-FULL N3)

`tests/lang_matrix.rs` gains three executed Nib programs — `12 & 10` → 8,
`12 | 3` → 15, `6 ^ 5` → 3 — asserted across native/LLVM/WASM/JVM/CLR/VM/JIT.
Backed by `nib-iir-compiler` 0.11.0 (lowers `& | ^` to the shared IIR
`and`/`or`/`xor`). The executed test surfaced a CLR backend gap — the textual
`.il` path didn't emit the bitwise opcodes — fixed in `iir-to-cil-bytecode`
0.19.0. No `lang-aot/src` change.

## 0.74.0 — 2026-06-13 — reassigning a parameter in a loop runs on LLVM too (LANG-FULL — LLVM first-class)

`tests/lang_matrix.rs` gains an executed Nib program that accumulates into a
**function parameter** across a loop — `fn run(acc: u8) { for i in 0 .. 7 { acc = acc + 6 } return acc }`
→ exit 42 — asserted across native/LLVM/WASM/JVM/CLR/VM/JIT. This was the
limitation surfaced (and scoped out) in N2: the IIR-to-LLVM backend kept params in
SSA, so a reassigned param was silently dropped. Fixed in `iir-to-llvm` 0.10.0
(reassigned params are promoted to i64 stack slots, initialised from the incoming
argument). The other backends already handled it; this closes the LLVM gap so the
column is genuinely first-class. No `lang-aot/src` change.

## 0.73.0 — 2026-06-13 — Nib `for` loops executed on every backend (LANG-FULL N2)

`tests/lang_matrix.rs` gains two executed Nib `for`-loop programs — a sum loop
`for i in 1 .. 6 { s = s + i }` → exit 15 (uses the loop variable) and a nested
loop (3 × 2) → exit 6 — asserted across native/LLVM/WASM/JVM/CLR/VM/JIT. Backed
by `nib-iir-compiler` 0.10.0 (lowers `for` to the canonical counter loop). The
matrix battery now exercises real cross-backend loop control flow with counter +
accumulator reassignment, not just straight-line arithmetic. No `lang-aot/src`
change.

## 0.72.0 — 2026-06-13 — Nib `*` and `/` executed on every backend (LANG-FULL N1)

First slice of the LANG-FULL campaign (full implementations of every matrix
language, each feature **verified by RUNNING** on every backend, not just
validated/encoded). `tests/lang_matrix.rs` gains two executed Nib programs —
`fn main() -> u8 { return 6 * 7; }` → exit 42 and `… 84 / 2; }` → exit 42 —
asserted across native/LLVM/WASM/JVM/CLR/VM/JIT. Backed by `nib-iir-compiler`
0.9.0 (lowers `*`/`/` to the shared IIR `mul`/`div`). No `lang-aot/src` change;
the matrix battery now has more than one executed program per language.

## 0.71.0 — 2026-06-13 — Every language on the generic JIT — JIT COLUMN COMPLETE → MATRIX COMPLETE (LANG-MATRIX Phase I)

Completes the **JIT column** — the last open column of the platform matrix. All six
matrix languages now run on the **generic JIT**, so every language runs on every backend
except BEAM, **verified by running**.

`tests/lang_matrix.rs` gains a `Backend::Jit` runner: source →
`compile_source_to_iir` (the *same* shared pipeline every other column uses) →
`jit_core::JITCore` driving the language-agnostic `GenericCirJit` over the shared IIR.
`execute_with_jit` eagerly compiles each fully-typed function to JIT bytecode (installing
a native handler) and interprets the rest on `VMCore`, so each program runs *through the
JIT pipeline*. The I/O builtins (`print_i64`/`putchar`/`getchar`) are registered as
callbacks on **both** the VM (interpreter-fallback tier) and the `GenericCirJit` backend
(compiled tier) — **no per-language code**, exactly the way a future Ruby/JS frontend
would plug in. Verified by RUNNING in-process: Twig→42, Nib→42, Oct→0, ALGOL `17 mod 5`→2,
BASIC `10 PRINT 42`→stdout `42`, Brainfuck `++++++++[…]>+.`→`A`. The matrix's floor test
asserts every JIT-tagged cell actually runs.

The Nib cell (`double(21)`) surfaced — and this slice fixes, in `jit-core` 0.4.0 — a
genuine generic-JIT gap: `GenericCirJit::run` ignored its `args`, so a JIT-compiled
function read its parameters as zero. See that crate's changelog; the upshot is any
frontend whose functions take arguments now JITs correctly.

### Also — fix the pre-existing `cil_emit.rs` test

`tests/cil_emit.rs` (CLR-emit, McCarthy W6a) no longer compiled: the `clr-simulator`
evaluation stack became `Vec<Option<Value>>` in W6b (#5296), but this scalar test — which
predates that change and only ever ran via its own (dev-dependency) edge — still expected a
bare `i32`. It now matches `Value::Int(n)`. Unrelated to the JIT work, but `cil_emit` is a
`lang-aot` test, so this slice (which touches `lang-aot`) restores it to green.

## 0.70.0 — 2026-06-13 — Brainfuck on the VM — VM COLUMN COMPLETE (LANG-MATRIX Phase V, slice 2)

Completes the **VM column**: Brainfuck now runs on the generic register VM too, so **all
six matrix languages run on the one `vm_core::VMCore` interpreter** via the shared IIR.
Verified by RUNNING `++++++++[>++++++++<-]>+.` on the VM: it prints `A`.

The lowering work is in `vm-core` 0.4.0 (the byte-tape ops `alloc_bytes`/`load_byte`/
`store_byte` over its flat `memory` — see that crate's changelog). This crate's change is
the test tag: `tests/lang_matrix.rs` adds `Vm` to the Brainfuck `Prog`. `run_vm` already
registered the `putchar` builtin (slice 1), so Brainfuck's `.` captures bytes (→ `A`) with
no further wiring. No `lang-aot/src` change.

## 0.69.0 — 2026-06-12 — generic register VM column (LANG-MATRIX Phase V, slice 1)

Begins the **VM column** of the matrix — and does it the way the project intends: a
**generic** register VM, not a per-language one. `lang_matrix.rs` gains a `Backend::Vm`
runner that interprets the **shared** `IIRModule` (the *same* `compile_source_to_iir`
output every code-gen column consumes) on `vm_core::VMCore`, the general register VM whose
instruction dispatch already covers arithmetic / comparison / bitwise / control-flow /
memory / `call_builtin`. **There is no per-language code in the runner** — a future Ruby/JS
frontend that lowers to IIR would run identically.

Correcting a prior mischaracterisation: the LM0 probe's "the VM rejects `add`/`mul`/`cmp_*`"
was about `mccarthy-lisp-vm`, a *deliberately separate* lisp interpreter (its value model is
`lispy-runtime`'s tagged `LispyValue`, and McCarthy lowers arithmetic to `call_builtin`, so
its IIR has no `add`). The general `VMCore` has always handled scalar arithmetic — Brainfuck
ran on it for years.

- `run_vm`: source → `compile_source_to_iir` → `VMCore::execute`. The I/O languages print
  through a registered builtin closure — `print_i64` (Dartmouth BASIC) appends to a capture
  buffer (the VM sibling of the wasm `PrintHost` / LLVM `@__print_i64` / JVM `BasicRuntime` /
  CLR `Console.WriteLine`); `putchar`/`getchar` are registered for the next slice. An
  expression language's `Int` result is the exit code; an I/O language's stdout is the buffer.
- `lang_matrix.rs` adds `Vm` to the `Backend` enum + the floor test (in-process, always runs)
  and tags Twig / Nib / Oct / ALGOL / Dartmouth BASIC with `Vm`.
- Verified by RUNNING in-process: Twig→42, Nib→42, Oct→0, ALGOL `17 mod 5`→2, BASIC→`42`.
  5/6 of the VM column; only Brainfuck-on-VM remains (it needs the `alloc_bytes`/`load_byte`/
  `store_byte` tape ops added to `vm-core`, the next slice). No `vm-core` change in this slice.

## 0.68.0 — 2026-06-12 — Brainfuck runs on CoreCLR — CODE-GEN MATRIX COMPLETE (LANG-MATRIX LM-C Brainfuck)

`lang_matrix.rs` greens **Brainfuck on real CoreCLR** — the **last code-gen cell** of the
LANG-PLATFORM-MATRIX. With it, **every language (Twig / Nib / Brainfuck / Dartmouth BASIC
/ Oct / ALGOL 60) runs on every code-gen backend (native-AOT / LLVM / WASM / JVM / CLR),
verified by running.** Only the deliberately-deferred VM and JIT columns remain. Verified
by RUNNING `++++++++[>++++++++<-]>+.` on `ilasm`+`dotnet`: it prints `A`.

The lowering work is in `iir-to-cil-bytecode` 0.18.0 (the textual `.il` byte-tape ops +
`putchar`/`getchar` + the launcher's `putchar`-aware "prints" detection — see that
crate's changelog). This crate's change is in the test harness: `tests/lang_matrix.rs`
adds `Clr` to the Brainfuck `Prog`. `run_clr` (assemble with real `ilasm` → run on real
`dotnet`) is unchanged — it already returns the captured `Console` output, so a
Brainfuck program's `Write(char)` byte stream (`A`) is compared against `Expect::Stdout`.

No `lang-aot/src` change: `lower_brainfuck_for_aot`'s rewrite + i64 widening (0.65.0) and
`concretize_scalar_any_for_cil` (which retypes Brainfuck back to `int32`, since it doesn't
call `print_i64`) already flowed to the CLR path.

## 0.67.0 — 2026-06-12 — Brainfuck runs on the JVM (LANG-MATRIX LM-J Brainfuck)

`lang_matrix.rs` greens **Brainfuck on the JVM backend** — the last code-gen gap in
Brainfuck's row (only the deferred VM/JIT columns remain). Verified by RUNNING
`++++++++[>++++++++<-]>+.` on real `java`: it prints `A`.

The lowering work is in `iir-to-jvm-class-file` 0.11.0 (byte-tape ops + i64 branch
conditions — see that crate's changelog). This crate's change is in the test harness:

- New `BF_RUNTIME_JAVA` host class (`env.BFRuntime`) — a static `byte[] __tape` (the
  30 000-cell tape `alloc_bytes` references), `putchar(int)` (writes a raw byte to
  stdout — so `.` of 65 yields the byte `A`, not the decimal `65` that BASIC's
  `println` would), and `getchar()` (returns `0` at EOF).
- `run_jvm` compiles the host class the program's I/O lowers to: `env.BFRuntime` for
  Brainfuck, `env.BasicRuntime` for Dartmouth BASIC. The existing discard-launcher
  (run the entry, `pop`/`pop2` its result) handles Brainfuck's `int` exit unchanged;
  the captured `System.out` bytes are the program's stdout.
- `tests/lang_matrix.rs` adds `Jvm` to the Brainfuck `Prog`.

No `lang-aot/src` change: `lower_brainfuck_for_aot`'s rewrite + i64 widening (added in
0.65.0 for LLVM) is backend-agnostic and already flowed to the JVM path.

## 0.66.0 — 2026-06-12 — Brainfuck runs on WASM (LANG-MATRIX LM-W Brainfuck)

`lang_matrix.rs` greens **Brainfuck on the WASM backend** — the last code-gen gap in
Brainfuck's row (only the VM/JIT columns remain). Verified by RUNNING
`++++++++[>++++++++<-]>+.` on the in-repo `wasm-runtime`: it prints `A`.

The lowering work is in `iir-to-wasm` 0.13.0 (byte-tape ops + i64↔i32 conversions for
the widened Brainfuck value model — see that crate's changelog). This crate's change is
in the test harness:

- `run_wasm`'s host (`PrintHost`) now also resolves Brainfuck's I/O imports:
  `env.putchar : (i32) -> ()` → a new `PutcharFunc` that captures raw output **bytes**
  (so `.` of cell value 65 yields stdout `A`, not the decimal `65` that `__print_i64`
  would), and `env.getchar : () -> i32` → a `GetcharFunc` returning EOF (`-1`).
- `run_wasm` prefers the byte stream when the program wrote any (Brainfuck), else the
  integer stream joined by newlines (BASIC) — the expression languages print nothing.
- `tests/lang_matrix.rs` adds `Wasm` to the Brainfuck `Prog`.

No `lang-aot/src` change: `lower_brainfuck_for_aot`'s i64 widening (added in 0.65.0 for
LLVM) is backend-agnostic and already flowed to the wasm path.

## 0.65.0 — 2026-06-12 — Brainfuck runs on LLVM (LANG-MATRIX LM-L Brainfuck)

`lang_matrix.rs` greens **Brainfuck on the LLVM backend** — the first cell of the
deferred Brainfuck row, and the last code-gen gap for that language (only the VM/JIT
columns remain). Verified by RUNNING `++++++++[>++++++++<-]>+.` on real `clang`: it
prints `A`.

The fix is split across two layers, neither of which touches the McCarthy-critical
`iir-to-llvm` stack-slot allocator:

- **`lower_brainfuck_for_aot` (this crate) — Step 5: widen narrow hints to `i64`.**
  The Brainfuck frontend emits `u8` (cells) / `u32` (pointer) `type_hint`s. The
  AOT/LLVM pass now widens every narrow-integer hint to `i64` so the value model is a
  uniform machine word. Byte width survives **only at the tape boundary**: `load_byte`
  zero-extends the 8-bit cell to `i64` and `store_byte` truncates back, so cell
  wrap-around (`255 + 1 == 0`) stays correct. This is what lets `iir-to-llvm` — which
  promotes any reassigned variable (BF's `ptr`/`v`/`c`/`k`) to an `alloca i64` slot —
  consume Brainfuck without a width mismatch (`'%__ld' defined with type 'i64' but
  expected 'i8'`). It is done in this BF-specific pass, **not the frontend**, so the
  frontend's `u8`/`u32` hints still reach `vm-core`/`jit-core`, whose `specialise` step
  keys CIR opcode widths (`add_u8`/`add_u32`) off them — the brainfuck-iir-compiler VM
  and JIT paths are untouched. Native AOT is unaffected: its byte ops already ignore
  the hint and its arithmetic runs in 64-bit registers regardless.
- **`iir-to-llvm` 0.9.0** grew the byte-tape ops (`alloc_bytes`/`load_byte`/`store_byte`)
  and the `putchar`/`getchar` libc builtins, plus a slot-dest SSA rename (see that
  crate's changelog).

`tests/lang_matrix.rs` adds `Llvm` to the Brainfuck `Prog`; two new `--lib` tests cover
the i64 widening (`brainfuck_lowering_widens_narrow_hints_to_i64`,
`is_narrow_int_hint_classifies_widths`).

## 0.64.0 — 2026-06-12 — Dartmouth BASIC completes the CLR column (LANG-MATRIX LM-C BASIC)

`lang_matrix.rs` greens **Dartmouth BASIC on the real CoreCLR**, completing the CLR
column for every non-Brainfuck language. The CIL backend (`iir-to-cil-bytecode` 0.17.0)
grew the `print_i64` → `Console.WriteLine(int32)` lowering and an I/O-aware launcher that
discards (rather than re-prints) the entry result for a printing program. `run_clr` now
returns the captured `Console` output as well as the parsed integer, so `assert_cell`
compares the one the program's `Expect` cares about (stdout for BASIC, exit-style int for
the expression languages). `Clr` is added to BASIC's `Prog`.

Verified by RUNNING on CoreCLR (dotnet 9.0.x): BASIC `10 PRINT 42` → `Console` `42`
(printed exactly once — no double-print from the launcher). **26 proven matrix cells**
(was 25); the `conformance`, `jvm_emit`, `wasm_emit` and `iir-to-cil-bytecode` suites all
still green. CLR column now green for every language except the deferred Brainfuck.

## 0.63.0 — 2026-06-12 — Expression languages join the CLR column (LANG-MATRIX LM-C)

`lang_matrix.rs` opens the **CLR** column on the **real CoreCLR** for the four
expression languages — Twig / Nib / Oct / ALGOL 60. New `Backend::Clr` runner: source →
textual `.il` (`compile_source_to_cil_text`) → real `ilasm` (`-exe`) → real `dotnet` →
parse the integer the entry `Console.WriteLine`s (the CLR-real path of the McCarthy
chapter, generalized). Gated on `dotnet` + a locatable `ilasm` (reuses
`clr_support::find_ilasm`'s NuGet-cache walk); skips gracefully when absent.

Two gaps surfaced **by running** on real `ilasm`/`dotnet`:
* **CIL backend missing arithmetic + comparisons.** `iir-to-cil-bytecode` rejected
  `add` (Nib), `cmp_eq` (Oct) and `mod` (ALGOL) — McCarthy only ever emitted a constant.
  Grown in `iir-to-cil-bytecode` 0.16.0 (CIL `add`/`sub`/`mul`/`div`/`rem` + `ceq`/`clt`/`cgt`).
* **`concretize_scalar_any_for_cil` parameter bug** (the CLR twin of the JVM fix): the
  pass retyped a scalar function's return + body to `i32` but left its **parameters**
  `i64`, so Nib's `double(x)` emitted the inconsistent CIL signature `int32(int64)` that
  CoreCLR's verifier rejects. The pass now concretizes parameter types too.

Verified by RUNNING on CoreCLR (dotnet 9.0.x): Twig→42, Nib→42, Oct→0, ALGOL `17 mod 5`→2.
**25 proven matrix cells** (was 21); the `conformance`, `jvm_emit`, `wasm_emit`,
`iir-to-cil-bytecode` suites all still green. CLR column now green for the expression
languages; BASIC pends a `Console`-writing `print_i64`, Brainfuck the tape ops.

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

## 0.61.0 — 2026-06-11 — Expression languages join the JVM column (LANG-MATRIX LM-J)

`lang_matrix.rs` opens the **JVM** column on **real `java`** for the four expression
languages — Twig / Nib / Oct / ALGOL 60. New `Backend::Jvm` runner: source →
`compile_source_to_jvm_class` → the W16 wrapper-launcher (inject a
`main([Ljava/lang/String;)V` that invokes the entry `main()I` and
`System.out.println`s its `int`) → real `java` → parse the printed integer. Gated on
`java`, skipping gracefully when absent (mirrors the LLVM column's `clang` gate).

**Backend fix found by running on real `java`:** `concretize_scalar_any_for_jvm`
retyped a scalar function's return type and instruction hints to `i32` but left its
**parameter** types `i64`. Nib's `double(x)` therefore emitted the inconsistent method
signature `(J)I` with an `int`-computing body, which real `java` rejects with
`VerifyError: Expecting to find integer on stack` (the laxer in-repo `jvm-simulator`
used by `jvm_emit.rs` never caught it). The pass now concretizes parameter types too —
a reusable correctness fix, not a Nib-specific hack, and safe because lisp/`any`-param
functions are already skipped by the `uses_lisp` guard.

Verified by RUNNING on real `java` (OpenJDK 21): Twig→42, Nib→42, Oct→0,
ALGOL `17 mod 5`→2. **20 proven matrix cells** (was 16). `jvm_emit` (simulator floor),
`conformance` (McCarthy W16) and `wasm_emit` suites all still green. JVM column now green
for the expression languages; BASIC pends an `env.BasicRuntime` host class (its
`print_i64` → `invokestatic env/BasicRuntime.println(J)V`), Brainfuck the tape ops.

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

## 0.59.0 — 2026-06-11 — Nib joins the WASM column (LANG-MATRIX LM-W Nib)

`lang_matrix.rs` greens **Nib on WASM** (the `Nib` program now lists `Wasm`). The fix is
in `nib-iir-compiler` 0.8.0 (it finishes materializing Nib integer types to `i64` — the
const literals and the un-annotated-literal fallback, which previously stayed `u8` and
trapped the strict WASM backend). Verified by RUNNING: Nib `double(21)` → 42 on the
in-process `wasm-runtime`, still → 42 on LLVM and native. WASM column now green for
Twig / Nib / Oct / ALGOL 60; BASIC (print import) and Brainfuck (tape ops) remain.
**15 proven matrix cells.**

## 0.58.0 — 2026-06-11 — WASM column for the expression languages (LANG-MATRIX LM-W)

`lang_matrix.rs` gains a `Backend::Wasm` runner: source → wasm bytes (`iir-to-wasm`)
→ the in-process `wasm-runtime`, with `main`'s wasm result read as the exit value
(in-process, so no host gate — a WASM-tagged program that stops running fails the
floor test loudly). Verified by RUNNING — the expression languages run on WASM:
Twig→42, Oct→0, ALGOL `17 mod 5`→2. **14 proven matrix cells** (6 native + 5 LLVM +
3 WASM).

The other three WASM cells are scoped as follow-ups (gaps found by running): **Nib**
traps (`type mismatch: expected i64, got I32(21)`) — the const literal still emits
`i32` while the now-i64 param wants i64 (LLVM tolerated this via param-typed calls;
`iir-to-wasm` is stricter); **BASIC** needs the `PRINT` env import wired into the
runtime (`no body for function 0`); **Brainfuck** is deferred — `iir-to-wasm` lacks
the tape ops (`UnsupportedOp: alloc_bytes`), the same class as Brainfuck-on-LLVM.

## 0.57.0 — 2026-06-11 — Dartmouth BASIC on LLVM (stdout path) (LANG-MATRIX LM-L BASIC)

Greens **Dartmouth BASIC on LLVM**, the first I/O language in the LLVM column. The
`lang_matrix` `run_llvm` runner grew a stdout path: a BASIC program's `.ll` emits
`call void @__print_i64(i64 42)`, so when the emitted `.ll` references `@__print_i64`
the runner compiles in a tiny **generic** `__print_i64` C runtime (the LLVM analog of
wasm's `env.__print_i64` / JVM `BasicRuntime.println(J)V` / CLR `Console.WriteLine`)
and the harness compares the program's **stdout**. Verified by RUNNING: `10 PRINT 42`
→ stdout `42` on real `clang`. The LLVM column is now green for Twig / Nib / Oct /
ALGOL 60 / BASIC; only **Brainfuck** remains — it needs the tape ops
(`alloc_bytes`/`load_byte`/`store_byte` + `putchar`) added to `iir-to-llvm`, a backend
codegen slice. (Test-only change; the print runtime links only when a program
actually prints, so the bare expression-language cells are unaffected.)

## 0.56.0 — 2026-06-11 — Nib joins the LLVM column (LANG-MATRIX LM-L Nib)

`lang_matrix.rs` greens **Nib on LLVM** — the `Nib` program now lists `Llvm` among its
proven backends. The fix is in `nib-iir-compiler` 0.7.0 (it materialises integer types
to `i64` uniformly so the function signature and instruction bodies agree, which
`iir-to-llvm` requires). Verified by RUNNING: Nib `double(21)` → 42 on real `clang`,
and still → 42 on native AOT. The LLVM column is now green for Twig / Nib / Oct /
ALGOL 60; Brainfuck/BASIC pend the stdout-capturing LLVM I/O runner.

## 0.55.0 — 2026-06-11 — LLVM column for the expression languages (LANG-MATRIX Phase L)

`tests/lang_matrix.rs` refactored into a `Backend`-keyed grid: every `Prog` lists the
backends a slice has **proven** run it, and `matrix_every_proven_cell_agrees` runs
each proven cell on its real toolchain and asserts the known result (a
`proven_columns_do_not_silently_skip` floor catches a tool-present cell that stops
running). Added a `clang`-gated **LLVM runner** (source → textual `.ll` via
`iir-to-llvm` → real `clang` → run). Verified by RUNNING — the expression languages
run on real LLVM: Twig→42, Oct→0, ALGOL `17 mod 5`→2 (no C runtime needed; they link a
bare `.ll`).

Two LLVM cells deferred to focused follow-up slices, with the gaps recorded by
running: **Nib** hits a real `iir-to-llvm` bug (`u8` operands mis-widened to `i64` —
`'%x' defined with type 'i8' but expected 'i64'` on `add`; native AOT runs Nib fine,
so the IIR is sound and only LLVM mishandles the narrow type), and **Brainfuck/BASIC**
need the stdout-capturing LLVM runner that links the C I/O runtime.

## 0.54.0 — 2026-06-11 — cross-language platform-matrix harness (LANG-MATRIX LM0)

First slice of the `LANG-PLATFORM-MATRIX` campaign (every language on every non-BEAM
backend). New `tests/lang_matrix.rs`: a per-language program battery (`Expect::Exit`
for the expression languages Twig/Nib/Oct/ALGOL, `Expect::Stdout` for the I/O
languages Brainfuck/BASIC) and a host-gated **native-AOT** runner. Proven by RUNNING
— all six non-Lisp languages compile to a host executable and run with the expected
result: Twig→42, Nib `double(21)`→42, Oct→0, ALGOL `17 mod 5`→2, Brainfuck→stdout `A`,
Dartmouth BASIC `PRINT 42`→stdout `42`. native-AOT is now a uniformly-green floor for
the matrix. Also fixed the stale `Language` enum doc comments (DartmouthBasic/Oct were
wrongly labelled "placeholder / no Rust frontend"; all six are wired into
`compile_source_to_iir`).

Ground truth the probe established (and recorded in the spec): the **VM and JIT are
McCarthy-specialized** — `mccarthy_lisp_vm::run` rejects ordinary `add`/`mul`/`cmp_*`/
`mod` and the I/O ops, so those two columns are real op-coverage work, while the
code-gen backends (native/LLVM/WASM/JVM/CLR) are general and need mostly conformance
tests.

## 0.53.0 — 2026-06-11 — wire **real CoreCLR** into the W16 conformance suite (CLR-real C6)

The capstone of the CLR-real verification chapter. `tests/conformance.rs` gains a
ninth backend column, **`CLR-real`**, that runs each McCarthy program on the actual
.NET runtime — textual `.il` → real `ilasm` → real `dotnet` — via the shared
`clr_support` harness. It is gated on `dotnet`+`ilasm` (skips when absent, exactly
like the JVM/BEAM/LLVM columns), so the in-process `clr-simulator` `CLR` column
remains the conformance floor while `CLR-real` proves the **same 19 programs**
agree on real CoreCLR when the toolchain is installed. Locally: 19 programs × 9
backends, all agree. CI (`ci.yml`) now installs `ilasm` whenever .NET is set up — a
`<PackageDownload>` of the RID-specific `runtime.<rid>.Microsoft.NETCore.ILAsm`
runtime pack into the NuGet cache where `find_ilasm()` looks — so the CLR column is
verified on the real runtime rather than only the simulator. This completes
**C1–C6: the CLR backend is verified on real CoreCLR.**

## 0.52.0 — 2026-06-11 — McCarthy **lambda / LABEL / recursion** on real CoreCLR (CLR-real C5)

`compile_source_to_cil_text` now compiles multi-function McCarthy programs (via
`iir-to-cil-bytecode` 0.15.0 — multi-method emission, by-name `call`, `ldarg`
params, `castclass object[]` for object-typed field operands, `is_null`). New e2e
`tests/clr_real_lambda.rs` runs them on **real CoreCLR**: `((LAMBDA (X) X) 5)`→5,
`((LAMBDA (X) (CAR X)) (CONS 7 9))`→7, `((LAMBDA (X Y) (EQ X Y)) 3 3)`→1, a
COND-body lambda→100, and a recursive `LABEL` descending CARs→7 — plus symbol/
cons/scalar regression. Gated on `dotnet`+`ilasm`; skips when absent. This closes
the McCarthy F1–F7 feature set on the CLR's real runtime.

## 0.51.0 — 2026-06-11 — McCarthy **symbols** on real CoreCLR (CLR-real C4)

`compile_source_to_cil_text` now exercised on McCarthy symbol programs (via
`iir-to-cil-bytecode` 0.14.0 — no new ops; `intern_symbols_structural` turns each
`(QUOTE S)` into a tagged-int atom that reuses the existing const/box/equal? path).
New e2e `tests/clr_real_symbols.rs` runs them on **real CoreCLR**:
`(EQ (QUOTE A) (QUOTE A))`→1, `(EQ (QUOTE A) (QUOTE B))`→0, `(ATOM (QUOTE A))`→1,
`(EQ (QUOTE FOO) (QUOTE FOO))`→1, `(EQ (QUOTE FOO) (QUOTE BAR))`→0, plus int-EQ /
cons regression. Gated on `dotnet`+`ilasm`; skips when absent.

## 0.50.0 — 2026-06-11 — McCarthy **predicates + COND** on real CoreCLR (CLR-real C3)

`compile_source_to_cil_text` now compiles McCarthy predicate and `COND` programs
(via `iir-to-cil-bytecode` 0.13.0). New e2e `tests/clr_real_predicates.rs` runs them
on **real CoreCLR** (real `ilasm` → PE → real `dotnet`, via the shared
`tests/clr_support` harness): `(ATOM 7)`→1, `(ATOM (CONS 1 2))`→0, `(EQ 7 7)`→1,
`(EQ 7 8)`→0, `(COND ((ATOM 7) 11) ((ATOM 8) 22))`→11, and
`(COND ((ATOM (CONS 1 2)) 11) ((EQ 5 5) 22))`→22 (a false first clause falling
through to a matching `EQ`). Gated on `dotnet`+`ilasm`; skips when absent.

## 0.49.0 — 2026-06-11 — McCarthy **cons/car/cdr** on real CoreCLR (CLR-real C2)

`compile_source_to_cil_text` now compiles cons programs (via `iir-to-cil-bytecode`
0.12.0). The `ilasm`/`dotnet` e2e harness is extracted to `tests/clr_support/mod.rs`
(shared by all CLR-real test files) with a robust `find_ilasm` that searches every
`*ilasm*` NuGet package — the binary ships only in the `runtime.<rid>.*` pack, not
the ref-only `microsoft.netcore.ilasm`, so picking the first match was fragile. New
e2e `tests/clr_real_cons.rs`: `(CAR (CONS 7 9))`→7, `(CDR …)`→9, nested→2 on **real
CoreCLR**; `clr_real_scalar.rs` refactored onto the shared harness.

## 0.48.0 — 2026-06-11 — McCarthy on **real CoreCLR** (scalar) — CLR real-runtime verification (CLR-real C1)

New `compile_source_to_cil_text(language, source, name) -> String`: the CLR analog
of `compile_source_to_llvm`. Where `compile_source_to_cil_artifact` yields raw method
bodies for the in-repo `clr-simulator`, this emits textual `.il` (via
`iir-to-cil-bytecode` 0.11.0) that real `ilasm` assembles into a loadable PE running
on real `dotnet`. New e2e test `tests/clr_real_scalar.rs` (gated on `dotnet`+`ilasm`,
skips when absent): `42`→42, `0`→0, `7`→7 on **real CoreCLR**. First slice of bringing
the CLR backend up to the same real-runtime verification bar as JVM/BEAM/LLVM/native.

## 0.47.0 — 2026-06-11 — **McCarthy Lisp arc COMPLETE** — L6: byte-identical to Twig on every historical-arch backend

The closing PR of the McCarthy Lisp arc.  One table-driven test
asserts that the integer-literal program `7`, compiled from **both**
Twig and McCarthy Lisp source, emits **byte-for-byte identical
machine code** on all six historical-arch backends — GE-225,
Intel 4004, Intel 8008, ARMv7, RV32I, IBM 704.

The IIR convergence proof on the historical lanes: one IR layer
(IIR/CIR/Backend trait), two surface languages (typed Twig vs.
dynamic-typed McCarthy 1960), six machine architectures spanning
71 years (1954 vacuum-tube IBM 704 → 2025 ARMv7), six bit-identical
outputs.

### What changed

* `lang-aot` v0.47.0.
* New `tests/historical_round_trip.rs`:
  - One `#[test] fn mccarthy_byte_identical_to_twig_on_every_historical_arch`.
  - Table of 6 backends; for each, compile `7` from `.twig` and
    `.mcl` sources to a temp dir, read both `.bin` outputs,
    `assert_eq!` the bytes.
  - Floor: every backend must be exercised.  Anything less is a
    regression.

### Why `7` and not the canonical `42`?

The Intel 4004 (1971) is a *4-bit* microprocessor; its
immediate-load instruction (`LDM`) holds a value in `[0, 15]`.
The canonical `42` literal that the IBM 704 / GE-225 / Intel 8008
/ ARMv7 / RV32I per-arch e2e tests use overflows that 4-bit
window, so the Intel 4004 backend rejects it on **both** languages
(consistent refusal, but trivially so).  Choosing `7` lets us
exercise all six backends with a single non-trivial byte sequence,
strengthening the convergence claim to "all 6 historical-arch
lanes agree byte-for-byte" rather than "5 agree, 1 jointly
refuses."

### Specs

`code/specs/MCCARTHY-LISP-PLAN.md` updated: **L6 ✓ — McCARTHY LISP
ARC COMPLETE**.

### McCarthy Lisp arc — full summary

| Phase | What | Status |
|-------|------|--------|
| L1 | mccarthy-lisp-lexer + mccarthy-lisp-parser | ✓ |
| L2a/b/c | IIR compiler + mccarthy-lisp-vm + closures | ✓ |
| L3a | Language::McCarthyLisp in lang-aot | ✓ |
| L3b-1..2c | Native tagged-word cons/symbol/predicate via lispy_runtime.c | ✓ |
| L3b-3 / W1–W11 | wasm + JVM + CLR + BEAM (all 7 features) | ✓ |
| W12–W13 | LLVM (all 7 features) | ✓ |
| W14a/b | Native AOT macOS Mach-O link gap + lambda | ✓ |
| W15a/b | Universal JIT (all 7 features) | ✓ |
| W16 | Cross-backend conformance suite (8 modern × 19 programs) | ✓ |
| L4 + L5 | IBM 704 encoder + backend + --emit=ibm704 wiring | ✓ |
| L7 | Metacircular evaluator on every modern backend | ✓ |
| **L6** | **Cross-backend byte-pinned matrix on all 6 historical-arch backends** | **✓ (this release)** |

**McCarthy Lisp now runs on 14 backends — 8 modern (VM, JIT, WASM,
CLR, JVM, BEAM, LLVM, native AOT) and 6 historical-arch (GE-225,
Intel 4004, Intel 8008, ARMv7, RV32I, IBM 704) — through one shared
IIR/CIR layer.**

## 0.46.0 — 2026-06-11 — **McCarthy's 1960 metacircular evaluator on every modern backend** — L7

McCarthy's 1960 paper closes with one of the most elegant ideas in
computing history: Lisp's `EVAL` function, written in Lisp itself.
Once you have the seven primitives (`QUOTE`, `ATOM`, `EQ`, `CAR`,
`CDR`, `CONS`, `COND`) plus `LAMBDA` and `LABEL`, you can express an
interpreter for the very language you're writing in.  That's the
**metacircular evaluator** — Lisp-in-Lisp.

This release adds exactly that test: McCarthy's 1960 `EVAL` authored
as McCarthy Lisp source (`(LABEL EVAL (LAMBDA (E) (COND ...)))`),
applied to 9 test programs, run through every modern backend.

### What changed

* New `tests/metacircular.rs` with:
  - `EVALUATOR_BODY` — McCarthy 1960 `EVAL` as McCarthy Lisp source.
  - `PROGRAMS` — 9 test inputs `(input-source, expected-integer)`:
    `42`, `0`, `(QUOTE 7)`, `(CAR (CONS 7 9))`, `(CDR (CONS 7 9))`,
    `(ATOM 7)`, `(EQ 5 5)`, `(EQ 5 6)`, `(CAR (CDR (CONS 1 (CONS 2 3))))`.
  - `metacircular_smoke_vm_evaluates_canonical_programs` — pure-VM
    smoke that validates `EVALUATOR_BODY` against every input.
  - `metacircular_eval_uniform_across_modern_backends` — runs the
    metacircular evaluator on the 8 modern backends, asserts integer
    agreement everywhere a result comes back.

### Conformance floor

| Backend | Floor? | Notes |
|---------|--------|-------|
| VM      | YES    | the reference interpreter (`mccarthy-lisp-vm`) |
| JIT     | YES    | `jit-core::GenericCirJit` |
| WASM    | YES    | in-repo `wasm-runtime` |
| CLR     | opt-in | `clr-simulator` panics on opcode `0x38` (long-form `br`) when the metacircular evaluator's nested-COND CIL exceeds short-branch range — documented simulator gap; the emitted CIL is valid (a real CLR loads it fine).  Caught via `catch_unwind`; once `clr-simulator` adds long-form-branch support this becomes a no-op. |
| JVM     | tool-gated on `java`                          | |
| BEAM    | tool-gated on `erl`                           | |
| LLVM    | tool-gated on `clang` + `lispy_runtime.c`     | |
| native  | macOS-only                                    | |

### Why this matters

After L1–L4 and the W1–W16 cascade, McCarthy Lisp runs on eight
backends through one shared IIR.  The W16 conformance suite proves
they agree on a hand-written program table.  **L7 raises the bar**:
the test programs become "anything the metacircular evaluator can
interpret," and the interpreter itself is a single McCarthy program
exercising COND, recursion, every primitive against itself.  Every
floor-required backend (VM/JIT/WASM) produces the identical integer
output across all 9 inputs.

### Specs

`code/specs/MCCARTHY-LISP-PLAN.md` updated — L7 marked ✓.

## 0.45.0 — 2026-06-11 — **McCarthy Lisp on the silicon Lisp was born on** — L4 + L5: `--emit=ibm704`

The closing half of the **CAR/CDR birthplace round-trip**.  `CAR` and
`CDR` — the two universal Lisp accessors McCarthy introduced in 1958 —
were *literally* IBM 704 instruction-word field mnemonics
(**C**ontents of the **A**ddress / **D**ecrement part of **R**egister).
The IBM 704 (1954) is the vacuum-tube mainframe McCarthy and his MIT
students Steve Russell, Tim Hart, and Mike Levin first ran Lisp on, in
1959.  This release lets McCarthy Lisp source compile back to that
silicon — the symmetric counterpart of the **Dartmouth BASIC → GE-225**
round-trip the historical-arch migration already established.

### What changed

* New `EmitMode::Ibm704Bin` variant + CLI `--emit=ibm704` (aliases
  `ibm-704`, `704`).
* New API `lang_aot::compile_file_to_ibm704_bin(src, out, language)`
  that routes source through the standard `aot_core::infer` +
  `aot_core::specialise` + `Backend::compile` pipeline against the new
  `ibm704-backend` v0.1.0 crate.
* New error variant `LangAotError::Ibm704BackendError(String)` carrying
  the backend's `BackendError` Display.
* New dev-deps: pulls `ibm704-backend` v0.1.0 + `ibm704-encoder` v0.1.0
  from the workspace.

### Wire format

Each 36-bit IBM 704 word is packed as **5 bytes**, low byte first.
The top 4 bits of the high byte are always zero — the 40-bit byte
window has 4 padding bits, which `ibm704-encoder::pack_word`
zeroes by construction.  Same convention `ge225-encoder` uses
(20-bit words → 3 bytes) extended to 36 bits.

### Pinned byte sequences

* **Twig `42`** → `[CLA 42; HTR 0]` =
  `[0xA_0000_002A, 0x8_8000_0000]` = the canonical 10 bytes
  `[0x2A, 0x00, 0x00, 0x00, 0x0A, 0x00, 0x00, 0x00, 0x80, 0x08]`.
* **McCarthy `42`** → byte-for-byte identical — the IIR
  convergence in action.  One source language, one
  `Language::McCarthyLisp` arm, one shared IIR, one shared
  backend, one machine-code sequence.

### v0.1.0 scope — minimal viable

Per the McCarthy Lisp v0.1.0 scope decision (confirmed in
`MCCARTHY-LISP-PLAN.md`), the IBM 704 backend handles only
`const_*` + `ret_*` + `ret_void`.  CONS-using programs are out
of scope for *every* historical-arch backend in v0.1.0; that
hasn't changed, and the IBM 704 follows the same convention as
GE-225 / Intel 4004 / Intel 8008 / ARMv7 / RV32I.

### Tests

Two new e2e tests in `tests/end_to_end_smoke.rs`:
* `end_to_end_twig_42_emits_ibm704_bin_via_lang_aot` — pins the 10
  bytes byte-for-byte.
* `end_to_end_mccarthy_42_emits_ibm704_bin_via_lang_aot` — pins the
  same 10 bytes, demonstrating the IIR convergence.

### Specs

New: `code/specs/ibm704-encoder.md`, `code/specs/ibm704-backend.md`.
`code/specs/MCCARTHY-LISP-PLAN.md` updated to mark L4 + L5 as ✓.

## 0.44.0 — 2026-06-10 — McCarthy **cross-backend conformance suite** — **THE PLATFORM MATRIX IS COMPLETE** (LANG77 / W16)

New `tests/conformance.rs`: one shared table of **19** McCarthy programs (F1–F7)
run through **all eight backends** — VM (`mccarthy_lisp_vm`), JIT
(`run_mccarthy_on_jit`), WASM (`wasm-runtime`), CLR (`clr-simulator`), JVM (real
`java`), BEAM (real `erl`), LLVM (`clang` + `lispy_runtime.c`), native AOT (system
`ld`) — each asserting the **identical** integer result. The four pure-in-process
backends (VM/JIT/WASM/CLR) always run (the conformance floor); the external-tool
backends skip gracefully when their tool is absent, so CI proves uniformity across
whatever is installed. On a fully-equipped host all eight agree on all 19 programs.
New dev-deps `mccarthy-lisp-vm` + `iir-to-jvm-class-file`. **This is the capstone:
one McCarthy source, eight independent code generators, three value models, one
answer — the proof the platform is complete and uniform.**

## 0.43.0 — 2026-06-10 — McCarthy **JIT lambda/LABEL** (F7) — **JIT COMPLETE; all eight backends done** (LANG77 / W15b)

`jit_lisp` registers `lispy_to_exit_code` (the polymorphic lambda-result exit
coercion — a runtime tag dispatch derived from `LispyValue`'s predicates, the only
builtin lambda needs beyond W15a's set). Together with the `vm-core` 0.3.0
register-sizing fix, McCarthy `LAMBDA`/`LABEL`/recursion now run on the universal
JIT. Verified by RUNNING (`tests/jit_mccarthy.rs`): `((LAMBDA (X) X) 5)`→5,
`((LAMBDA (X) (CAR X)) (CONS 7 9))`→7, `((LAMBDA (X Y) (EQ X Y)) 3 3)`→1,
lambda-with-`COND`-body→100/200, recursive `LABEL`→7. **The JIT is the eighth and
final backend — McCarthy 1960 LISP now runs on every LANG VM backend (F1–F7).**

## 0.42.0 — 2026-06-10 — McCarthy on the **universal JIT** (F1–F6) (LANG77 / W15a)

Adds `run_mccarthy_on_jit(source)` + the `jit_lisp` module: McCarthy now runs on
`jit-core`'s `GenericCirJit` — the eighth and final backend. The JIT dispatches
`call_builtin "lispy_*"` to Rust callbacks (not native `__twig_lispy_*` calls like
the AOT/LLVM path), so the lisp ops are registered against the shared `lispy-runtime`
crate (the C runtime's Rust twin — identical `u64` tagged-word model). A `LispyValue`
rides inside `Value::Int` as its bit pattern; `unbox_int`/`truthy` are derived from
`LispyValue::as_int`/`is_truthy` (existing primitives, not duplicated). New deps:
`vm-core`, `lispy-runtime`. New error variant `LangAotError::JitBackendError`.
Verified by RUNNING (`tests/jit_mccarthy.rs`): `(CAR (CONS 7 9))`→7, `(ATOM 7)`→1,
`(EQ 7 7)`→1, nested `COND`→44, `(EQ (QUOTE A) (QUOTE A))`→1. Lambda (F7) is W15b —
the VM's user-`call` path needs work first.

## 0.41.0 — 2026-06-10 — McCarthy **native AOT lambda** (F7) — **NATIVE AOT COMPLETE F1–F7** (LANG77 / W14b)

No `lang-aot` source change — the fix is in `aarch64-backend` 0.9.0 + `x86_64-backend`
0.11.0 (`lispy_to_exit_code` added to `V1_BUILTINS`), which the native path already
drives. `tests/macos_native_lisp.rs` gains `mccarthy_lambda_runs_natively_on_macos`:
`((LAMBDA (X) X) 5)`→5, `((LAMBDA (X) (CAR X)) (CONS 7 9))`→7,
`((LAMBDA (X Y) (EQ X Y)) 3 3)`→1, `((LAMBDA (X) (ATOM X)) 7)`→1,
lambda-with-`COND`-body→100/200. **Native AOT is now McCarthy-complete (F1–F7)** — the
seventh backend, after VM/WASM/JVM/CLR/BEAM/LLVM. Only the JIT (W15) remains.

## 0.40.0 — 2026-06-10 — McCarthy **native AOT** (F2–F6) now links + runs on macOS arm64 (LANG77 / W14a)

No `lang-aot` source change — the fix is in `code-packager` 0.5.0 (Mach-O external
symbols now carry the leading `_` C decoration), which the native macOS path already
drives. New verify-by-running test `tests/macos_native_lisp.rs` (gated to
`target_os = "macos"`): compiles McCarthy through the native `aarch64-backend`,
links the runtime archive with `ld`, and runs — `(CAR (CONS 7 9))`→7, `(CDR …)`→9,
`(ATOM 7)`→1, `(EQ 7 7)`→1, `(COND …)`→11, `(EQ (QUOTE A) (QUOTE A))`→1. Closes the
macOS runtime-link gap that previously failed native lisp at link time. Lambda (F7)
is still backend-refused — the separate W14b slice.

## 0.39.0 — 2026-06-10 — McCarthy → **LLVM lambda** (F7) on a clang-built executable — **LLVM COMPLETE F1–F7** (LANG77 / W13b)

No `lang-aot` source change — the work is in `iir-builtin-lowering` 0.16.0 (lambda
arg boxing + polymorphic result coercion), `twig-aot`'s `lispy_runtime.c` 0.14.0
(the `__twig_lispy_to_exit_code` runtime switch) and `iir-to-llvm` 0.8.0 (declaring
it), all of which `compile_source_to_llvm` already drives. New verify-by-running
test `tests/llvm_lambda.rs` (link `lispy_runtime.c` with clang, run):
`((LAMBDA (X) X) 5)`→5, `((LAMBDA (X) (CAR X)) (CONS 7 9))`→7,
`((LAMBDA (X Y) (EQ X Y)) 3 3)`→1, `((LAMBDA (X) (ATOM X)) 7)`→1,
lambda-with-`COND`-body→100/200. **LLVM is now McCarthy-complete (F1–F7)** — the
sixth backend to finish, after VM/WASM/JVM/CLR/BEAM.

## 0.38.0 — 2026-06-10 — McCarthy → **LLVM symbols** (F6) on a clang-built executable (LANG77 / W13a)

No `lang-aot` source change — the work is in `iir-to-llvm` 0.7.0 (`symbol`→`i64`) and
`iir-builtin-lowering` 0.15.0 (symbol-result returned verbatim), which
`compile_source_to_llvm` already drives. New verify-by-running test
`tests/llvm_symbols.rs` (link `lispy_runtime.c` with clang, run):
`(EQ (QUOTE A) (QUOTE A))`→1, `(EQ (QUOTE A) (QUOTE B))`→0, `(ATOM (QUOTE A))`→1,
symbol-in-`COND`→11, `(QUOTE A)`→its tagged word. LLVM is now F1–F6; only lambda
(F7, W13b) remains.

## 0.37.0 — 2026-06-10 — McCarthy → **LLVM `COND`** (F5) on a clang-built executable — LLVM core F1–F5 (LANG77 / W12b-3)

No `lang-aot` source change — the work is in `iir-to-llvm` 0.6.0 (cross-block
SSA-merge via stack-slot/`alloca` promotion, plus the `jmp_if` void-cond and
empty-block fallthrough fixes), which `compile_source_to_llvm` already drives. New
verify-by-running test `tests/llvm_cond.rs` (link `lispy_runtime.c` with clang,
run): `(COND ((ATOM 7) 11) ((ATOM 8) 22))`→11, second-clause→22, nested `COND`→44;
cons/predicate/scalar all still pass. **LLVM is now F1–F5 (only symbols+lambda,
W13, remain).**

## 0.36.1 — 2026-06-10 — McCarthy → **LLVM predicates** ATOM/EQ (F3–F4) on a clang-built executable (LANG77 / W12b-2)

No `lang-aot` source change — the fix is in the shared `iir-builtin-lowering`
`lower_lisp_repr` (0.14.0), which `compile_source_to_llvm` already runs: a boolean
program result (a predicate) is now coerced with `lispy_truthy` (→ raw `0`/`1`)
instead of `lispy_unbox_int` (which gave `0` for *true*). New verify-by-running
test `tests/llvm_predicates.rs` (link `lispy_runtime.c` with clang, run):
`(ATOM 7)`→1, `(ATOM (CONS 1 2))`→0, `(EQ 7 7)`→1, `(EQ 7 8)`→0. (`COND`, F5,
needs PHI-node merge of clause values across blocks — W12b-3.)

## 0.36.0 — 2026-06-10 — McCarthy → **LLVM cons** (F2) on a clang-built executable (LANG77 / W12b-1)

`compile_source_to_llvm` now runs the **native tagged-word lisp pipeline** — the
SAME passes the native AOT path runs, NOT the managed structural pass:
`lower_heap_builtins_runtime` (cons/car/cdr → `call_builtin "lispy_*"`) →
`intern_symbols` → `lower_lisp_repr` (boxes int literals to tagged words, inserts the
final `lispy_unbox_int` so the result is a plain `i64`). `iir-to-llvm` (0.5.0) lowers
each `lispy_*` to `call @__twig_lispy_*`. A pure-scalar program never enters those
passes (then `concretize_scalar_any_for_llvm` handles `any`→`i64` as before).

**Verified by RUNNING** (`tests/llvm_cons.rs`): emit host-triple IR, **link
`twig-aot/runtime/lispy_runtime.c`** with `clang` (`-x ir <ours> -x none <runtime.c>`),
run the native executable — exit code = result: `(CAR (CONS 7 9))`→7,
`(CDR (CONS 7 9))`→9, `(CAR (CDR (CONS 1 (CONS 2 3))))`→2, scalar `42`→42 (no
regression). Predicates (pair?/equal?/not, COND — F3–F5) are W12b-2 (their
tagged-boolean result needs its own handling).

## 0.35.0 — 2026-06-10 — McCarthy → **LLVM** scalar run-foundation via `clang` (LANG77 / W12a)

Establishes the LLVM **verify-by-running** substrate — the first **tagged-word**
target (the LLVM/AOT/JIT family that links the shared `lispy_runtime.c`). New
`compile_source_to_llvm` / `compile_source_to_llvm_with_target`: concretize scalar
`any`→`i64`, lower to LLVM IR text. The new `tests/llvm_scalar.rs` emits **host**-
triple IR (`clang -dumpmachine`), builds it with `clang -x ir`, and **runs** the
native executable — its process exit code carries the McCarthy result: `42`→42,
`7`→7, `0`→0, `100`→100 (+ Twig `42`→42). Uses the `clang` already on the box (no
extra toolchain; self-skips if absent) — the LLVM analogue of `wasm-runtime` /
`clr-simulator` / real `erl`. The cons/predicate/symbol/lambda lowering
(`call __twig_lispy_*`) is W12b+.

## 0.34.0 — 2026-06-10 — McCarthy → **BEAM symbols + lambda** — BEAM backend COMPLETE (LANG77 / W11, F6–F7)

Symbols (F6) and lambda (F7) run on the BEAM — **completing the entire BEAM
backend (F1–F7)**, the FIFTH backend to reach full McCarthy support (after VM,
WASM, JVM, CLR). One-line pipeline addition: `compile_source_to_beam` now runs
`intern_symbols_structural`, so each symbol interns to a stable `i32` id
(`SYMBOL_ID_BASE = 1<<29`) — the SAME id the wasm/JVM/CLR backends assign — which
the BEAM carries as a native Erlang integer (`EQ` → `is_eq_exact`). **Lambda
needed nothing extra** — a `(LAMBDA …)` application is a method `call`, which
`iir-to-beam` already lowers natively (a BEAM fun). Verified by RUNNING on a real
`erl`: `(QUOTE A)`→536870912, `(EQ (QUOTE A) (QUOTE A))`→1,
`((LAMBDA (X) (CAR X)) (CONS 7 9))`→7, `((LAMBDA (X) (EQ X (QUOTE A))) (QUOTE A))`→1.
New `tests/beam_symbols_lambda.rs`.

## 0.33.0 — 2026-06-10 — McCarthy → **BEAM ATOM/EQ/COND** on a real `erl` (LANG77 / W10, F3–F5)

McCarthy's predicates run on the BEAM (`iir-to-beam` 0.5.0 lowers `pair?`→
`is_nonempty_list`, `equal?`→`is_eq_exact`, `not`→`x==0`; `COND`→`jmp_if`). The
`compile_source_to_beam` pipeline is unchanged — the predicates flow through the
existing `lower_heap_builtins` + concretize path. New `tests/beam_predicates.rs`:
`(ATOM 7)`→1, `(ATOM (CONS 1 2))`→0, `(EQ 7 7)`→1, `(COND …)`→100/200, on a real `erl`.

## 0.32.0 — 2026-06-10 — McCarthy → **BEAM cons** on a real `erl` (LANG77 / W9b, F2)

McCarthy cons runs on the BEAM (Erlang VM) — using the **native Erlang-terms**
model, NOT the boxing structural pass the managed backends use. A cons cell is a
native list cell `[H|T]`; `car`/`cdr` are `hd`/`tl`; integers are native. Two
pipeline changes in `compile_source_to_beam`:
- Run `lower_heap_builtins` so `cons`/`car`/`cdr` become `alloc ref<LispyPair>` +
  `field_store`/`field_load`, which `iir-to-beam` already maps to `put_list` /
  `get_hd` / `get_tl`.
- Generalize `concretize_scalar_any_for_beam` to concretize `any`→`i64`
  **per-instruction in every function** (BEAM is dynamically typed; `i64` is the
  universal native-term placeholder), leaving `ref<LispyPair>` cons cells for the
  list lowering — previously it skipped any heap-using function wholesale.

Verified by RUNNING on a real `erl`: `(CAR (CONS 7 9))`→7, `(CDR (CONS 7 9))`→9,
nested→2, and `(CONS 7 9)`→`[7|9]` (a genuine Erlang list cell). New
`tests/beam_cons.rs`. (`iir-to-beam` is unchanged — it already had the list ops.)

## 0.31.0 — 2026-06-10 — McCarthy → **CLR lambda** — CLR backend COMPLETE (LANG77 / W8b, F7)

Lambda (F7) runs on the CLR — **completing the entire CLR backend (F1–F7)**, the
third managed backend to reach full McCarthy support after WASM and JVM.
`(LAMBDA (args…) body)` applied lowers to a CLR method `call`: the structural pass
hoists the lambda into its own method (params → `ldarg.N`), `iir-to-cil-bytecode`
0.10.0 validates+emits `call <MethodDef>` (args boxed, result `ref<any>`), and
`clr-simulator` 0.4.0 executes it via an inter-method **call-frame** model.
`((LAMBDA (X) (CAR X)) (CONS 7 9))`→7, `((LAMBDA (X Y) (EQ X Y)) 3 3)`→1, and
backward-compat (scalar/cons) verified on the simulator. New `tests/cil_lambda.rs`.

## 0.30.0 — 2026-06-10 — McCarthy → **CLR symbols** on the simulator (LANG77 / W8a, F6)

Symbols (F6) run on the CLR with **zero new backend code** — pure structural-pass
reuse. The shared `intern_symbols_structural` pass interns each distinct symbol to
a stable `i32` id in a reserved range (`SYMBOL_ID_BASE = 1 << 29`); W6b boxing +
W7 `equal?`/`pair?`/`jmp_if` then execute `QUOTE`/`EQ`/`ATOM`/`COND` on symbols.
`(QUOTE A)`→536870912, `(EQ (QUOTE A) (QUOTE A))`→1, `(EQ (QUOTE A) (QUOTE B))`→0,
`(ATOM (QUOTE A))`→1, on the `clr-simulator`. New `tests/cil_symbols.rs`. (The CLR
backend itself is unchanged — this release adds the regression test + the F6 tick.)
Remaining for CLR: **W8b lambda (F7)** — `call` lowering + simulator call frames.

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
