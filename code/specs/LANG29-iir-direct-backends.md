# LANG29 — Direct IIR Backends (BEAM, WASM, JVM, CLR)

## Context

LANG27 defines the long-term host-VM backend architecture: an `iir-host-lowering` crate with
full `HostVmBackend` trait support, ALGOL-style activation frames, closure lifting, descriptor
layouts, nonlocal-goto unwind, and a unified `lang_host_compile(module, target)` API.  That full
architecture is the target destination.

LANG29 delivers the **first usable increment**: four direct IIR backends that lower a typed
`IIRModule` to target bytecode without routing through the deprecated `compiler-ir::IrProgram`
layer.  The `compiler-ir` / `IrOp` layer is deprecated; all new compilation paths consume
`IIRModule` / `IIRInstr` from `interpreter-ir`.

## Pipeline position

```text
Language frontend (Tetrad, BASIC, Nib, Brainfuck, …)
    ↓  IIRModule (type_status = Untyped / PartiallyTyped)
[ vm-core: interpret + profile ]
    ↓  IIRModule (observed_type / SlotState populated)
[ jit-core / aot-core: specialise ]
    ↓  IIRModule (type_status = FullyTyped, all type_hints concrete)
    ↓
┌─────────────────────────────────────────────────────────────┐
│  LANG29 — Direct IIR Backends                               │
│  iir-to-beam           →  BEAMModule   → .beam binary       │
│  iir-to-wasm           →  WasmModule   → .wasm binary       │
│  iir-to-jvm-class-file →  JvmClassFile → .class bytes       │
│  iir-to-cil-bytecode   →  CILProgramArtifact → CIL methods  │
└─────────────────────────────────────────────────────────────┘
```

Target-specific *encoding* infrastructure is reused from existing crates; only the *lowering
pass* (IIR → backend IR) is new.  Each crate depends on `interpreter-ir` but NOT on
`compiler-ir`.

## V1 scope: typed scalar core

V1 handles `IIRModule`s where every `IIRFunction` has `type_status == FullyTyped` and every
`IIRInstr::type_hint` is a concrete numeric or boolean type.  Programs with `"any"`,
`"polymorphic"`, `"str"`, or `"ref<T>"` type hints are rejected at validation time.

### Supported IIR opcodes (all four backends)

| IIR op | V1 semantics |
|--------|-------------|
| `const` (Int / Bool) | Load integer or boolean immediate into destination variable |
| `add`, `sub`, `mul`, `div`, `mod` | Integer arithmetic on two variables |
| `neg` | Integer unary negation |
| `and`, `or`, `xor`, `not` | Bitwise operations |
| `shl`, `shr` | Bit shifts |
| `cmp_eq`, `cmp_ne`, `cmp_lt`, `cmp_le`, `cmp_gt`, `cmp_ge` | Integer comparison → bool (0 / 1) |
| `label` | Block label definition |
| `jmp` | Unconditional branch |
| `jmp_if_true`, `jmp_if_false` | Conditional branch on a boolean variable |
| `ret` | Return a value from the current function |
| `ret_void` | Return void |
| `call` | Call another `IIRFunction` in the same module by name |
| `load_reg`, `store_reg` | Explicit register copy (rarely emitted by frontends, must still lower) |
| `type_assert` | Lowered to **nothing** — type guard already fired in vm-core |

### WASM and JVM only (BEAM / CLR: unsupported in V1)

| IIR op | V1 status |
|--------|-----------|
| `const` (Float f64 / f32) | WASM + JVM only.  BEAM float requires Erlang bignum/float terms (V2). CLR needs properly typed method signature (V2). |

### Unsupported in V1 (validation error on any backend)

| IIR op | Reason |
|--------|--------|
| `call_builtin` | No stdlib mapping yet (LANG26/LANG27 future) |
| `io_in`, `io_out` | Backend-specific I/O (WASI, Erlang I/O, etc.) |
| `cast` | Type coercion not in V1 |
| `load_mem`, `store_mem` | BEAM has no linear memory; uniform policy deferred to LANG27 |
| `alloc`, `box`, `unbox`, `field_load`, `field_store`, `is_null`, `safepoint` | Heap / GC (LANG16) not in V1 |

### Rejected IIR features (validation errors)

| Feature | Error |
|---------|-------|
| `type_hint == "any"` | `ValidationError::UntypedInstruction` |
| `type_hint == "polymorphic"` | `ValidationError::PolymorphicInstruction` |
| `type_hint == "str"` | `ValidationError::UnsupportedType("str")` |
| `type_hint` starts with `"ref<"` | `ValidationError::HeapRefType` |
| Function with zero instructions | `ValidationError::EmptyFunction(name)` |
| Module with zero functions | `ValidationError::EmptyModule` |
| Unsupported op encountered | `ValidationError::UnsupportedOp { fn_name, op }` |
| BEAM/CLR: float const encountered | `ValidationError::UnsupportedType("f64")` (BEAM/CLR only) |

---

## New packages (four)

```
code/packages/rust/
├── iir-to-beam/
├── iir-to-wasm/
├── iir-to-jvm-class-file/
└── iir-to-cil-bytecode/
```

Each package has the same internal layout:

```
iir-to-<backend>/
├── Cargo.toml
├── BUILD
├── CHANGELOG.md
├── README.md
└── src/
    ├── lib.rs       — re-exports + module-level doc
    ├── validate.rs  — validate_for_<backend>() + error types
    ├── lower.rs     — lower_iir_to_<backend>() + _IIRLowerer
    └── codegen.rs   — IIR<Backend>CodeGenerator (LANG20 CodeGenerator<IIRModule, Artifact>)
tests/
└── test_backend.rs  — ≥ 40 tests
```

---

## Common architecture

### Validation function

```rust
pub fn validate_for_<backend>(module: &IIRModule) -> Vec<String>
```

Returns a list of human-readable errors; empty = valid.  Checks:
1. Module has at least one function.
2. Each function has at least one instruction.
3. No `type_hint == "any"` / `"polymorphic"` / `"str"` / `"ref<T>"`.
4. No unsupported opcodes.
5. (BEAM/CLR) No float constants.

### Lowering function

```rust
pub fn lower_iir_to_<backend>(
    module: &IIRModule,
    config: &IIR<Backend>Config,
) -> Result<<Artifact>, IIR<Backend>Error>
```

Calls `validate_for_<backend>` first; returns `Err(ValidationFailed(errors))` if invalid.

### Config struct

```rust
pub struct IIR<Backend>Config {
    pub module_name: String,   // e.g. "hello" for BEAM atom, class name for JVM/CLR, etc.
}
impl Default for IIR<Backend>Config {
    fn default() -> Self { Self { module_name: "iir_module".to_string() } }
}
```

### Error enum

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum IIR<Backend>Error {
    ValidationFailed(Vec<String>),
    UnsupportedOp     { function: String, op: String },
    UnsupportedType   { function: String, type_hint: String },
    UndefinedLabel    { function: String, label: String },
    UndefinedVariable { function: String, name: String },
    InvalidOperand    { function: String, detail: String },
}
```

### CodeGenerator adapter (LANG20 protocol)

```rust
pub struct IIR<Backend>CodeGenerator {
    config: IIR<Backend>Config,
}

// Satisfies codegen_core::CodeGenerator<IIRModule, <Artifact>>
impl IIR<Backend>CodeGenerator {
    pub fn new(module_name: impl Into<String>) -> Self { … }
    pub fn default_name()               -> Self { Self::new("iir_module") }
    pub fn name(&self) -> &str          { "iir-<backend>" }
    pub fn validate(&self, ir: &IIRModule) -> Vec<String>  { validate_for_<backend>(ir) }
    pub fn generate(&self, ir: &IIRModule) -> <Artifact>   {
        lower_iir_to_<backend>(ir, &self.config)
            .expect("generate() called on validated module")
    }
}
```

### Common lowering algorithm (two-pass per IIRFunction)

**Pass 1 — collect variables → assign integer register indices**

```
reg_map: HashMap<String, usize>
next_reg: usize = 0

// Parameters come first (index 0 .. params.len()-1)
for (name, _type) in function.params.iter():
    reg_map.insert(name, next_reg++)

// Walk instructions for dests and Var srcs
for instr in function.instructions.iter():
    if let Some(dest) = &instr.dest:
        reg_map.entry(dest).or_insert_with(|| { let r = next_reg; next_reg++; r })
    for src in &instr.srcs:
        if Operand::Var(name) = src:
            reg_map.entry(name).or_insert_with(|| { let r = next_reg; next_reg++; r })
```

Result: every named variable (params + dests + Var srcs) has a stable integer index.

**Pass 2 — emit backend instructions**

Dispatch on `instr.op` string, resolve variable names through `reg_map`, emit
backend-specific instructions.  Details per backend below.

### Synthetic-label counter

Comparison synthesis (BEAM) and JVM/CLR jump backpatching require unique synthetic label
IDs per lowering unit.  Each `_IIRLowerer` holds a monotonically incrementing
`next_synth_label: u32` counter.  Synthetic labels are named `_synth_N` and never
conflict with user-defined label names.

---

## Backend 1: `iir-to-beam`

### Cargo dependencies

```toml
[dependencies]
interpreter-ir = { path = "../interpreter-ir" }
ir-to-beam     = { path = "../ir-to-beam" }   # reuse BEAMModule / BEAMInstruction / encode_beam
codegen-core   = { path = "../codegen-core" }
```

Note: `ir-to-beam` transitively pulls in `compiler-ir`, but `iir-to-beam` never touches
any `IrProgram` / `IrOp` type.  This transitive dependency will be eliminated when the BEAM
encoder is factored into a standalone `beam-encoder` crate (LANG27).

### Artifact type: `BEAMModule` (from `ir_to_beam::encoder`)

### Register model

IIR variables → BEAM **x-registers**.  Parameters at x0..x(N-1).  All other variables
at x(N)..  A single `reg_map: HashMap<String, u8>` maps names to x-register indices.
Register index is `u8`; modules with > 255 variables are rejected at lowering time with
`InvalidOperand`.

### Multi-function model

Each `IIRFunction` becomes one exported BEAM function in the module.

A global `label_counter: u32` assigns unique BEAM label numbers across all functions.
Layout per function `F` with arity `A`:

```
{label, {u, L_fi_N}}             ← func_info header label
{func_info, {a, Module}, {a, F}, {u, A}}.
{label, {u, L_entry_N}}          ← exported entry label
  … translated instructions …
```

Label numbers start at 1 globally.  The ExportTable entry for function `F/A` references
`L_entry_N`.

### BEAM import table

Only BIFs that appear in the lowered module are added to the import table.  The
`_ImportTable` helper interns each `(module_atom, function_atom, arity)` tuple and
returns a 0-based index.

Used BIFs:

| Erlang BIF | Used by |
|-----------|---------|
| `erlang:+/2` | `add` |
| `erlang:-/2` | `sub` |
| `erlang:'*'/2` | `mul` |
| `erlang:div/2` | `div` |
| `erlang:rem/2` | `mod` |
| `erlang:-/1` | `neg` |
| `erlang:band/2` | `and` |
| `erlang:bor/2` | `or` |
| `erlang:bxor/2` | `xor` |
| `erlang:bnot/1` | `not` |
| `erlang:bsl/2` | `shl` |
| `erlang:bsr/2` | `shr` |

### Opcode mapping

| IIR op | BEAM instruction(s) | Notes |
|--------|---------------------|-------|
| `const` Int v → rd | `{move, {i,v}, {x,rd}}` | |
| `const` Bool true → rd | `{move, {i,1}, {x,rd}}` | |
| `const` Bool false → rd | `{move, {i,0}, {x,rd}}` | |
| `add` r1,r2 → rd | `{gc_bif2, {f,0}, {u,live}, erlang:+/2, {x,r1}, {x,r2}, {x,rd}}` | `live` = next_reg |
| `sub` r1,r2 → rd | gc_bif2 `erlang:-/2` | |
| `mul` r1,r2 → rd | gc_bif2 `erlang:'*'/2` | |
| `div` r1,r2 → rd | gc_bif2 `erlang:div/2` | integer division |
| `mod` r1,r2 → rd | gc_bif2 `erlang:rem/2` | |
| `neg` r → rd | `{gc_bif1, {f,0}, {u,live}, erlang:-/1, {x,r}, {x,rd}}` | `OP_GC_BIF1 = 124` |
| `and` r1,r2 → rd | gc_bif2 `erlang:band/2` | |
| `or` r1,r2 → rd | gc_bif2 `erlang:bor/2` | |
| `xor` r1,r2 → rd | gc_bif2 `erlang:bxor/2` | |
| `not` r → rd | gc_bif1 `erlang:bnot/1` | |
| `shl` r1,r2 → rd | gc_bif2 `erlang:bsl/2` | |
| `shr` r1,r2 → rd | gc_bif2 `erlang:bsr/2` | |
| `cmp_eq` r1,r2 → rd | `{move,{i,0},{x,rd}}` `{is_eq_exact,{f,_synth_N},{x,r1},{x,r2}}` `{move,{i,1},{x,rd}}` `{label,_synth_N}` | `is_eq_exact` falls through if equal; branches to fail on not-equal |
| `cmp_ne` r1,r2 → rd | Same structure, `is_ne_exact` | |
| `cmp_lt` r1,r2 → rd | `is_lt` | BEAM `is_lt {f,fail} A B` falls through if A < B |
| `cmp_gt` r1,r2 → rd | `is_lt` (swap operands: `is_lt A B` → gt becomes `is_lt B A`) | |
| `cmp_le` r1,r2 → rd | `is_ge` (negated: `is_ge {f,fail} B A`) | |
| `cmp_ge` r1,r2 → rd | `is_ge` | |
| `label` name | `{label, {u,N}}` where N from label map | |
| `jmp` name | `{jump, {f,N}}` | |
| `jmp_if_true` cond, name | `{is_eq_exact,{f,fall},{x,cond},{i,0}}` `{jump,{f,N}}` `{label,fall}` | branch if cond != 0 |
| `jmp_if_false` cond, name | `{is_ne_exact,{f,fall},{x,cond},{i,0}}` `{jump,{f,N}}` `{label,fall}` | branch if cond == 0 |
| `ret` r | `{move,{x,r},{x,0}}` `{return}` | return value in x0 |
| `ret_void` | `{return}` | |
| `call` fn_name, args → rd | Move args into x0..x(arity-1); `{call,{u,arity},{f,entry_label}}`; `{move,{x,0},{x,rd}}` | result in x0 per Erlang convention |
| `load_reg` v → rd | `{move,{x,v_reg},{x,rd}}` | |
| `store_reg` v, src | `{move,{x,src_reg},{x,v_reg}}` | |
| `type_assert` | (nothing) | |

---

## Backend 2: `iir-to-wasm`

### Cargo dependencies

```toml
[dependencies]
interpreter-ir       = { path = "../interpreter-ir" }
wasm-types           = { path = "../wasm-types" }       # WasmModule, FuncType, FunctionBody, ValueType
wasm-module-encoder  = { path = "../wasm-module-encoder" }
wasm-leb128          = { path = "../wasm-leb128" }
codegen-core         = { path = "../codegen-core" }
```

### Artifact type: `WasmModule` (from `wasm_types`)

### Type mapping

| IIR `type_hint` | WASM `ValueType` |
|----------------|-----------------|
| `i8`, `i16`, `i32`, `u8`, `u16`, `u32`, `bool` | `I32` |
| `i64`, `u64` | `I64` |
| `f32` | `F32` |
| `f64` | `F64` |
| `any` / `polymorphic` / `str` / `ref<T>` | validation error |

### Multi-function model

Each `IIRFunction` becomes one WASM function.  Function signatures are derived from
`function.params` (type_hint → ValueType) and `function.return_type` (or `[]` for void /
unknown).  The module's `entry_point` function is also added to the WASM `export` section
as `"main"`.

### Control-flow strategy: dispatch-loop

WASM requires structured control flow.  IIR can have arbitrary forward / backward branches.
V1 uses the **dispatch-loop** lowering strategy (same approach as the Python ALGOL WASM
backend) for correctness across all programs:

```wasm
(func $fn_name (param ...) (result ...)
  (local $dispatch i32)
  (local $v0 i32) (local $v1 i32) …   ;; all variables as locals
  ;; init params
  i32.const 0
  local.set $dispatch                  ;; dispatch = 0 (first block)
  (block $exit
    (loop $loop
      ;; nested blocks — one per label, in reverse order for br_table
      (block $L_N … (block $L_0
        local.get $dispatch
        br_table 0 1 2 … N            ;; dispatch on current label index
      ) ;; L_0 body: instructions for block starting at label 0
      … instructions …
      i32.const NEXT_LABEL_IDX
      local.set $dispatch
      br $loop                        ;; continue loop
      ) ;; L_1 body …
      …
    ) ;; loop
  ) ;; exit
)
```

Label 0 is the function entry.  IIR `label` instructions index from 0 in definition order
within the function.

### Opcode mapping (i32 variants; i64/f32/f64 use analogous WASM opcodes)

| IIR op | WASM bytecode |
|--------|--------------|
| `const` Int v | `i32.const v` / `i64.const v` |
| `const` Bool true | `i32.const 1` |
| `const` Bool false | `i32.const 0` |
| `const` Float v | `f64.const v` |
| `add` (i32) | `local.get r1; local.get r2; i32.add; local.set rd` |
| `sub` | `i32.sub` |
| `mul` | `i32.mul` |
| `div` (signed) | `i32.div_s` |
| `div` (unsigned, u8/u16/u32) | `i32.div_u` |
| `mod` (signed) | `i32.rem_s` |
| `neg` | `i32.const 0; local.get r; i32.sub` |
| `and` | `i32.and` |
| `or` | `i32.or` |
| `xor` | `i32.xor` |
| `not` | `local.get r; i32.const -1; i32.xor` |
| `shl` | `i32.shl` |
| `shr` (signed) | `i32.shr_s` |
| `shr` (unsigned) | `i32.shr_u` |
| `cmp_eq` | `i32.eq` |
| `cmp_ne` | `i32.ne` |
| `cmp_lt` (signed) | `i32.lt_s` |
| `cmp_gt` (signed) | `i32.gt_s` |
| `cmp_le` (signed) | `i32.le_s` |
| `cmp_ge` (signed) | `i32.ge_s` |
| `label` N | (sets current dispatch block index) |
| `jmp` L | `i32.const L_idx; local.set $dispatch; br $loop` |
| `jmp_if_true` cond, L | `local.get cond; if (then i32.const L_idx; local.set $dispatch; br $loop) end` |
| `jmp_if_false` cond, L | `local.get cond; i32.eqz; if (then i32.const L_idx; local.set $dispatch; br $loop) end` |
| `call` fn → rd | `local.get a0; … call $fn_idx; local.set rd` |
| `ret` r | `local.get r; return` |
| `ret_void` | `return` |
| `load_reg` v → rd | `local.get $v; local.set $rd` |
| `store_reg` v, src | `local.get $src; local.set $v` |
| `type_assert` | (nothing) |

---

## Backend 3: `iir-to-jvm-class-file`

### Cargo dependencies

```toml
[dependencies]
interpreter-ir  = { path = "../interpreter-ir" }
jvm-class-file  = { path = "../jvm-class-file" }  # JvmClassFile, BuildMinimalClassFileParams, etc.
codegen-core    = { path = "../codegen-core" }
```

### Artifact type: `JvmClassFile` (from `jvm_class_file`)

### Class structure

One `public final class <ModuleName>` per `IIRModule`.  Each `IIRFunction` becomes a
`public static` method.  The entry-point function additionally gets a
`public static void main(String[] args)` wrapper that calls it.

Method descriptor derived from `function.params` and `function.return_type`:

| IIR type_hint | JVM type | Descriptor char |
|---------------|---------|-----------------|
| `i8`, `i16`, `i32`, `u8`, `u16`, `u32`, `bool` | `int` | `I` |
| `i64`, `u64` | `long` | `J` |
| `f32` | `float` | `F` |
| `f64` | `double` | `D` |
| `void` (return only) | `void` | `V` |

### Register model

IIR variables → JVM **local variable slots**.  Parameters at slots 0..N-1.  Locals at N..

### Bytecode emission with backpatching

Code is emitted into a `Vec<u8>` byte buffer.  Forward-jump targets are unknown at first
pass; placeholders (2 or 4-byte zeros) are emitted and patched once the target label's PC
offset is known.  `label` instructions record their current byte-buffer offset into
`label_offsets: HashMap<String, u32>`.

### Opcode mapping

| IIR op | JVM bytecode |
|--------|-------------|
| `const` -1 | `iconst_m1` |
| `const` 0..5 | `iconst_<n>` |
| `const` -128..127 | `bipush <byte>` |
| `const` -32768..32767 | `sipush <short>` |
| `const` i32 other | `ldc` (constant pool entry) |
| `const` i64 | `ldc2_w` (long constant) |
| `const` f64 | `ldc2_w` (double constant) |
| `const` Bool true | `iconst_1` |
| `const` Bool false | `iconst_0` |
| `add` (int) | `iload r1; iload r2; iadd; istore rd` |
| `sub` | `isub` |
| `mul` | `imul` |
| `div` | `idiv` |
| `mod` | `irem` |
| `neg` | `iload r; ineg; istore rd` |
| `and` | `iand` |
| `or` | `ior` |
| `xor` | `ixor` |
| `not` | `iload r; iconst_m1; ixor; istore rd` |
| `shl` | `ishl` |
| `shr` (signed) | `ishr` |
| `shr` (unsigned) | `iushr` |
| `cmp_eq` | `if_icmpne +7; iconst_1; goto +4; iconst_0; istore rd` |
| `cmp_ne` | `if_icmpeq +7; …` |
| `cmp_lt` | `if_icmpge +7; …` |
| `cmp_gt` | `if_icmple +7; …` |
| `cmp_le` | `if_icmpgt +7; …` |
| `cmp_ge` | `if_icmplt +7; …` |
| `label` N | (record PC offset for backpatching) |
| `jmp` L | `goto <offset>` (backpatched) |
| `jmp_if_true` cond, L | `iload cond; ifne <offset>` |
| `jmp_if_false` cond, L | `iload cond; ifeq <offset>` |
| `call` fn, args → rd | `iload a0; … invokestatic ClassName/fn_name desc; istore rd` |
| `ret` r | `iload r; ireturn` |
| `ret_void` | `return` |
| `load_reg` v → rd | `iload v; istore rd` |
| `store_reg` v, src | `iload src; istore v` |
| `type_assert` | (nothing) |

---

## Backend 4: `iir-to-cil-bytecode`

### Cargo dependencies

```toml
[dependencies]
interpreter-ir     = { path = "../interpreter-ir" }
ir-to-cil-bytecode = { path = "../ir-to-cil-bytecode" }  # CILBytecodeBuilder, CILProgramArtifact, CILOpcode
codegen-core       = { path = "../codegen-core" }
```

Note: `ir-to-cil-bytecode` transitively pulls in `compiler-ir`, but `iir-to-cil-bytecode`
never uses any `IrProgram` / `IrOp` types.  The dependency will be removed when CIL builder
types are factored into a standalone crate (LANG27).

### Artifact type: `CILProgramArtifact` (from `ir_to_cil_bytecode`)

### Assembly structure

One CIL assembly per `IIRModule`.  Each `IIRFunction` → one `static` CIL method.  The
entry-point function gets an `.entrypoint` stub that calls it.

### Type mapping

| IIR type_hint | CIL type | ELEMENT_TYPE byte |
|---------------|---------|-------------------|
| `i8`, `i16`, `i32`, `u8`, `u16`, `u32`, `bool` | `int32` | `0x08` |
| `i64`, `u64` | `int64` | `0x0A` |
| `f32` | `float32` | `0x0C` |
| `f64` | `float64` | `0x0D` |
| `void` (return only) | `void` | `0x01` |

### Register model

IIR variables → CIL **local variable slots**.  Parameters accessed via `ldarg`/`starg` at
indices 0..N-1.  Locals via `ldloc`/`stloc` at indices 0..(total_vars - N - 1).

### Opcode mapping

| IIR op | CIL opcodes |
|--------|------------|
| `const` -1 | `ldc.i4.m1` |
| `const` 0..8 | `ldc.i4.0` .. `ldc.i4.8` |
| `const` -128..127 | `ldc.i4.s <byte>` |
| `const` i32 other | `ldc.i4 <int32>` |
| `const` i64 | `ldc.i8 <int64>` |
| `const` f64 | `ldc.r8 <float64>` |
| `const` Bool true | `ldc.i4.1` |
| `const` Bool false | `ldc.i4.0` |
| `add` | `ldloc r1; ldloc r2; add; stloc rd` |
| `sub` | `sub` |
| `mul` | `mul` |
| `div` (signed) | `div` |
| `div` (unsigned) | `div.un` |
| `mod` | `rem` |
| `neg` | `ldloc r; neg; stloc rd` |
| `and` | `and` |
| `or` | `or` |
| `xor` | `xor` |
| `not` | `ldloc r; not; stloc rd` (CIL has native `not`) |
| `shl` | `shl` |
| `shr` (signed) | `shr` |
| `shr` (unsigned) | `shr.un` |
| `cmp_eq` | `ldloc r1; ldloc r2; ceq; stloc rd` |
| `cmp_lt` (signed) | `clt` |
| `cmp_gt` (signed) | `cgt` |
| `cmp_ne` | `ceq; ldc.i4.0; ceq; stloc rd` (NOT of ceq) |
| `cmp_le` | `cgt; ldc.i4.0; ceq; stloc rd` (NOT of cgt) |
| `cmp_ge` | `clt; ldc.i4.0; ceq; stloc rd` (NOT of clt) |
| `label` N | `CILBytecodeBuilder::mark_label(name)` |
| `jmp` L | `br <label>` |
| `jmp_if_true` cond, L | `ldloc cond; brtrue <label>` |
| `jmp_if_false` cond, L | `ldloc cond; brfalse <label>` |
| `call` fn, args → rd | `ldloc a0; … call <method_token>; stloc rd` |
| `ret` r | `ldloc r; ret` |
| `ret_void` | `ret` |
| `load_reg` v → rd | `ldloc v; stloc rd` |
| `store_reg` v, src | `ldloc src; stloc v` |
| `type_assert` | (nothing) |

---

## Test plan

### ≥ 40 tests per backend, targeting ≥ 85% line coverage

Tests live in `tests/test_backend.rs`.  A shared `make_module()` helper constructs
minimal `IIRModule`s.

#### Validation tests (10)
1. `test_empty_module_rejected` — zero functions → `ValidationError::EmptyModule`
2. `test_empty_function_rejected` — function with zero instructions → `EmptyFunction`
3. `test_any_type_hint_rejected` — instr with `type_hint = "any"` → `UntypedInstruction`
4. `test_polymorphic_type_hint_rejected`
5. `test_str_type_hint_rejected`
6. `test_ref_type_hint_rejected` — `"ref<i32>"`
7. `test_call_builtin_rejected` → `UnsupportedOp`
8. `test_io_out_rejected`
9. `test_alloc_rejected`
10. (BEAM/CLR only) `test_float_const_rejected`

#### Constant loads (3)
11. `test_const_i32` — `const 42` → lowers without error
12. `test_const_bool_true` — `const true`
13. `test_const_bool_false`
14. (WASM/JVM only) `test_const_f64`

#### Arithmetic (7)
15. `test_add_i32`
16. `test_sub_i32`
17. `test_mul_i32`
18. `test_div_i32`
19. `test_mod_i32`
20. `test_neg_i32`

#### Bitwise (6)
21. `test_and_i32`
22. `test_or_i32`
23. `test_xor_i32`
24. `test_not_i32`
25. `test_shl_i32`
26. `test_shr_i32`

#### Comparisons (6)
27. `test_cmp_eq`
28. `test_cmp_ne`
29. `test_cmp_lt`
30. `test_cmp_le`
31. `test_cmp_gt`
32. `test_cmp_ge`

#### Control flow (4)
33. `test_label_and_jmp`
34. `test_jmp_if_true`
35. `test_jmp_if_false`
36. `test_type_assert_is_nop` — lowers cleanly, produces no extra instructions

#### Functions (5)
37. `test_ret_void`
38. `test_ret_with_value`
39. `test_call_function` — two-function module, one calls the other
40. `test_multi_function_module` — three functions
41. `test_params_get_first_registers` — params assigned register indices 0, 1, 2, …

#### Register allocation (3)
42. `test_register_reuse_same_variable_gets_same_index`
43. `test_distinct_variables_get_distinct_indices`
44. `test_load_reg_store_reg`

#### Round-trip (2)
45. `test_validate_then_lower_succeeds_on_minimal_module`
46. `test_lowering_produces_nonempty_artifact`

---

## Cargo workspace

Add four new members to `code/packages/rust/Cargo.toml`:

```toml
"iir-to-beam",
"iir-to-wasm",
"iir-to-jvm-class-file",
"iir-to-cil-bytecode",
```

---

## Files to create / update

| File | Action |
|------|--------|
| `code/specs/LANG29-iir-direct-backends.md` | CREATE (this document) |
| `code/packages/rust/Cargo.toml` | UPDATE (add 4 members) |
| `code/packages/rust/iir-to-beam/Cargo.toml` | CREATE |
| `code/packages/rust/iir-to-beam/BUILD` | CREATE |
| `code/packages/rust/iir-to-beam/CHANGELOG.md` | CREATE |
| `code/packages/rust/iir-to-beam/README.md` | CREATE |
| `code/packages/rust/iir-to-beam/src/lib.rs` | CREATE |
| `code/packages/rust/iir-to-beam/src/validate.rs` | CREATE |
| `code/packages/rust/iir-to-beam/src/lower.rs` | CREATE |
| `code/packages/rust/iir-to-beam/src/codegen.rs` | CREATE |
| `code/packages/rust/iir-to-beam/tests/test_backend.rs` | CREATE |
| `code/packages/rust/iir-to-wasm/Cargo.toml` | CREATE |
| `code/packages/rust/iir-to-wasm/BUILD` | CREATE |
| `code/packages/rust/iir-to-wasm/CHANGELOG.md` | CREATE |
| `code/packages/rust/iir-to-wasm/README.md` | CREATE |
| `code/packages/rust/iir-to-wasm/src/lib.rs` | CREATE |
| `code/packages/rust/iir-to-wasm/src/validate.rs` | CREATE |
| `code/packages/rust/iir-to-wasm/src/lower.rs` | CREATE |
| `code/packages/rust/iir-to-wasm/src/codegen.rs` | CREATE |
| `code/packages/rust/iir-to-wasm/tests/test_backend.rs` | CREATE |
| `code/packages/rust/iir-to-jvm-class-file/Cargo.toml` | CREATE |
| `code/packages/rust/iir-to-jvm-class-file/BUILD` | CREATE |
| `code/packages/rust/iir-to-jvm-class-file/CHANGELOG.md` | CREATE |
| `code/packages/rust/iir-to-jvm-class-file/README.md` | CREATE |
| `code/packages/rust/iir-to-jvm-class-file/src/lib.rs` | CREATE |
| `code/packages/rust/iir-to-jvm-class-file/src/validate.rs` | CREATE |
| `code/packages/rust/iir-to-jvm-class-file/src/lower.rs` | CREATE |
| `code/packages/rust/iir-to-jvm-class-file/src/codegen.rs` | CREATE |
| `code/packages/rust/iir-to-jvm-class-file/tests/test_backend.rs` | CREATE |
| `code/packages/rust/iir-to-cil-bytecode/Cargo.toml` | CREATE |
| `code/packages/rust/iir-to-cil-bytecode/BUILD` | CREATE |
| `code/packages/rust/iir-to-cil-bytecode/CHANGELOG.md` | CREATE |
| `code/packages/rust/iir-to-cil-bytecode/README.md` | CREATE |
| `code/packages/rust/iir-to-cil-bytecode/src/lib.rs` | CREATE |
| `code/packages/rust/iir-to-cil-bytecode/src/validate.rs` | CREATE |
| `code/packages/rust/iir-to-cil-bytecode/src/lower.rs` | CREATE |
| `code/packages/rust/iir-to-cil-bytecode/src/codegen.rs` | CREATE |
| `code/packages/rust/iir-to-cil-bytecode/tests/test_backend.rs` | CREATE |

---

## Verification

```bash
# Build all four new packages in the workspace
cd code/packages/rust
cargo build --workspace 2>&1 | tail -20

# Run all tests
cargo test -p iir-to-beam           -- --nocapture
cargo test -p iir-to-wasm           -- --nocapture
cargo test -p iir-to-jvm-class-file -- --nocapture
cargo test -p iir-to-cil-bytecode   -- --nocapture

# Coverage check (llvm-cov or tarpaulin)
cargo llvm-cov --package iir-to-beam --summary-only
```

End-to-end smoke (after all four backends build):

```rust
use interpreter_ir::{IIRModule, IIRFunction, IIRInstr, Operand};
use iir_to_beam::{validate_for_beam, lower_iir_to_beam, IIRBeamConfig};

let fn_ = IIRFunction::new("add_two", vec![("a", "i32"), ("b", "i32")], "i32", vec![
    IIRInstr::new("add", Some("result"), vec![Operand::Var("a"), Operand::Var("b")], "i32"),
    IIRInstr::new("ret",  None,          vec![Operand::Var("result")],               "i32"),
]);
let module = IIRModule { name: "smoke".into(), functions: vec![fn_], entry_point: Some("add_two".into()), language: "test".into() };

assert_eq!(validate_for_beam(&module), vec![]);
let beam_module = lower_iir_to_beam(&module, &IIRBeamConfig::default()).unwrap();
assert!(!beam_module.atoms.is_empty());
println!("smoke: OK");
```

---

## Out of scope (future)

- **LANG27 full**: `iir-host-lowering` with ALGOL frames, closure lifting, descriptor layouts,
  nonlocal-goto unwind, and `lang_host_compile()` unified API
- **Float on BEAM/CLR**: requires Erlang float-term encoding / proper CLR method signatures
- **`call_builtin`**: LANG26 stdlib integration
- **`io_in` / `io_out`**: WASI / Erlang I/O (LANG27)
- **Memory ops** (`load_mem`, `store_mem`): WASM linear memory, JVM byte arrays
- **Heap / GC ops** (LANG16)
- **Multi-class JVM output**: closures, runtime helper class
- **Source map / debug info threading** from IIR `source_map` to backend line tables
- **`iir-to-ge225`**, **`iir-to-intel-4004`**, **`iir-to-intel-8008`**: other LANG20 backends
