# LANG31 — IIR Builtin Lowering and End-to-End Pipeline

## Context

The four IIR direct backends (`iir-to-beam`, `iir-to-wasm`, `iir-to-jvm-class-file`,
`iir-to-cil-bytecode`) from LANG29 handle a fully-typed, `call_builtin`-free `IIRModule`.
The Twig front-end (`twig-ir-compiler`) produces an `IIRModule` where:

- every type hint is `"any"`;
- all arithmetic and heap operations are expressed as `call_builtin "<name>"`.

Two passes bridge the gap today: `iir-type-checker` promotes type hints via SSA
inference. But no pass resolves `call_builtin` into the typed arithmetic / heap IIR ops
the backends need.

LANG31 delivers:

1. **`iir-builtin-lowering`** — a pure `IIRModule → IIRModule` transformation that
   converts `call_builtin` instructions into the typed IIR arithmetic, comparison, and
   heap opcodes the backends understand.
2. **`twig-to-beam`**, **`twig-to-wasm`**, **`twig-to-jvm`**, **`twig-to-cil`** — four
   thin pipeline crates that wire `twig-ir-compiler → iir-type-checker →
   iir-builtin-lowering → iir-to-<target>` into a single call.
3. **Backend heap-op extensions** — extend each `iir-to-*` backend to lower the IIR
   heap opcodes (`alloc`, `field_load`, `field_store`, `is_null`) using the target VM's
   native GC.
4. **wasm-types WasmGC extension** — add the minimal struct/ref types needed to use
   the WASM GC proposal (now standardized and shipping in all major runtimes).
5. **Real-VM integration tests** — harnesses that compile a Twig program to bytecode
   and run it under `erl`, `wasmtime`, `java`, and `dotnet`, asserting exit code and
   stdout.

This spec does **not** cover closures (`make_closure` / `apply_closure`), global
variable defines (`global_set` / `global_get`), or I/O (`io_in` / `io_out`).  Those
are tracked by BEAM02, CLR02, and LANG27.

---

## Relationship to existing specs

| Spec | Relationship |
|------|-------------|
| LANG01 | Defines `IIRModule` / `IIRInstr` — the data types this spec transforms. |
| LANG16 | Defines the IIR heap opcode family (`alloc`, `field_load`, `field_store`, `is_null`, `safepoint`). LANG31 is the first consumer of these ops on compiled Twig programs. |
| LANG22 | Typing spectrum — `iir-type-checker` promotes `"any"` → concrete types before builtin lowering. |
| LANG29 | The four IIR direct backends. LANG31 extends their heap-op reject lists into accept lists. |
| LANG30 | `iir-codegen-adapters` — the unified dispatch crate. LANG31 pipeline crates use it or can be added as new entries. |
| TW03 | Twig full Lisp surface + GC. TW03 specifies the language semantics; LANG31 specifies the IIR-level mechanism. TW03 originally targeted `compiler-ir` (deprecated); LANG31 supersedes that design for the IIR path. |
| BEAM01 | Real BEAM execution tests. LANG31 integration tests extend this to the IIR path. |
| CLR01 | Real dotnet conformance. Same relationship. |

---

## Spec-first prerequisites

Before implementation begins, confirm:

- `interpreter-ir::is_heap()` recognises `"alloc"`, `"field_load"`, `"field_store"`,
  `"is_null"` (confirmed in existing codebase).
- `IIRInstr.may_alloc` is set to `true` for `"alloc"` and `"box"` (confirmed).
- `type_hint` accepts `ref<T>` strings (confirmed — `is_concrete_type` returns `true`
  for any string starting with `"ref<"` and ending with `">"`).

---

## Phase 1 — Numeric builtin lowering (no GC)

Phase 1 gets arithmetic and comparison Twig programs running end-to-end on all four
targets without touching GC.  The only new crate in this phase is
`iir-builtin-lowering`.

### 1.1  `iir-builtin-lowering`

**Crate path:** `code/packages/rust/iir-builtin-lowering/`

**Dependencies:** `interpreter-ir` (no `compiler-ir`, no backend crates).

**Public API:**

```rust
/// Lower all recognized `call_builtin` instructions in `module` to typed IIR ops.
///
/// Instructions whose builtin name is not in the lowering table are left unchanged
/// so that later passes (e.g. the vm-core dispatcher) can still handle them.
pub fn lower_builtins(module: &mut IIRModule);

/// Same as `lower_builtins`, but returns a fresh `IIRModule` and leaves the
/// original untouched.
pub fn lower_builtins_cloned(module: &IIRModule) -> IIRModule;

/// Errors emitted when a call_builtin cannot be lowered because operand count or
/// type hints are inconsistent.
pub enum BuiltinLoweringError { ... }
```

**Lowering table — numeric (Phase 1):**

Every arithmetic/comparison `call_builtin` whose operands have been typed by
`iir-type-checker` is rewritten in-place.  The `srcs` layout of
`call_builtin "<name>", ...args` is: `srcs[0] = Operand::Var("<name>")`, then
operand args.  After type-checking, the dest and src type hints are concrete.

| `call_builtin` name | Operand count | Replacement op | Notes |
|---------------------|:-------------:|---------------|-------|
| `"+"` | 2 | `"add"` | |
| `"-"` | 2 | `"sub"` | |
| `"*"` | 2 | `"mul"` | |
| `"/"` | 2 | `"div"` | |
| `"%"` | 2 | `"mod"` | |
| `"neg"` | 1 | `"neg"` | |
| `"="` | 2 | `"cmp_eq"` | |
| `"!="` | 2 | `"cmp_ne"` | |
| `"<"` | 2 | `"cmp_lt"` | |
| `"<="` | 2 | `"cmp_le"` | |
| `">"` | 2 | `"cmp_gt"` | |
| `">="` | 2 | `"cmp_ge"` | |
| `"and"` | 2 | `"and"` | bitwise AND on bool operands |
| `"or"` | 2 | `"or"` | bitwise OR on bool operands |
| `"not"` | 1 | `"not"` | bitwise NOT on bool |
| `"shl"` | 2 | `"shl"` | |
| `"shr"` | 2 | `"shr"` | |
| `"xor"` | 2 | `"xor"` | |

**Instruction rewrite algorithm (numeric):**

```
For each instruction I in function F:
  if I.op != "call_builtin": continue
  name = I.srcs[0] (must be a string literal var named after the builtin)
  args = I.srcs[1..]
  if name not in numeric-table: continue
  entry = numeric-table[name]
  if args.len() != entry.arity: emit BuiltinLoweringError::WrongArity
  replace I with:
    op    = entry.replacement_op
    dest  = I.dest
    srcs  = args              ← drop the builtin-name src[0]
    type_hint = I.type_hint   ← preserved from the original
    may_alloc = false
```

The `type_hint` field is already set to a concrete type by `iir-type-checker` before
this pass runs; no inference is needed here.

**What is left unchanged by Phase 1:**

- `call_builtin "cons"`, `"car"`, `"cdr"`, `"null?"`, `"pair?"` — Phase 2.
- `call_builtin "make_closure"`, `"apply_closure"`, `"make_builtin_closure"` — tracked by BEAM02 / CLR02.
- `call_builtin "global_set"`, `"global_get"` — tracked by LANG27.
- `call_builtin "make_symbol"`, `"make_nil"`, `"print"` — tracked by LANG27.
- Any unrecognised name — left unchanged.

### 1.2  Pipeline crates — Phase 1

Four thin crates, one per target.  Each follows the same pattern:

```rust
// e.g. twig-to-beam/src/lib.rs
pub fn compile_twig_to_beam(source: &str, module_name: &str)
    -> Result<Vec<u8>, TwigToBeamError>
{
    // 1. Parse + compile Twig source to IIR
    let mut module = twig_ir_compiler::compile_source(source, module_name)?;
    // 2. Promote type hints
    iir_type_checker::infer_and_check(&mut module)?;
    // 3. Lower call_builtin → typed IIR ops
    iir_builtin_lowering::lower_builtins(&mut module);
    // 4. Validate and lower to BEAM bytecode
    let beam_module = iir_to_beam::lower_iir_to_beam(&module)?;
    // 5. Encode to bytes
    Ok(ir_to_beam::encode_beam(&beam_module))
}
```

| Crate | Output type | Encoding crate |
|-------|-------------|----------------|
| `twig-to-beam` | `Vec<u8>` (BEAM .beam chunk) | `ir-to-beam::encode_beam` |
| `twig-to-wasm` | `Vec<u8>` (.wasm binary) | `wasm-module-encoder::encode_module` |
| `twig-to-jvm` | `Vec<u8>` (.class file) | `jvm-class-file::build_minimal_class_file` |
| `twig-to-cil` | `CILProgramArtifact` | `ir-to-cil-bytecode::builder` |

Each crate has an error type that wraps the possible failures at each stage:

```rust
pub enum TwigToBeamError {
    Compile(twig_ir_compiler::TwigCompileError),
    TypeCheck(iir_type_checker::TypeCheckError),
    Lower(iir_to_beam::LoweringError),
}
```

### 1.3  Phase 1 validation: the `call_builtin` contract

Numeric `call_builtin` instructions can only be lowered if their operands are typed.
`iir-builtin-lowering` must run **after** `iir-type-checker`; running it before risks
leaving `"any"`-typed arithmetic in the module, which the backends reject.

Execution order enforced in pipeline crates:
```
compile_source → infer_and_check → lower_builtins → validate_for_<backend>
```

If `lower_builtins` encounters a numeric `call_builtin` whose type hints are still
`"any"`, it emits a `BuiltinLoweringError::UntypedBuiltin` and aborts.  This is a
programming error in the pipeline, not a user error.

### 1.4  Phase 1 acceptance criterion

The following Twig program compiles to bytecode and executes correctly (returns 120)
on all four real runtimes:

```scheme
(define (fact n)
  (if (= n 0)
      1
      (* n (fact (- n 1)))))

(fact 5)
```

---

## Phase 2 — Heap builtin lowering (native GC)

Phase 2 adds `cons`, `car`, `cdr`, `null?`, and `pair?` to the lowering table.  These
require each backend to be extended with heap allocation primitives that use the target
VM's native garbage collector.

### 2.1  IIR representation of a Lisp pair

`iir-builtin-lowering` converts heap builtins to combinations of the LANG16 heap ops:

```
"alloc"       — allocate a typed GC object
"field_store" — initialise a field of an allocated object
"field_load"  — read a field of a GC object
"is_null"     — test whether a value is the nil sentinel
```

The type hint `"ref<LispyPair>"` identifies a two-field heap object whose fields are
themselves GC-managed references (`"ref<any>"`).

**`(cons head tail)` lowering:**

```
; Before (call_builtin)
%result = call_builtin "cons", %head, %tail   type_hint="any"

; After (IIR heap ops)
%cell   = alloc        type_hint="ref<LispyPair>"  may_alloc=true  srcs=[]
          field_store   srcs=[Var(%cell), Int(0), Var(%head)]
          field_store   srcs=[Var(%cell), Int(1), Var(%tail)]
; %cell becomes %result
```

**`(car p)` lowering:**

```
; Before
%result = call_builtin "car", %p   type_hint="any"

; After
%result = field_load   srcs=[Var(%p), Int(0)]   type_hint="ref<any>"
```

**`(cdr p)` lowering:**

```
; Before
%result = call_builtin "cdr", %p   type_hint="any"

; After
%result = field_load   srcs=[Var(%p), Int(1)]   type_hint="ref<any>"
```

**`(null? x)` lowering:**

```
; Before
%result = call_builtin "null?", %x   type_hint="any"

; After
%result = is_null   srcs=[Var(%x)]   type_hint="bool"
```

**`(pair? x)` lowering:**

```
; Before
%result = call_builtin "pair?", %x   type_hint="any"

; After (is_null checks for nil; pair? = not nil AND is a ref<LispyPair>)
; Simplified: emit a call_builtin "is_pair" placeholder for now —
; backend lowers to a type-tag check.  Full implementation in a later spec.
```

`pair?` involves a runtime type tag check and is deferred to LANG32 (symbol/type
tagging).  `null?` is implemented in Phase 2.

**`make_nil` lowering:**

The Twig compiler emits `call_builtin "make_nil"` for the empty list.  This becomes
a load of the nil sentinel:

```
%nil = const 0   type_hint="ref<LispyPair>"
; nil is represented as a null ref — is_null(%nil) == true
```

The exact nil representation is backend-specific (see §2.2).

### 2.2  Per-backend heap op lowering

#### BEAM

BEAM's term model natively includes list cons cells.  No struct registration is needed.

**Nil:** The BEAM atom `[]` (empty list).  Intern `"[]"` in the atom table; operand tag
`{a, nil_idx}`.

**`alloc ref<LispyPair>` + 2 `field_store`:** Pattern-matched as a unit and emitted as
a single `put_list` instruction:

```
put_list  %head  %tail  %dst
; opcode byte: 69  (OTP_25 BEAM format)
```

The lowering pass peeks ahead: when it sees `alloc ref<LispyPair>` followed immediately
by `field_store %cell 0 %h` and `field_store %cell 1 %t`, it fuses all three into
`put_list %h, %t, %cell`.

**`field_load %result, %pair, 0` (car):** `get_list %pair  %result  %ignored`
**`field_load %result, %pair, 1` (cdr):** `get_list %pair  %ignored  %result`

New opcode constants needed in `iir-to-beam` (not in encoder; just constant definitions
in `lower.rs`):

```rust
const OP_PUT_LIST:         u8 = 69;
const OP_GET_LIST:         u8 = 65;
const OP_IS_NIL:           u8 = 52;
const OP_IS_NONEMPTY_LIST: u8 = 56;
```

**`is_null %result, %x`:** `is_nil {f, fail_label}  %x` — if x is `[]`, continue;
else jump.  Emit as synthesised comparison (same pattern as existing `cmp_eq`
synthesis): move `true` into `%result`, emit `is_nil` conditional, move `false` on
failure path.

**GC:** BEAM's per-process copying GC handles all list cells automatically.  No extra
work.

---

#### JVM

JVM objects are managed by the JVM's generational GC.  To avoid emitting inner class
definitions (which requires significant `jvm-class-file` extension), Lisp pairs are
represented as **2-element `Object[]` arrays**:

```
cons head tail   →   new Object[2]   then   arr[0]=head, arr[1]=tail
car  pair        →   pair[0]
cdr  pair        →   pair[1]
null?            →   pair == null
nil              →   null
```

This approach:
- requires only `anewarray`, `aaload`, `aastore`, `aconst_null`, `ifnull`/`ifnonnull` bytecodes;
- uses the standard JVM GC without any class metadata;
- is safe and idiomatic in the JVM object model.

**New bytecodes** needed in `jvm-class-file` (emitted as raw bytes in the `code` vec,
all standard JVM bytecode):

| Bytecode | Hex | Description |
|----------|-----|-------------|
| `aconst_null` | `0x01` | Push `null` reference |
| `anewarray` + type index | `0xBD` + u16 | Allocate `T[]` |
| `aaload` | `0x32` | Load `Object` from array |
| `aastore` | `0x53` | Store `Object` into array |
| `ifnull` + offset | `0xC6` + i16 | Branch if null |
| `ifnonnull` + offset | `0xC7` + i16 | Branch if non-null |
| `checkcast` + type index | `0xC0` + u16 | Cast (for typed unboxing) |
| `dup` | `0x59` | Duplicate top of stack |
| `swap` | `0x5F` | Swap top two stack values |

The `jvm-class-file` constant pool builder needs a new `Class` entry for
`"[Ljava/lang/Object;"` (the array type descriptor), added automatically when
`anewarray` is emitted.

**`alloc ref<LispyPair>` lowering sequence:**

```
iconst_2
anewarray  java/lang/Object       ; stack: [Object[]]
; store into local reg %cell
astore     <cell_local>
```

**`field_store %cell 0 %h`:**

```
aload      <cell_local>
iconst_0
aload      <h_local>
aastore
```

**`field_load %result %cell 0` (car):**

```
aload      <cell_local>
iconst_0
aaload
astore     <result_local>
```

**`is_null %result %x`:**

```
; synthesise: %result = (%x == null)
iconst_1                    ; assume true
aload      <x_local>
ifnull     +4               ; if null, keep 1
iconst_0                    ; else store 0
istore     <result_local>
```

**GC:** `Object[]` instances are first-class JVM heap objects.  The JVM GC collects
them automatically.

---

#### CLR

CLR mirrors the JVM approach: Lisp pairs are `object[]` (2-element managed arrays).
CLR's generational GC handles them.

```
cons head tail   →   new object[2], arr[0]=head, arr[1]=tail
car  pair        →   (object)pair[0]
cdr  pair        →   (object)pair[1]
null?            →   pair == null
nil              →   null
```

**New typed emit methods** needed in `CILBytecodeBuilder` (all use `emit_raw` internally):

```rust
impl CILBytecodeBuilder {
    /// `ldnull` — push null reference (0x14)
    pub fn emit_ldnull(&mut self);
    /// `newarr <type_token>` — allocate 1-D managed array (0x8D + u32)
    pub fn emit_newarr(&mut self, type_token: u32);
    /// `ldelem.ref` — load object reference from array (0xA2)
    pub fn emit_ldelem_ref(&mut self);
    /// `stelem.ref` — store object reference into array (0xA4)
    pub fn emit_stelem_ref(&mut self);
    /// `ldnull` + `ceq` synthesis for null check
    pub fn emit_is_null_check(&mut self);
    /// `brfalse.s` — branch if false/null/zero, short form (0x2C + i8)
    pub fn emit_brfalse_s(&mut self, offset: i8);
    /// `brtrue.s` — branch if true/non-null/non-zero, short form (0x2D + i8)
    pub fn emit_brtrue_s(&mut self, offset: i8);
}
```

**Token for `object[]`:** The type token for `[mscorlib]System.Object` must be available
in the CIL token space.  The `CILTokenProvider` trait gains a method:

```rust
fn object_array_type_token(&self) -> u32;
```

The default simulation token provider returns a stable sentinel value.

**`alloc ref<LispyPair>` lowering sequence:**

```
ldc.i4.2
newarr     <object_array_type_token>
stloc      <cell_local>
```

**`field_store %cell 0 %h`:**

```
ldloc      <cell_local>
ldc.i4.0
ldloc      <h_local>
stelem.ref
```

**`field_load %result %cell 0` (car):**

```
ldloc      <cell_local>
ldc.i4.0
ldelem.ref
stloc      <result_local>
```

**`is_null %result %x`:**

```
ldloc      <x_local>
ldnull
ceq                      ; 1 if equal (null), 0 otherwise
stloc      <result_local>
```

**GC:** `object[]` is a managed type.  CLR GC reclaims unreachable instances.

---

#### WASM (WasmGC)

The WebAssembly GC proposal (standardised 2023, shipping in V8 ≥ Chrome 119, 
SpiderMonkey ≥ Firefox 120, wasmtime ≥ 14.0) adds:
- Struct and array heap types with native GC semantics.
- `ref.null`, `struct.new`, `struct.get`, `struct.set`, `ref.is_null`.

Lisp pairs are a WasmGC struct:

```wat
(type $LispyPair (struct
  (field $head (mut (ref null any)))
  (field $tail (mut (ref null any)))))
```

Integers remain `i32` on the stack; when stored in `$head`/`$tail` they are boxed via
`i31.new` (WasmGC i31ref).  Nil is `ref.null none` cast to `(ref null any)`.

**wasm-types extensions (minimal WasmGC subset):**

```rust
// New value type variants
pub enum ValueType {
    I32, I64, F32, F64,      // existing
    // WasmGC additions:
    Anyref,                  // (ref null any) — nullable any reference
    I31ref,                  // (ref i31) — integer reference
    StructRef(u32),          // (ref null $T) — nullable concrete struct ref
    EqRef,                   // (ref null eq) — equatable reference
}

// New type section entry
pub enum CompositeType {
    Func(FuncType),          // existing
    Struct(StructType),      // new
}

pub struct StructType {
    pub fields: Vec<FieldType>,
}

pub struct FieldType {
    pub val_type: StorageType,
    pub mutable: bool,
}

pub enum StorageType {
    Val(ValueType),
    PackedI8,
    PackedI16,
}
```

**New GC instruction bytes** (encoded as raw `u8` in `FunctionBody.code`):

```
ref.null  any     : 0xD0 0x6E
ref.is_null       : 0xD1
i31.new           : 0xFB 0x1C
struct.new $T     : 0xFB 0x00 <typeindex:u32>
struct.get $T $F  : 0xFB 0x02 <typeindex:u32> <fieldidx:u32>
struct.set $T $F  : 0xFB 0x04 <typeindex:u32> <fieldidx:u32>
any.convert_extern: 0xFB 0x1A  (for externref ↔ anyref conversion if needed)
```

The `iir-to-wasm` lowering will define helper constants for these byte sequences.

**`alloc ref<LispyPair>` + 2 `field_store` → `struct.new`:**

Because WasmGC's `struct.new` initialises all fields at construction, the
`alloc + field_store × 2` sequence is fused by the lowering pass:

```wasm
;; cons head tail — stack: [... head tail]
local.get $head_boxed   ;; i31.new or anyref coercion if integer
local.get $tail_boxed
struct.new $LispyPair   ;; pops tail, head; pushes (ref $LispyPair)
local.set $cell
```

**`field_load %result %cell 0` (car):**

```wasm
local.get $cell
struct.get $LispyPair 0   ;; pushes (ref null any)
local.set $result
```

**`is_null %result %x`:**

```wasm
local.get $x
ref.is_null               ;; pushes i32 (0 or 1)
local.set $result
```

**Nil representation:**

```wasm
ref.null none             ;; typed null — (ref null none)
```

**wasm-module-encoder changes:**

- Add `encode_struct_type()` to the type section encoder.
- Add `encode_gc_instruction()` helper for GC opcodes (0xFB prefix family).
- The module's type section gains an entry for `$LispyPair` when the module
  contains any `ref<LispyPair>` typed instruction.

**GC:** WasmGC objects are collected by the host engine (V8/SpiderMonkey/wasmtime).
No linear-memory sweep needed.

---

### 2.3  `iir-to-*` backend changes for heap ops

All four backends currently reject heap IIR opcodes in their `validate_for_<backend>()`
functions.  Phase 2 changes this:

| Opcode | BEAM | JVM | CLR | WASM |
|--------|:----:|:---:|:---:|:----:|
| `alloc` (type_hint `"ref<LispyPair>"`) | ✅ | ✅ | ✅ | ✅ |
| `field_load` | ✅ | ✅ | ✅ | ✅ |
| `field_store` | ✅ | ✅ | ✅ | ✅ |
| `is_null` | ✅ | ✅ | ✅ | ✅ |
| `alloc` (other types) | ❌ | ❌ | ❌ | ❌ |
| `box` / `unbox` / `safepoint` | ❌ | ❌ | ❌ | ❌ |

Unrecognised `alloc` types (anything other than `"ref<LispyPair>"`) produce a
`LoweringError::UnsupportedHeapType`.

### 2.4  Phase 2 acceptance criterion

The following Twig program compiles and returns 3 on all four runtimes:

```scheme
(define (length xs)
  (if (null? xs)
      0
      (+ 1 (length (cdr xs)))))

(length (cons 1 (cons 2 (cons 3 nil))))
```

---

## Integration test harnesses

Each pipeline crate ships an integration test file at `tests/integration.rs` that:

1. Compiles a Twig source string to bytecode.
2. Writes the bytecode to a `tempfile`.
3. Spawns the appropriate runtime with `std::process::Command`.
4. Asserts stdout and exit code.

### Runtime invocations

```rust
// BEAM
Command::new("erl")
    .args(["-noshell", "-s", "<module>", "main", "-s", "init", "stop"])
    .assert_stdout("120\n");

// WASM (via wasmtime CLI)
Command::new("wasmtime").arg(wasm_path).assert_stdout("120\n");

// JVM
Command::new("java").args(["-cp", tmpdir, "<ClassName>"]).assert_stdout("120\n");

// CLR
Command::new("dotnet").args(["run", "--project", tmpdir]).assert_stdout("120\n");
```

Tests are gated behind a Cargo feature flag `integration-real-vm` so CI runs them only
when the real runtimes are available in the PATH:

```toml
[features]
integration-real-vm = []
```

CI jobs add the `--features integration-real-vm` flag and install `erl`, `java`,
`wasmtime`, and `dotnet` via the step `mise install`.

### Required test programs (both phases)

| Test | Expected output |
|------|----------------|
| `(+ 1 2)` | `3` |
| `(define (fact n) (if (= n 0) 1 (* n (fact (- n 1))))) (fact 5)` | `120` |
| `(define (fib n) (if (< n 2) n (+ (fib (- n 1)) (fib (- n 2))))) (fib 10)` | `55` |
| `(length (cons 1 (cons 2 (cons 3 nil))))` (Phase 2) | `3` |
| `(car (cons 42 nil))` (Phase 2) | `42` |

---

## New crates

| Crate | Path | New / Extended |
|-------|------|---------------|
| `iir-builtin-lowering` | `code/packages/rust/iir-builtin-lowering/` | New |
| `twig-to-beam` | `code/packages/rust/twig-to-beam/` | New |
| `twig-to-wasm` | `code/packages/rust/twig-to-wasm/` | New |
| `twig-to-jvm` | `code/packages/rust/twig-to-jvm/` | New |
| `twig-to-cil` | `code/packages/rust/twig-to-cil/` | New |

**Extended:**

| Crate | What changes |
|-------|-------------|
| `wasm-types` | Add `ValueType::Anyref`, `ValueType::I31ref`, `ValueType::StructRef`, `CompositeType::Struct`, `StructType`, `FieldType` |
| `wasm-module-encoder` | Encode GC type section, encode GC instruction opcodes |
| `jvm-class-file` | Add `anewarray`, `aaload`, `aastore`, `aconst_null`, `ifnull`, `ifnonnull` bytecode helpers |
| `ir-to-cil-bytecode` | Add `emit_ldnull`, `emit_newarr`, `emit_ldelem_ref`, `emit_stelem_ref`, `emit_brfalse_s`, `emit_brtrue_s` to `CILBytecodeBuilder` |
| `iir-to-beam` | Accept `alloc(ref<LispyPair>)`, `field_load`, `field_store`, `is_null`; add `put_list`/`get_list`/`is_nil` opcode constants |
| `iir-to-wasm` | Accept heap ops; add WasmGC struct/ref instruction emission |
| `iir-to-jvm-class-file` | Accept heap ops; emit `Object[]` allocation sequence |
| `iir-to-cil-bytecode` | Accept heap ops; emit `object[]` allocation sequence |

---

## Internal layout (each new crate)

```
src/
  lib.rs       — public API + module docs
  pipeline.rs  — compile_twig_to_<target>() implementation
  error.rs     — error type
tests/
  test_pipeline.rs     — unit tests (no real VM needed)
  integration.rs       — real-VM integration tests (feature-gated)
```

`iir-builtin-lowering` has its own layout:

```
src/
  lib.rs        — public API
  numeric.rs    — numeric/comparison builtin lowering
  heap.rs       — heap builtin lowering (Phase 2)
  error.rs
tests/
  test_lowering.rs   — ≥ 40 tests, ≥ 85% coverage
```

---

## Workspace and feature flags

`code/packages/rust/Cargo.toml` additions:

```toml
[workspace]
members = [
  # ... existing members ...
  "iir-builtin-lowering",
  "twig-to-beam",
  "twig-to-wasm",
  "twig-to-jvm",
  "twig-to-cil",
]
```

---

## Workflow

1. Feature branch: `feat/lang31-twig-e2e-native-gc`
2. Commit 1: this spec + updated TW03 (IIR path replaces compiler-ir path)
3. Commit 2: `iir-builtin-lowering` (numeric only, Phase 1)
4. Commit 3: `twig-to-{beam,wasm,jvm,cil}` pipeline crates (Phase 1)
5. Commit 4: Phase 1 integration tests pass on real VMs
6. Commit 5: wasm-types + wasm-module-encoder WasmGC extension
7. Commit 6: jvm-class-file + ir-to-cil-bytecode heap op additions
8. Commit 7: `iir-builtin-lowering` heap Phase 2 + backend heap extensions
9. Commit 8: Phase 2 integration tests pass on real VMs
10. Commit 9: workspace `cargo build --workspace` clean + PR

---

## Verification

```bash
# Phase 1 — numeric only
cargo test -p iir-builtin-lowering  -- --nocapture
cargo test -p twig-to-beam          -- --nocapture
cargo test -p twig-to-wasm          -- --nocapture
cargo test -p twig-to-jvm           -- --nocapture
cargo test -p twig-to-cil           -- --nocapture

# Phase 1 — real VMs
cargo test -p twig-to-beam --features integration-real-vm -- --nocapture
cargo test -p twig-to-wasm --features integration-real-vm -- --nocapture
cargo test -p twig-to-jvm  --features integration-real-vm -- --nocapture
cargo test -p twig-to-cil  --features integration-real-vm -- --nocapture

# Phase 2 — heap + GC
cargo test -p iir-builtin-lowering --features heap -- --nocapture
cargo test -p twig-to-beam --features integration-real-vm,heap -- --nocapture
# ... etc.

# Full workspace
cargo build --workspace
```

---

## Definition of done

LANG31 is complete when:

- `cargo test -p iir-builtin-lowering` passes with ≥ 85% coverage.
- The `fact 5 = 120` and `fib 10 = 55` programs run correctly under real `erl`,
  `wasmtime`, `java`, and `dotnet`.
- The `length (cons ...)` program runs correctly under all four runtimes (Phase 2).
- `cargo build --workspace` is clean with no warnings.
- Each of the five new crates has a `README.md` and `CHANGELOG.md`.
- The TW03 spec is updated to reflect the IIR path (the `compiler-ir` heap-op
  additions described there are superseded by this spec).
