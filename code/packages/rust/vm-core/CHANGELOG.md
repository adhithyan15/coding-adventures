# Changelog — vm-core

## [0.18.0] — 2026-07-14 (E6d union `match` on the generic VM — `box` + dynamic builtins)

Builds on 0.17.0 (heap objects): union `match` now runs on the generic VM (and
the JIT via its VM fallback), so E6d union `match` runs on **all seven engines**.

- **`box` / `unbox` opcodes** — the **identity** on the VM: its `Value` is already
  the dynamic value (no separate boxed form), so both are a register copy. The
  union constructor `emit_union_def` emits `box` on the variant tag + fields; that
  op reaches the generic VM, where it is a no-op copy.
- **Dynamic-dispatch builtins `=` / `+` / `-` / `*`** (registered by default) —
  the `any`-typed primitives the frontend emits as `call_builtin` when an
  operand's static type is `any` (a `match`-bound field, a value read from a cons
  cell). `=` is a direct `Value` compare → boolean (the tag test); the arithmetic
  ops compute on same-kind operands (integers wrap on overflow — the i64 tagged
  model; floats in `f64`), with a clean type error otherwise. This also unblocks
  E6d-2 dynamic integer arithmetic on the VM.

Verified: both union `match` cells run on the `Vm` + `Jit` matrix columns (Some →
42, None → 42), plus new `builtins` unit tests (`=`, `+`/`-`/`*` incl. overflow
wrap + arity/type errors). `match` needs no `is_null`, so no nil-handle
disambiguation is required here. Closures (`alloc_closure`/`call_closure`) and list
builtins (`cons`/`car`/…) on the generic VM remain follow-ups.

## [0.17.0] — 2026-07-14 (E6d heap objects — `alloc`/`field_store`/`field_load` on the generic VM)

The dispatcher now executes the word-granular heap ops that Twig
records/unions/closures build their `(car . cdr)` cons cells from — so the E6d
dynamic features, previously runnable only on the five code-gen backends and the
Twig-specific interpreter, now also run on the **generic `vm-core`** engine (and
the JIT via its VM fallback).

- `alloc [<size_bytes>] -> dest` allocates a fixed-size object on the existing
  bounds-checked array heap (`ctx.arrays`), returning its integer handle; a field
  is one 8-byte word, so the element count is `size_bytes / 8` (default 16 bytes =
  a 2-word `LispyPair`, matching the native `__twig_gc_alloc` default). It shares
  the array heap's `max_memory_entries` aggregate cap, so no crafted `alloc` can
  OOM the process.
- `field_store` / `field_load` reuse the `array_set` / `array_get` handlers
  verbatim — identical handle+index model — so they inherit the same bounds
  checking.

First slice: **records** (verified on the `Vm` + `Jit` matrix columns — `point-x`/
`point-y` = 42, and three `vm-core` unit tests). Purely additive: these three
opcodes were previously unsupported (`_ => None`). Unions/`match` (which test the
nil sentinel via `is_null`, and a nil `Int(0)` is presently indistinguishable from
the first-allocated object handle `0`) are a follow-up needing nil-handle
disambiguation; records never dereference nil, so they are sound today.

## [0.16.0] — 2026-06-29 (LANG-FULL BA-pow — `f64_pow` VM dispatch handler)

Added `handle_f64_pow` to the dispatch table.  The handler extracts two source
operands as `Value::Float` (base and exponent), calls Rust's `f64::powf()`
(IEEE-754 `pow` — NaN propagates, negative base with non-integer exp returns NaN),
and writes `Value::Float(result)` to the dest slot.  The JIT falls back to the
VM for all `_f64`-suffix ops via the existing fallback path, so no JIT changes
were needed.
## [0.15.0] — 2026-06-29 (LANG-FULL AL8-arctan — `f64_atan/f64_tan` VM dispatch handlers)

Added two handlers registered in `lookup_standard`:
- `f64_atan` → `handle_f64_atan` using `f64::atan` (inverse tangent; range −π/2 to π/2; f64_atan(0.0) = 0.0 exactly)
- `f64_tan`  → `handle_f64_tan` using `f64::tan` (tangent; f64_tan(0.0) = 0.0 exactly)

Both delegate through the existing `handle_f64_transcendental` helper — one source
operand extracted as `Value::Float`, function applied, result stored.  JIT inherits both
ops via the `lookup_standard` fallback at no extra cost.

## [0.14.0] — 2026-06-28 (LANG-FULL AL8-trig — `f64_sin/cos/ln/exp` VM dispatch handlers)

Added `handle_f64_transcendental` generic helper and four handlers registered in the
dispatch table: `f64_sin` → `f64::sin`, `f64_cos` → `f64::cos`, `f64_ln` → `f64::ln`,
`f64_exp` → `f64::exp`.  Each extracts the first operand as `f64`, applies the function,
and stores the result.  JIT inherits all four via the standard `lookup_standard` fallback.

## [0.13.0] — 2026-06-28 (LANG-FULL AL8-sqrt — `f64_sqrt` VM dispatch handler)

Added `handle_f64_sqrt` to the dispatch table.  The handler extracts the first
source operand as a `Value::Float`, calls Rust's `f64::sqrt()` (IEEE-754
hardware sqrt — NaN propagates, negative input returns NaN), and writes
`Value::Float(result)` to the dest slot.  The JIT falls back to the VM for all
`_f64`-suffix ops via the existing fallback path, so no JIT changes were needed.

## [0.12.0] — 2026-06-28 (LANG-FULL E4 — reference string comparison)

The reference VM now implements `str_cmp`, returning the shared E4 three-way
byte ordering convention: `-1`, `0`, or `1`.

## [0.11.0] — 2026-06-28 (LANG-FULL E4 — reference string slicing)

The reference VM now implements shared E4 `str_slice` semantics over
`Value::Str`: source string plus start/end integer bounds produce a fresh
substring. The operation is byte-indexed, traps on negative, inverted, or
out-of-bounds ranges, and rejects ranges that do not preserve UTF-8 boundaries.

## [0.10.0] — 2026-06-27 (LANG-FULL E4 — reference string ops, VM slice)

Reference VM semantics for the six shared E4 string opcodes:

- **`str_const`** materialises an `Operand::Str` literal as `Value::Str`.
- **`str_len`** returns the byte length.
- **`str_index`** returns an unsigned byte and traps on negative/out-of-range
  indexes.
- **`str_concat`** produces a fresh immutable string.
- **`str_eq`** returns the IIR integer-bool convention (`1` / `0`).
- **`print_str`** writes through the built-in registry's `print_str` sink with
  no implicit newline, so embedders/tests can capture output while the default
  registry writes to stdout.

Verified by `tests/e4_strings.rs`, covering byte length, concat+equality,
indexing, the bounds trap, and sink-routed output capture.

## [0.9.0] — 2026-06-22 (LANG-FULL E8 — numeric conversions, PR-1)

Reference VM semantics for the three `integer`↔`real` conversion opcodes
(spec `code/specs/lang-full-e8-numeric-conversions.md`). E3 gave the VM f64
*arithmetic*; these are the convert opcodes that sit next to it — every backend
has a one-instruction equivalent, and this is the behaviour they must agree
with.

- **`int_to_real`** — `i64` → `f64` (IEEE-754, exact for |x| < 2⁵³).
- **`real_to_int_trunc`** — `f64` → `i64` rounding toward **zero** (C / BASIC
  `INT()`): `2.7 → 2`, `-2.7 → -2`.
- **`real_to_int_floor`** — `f64` → `i64` rounding toward **−∞** (ALGOL
  `entier`): `2.7 → 2`, `-2.7 → -3`.
- Both `real_to_int_*` **trap** (fail-closed, like array-bounds and
  divide-by-zero) on a NaN/±∞ or out-of-`i64`-range operand — never a silent
  wrap. The range check is exact (`i64::MAX` is unrepresentable as `f64`, so the
  bound is `< 2⁶³` via `-(i64::MIN as f64)`).
- 5 unit tests (each direction + rounding-sign + the NaN/∞ and out-of-range
  traps); the JIT tier inherits all three via cold-interpret (proved by
  `jit-core/tests/e8_conversions_jit.rs`, an integer→real→integer round trip).

## [0.8.0] — 2026-06-22 (LANG-FULL E6 layer 1 — typed module globals)

### Added
- **`global_load` / `global_store`** — the VM now executes the *lowered, typed*
  global IIR ops over a new name-keyed `globals: HashMap<String, Value>`:
  - `global_store("g", %v)` writes; `global_load("g") -> %dest` reads.
  - The global name is an `Operand::Str` literal (never resolved as a register).
  - A global that was never written reads as `Int(0)` — matching the zero-init
    the code-gen backends give their `_twig_globals` slots / static fields.
  - This is **distinct** from the dynamic `call_builtin "global_get"/"global_set"`
    table the Twig front-end uses; these typed ops are what a statically-typed
    frontend (e.g. ALGOL procedures over an enclosing-block variable, E6's proof
    program) emits directly — so such globals now run on the VM.
- Because the **JIT** (`JITCore` + `GenericCirJit`) cold-interprets on this VM
  (only hot functions promote, and a global-using function the compiler doesn't
  lower simply stays interpreted), the **JIT column gets globals for free** —
  covered by a `jit-core` integration test.

### Verified
- `tests/e6_globals.rs`: a cross-function program (`main` seeds `g`, a separate
  `bump` reads/increments/writes it) ⇒ 42; an unwritten global reads as 0.
- `jit-core/tests/e6_globals_jit.rs`: the same program runs to 42 through the
  full JIT path.

## [0.7.0] — 2026-06-20 (LANG-FULL E5 — bounds-checked arrays)

### Added — `alloc_array` / `array_len` / `array_get` / `array_set`

The reference-interpreter execution of the E5 array primitive. A new `arrays:
Vec<Vec<Value>>` heap holds each allocation; `alloc_array count` pushes a fresh
`count`-element `Vec` (default-initialised — `f64` arrays to `0.0`, else `0`) and
binds its 0-based index as the array *handle*. `array_get`/`array_set` are
**bounds-checked**: a negative or `>= len` index returns a `VMError` (the
interpreter's analogue of the managed runtimes' `IndexOutOfBoundsException` and
the native backends' trap). `array_len` reads the length.

Per-allocation `Vec`s mean **distinct arrays never alias** (unlike the single
Brainfuck byte-tape, which is one flat space). `max_memory_entries` is enforced as
a true **aggregate** ceiling — both the number of arrays and the running total of
elements across every live array are bounded — so neither a single
`alloc_array i64::MAX` nor a loop allocating many arrays can OOM the process.
8 unit tests: round-trip, default-init, length, f64 arrays, out-of-bounds trap,
negative-index trap, no-alias, and the aggregate allocation cap. Integer/float
programs are unaffected (`DispatchCtx` gains an `arrays` field; existing handlers
are untouched). Uses `interpreter-ir` 0.7.0's `array<T>` type + opcodes.

## [0.6.0] — 2026-06-20 (LANG-FULL E3 — floating-point execution)

### Added — `f64` arithmetic and ordered comparisons

The interpreter previously resolved an `Operand::Float` literal but rejected
any *arithmetic* on it (`int_srcs` coerced to `i64`, erroring on a float). Now
`add`/`sub`/`mul`/`div`/`neg` and the ordered comparisons (`cmp_lt`/`cmp_le`/
`cmp_gt`/`cmp_ge`) take a **float track** when an op is `f64`/`f32`-typed or has
a float operand, computing in `f64` and producing a `Value::Float` — never
width-masked. `cmp_eq`/`cmp_ne` already compared floats via `Value`'s
`PartialEq`.

- A new `float_srcs(frame, srcs, type_hint)` helper returns `Some((f64, f64))`
  on a float signal and `None` to fall through to the unchanged integer path —
  so **every existing integer program behaves exactly as before** (the float
  path is taken only on a genuine float signal).
- **Float division is IEEE-754**: `x / 0.0` is `±inf`/`NaN`, *not* an error.
  This matches the LLVM/WASM/JVM `fdiv` the code-gen backends emit, so a
  real-division program agrees across every backend instead of trapping. Only
  *integer* division still traps on a zero divisor.
- An integer operand on the float track is widened (`n as f64`) so a future
  mixed expression degrades gracefully; the current ALGOL frontend never mixes.

This is the reference-tier half of enabler **E3** (floating-point): it lets the
VM execute the `f64` IIR that the ALGOL 60 `real` frontend (AL1) now emits.
Five unit tests cover arith, division-by-zero → inf, ordered/equality
comparisons, and float negation.

## [0.5.0] — 2026-06-14 (LANG-FULL E2 — register width & wrap, backend 1 of 6)

### Added — narrow-width integer arithmetic wraps mod-2ⁿ by `type_hint`

Until now integer arithmetic ran at full `i64` width regardless of the
instruction's `type_hint`, so `200u8 + 100u8` produced `300` instead of `44`
and `~x` on a `u8` flipped 64 bits.  Narrow types existed only at the byte-tape
boundary (`store_byte` masks `& 0xFF`).

This is the first backend in the **E2 (integer width & wrap)** enabler: a new
`mask_result(v, type_hint, u8_wrap)` masks every integer arithmetic / bitwise /
shift result to the width its `type_hint` names —

| hint | mask | example |
|------|------|---------|
| `u4` | `& 0xF` | `10 + 10` → `4` |
| `u8` | `& 0xFF` | `200 + 100` → `44`; `~0` → `255`; `1 << 8` → `0` |
| `u16` | `& 0xFFFF` | `60000 + 10000` → `4464` |
| `u32` | `& 0xFFFF_FFFF` | `2³²` → `0` |
| other (`i64`/`u64`/`any`) | — | full machine width, unchanged |

— the register-arithmetic analogue of the byte-tape `store_byte` mask.  Applied
in `add`/`sub`/`mul`/`div`/`mod`/`neg`/`and`/`or`/`xor`/`not`/`shl`/`shr`.  The
legacy whole-module `u8_wrap` flag (Brainfuck's cell wrap, whose frontend widens
its hint to `i64`) is preserved and applied last, so Brainfuck is unaffected.
Signed narrow types (`i8`/`i16`/`i32`) are intentionally not masked here —
correct two's-complement wrap needs sign-extension via the `cast` op; the
LANG-FULL frontends use the unsigned widths.

## [0.4.0] — 2026-06-13 (LANG-MATRIX Phase V — byte-tape ops; Brainfuck on the VM)

Adds the lowered byte-tape ops to the dispatch table so the **generic** register VM runs
Brainfuck — completing the matrix's VM column (every language now runs on this one
interpreter, no per-language code). Verified by RUNNING `++++++++[>++++++++<-]>+.` on the
VM in `lang-aot/tests/lang_matrix.rs`: it prints `A`.

`lang-aot::lower_brainfuck_for_aot` rewrites Brainfuck's tape into `alloc_bytes` /
`load_byte` / `store_byte` (the same ops every code-gen backend grew for LM-L/W/J/C). The
VM implements them over its **existing flat `memory` address space** (the same
`HashMap<i64, Value>` `load_mem`/`store_mem` use) — no new value kind, no new state:

- **`alloc_bytes dest <- size`** → binds `dest` to the tape base address `0`. Each function
  allocs one tape; cells are sparse `memory` entries keyed by `base + idx`, so an untouched
  cell reads `0` (Brainfuck's zero-cell convention). `size` is advisory.
- **`load_byte dest <- base, idx`** → `memory[base + idx]` (default `0`), masked to a byte
  (unsigned: 200 reads as 200, never sign-extended).
- **`store_byte base, idx, val`** → writes `val & 0xFF` to `memory[base + idx]` — the mask is
  Brainfuck's 8-bit cell wrap-around. Reuses `store_mem`'s `max_memory_entries` cap (a loop
  storing to distinct cells can't grow the map without bound).

Two new tests in `src/core.rs` (`byte_tape_round_trips_unsigned_and_wraps`,
`untouched_byte_tape_cell_reads_zero`).

## [0.3.0] — 2026-06-10 (McCarthy W15b — lambda-frame register sizing)

### Fixed

- `VMFrame::for_function` now sizes the register file to
  `max(register_count, params.len())`. A frontend may under-report
  `register_count` for a function whose only registers are its parameters — a
  hoisted McCarthy `LAMBDA` body like `(LAMBDA (X) X)` reports `register_count = 0` —
  and the dispatcher writes the call arguments directly at indices `0..params.len()`.
  Previously the register file was sized to `register_count` alone, so a lambda call
  indexed past the end of `registers` and **panicked**. This mirrors how `assign`
  already grows the file for under-reported *locals*. Unblocks McCarthy `LAMBDA`/
  `LABEL` on the universal JIT (the eighth and final backend). New unit test
  `for_function_sizes_registers_to_cover_params_when_count_underreports`.

## [0.2.1] — 2026-05-22

### Added (LANG74 follow-up — universal `mov` dispatch)

- `dispatch.rs`: new `handle_mov` + `"mov" => Some(handle_mov)` entry in
  the standard opcode table.  Implements the IIR canonical
  `mov dest = src` semantics — resolve `src`, assign to the named slot
  `dest` in the current frame.
- Unblocks the JIT chain for frontends that emit `mov` directly
  (e.g. `dartmouth-basic-iir-compiler`, `oct-iir-compiler`).
  Previously these programs ran fine through `lang-aot` (the AOT
  specialiser rewrites `mov` to the typed `mov_<ty>` CIR variant the
  backends handle) but tripped `VMError::UnknownOpcode("mov")` the
  moment `VMCore::execute` saw them.

### Proof

`dartmouth-basic-iir-compiler` ships a new
`tests/jit_smoke.rs` that runs four BASIC programs (PRINT-only, LET +
arithmetic + PRINT, FOR/NEXT, IF/THEN/GOTO) through
`JITCore::execute_with_jit`, registering `print_i64` on a custom
`BuiltinRegistry` to capture output.  All four pass — meaning every
language in the LANG74 roadmap now runs end-to-end through **both** the
AOT chain (`lang-aot`) and the JIT chain (`vm-core` + `jit-core`).

## [0.2.0] — 2026-05-11

### Changed (LANG32 — Operand::Str exhaustiveness)

- `dispatch.rs`: `resolve_operand` now handles `Operand::Str(s)` — converts
  the compile-time string literal to `Value::Str(s.clone())`.

## [0.1.0] — 2026-04-27

Initial Rust port of the Python `vm-core` package (LANG02).

### Added

- `Value` enum — `Int(i64) | Float(f64) | Bool(bool) | Str(String) | Null`.
  `iir_type_name()` performs range-aware integer classification
  (`0–255 → "u8"`, `0–65535 → "u16"`, …).

- `VMError` — `UnknownOpcode`, `FrameOverflow`, `UndefinedVariable`,
  `TypeError`, `DivisionByZero`, `UndefinedLabel`, `Custom`.

- `VMFrame` — per-call state: flat register file (`Vec<Value>`), variable
  name → register index map (`HashMap<String, usize>`), instruction pointer,
  and caller return-destination register.  `assign()` grows the register file
  on demand (no bounds-error on well-formed IIR).

- `VMProfiler` — observes runtime `Value` types for `"any"`-typed instructions
  and records them in the instruction's `SlotState`.  Supports custom type
  mapper functions (`VMProfiler::with_mapper`).

- `BuiltinRegistry` — named built-in handlers callable via `call_builtin`.
  Pre-registered: `noop`, `assert_eq`, `print`.

- `DispatchCtx` — all mutable execution state in one struct (frame stack,
  module functions, flat memory, builtins, counters).  `extra_opcodes` and
  `jit_handlers` are intentionally **not** fields — they are passed as
  separate `&HashMap` references to the dispatch loop to avoid Rust
  borrow-checker conflicts when handler closures also need to mutate ctx.

- Standard opcode handlers — `const`, `add/sub/mul/div/mod/neg`,
  `and/or/xor/not/shl/shr`, `cmp_eq/ne/lt/le/gt/ge`, `label/jmp/jmp_if_true/
  jmp_if_false`, `ret/ret_void`, `load_reg/store_reg`, `load_mem/store_mem`,
  `call/call_builtin`, `io_in/io_out`, `cast`, `type_assert`.

- `VMCore` — public execution API: `execute()`, `register_jit_handler()`,
  `register_opcode()`, `builtins_mut()`, `metrics_instrs()`,
  `metrics_jit_hits()`, `fn_call_counts()`, `total_observations()`.

- `u8_wrap` mode — masks all arithmetic results with `& 0xFF` for Tetrad
  8-bit register semantics.

- 29 unit tests + 6 doctests.

### Architecture notes

The borrow-checker challenge: the dispatch loop needs `&mut DispatchCtx` (to
mutate frame state) AND needs to call handlers that also take `&mut DispatchCtx`.
Solution: handlers take `&mut DispatchCtx` directly (no separate `&mut VMFrame`
parameter); each handler opens a nested block to release the frame borrow before
accessing other `DispatchCtx` fields.  Read-only lookup tables (`extra_opcodes`,
`jit_handlers`) are passed as separate parameters to `run_dispatch_loop`.
