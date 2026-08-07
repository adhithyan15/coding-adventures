# iir-to-cil-bytecode

Lowers an `IIRModule` (from `interpreter-ir`) directly to a `CILProgramArtifact`
(from `ir-to-cil-bytecode`) **without going through the deprecated `compiler-ir`
layer**.

## What is CIL?

Common Intermediate Language (CIL) is the stack-based bytecode format of the
Common Language Runtime (CLR), the virtual machine powering .NET, Mono, and
Xamarin.  Unlike the JVM — which encodes types in the opcode (`iadd` for
integers, `fadd` for floats) — CIL infers operand types from the evaluation
stack at JIT time:

```text
JVM:  iadd          ← "i" means int32, encoded in the opcode
CIL:  add           ← type is inferred at JIT time from the stack
```

## Pipeline

```text
IIRModule
  → validate_iir_for_clr()      — pre-flight validation
  → lower_iir_to_cil()          — emit CIL body bytes per function
  → CILProgramArtifact          — structured multi-method artifact
       ↓ CLR simulator          — run directly (fast, zero-dep)
  → emit_il()                   — emit TEXTUAL CIL (.il)
       ↓ real ilasm → PE → real dotnet   — run on REAL CoreCLR
```

Two outputs from the same lowered program: binary method bodies for the in-repo
`clr-simulator` (fast unit checks), and **textual `.il`** (`emit_il`) for the
**real CoreCLR** path — assembled by real `ilasm`, run on real `dotnet`. This is
the exact analog of how `iir-to-llvm` emits textual `.ll` for real `clang`; `ilasm`
owns the PE/metadata so we don't hand-roll ECMA-335. (The full McCarthy F1–F7 set
runs today — scalar, cons/car/cdr, **predicates + `COND`**, **symbols**, and
**lambda / LABEL / recursion**. A cons is a 2-element `System.Object[]`, atoms
`box`ed `System.Int32`; `pair?` is `isinst object[]`, `not` is `xor 1`, `equal?` is
`unbox.any int32` ×2 + `ceq`, `COND` lowers to `label`/`br`/`brfalse`. Symbols need
no new ops — `intern_symbols_structural` makes each `(QUOTE S)` a tagged-int boxed
atom. Lambda/LABEL make the module **multi-method**: each hoisted function is its
own static `.method`, application is a by-name `call`, params live in `ldarg`, and a
`field_*` on an `object`-typed param is preceded by `castclass object[]` so real
CoreCLR's `ldelem.ref` sees an array.)

## Quick start

```rust
use interpreter_ir::{IIRModule, IIRFunction, IIRInstr, Operand};
use iir_to_cil_bytecode::{IIRClrConfig, lower_iir_to_cil, validate_iir_for_clr};

// Build: add(a: i32, b: i32) -> i32  { ret a + b }
let fn_ = IIRFunction::new(
    "add",
    vec![("a".into(), "i32".into()), ("b".into(), "i32".into())],
    "i32",
    vec![
        IIRInstr::new("add", Some("v0".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())], "i32"),
        IIRInstr::new("ret", None,
            vec![Operand::Var("v0".into())], "i32"),
    ],
);
let mut module = IIRModule::new("example", "tetrad");
module.entry_point = Some("add".into());
module.add_or_replace(fn_);

let errors = validate_iir_for_clr(&module);
assert!(errors.is_empty());

let config = IIRClrConfig::default();
let artifact = lower_iir_to_cil(&module, &config).unwrap();
assert!(!artifact.methods[0].body.is_empty());
assert!(artifact.methods[0].body.contains(&0x2A)); // ret
```

## Closure lowering (LANG37)

Since LANG37 the CLR backend supports first-class closures via an `int32[]`
dispatch table.

```rust
// alloc_closure: build a closure over captured i32 values
// call_closure: call a closure at runtime via __callClosure dispatch
```

| Closure type | Support |
|---|---|
| `i32` / `bool` captures | ✅ LANG37 |
| `i64` / `u64` / `f32` / `f64` captures | ❌ LANG38 (ClosureOpcode error) |

A closure is an `int32[]` array:
- `[0]` = function dispatch index (alphabetical order, deterministic)
- `[1..n]` = captured values as `int32`

A synthetic `__callClosure(int32[], int32[]) → int32` method is appended to
the program artifact whenever any `alloc_closure` instruction appears.

## Validation errors

| Error | Condition |
|-------|-----------|
| `EmptyModule` | Module has no functions |
| `EmptyFunction` | A function has no instructions |
| `ClosureOpcode` | `alloc_closure` with i64/u64/f32/f64 capture — deferred to LANG38 |
| `UntypedInstruction` | `type_hint` is `"any"` or `"polymorphic"` (except `call_closure` which is exempt) |
| `UnsupportedType` | `type_hint` is unsupported (`"str"` except `str_const`, or unsupported `ref<...>`) |
| `UnsupportedOp` | Any unsupported opcode (see below) |

## Supported IIR opcodes

| Category | Opcodes |
|----------|---------|
| Constants | `const` (Int, Bool; Float **unsupported** in v1) |
| Arithmetic | `add`, `sub`, `mul`, `div`, `mod`, `neg` |
| Bitwise | `and`, `or`, `xor`, `not`, `shl`, `shr` |
| Comparison | `cmp_eq`, `cmp_ne`, `cmp_lt`, `cmp_le`, `cmp_gt`, `cmp_ge` |
| Control flow | `label`, `jmp`, `jmp_if_true`, `jmp_if_false` |
| Return | `ret`, `ret_void` |
| Calls | `call`, `call_closure` (LANG37) |
| Closures | `alloc_closure` (LANG37, i32/bool captures only) |
| Heap | `alloc` (`ref<LispyPair>` only), `field_load`, `field_store`, `is_null` |
| Register | `load_reg`, `store_reg` |
| Coercion | `type_assert` (becomes `nop`) |
| Strings | `str_const`, `str_concat`, `str_slice`, `str_len`, `str_index`, `str_eq`, `str_cmp`, `print_str` on the textual `.il` path (ASCII literal foothold) |

As of 0.16.0 the **textual `.il`** emitter (`emit_il`, the `ilasm`/`dotnet` path) also
covers the integer **arithmetic** (`add`/`sub`/`mul`/`div`/`mod` → `add`/`sub`/`mul`/`div`/`rem`)
and **comparison** (`cmp_*` → `ceq`/`clt`/`cgt`, negating the other three with `ldc.i4.0; ceq`)
rows above — previously only the binary codegen path emitted them, which is why running
the LANG-MATRIX expression languages (Nib/Oct/ALGOL) on the real CLR first surfaced the gap.
As of 0.19.0 the textual path likewise covers the **binary bitwise/shift** row
(`and`/`or`/`xor`/`shl`/`shr` → the identically named CIL opcodes) — the same kind of
bytecode-path-only gap, surfaced by running Nib `& | ^` on the real CLR (LANG-FULL N3).
As of **0.21.0** it also covers the **unary `not`** op (Nib `~`) → the CIL `not` opcode
(one's complement) + the E2 narrow mask, so `~0u8 = 255` / `~15u4 = 0` assemble on real
CoreCLR — the last `not`-shaped gap (the bytecode path had it since the E2 work; the
textual `.il` path had no `not` arm at all, only the lispy `call_builtin "not"`).

As of 0.17.0 it also emits the **`print_i64`** I/O primitive (Dartmouth BASIC's `PRINT`)
as `call void [System.Console]System.Console::WriteLine(int32)`; for a program that
prints, the `Run()` launcher discards the entry method's result (`pop`) instead of
`Console.WriteLine`-ing it, so the program prints exactly once.

As of 0.29.0 the textual `.il` path emits the **E4 literal string** foothold:
`str_const` lowers to `ldstr` into a `string` local, `print_str` lowers to
`Console.Write(string)`, and direct-literal `str_len` calls
`String::get_Length()` while direct-literal `str_index` calls
`String::get_Chars(int32)`, direct-literal `str_eq` calls
`String::Equals(string,string)`, direct-literal `str_cmp` calls
`String::CompareOrdinal(string,string)` plus `Math::Sign(int32)`, and direct-literal `str_concat` calls
`String::Concat(string,string)`. This proves Dartmouth BASIC `PRINT "HELLO"` plus
Twig `(string-length "HELLO")`, `(string-ref "ABC" 1)`,
`(string=? "HELLO" "HELLO")`, `(string<? "ALPHA" "BETA")`, and
`(string-length (string-append "AB" "CDE"))`
on real CoreCLR while non-literal string values remain rejected until the CLR
representation owns the shared UTF-8 byte semantics.

As of 0.18.0 it emits the **Brainfuck byte-tape ops** (LANG-MATRIX LM-C Brainfuck — the
last code-gen cell): `alloc_bytes` → `newarr [System.Runtime]System.Byte` into an
`unsigned int8[]` local (the tape), `load_byte` → `ldelem.u1` (unsigned cell), `store_byte`
→ `stelem.i1` (8-bit wrap-around), and `putchar`/`getchar` → `Console::Write(char)` /
`Console::Read()`. `putchar` joins `print_i64` as a "this program prints" signal, so a
Brainfuck program's launcher discards the entry result rather than re-printing it. CIL
`brfalse`/`brtrue` test any integer width against zero, so the loop guard needs no
special i64 handling (unlike the JVM's `lcmp` / wasm's `i64.eqz`).

As of 0.23.0 the **textual `.il` emitter** lowers the **E5 array ops** (LANG-FULL
enabler E5 — ALGOL 1-D arrays) to native single-dimensional CIL arrays:
`alloc_array` → `newarr [System.Runtime]System.Int32` (or `…Double`/`…Single`) into
an `int32[]`/`float64[]` handle local; `array_get` → `ldelem.i4`/`.r8`; `array_set` →
`stelem.i4`/`.r8`; `array_len` → `ldlen; conv.i4`. CoreCLR bounds-checks every
`ldelem`/`stelem` natively, so an out-of-range index throws
`System.IndexOutOfRangeException` (E5's trap) with no explicit guard. `i64` elements
collapse to `int32[]` (CIL stack ints are 32-bit here, like scalar `i64`). A **`str`
element** (E4d-BA-arr, BASIC `DIM A$(n)`) is a `System.String[]` reference array —
`newarr [System.Runtime]System.String` + `ldelem.ref`/`stelem.ref` — reusing the same
reference element machinery the McCarthy `object[]` cons cells use. The binary
`CILProgramArtifact` (`clr-simulator`) emitter doesn't lower the array ops yet.

**Narrow-width register arithmetic wraps mod-2ⁿ** (LANG-FULL E2, backend 5/6, v0.20.0).
A CIL arithmetic/bitwise op runs on a full 32-bit `int32` slot, so a narrow unsigned
value overflows its width unless masked. After a `u4`/`u8`/`u16` `add`/`sub`/`mul`/
`div`/`mod`/`neg`/`and`/`or`/`xor`/`shl`/`shr`/`not`, **both** emitters append a
`ldc.i4 <mask>; and` (`0xF`/`0xFF`/`0xFFFF`) so `200u8+100u8=44` and `~0u8=255`:

```text
  add ; ldc.i4 0xFF ; and      ←  (200 + 100) & 0xFF = 44
```

`u32`/`i32` already wrap mod-2³² via the 32-bit op (no mask). A positive mask + `and`
is used — not `conv.u1`/`conv.i1`, which sign-extend — to keep the unsigned widths
unsigned, exactly like the JVM `iand` and wasm `i32.and` masks. The narrow `type_hint`s
that trigger the mask are wired into the Nib/Oct frontends in the E2 integration PR (6/6).

As of v0.20.1 this is **verified to work for the i64 frontend value model**. A real
frontend (Nib) materialises every `const`/`let` as `i64` and carries the narrow width
only on the op. The wasm and jvm backends had to grow an i64/long register model so a
narrow op wouldn't trap over those `i64` operands — but the CIL backend needs no such
rework, because it is **uniformly int32**: `cil_local_type` maps every scalar (incl.
`i64`) to `int32`, and `const` emits `ldc.i4`. So the `i64` consts collapse to `int32`
and the mask stays int32-consistent. The `e2_u8_op_over_i64_operands_stays_int32`
regression test asserts the emitted IL has no `int64`/`ldc.i8` and still masks.

## How CIL synthesis works for derived operations

CIL lacks native opcodes for some logical operations, so we synthesize them
from primitives:

| IIR op | CIL synthesis |
|--------|---------------|
| `mod` | `rem` opcode (0x5D — not in the `CILOpcode` enum, emitted as raw byte) |
| `neg` | `neg` opcode (0x65 — raw byte) |
| `not` | `not` opcode (0x66 — bitwise complement, raw byte) |
| `cmp_ne r1, r2` | `ceq; ldc.i4.0; ceq` (NOT of equal) |
| `cmp_le r1, r2` | `cgt; ldc.i4.0; ceq` (NOT of greater-than) |
| `cmp_ge r1, r2` | `clt; ldc.i4.0; ceq` (NOT of less-than) |

## Register model

IIR uses named SSA variables.  This backend maps them to CIL locals and
method arguments:

- **Parameters** → `ldarg`/`starg` (indices 0..N-1).
- **Locals** → `ldloc`/`stloc` (indices 0..M-1 where M = unique vars − params).

A two-pass scan assigns each distinct variable name a stable slot index.

## Module structure

| Module | Contents |
|--------|----------|
| `validate` | `validate_iir_for_clr` — pre-flight checks |
| `lower` | `IIRClrConfig`, `IIRClrError`, `lower_iir_to_cil` — main lowering pass |
| `codegen` | `IIRClrCodeGenerator` — `CodeGenerator` protocol adapter |

## How it fits in the stack

```text
Language Frontend
     │  IIRModule
     ▼
iir-to-cil-bytecode   ← this crate
     │  CILProgramArtifact
     ▼
CLR simulator / PE packager
```
