# Changelog — brainfuck-iir-compiler

## [0.4.0] — 2026-06-17 (LANG-FULL B1-eof — `,` normalises EOF to 0, so cat runs cross-backend)

### Fixed — `,` stores 0 at end-of-input, on every backend

`getchar` yields a byte (0–255) or a negative sentinel at end-of-input — but the
sentinel differs by runtime: libc `getchar` (native/LLVM), `Console.Read` (CLR) and the
wasm host return `-1`, while the JVM/VM/JIT stubs return `0`. The old lowering stored the
raw result into the `u8` cell, so on the `-1` runtimes EOF truncated to `255`. The
canonical cat `,[.,]` (loop while cell ≠ 0) therefore **looped forever** on those
backends, while halting on the `0` ones — the deferred half of B1-stdin.

`,` now normalises EOF to `0` **in the shared IIR**, so behaviour is identical on every
backend rather than patched per-backend:

```text
call_builtin v getchar      i64   ; read at i64 so -1 survives (a u8 read masks it to 255)
const        z 0            i64
cmp_lt       c v z          i64   ; c = (v < 0)  → EOF?
jmp_if_false c input_N_store      ; byte in range → store as-is
const        v 0            i64   ; EOF → cell = 0
label          input_N_store
store_mem      ptr v        u8
```

This is the "EOF leaves 0" convention. A program that never reads past its input
(`,+.`, `,.,.`) never reaches the clamp, so its lowering is unchanged. **Verified by
RUNNING** the cat `,[.,]` with input `"Hi"` → stdout `"Hi"` on native/LLVM/WASM/JVM/CLR/
VM/JIT (`lang-aot` `tests/lang_matrix.rs`). New unit tests
`comma_emits_getchar_with_eof_clamp_then_store`, `two_commas_use_distinct_clamp_labels`.

## [0.3.3] — 2026-05-22 (BF → CLR end-to-end + BEAM rejection doc)

### Added — `tests/clr_e2e.rs`

Stage 3 of 4 for the BF→{wasm,jvm,clr,beam} story.  Walks the new
IIR-based chain through the CLR backend:

```text
BF source → IIRModule → iir-to-cil-bytecode validator → lower_iir_to_cil
          → CILProgramArtifact (with reserved env.BFRuntime tokens)
```

Before iir-to-cil-bytecode 0.5.0, the validator rejected `load_mem` /
`store_mem` and any `call_builtin`.  With 0.5.0 those are accepted,
and this e2e test locks the path in for Brainfuck with byte-exact
sequence assertions:

- `brainfuck_three_increments_lowers_to_cil` — `+++.` lowers; the
  main method's CIL body contains `[0x28, 0x03, 0x00, 0x00, 0x0A]`
  (call BF_PUTCHAR_TOKEN at MemberRef row 3).
- `brainfuck_loop_lowers_to_cil` — `++[-]` lowers; the body contains
  `[0x7E, 0x01, 0x00, 0x00, 0x04]` (ldsfld BF_TAPE_TOKEN, FieldRef row 1).
- `brainfuck_input_emits_getchar_call` — `,.` emits both
  `call BF_GETCHAR_TOKEN` and `call BF_PUTCHAR_TOKEN`.
- `brainfuck_empty_program_emits_minimal_cil` — the empty BF program
  contains **none** of the BF runtime token sequences.

### Added — README "Cross-backend compilation status" section

Documents that BF flows through wasm/jvm/clr but **not** BEAM, with
a full rationale:

> BEAM's substrate is purely functional.  Brainfuck's tape is mutable
> bytes in random-access addressing.  Compiled to vanilla BEAM
> bytecode, every `store_mem` would copy the whole 30 KB tape — O(N²·M)
> on a `,[.,]` cat over a non-trivial input.  Alternatives (ETS,
> process dictionary, NIF) exist but each one abandons the "compile to
> vanilla bytecode" promise.  Documented rejection is the right call.

(Closes Stage 4 of the BF→{wasm,jvm,clr,beam} story.)

## [0.3.2] — 2026-05-22 (BF → JVM end-to-end test)

### Added — `tests/jvm_e2e.rs`

Stage 2 of 4 for the BF → {wasm, jvm, clr, beam} story.  Walks the new
IIR-based chain through the JVM backend:

```text
BF source → IIRModule → iir-to-jvm-class-file validator → lower_iir_to_jvm
          → JvmClassFile → serialize_jvm_class_file → .class bytes
```

Before iir-to-jvm-class-file 0.5.0, the validator rejected `load_mem` /
`store_mem` and any `call_builtin` (including BF's `putchar` /
`getchar`).  With 0.5.0 those are accepted, and this e2e test locks
the path in for Brainfuck:

- `brainfuck_three_increments_lowers_to_jvm_class` — `+++.` compiles
  through; the resulting `JvmClassFile` has constant-pool entries
  referencing `env/BFRuntime`, and the serialized bytes start with the
  canonical `CAFEBABE` magic.
- `brainfuck_loop_lowers_to_jvm_class` — `++[-]` (a non-trivial loop)
  compiles through; serialized bytes have the right magic.
- `brainfuck_input_emits_getchar_methodref` — `,.` emits both
  `getchar` and `putchar` method references into the constant pool.
- `brainfuck_empty_program_emits_minimal_jvm` — the empty BF program
  emits a class with **no** `env/BFRuntime` references, proving the
  CP injection is correctly conditional (no burden on non-BF callers).

No source-code changes in this crate — the test runs against the
existing `compile_source`.  Only the JVM e2e test file is new.

## [0.3.1] — 2026-05-22 (BF → WASM end-to-end test)

### Added — `tests/wasm_e2e.rs`

Stage 1 of 4 for the BF → {wasm, jvm, clr, beam} story.  Walks the new
IIR-based chain end-to-end:

```text
BF source → IIRModule → iir-to-wasm validator → lower_iir_to_wasm → .wasm bytes
```

Before iir-to-wasm 0.4.0, the validator rejected `load_mem` /
`store_mem` and any `call_builtin` (including BF's `putchar` /
`getchar`).  With iir-to-wasm 0.4.0 those are accepted, and this
e2e test locks the path in for Brainfuck:

- `brainfuck_three_increments_lowers_to_wasm_bytes` — `+++.` compiles
  through, the resulting `WasmModule` has an `env.putchar` import,
  a 1-page linear memory, and a `main` export.  The encoded bytes
  start with the canonical WASM magic + version.
- `brainfuck_loop_lowers_to_wasm_bytes` — `++[-]` (a non-trivial
  loop) compiles through; declares a memory; no putchar.
- `brainfuck_input_emits_getchar_import` — `,.` emits both
  `env.getchar` and `env.putchar` imports.
- `brainfuck_empty_program_emits_minimal_wasm` — the empty BF
  program emits no memory and no imports, proving feature detection
  is conditional (doesn't burden modules that don't need tape/IO).

No source-code changes in this crate — the test runs against the
existing `compile_source`.  Only the WASM e2e test file is new.

Stages 2–4 (JVM, CLR, BEAM) are queued as follow-on PRs.

## [0.3.0] — 2026-05-22 (BF05 — real CIR-bytecode JIT backend)

### Added — `BrainfuckCirJit`, a real `jit_core::backend::Backend`

`BrainfuckCirJit` replaces the placeholder `InterpOnlyBackend` from
0.2.0.  It is a real JIT in the classic, historical sense — the same
shape used by the JVM (Ignition tier), Smalltalk-80, V8 Ignition, Lua,
and many other production JITs as their first tier:

1. **`compile()`** translates Brainfuck's CIR (post-specialise,
   post-`CIROptimizer`) into a packed register-machine bytecode.
   Encoding: 1-byte opcode tags, 1-byte register indices, `i16`
   little-endian branch offsets, natural-width literals (`u8`/`u32`).
   14 opcodes total covering BF's full CIR vocabulary
   (`CONST_U8/U32`, `ADD/SUB_U8/U32`, `LOAD_MEM`, `STORE_MEM`,
   `PUTCHAR`, `GETCHAR`, `JMP`, `JMP_IF_FALSE`, `JMP_IF_TRUE`, `RET`).
2. **`run()`** interprets that bytecode in a tight `match`-loop owning
   a fresh tape per call.  Bypasses `vm-core`'s generic IIR dispatch
   entirely — no `HashMap<String, OpcodeHandler>` lookup per
   instruction, no string-keyed register file, no IIR-level operand
   resolution.

#### What this is *not*

This isn't a native-code JIT (no Cranelift, no hand-rolled x86_64 /
aarch64).  That's separate work.  When a backend grows real
machine-code generation for BF's CIR, swapping it in here is a
one-line change.

#### Literal materialization

`CIROptimizer`'s constant-propagation pass folds `const k 1; add v v k`
into `add v v 1` (literal-in-source).  The backend handles this by
materializing literal operands inline: when a binary op or memory op
sees an `Int(n)` operand, the backend allocates a fresh anonymous
register and emits a `CONST_U8` / `CONST_U32` to load `n` into it
*before* the consuming instruction — keeping the runtime interpreter
simple (registers only, no immediate-operand opcode variants).  See
`src/jit_backend.rs::resolve_operand`.

#### Error reporting

`Backend::run`'s signature is `(&self, &[u8], &[Value]) -> Value` —
no `Result` return.  The backend captures an
`Arc<Mutex<Option<String>>>` error slot at construction and writes
the failure reason there on out-of-bounds writes / fuel-cap
exhaustion / malformed bytecode.  `BrainfuckVM::execute_module`
inspects the slot after `JITCore::execute_with_jit` returns and
propagates back to `BrainfuckError`.

### Added — `BrainfuckVM::jit_bytecode_len`

A diagnostic API that compiles `source` through the same
specialise → optimize → bytecode-compile pipeline that
`JITCore::execute_with_jit` runs internally, and returns the
generated bytecode length in bytes (or `None` if compile() refused).
Useful for confirming the JIT path actually does work rather than
silently falling back to the interpreter via the standard
`compile()` returning `None` no-cache-entry path.

### Tests

- `src/jit_backend.rs` (8 unit tests): name, empty-program compile,
  const_u32 LE-byte layout, unknown-op rejection, label-position
  recording, jmp offset resolution, jmp to unknown label rejection,
  end-to-end run of hand-built `+++.` CIR.
- `tests/jit_smoke.rs` adds 3 "JIT realness" tests:
  - `jit_emits_real_bytecode_for_three_increments` — `+++.` must
    compile to >= 15 bytes of bytecode.
  - `jit_emits_bytecode_for_loop_program` — `++[-]` must compile to
    > 20 bytes (label + loop body + branch fixup).
  - `jit_runs_loop_program_correctly` — `++[-].` runs through JIT
    and interpreter and produces matching `[0]` output.
- All 63 unit tests + 9 doc-tests + 9 JIT smoke tests pass on Windows.

### Removed

- The private `InterpOnlyBackend` from 0.2.0 (it was a stop-gap
  placeholder waiting for a real backend — now replaced).

## [0.2.0] — 2026-05-22 (BF05 — Brainfuck on the LANG VM JIT chain)

### Added — JIT mode (BF05)

`BrainfuckVM::execute_module` now dispatches through
[`jit_core::core::JITCore`] when constructed with `jit=true`, replacing
the previous `"jit=true is not yet supported in BF04"` early-error.
Brainfuck programs now run through the full LANG VM JIT chain
(`vm-core` + `jit-core`) in the same shape as Dartmouth BASIC's
`jit_smoke.rs` did after PR #3888.

#### Why this matters (and the careful caveat)

Brainfuck's IIR is already `FullyTyped` from birth — every instruction
carries a concrete `type_hint` (`"u8"` / `"u32"` / `"void"`).  That
means the JIT chain *would* eagerly tier-promote `main` to a native
backend on the very first call (see `JITCore::execute_with_jit` Phase
1).  In practice, none of the existing JIT backends (NullBackend,
EchoBackend, future WASM/x86_64 ports) know how to lower Brainfuck's
custom `load_mem` / `store_mem` opcodes — the tape memory model is
specific to the Brainfuck wrapper, not part of the standard CIR
vocabulary.

Compiling with a backend that silently can't handle those opcodes
would replace `main` with a stub that returns `Null`, producing no
output.  To prevent that, the wrapper now ships a private
`InterpOnlyBackend` whose `compile()` always returns `None`, telling
`JITCore`:

> "I refuse to compile this function — keep it interpreted forever."

The plumbing is real — the same `VMCore` runs the program with all
five Brainfuck-specific handlers (`putchar`, `getchar`, `load_mem`,
`store_mem`, `label`), the JIT's tier-up path is observable via
`JITCore::cache_stats()`, and the `Phase 3` hot-function promotion
runs (as a no-op against `InterpOnlyBackend`).  When a future backend
learns Brainfuck's tape memory model, swap it in here and BF programs
tier-promote automatically — no other changes needed.

#### What about the JIT vs interpreter parity?

The new `tests/jit_smoke.rs` runs every program twice — once with
`jit=false` (pure interpreter) and once with `jit=true` (JIT chain) —
and asserts byte-identical output.  If they ever diverge, the wiring
is broken.

### Tests

- `tests/jit_smoke.rs` (6 tests): runs through `JITCore::execute_with_jit`
  - `jit_brainfuck_three_increments_print` — `+++.` → chr(3)
  - `jit_brainfuck_pointer_arithmetic` — `>+<.` → chr(0)
  - `jit_brainfuck_cat_with_input` — `,[.,]` echoes `"hello"` → `"hello"`
  - `jit_brainfuck_multiply_2_times_3` — classic BF multiplication idiom → chr(6)
  - `jit_brainfuck_u8_wraparound` — `-.` (0 - 1 wraps to 255)
  - `jit_brainfuck_multiple_outputs` — `+.++.+++.` → chr(1) chr(3) chr(6)
- Updated `vm::tests::jit_true_returns_error_on_run` (which asserted
  the BF04 early-error) into three new tests that verify the JIT path
  produces the same results as the interpreter:
  - `jit_true_runs_simple_program`
  - `jit_and_interp_paths_agree_on_hello_h`
  - `jit_handles_loop_with_input`

### Dependencies

- Added `jit-core` as a runtime dependency (was previously not
  reachable from this crate).

### Compatibility

- **Public API unchanged.**  `BrainfuckVM::new(true, ...)` no longer
  returns an error on subsequent `run` / `execute_module` calls — that
  was the only observable behaviour change.  All existing callers that
  passed `jit=false` continue to behave exactly as before.

## [0.1.2] — 2026-05-11

### Fixed (LANG32 — `Operand::Str` exhaustiveness)

- `resolve_i64` and `resolve_value` in `vm.rs` now handle `Operand::Str`.
  `resolve_i64` maps string literals to `0` (strings have no numeric
  representation in Brainfuck byte-cell semantics). `resolve_value` maps them
  to `Value::Str`, matching the `vm-core` Value type.  Brainfuck programs should
  never produce `Operand::Str` in practice, but the arms are required because
  the `Operand` enum is now non-exhaustive at the binary level after LANG32
  added the `Str` variant.

## [0.1.1] — 2026-05-04

### Fixed (LANG23 PR 23-E compatibility)

- `IIRFunction` struct literals in `compiler.rs` updated to include
  `param_refinements: Vec::new()` and `return_refinement: None` after
  `interpreter-ir` 0.2.0 added those fields.  No behavioural change.

## [0.1.0] — 2026-04-29

### Added

- **BF04 — Rust port** of the Python `brainfuck-iir-compiler` package.
- `compile_source(source, module_name)` — lex + parse + compile Brainfuck to
  `IIRModule` in one call.
- `compile_to_iir(ast, module_name)` — compile an existing `GrammarASTNode`
  from `brainfuck::parser::parse_brainfuck` to `IIRModule`.
- `BrainfuckVM` — high-level wrapper around `vm_core::VMCore` configured for
  Brainfuck semantics:
  - `u8_wrap = true` (cell wraparound on arithmetic)
  - `putchar` / `getchar` builtins wired to per-run byte buffers
  - Bounds-checked `load_mem` / `store_mem` custom opcode handlers
  - `max_steps` label-crossing fuel cap
  - `jit = true` placeholder (errors in BF04; JIT arrives in BF05)
- `BrainfuckError` — dedicated error type for Brainfuck-level failures
  (out-of-bounds, fuel cap exceeded, JIT not available).
- 52 unit tests + 8 doc-tests (60 total).

### Design notes

- Fixed register names (`ptr`, `v`, `c`, `k`) rather than SSA form —
  `vm-core`'s mutable register file means SSA naming would break
  loop-body definitions when the body is skipped.
- All instructions carry concrete `type_hint` (`"u8"` / `"u32"` / `"void"`),
  producing a `FunctionTypeStatus::FullyTyped` module so BF05's JIT tiers
  up immediately on first call.
- Loop shape (`label start` → `load_mem c ptr` → `jmp_if_false c end` →
  body → `jmp start` → `label end`) matches the canonical form expected by
  `ir-to-wasm-compiler` for structured-loop lowering.
