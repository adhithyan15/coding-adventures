# Changelog — iir-to-jvm-class-file

All notable changes to this crate are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.4.1] — 2026-05-13

### Fixed (Multi-backend demo — fib(10)=55)

- **`"mov"` opcode support** — added handling for the `mov` IIR instruction
  (pre-lowered form of `call_builtin "_move"`).  The lowerer now emits the
  appropriate load + store sequence for Long / Int slots.
- **Long arithmetic for integer parameters** — parameters typed as `"i64"`
  (after the `fixup_control_flow_types` Pass 0 normalization) now use
  `lload`/`lstore` instead of `iload`/`istore`, preventing JVM verifier
  errors (`Bad local variable type`).
- **Long comparison** — `cmp_lt`/`cmp_gt`/`cmp_le`/`cmp_ge` now emit
  `lcmp` + conditional branch (not `if_icmp*`) when operands are `Long`.
  The `emit_long_compare` helper sequences `lcmp; ifXX 7; iconst_1; goto 4;
  iconst_0` to produce a boolean result.
- **`emit_lconst` fixed** — values 2–127 are now synthesised with
  `iconst_N; i2l` (or `bipush; i2l` / `sipush; i2l`) instead of an
  invalid `ldc2_w #0` placeholder that caused `VerifyError`.
- **Class file version 49** — downgraded from Java 8 (52) to Java 5 (49)
  to use the old type-inferencing verifier, removing the requirement for
  `StackMapTable` attributes in branching methods.

## [0.4.0] — 2026-05-12

### Added (LANG36 — JVM Closure Lowering)

This release promotes the JVM backend from "reject closures with ClosureOpcode"
to a full `long[]`-based dispatch-table implementation of first-class closures.

#### Closure representation

A JVM closure is a **`long[]` array** where `closure[0]` holds the function
dispatch index and `closure[1..]` holds the captured values (as `long`).
Integer captures (`i32`, `u32`, `bool`) are sign-extended to `long` via `i2l`;
`i64`/`u64` captures are stored directly.  Float captures (`f32`, `f64`) are
deferred to LANG38 and still produce a `ClosureOpcode` error.

#### `__callClosure` dispatch method

When a module contains any `alloc_closure` instruction, the lowering pass
generates a synthetic `static long __callClosure(long[] closure, long[] args)`
method.  It reads `closure[0]` as a dispatch index and uses a chain of
`lcmp` / `ifeq` branches — one branch per closure-eligible function — to
reconstruct the correct static call.  Dispatch indices are assigned
alphabetically (deterministic byte-identical output).

#### New JVM opcodes emitted

| Opcode     | Byte   | Description                                         |
|------------|--------|-----------------------------------------------------|
| `NEWARRAY` | `0xBC` | Allocate primitive array; operand `0x0B` = `T_LONG` |
| `LALOAD`   | `0x2F` | Load `long` from `long[]`                           |
| `LASTORE`  | `0x50` | Store `long` into `long[]`                          |
| `LCMP`     | `0x94` | Compare two longs (`-1`, `0`, or `1`)               |
| `L2I`      | `0x88` | Long → int narrowing conversion                     |
| `I2L`      | `0x85` | Int → long sign-extending conversion                |

#### `alloc_closure` lowering

```text
dest = alloc_closure(Str("fn_name"), Var(cap0)) : "closure"
→  iconst_2; newarray T_LONG
   dup; iconst_0; ldc2_w fn_idx; lastore   (closure[0] = dispatch_idx)
   dup; iconst_1; iload cap0_slot; i2l; lastore  (closure[1] = cap0)
   astore dest_slot
```

#### `call_closure` lowering

```text
dest = call_closure(Var(handle), Var(arg0)) : "any"
→  aload handle_slot
   iconst_1; newarray T_LONG
   dup; iconst_0; lload arg0_slot; lastore  (args[0] = arg0)
   invokestatic ClassName.__callClosure([J[J)J
   lstore dest_slot
```

#### Validator changes

- `alloc_closure` with non-float captures → accepted (no longer `ClosureOpcode`).
- `call_closure` → accepted.
- `alloc_closure` with `f32`/`f64` capture type hints → still emits `ClosureOpcode`
  (deferred to LANG38).

#### `serialize_jvm_class_file`

New public function `serialize_jvm_class_file(class_file: &JvmClassFile) -> Vec<u8>`
serializes a `JvmClassFile` to a valid `.class` byte stream (JVMS §4).
Used by the real-JVM round-trip test.

#### Tests

- `lang36_alloc_closure_accepted_by_jvm_validator`
- `lang36_call_closure_accepted_by_jvm_validator`
- `lang36_float_closure_still_rejected`
- `lang36_alloc_closure_emits_newarray`
- `lang36_alloc_closure_emits_lastore`
- `lang36_call_closure_emits_invokestatic_dispatch`
- `lang36_dispatch_method_generated`
- `lang36_dispatch_method_contains_lcmp`
- `lang36_real_jvm_closure_adder` — compiles a two-function module, serializes
  to a `.class` file, runs with `java -Xverify:none`, asserts output is `7`.
  Gated by `java_available()`.

---

## [0.3.0] — 2026-05-12

### Added (LANG35 — Closure Backend Integration)

#### Improved `ClosureOpcode` validator error

- `validate_for_jvm` now emits a dedicated `ClosureOpcode` error message
  (format: `"[fn_name] ClosureOpcode: alloc_closure/call_closure require the
  BEAM backend — JVM does not support heap-allocated closures"`) when it
  encounters `alloc_closure` or `call_closure`.
- Previously these fell through to the generic `UntypedInstruction` path;
  the closure check now runs first to give a more actionable error message.

#### Tests

- `lang35_alloc_closure_closure_opcode_error`: asserts `validate_for_jvm`
  returns an error containing "ClosureOpcode" for a module with `alloc_closure`.
- `lang35_call_closure_closure_opcode_error`: same for `call_closure`.
- `lang35_closure_opcode_error_not_untyped`: asserts the error does NOT
  contain "UntypedInstruction".

---

## [0.2.0] — 2026-05-11

### Added (LANG32 — Global Variables and I/O)

#### I/O support

- `io_out %v` → `getstatic java/lang/System.out` (Ljava/io/PrintStream;) +
  `lload <slot>` + `invokevirtual java/io/PrintStream.println(J)V`.
- Added `INVOKEVIRTUAL: u8 = 0xB6` and `GETSTATIC: u8 = 0xB2` bytecode
  constants.
- Added `add_fieldref` to `ConstantPoolBuilder`.

#### Global variables (LANG32b — deferred)

- `global_load` and `global_store` return `UnsupportedOp` with a clear
  LANG32b tracking note.  Full JVM static-field globals require extending
  `JvmClassFile` with a `fields: Vec<JvmFieldInfo>` table and adding
  `getstatic`/`putstatic` sequences; tracked in a follow-up PR.

#### Exhaustiveness fixes

- `Operand::Str` arms added to all `match` blocks in `lower.rs` (const,
  ret, call argument loops).

---

## [0.1.0] — 2026-05-11

### Added

- `validate::validate_for_jvm(module: &IIRModule) -> Vec<String>` — pre-flight
  validation pass that rejects modules containing JVM-incompatible instructions
  or types before any lowering starts. Catches:
  - Empty module (no functions)
  - Empty function (function with no instructions)
  - Untyped instructions (`type_hint == "any"` or `"polymorphic"`)
  - Unsupported types (`"str"`, `ref<…>`)
  - Unsupported opcodes (`call_builtin`, `io_in`, `io_out`, `cast`, memory ops,
    GC ops, `safepoint`)
  - Float type hints and float constants are **supported** (unlike the BEAM
    backend), since the JVM has native `fload`/`dload`/`fadd`/`dadd` opcodes.

- `lower::IIRJvmConfig` — lowering configuration: `class_name` String.
  Implements `Default` (uses `"IIRModule"`) and `new(class_name)`.

- `lower::IIRJvmError` — typed error variants:
  `ValidationFailed`, `UnsupportedOp`, `UnsupportedType`, `UndefinedLabel`,
  `UndefinedVariable`, `InvalidOperand`. Implements `Display` and `std::error::Error`.

- `lower::lower_iir_to_jvm(module: &IIRModule, config: &IIRJvmConfig) -> Result<JvmClassFile, IIRJvmError>` —
  two-pass lowering algorithm:
  - Pass 1 per function: assign JVM local variable slots to params (0..N-1)
    then walk dests and src Var operands in order for locals (N..).
  - Pass 2: emit raw JVM bytecode (Vec<u8>) per method using emit_* helpers.
  - Build `JvmClassFile` directly (Java 8, version 52.0).
  - Two-pass backpatching for forward label/jump references.

- Supported IIR opcodes:
  `const` (Int, Float, Bool), `add`, `sub`, `mul`, `div`, `mod`, `neg`,
  `and`, `or`, `xor`, `not`, `shl`, `shr`,
  `cmp_eq`, `cmp_ne`, `cmp_lt`, `cmp_le`, `cmp_gt`, `cmp_ge`,
  `label`, `jmp`, `jmp_if_true`, `jmp_if_false`,
  `ret`, `ret_void`, `call`, `load_reg`, `store_reg`, `type_assert`.

- Type mapping: `i8/i16/i32/u8/u16/u32/bool → int (I)`, `i64/u64 → long (J)`,
  `f32 → float (F)`, `f64 → double (D)`, `void → void (V)`.

- `codegen::IIRJvmCodeGenerator` — thin adapter that wires `validate_for_jvm`
  and `lower_iir_to_jvm` behind the `name()` / `validate()` / `generate()` API.

- 40+ integration tests in `tests/test_backend.rs` covering validation, lowering,
  instruction emission, register allocation, multi-function modules, float support,
  comparison synthesis, and bytecode non-emptiness checks.
