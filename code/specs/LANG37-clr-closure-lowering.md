# LANG37 — CLR Closure Lowering

**Status:** In progress  
**Depends on:** LANG34 (first-class closure opcodes), LANG35 (BEAM closure lowering), LANG36 (JVM closure lowering)  
**Crate:** `iir-to-cil-bytecode` v0.4.0

---

## Context

LANG35 added `alloc_closure` / `call_closure` support to the BEAM backend and
added `ClosureOpcode` validation errors to the JVM, CLR, and WASM backends.
LANG36 promoted the JVM backend to a full `long[]`-based dispatch-table
implementation.  LANG37 promotes the CLR backend from "reject with
ClosureOpcode" to "lower closures to an `int32[]`-based dispatch-table
approach."

---

## Closure Representation

A CLR closure is an **`int32[]` array**:

```text
closure[0]  = function dispatch index (alphabetically assigned, stored as int32)
closure[1]  = first captured value (as int32)
closure[2]  = second captured value (as int32)
…
```

All captured values and call-time arguments are represented as `int32`:

| IIR type             | CLR representation       |
|----------------------|--------------------------|
| `i32`, `u32`, `bool` | direct `int32`           |
| `i64`, `u64`         | deferred — `ClosureOpcode` error in v1 |
| `f32`, `f64`         | deferred — `ClosureOpcode` error in v1 |

This mirrors the BEAM and JVM backends where values are normalised to a single
width.  Int64, float, and reference captures are deferred to LANG38.

The choice of `int32[]` (rather than `object[]` with boxing) avoids the need
for `box`/`unbox.any` CIL instructions, which would require additional TypeRef
tokens not currently in the `ir-to-cil-bytecode` token set.

---

## Dispatch Method

For any module that contains `alloc_closure` or `call_closure` instructions,
the lowering pass generates a synthetic **`__callClosure`** static method:

```cil
// Signature: static int32 __callClosure(int32[] closure, int32[] args)
.method static int32 __callClosure(int32[] closure, int32[] args) {
    .maxstack 4
    .locals (int32 dispatch_idx)

    // Load dispatch index from closure[0]
    ldarg.0
    ldc.i4.0
    ldelem.i4
    stloc.0         // dispatch_idx = closure[0]

    // case 0: __lambda_0 — 1 capture, 1 arg
    ldloc.0
    ldc.i4.0
    beq case_0

    // case 1: __add_fn — 0 captures, 1 arg
    ldloc.0
    ldc.i4.1
    beq case_1

    // … one beq branch per closure-eligible function

    // Unreachable default
    ldc.i4.0
    ret

case_0:
    ldarg.0; ldc.i4.1; ldelem.i4    // closure[1] = cap0
    ldarg.1; ldc.i4.0; ldelem.i4    // args[0]
    call int32 ClassName::__lambda_0(int32, int32)
    ret

case_1:
    ldarg.1; ldc.i4.0; ldelem.i4    // args[0]
    call int32 ClassName::__add_fn(int32)
    ret
}
```

The dispatch table is built identically to LANG36:

1. **Pre-pass**: collect every function name that appears as `srcs[0]` of any
   `alloc_closure` instruction.
2. Assign each collected name a stable integer index (alphabetical order for
   determinism).
3. In each `beq case_N` branch, reconstruct the static call using:
   - `closure[1..]` for the captured variables (in declaration order)
   - `args[0..]` for the call-time arguments

The method token for `__callClosure` is
`0x0600_0001 + module.functions.len()` — the next slot after all user functions.

---

## New Token

| Token | Value | Description |
|-------|-------|-------------|
| `INT32_ARRAY_TYPE_TOKEN` | `0x0100_0002` | TypeRef for `System.Int32[]` — used with `newarr` |

Added to `ir-to-cil-bytecode/src/lib.rs` alongside the existing
`OBJECT_ARRAY_TYPE_TOKEN = 0x0100_0001`.

---

## New CIL Instructions Emitted

All opcodes were already in the `CILOpcode` enum — no new variants needed.

| Opcode      | Byte   | Emit method                         | Description                                   |
|-------------|--------|-------------------------------------|-----------------------------------------------|
| `ldelem.i4` | `0x94` | `emit_opcode(CILOpcode::LdElemI4)` | Load `int32` element from `int32[]`           |
| `stelem.i4` | `0x9E` | `emit_opcode(CILOpcode::StElemI4)` | Store `int32` element into `int32[]`          |
| `newarr`    | `0x8D` | `emit_newarr(INT32_ARRAY_TYPE_TOKEN)` | Allocate `int32[]` array                    |

`dup` (0x25), `ldc.i4` variants, `ldarg`, `stloc`, `ldloc`, `call` (0x28), and
`beq`/`beq.s` (0x3B/0x2E) are already emitted by the backend for other ops.

---

## `alloc_closure` Lowering

```text
IIR:  dest = alloc_closure(Str("fn_name"), Var(cap0), Var(cap1)) : "closure"

CIL sequence:
  ldc.i4 {n+1}                  ; array size = 1 (idx) + n (captures)
  newarr [System.Int32]         ; int32[] closure_arr = new int32[n+1]
  dup
  ldc.i4.0                      ; index 0
  ldc.i4 {dispatch_idx}         ; function dispatch index
  stelem.i4                     ; closure_arr[0] = dispatch_idx
  dup
  ldc.i4.1                      ; index 1
  ldloc cap0_slot               ; i32 capture 0
  stelem.i4                     ; closure_arr[1] = cap0
  dup
  ldc.i4.2
  ldloc cap1_slot
  stelem.i4                     ; closure_arr[2] = cap1
  stloc dest_slot               ; dest = closure_arr
```

Notes:
- `"closure"` type hint → new local type `ClrType::IntArray` (stored/loaded with
  `ldloc`/`stloc` like any other reference; the CLR doesn't distinguish
  reference-typed locals at the ldloc/stloc level).
- Only `i32`/`bool` captures supported in v1; `i64`/`f32`/`f64` captures still
  produce a `ClosureOpcode` validation error.

---

## `call_closure` Lowering

```text
IIR:  dest = call_closure(Var(handle), Var(arg0), Var(arg1)) : "any"

CIL sequence:
  ldloc handle_slot             ; push closure handle (int32[])
  ldc.i4.2                      ; args array size = 2
  newarr [System.Int32]         ; int32[] args_arr = new int32[2]
  dup
  ldc.i4.0
  ldloc arg0_slot               ; arg0 (i32)
  stelem.i4                     ; args_arr[0] = arg0
  dup
  ldc.i4.1
  ldloc arg1_slot
  stelem.i4                     ; args_arr[1] = arg1
  call int32 ClassName::__callClosure(int32[], int32[])
  stloc dest_slot               ; dest = result
```

For `dest` of non-int32 type (currently all i32/bool since i64 is deferred):
result is already `int32`, stored with `stloc`.

---

## Validator Change

`validate_iir_for_clr` changes in `validate.rs`:
- Remove `alloc_closure` and `call_closure` from the `ClosureOpcode` reject path.
- Add early-accept (matching LANG36's Check 2.5 pattern).
- Reject `alloc_closure` instructions whose captures have `i64`, `u64`, `f32`,
  or `f64` type hints (these need wider storage than `int32[]`).

Updated `ClosureOpcode` error message:
```text
"[fn_name] ClosureOpcode: alloc_closure captures variable [cap] of type [type];
 only i32/bool captures are supported by the CLR backend in v1 — use integer
 types or upgrade to LANG38"
```

---

## Dispatch Index Assignment

Identical to LANG36: sort eligible function names alphabetically, assign
indices 0..N-1.  Validates at lowering time that
`n_captures + n_call_args == func.params.len()`.

The `__callClosure` method token:
```rust
0x0600_0001u32 + module.functions.len() as u32
```

---

## Files Changed

| File | Change |
|------|--------|
| `ir-to-cil-bytecode/src/lib.rs` | Add `INT32_ARRAY_TYPE_TOKEN = 0x0100_0002` |
| `iir-to-cil-bytecode/src/validate.rs` | Remove `alloc_closure`/`call_closure` from ClosureOpcode; add early-accept; reject i64/float captures |
| `iir-to-cil-bytecode/src/lower.rs` | Closure pre-pass; `alloc_closure`/`call_closure` arms; `generate_call_closure_dispatch`; wire into `lower_iir_to_cil` |
| `iir-to-cil-bytecode/tests/test_backend.rs` | Replace 3 LANG35 tests with LANG37 tests |
| `iir-to-cil-bytecode/CHANGELOG.md` | Add v0.4.0 entry |
| `iir-to-cil-bytecode/Cargo.toml` | Bump to 0.4.0 |

---

## Tests

### Validator tests

- `lang37_alloc_closure_accepted_by_clr_validator`: `alloc_closure` with `i32`
  captures no longer returns `ClosureOpcode`.
- `lang37_call_closure_accepted_by_clr_validator`: `call_closure` with `"any"`
  type_hint passes validation.
- `lang37_i64_capture_still_rejected`: `alloc_closure` with an `i64` capture
  returns `ClosureOpcode`.
- `lang37_float_capture_still_rejected`: `alloc_closure` with an `f32` capture
  returns `ClosureOpcode`.

### Lowering tests

- `lang37_alloc_closure_emits_newarr`: `alloc_closure` emits `NEWARR` (0x8D).
- `lang37_alloc_closure_emits_stelem_i4`: `alloc_closure` emits `STELEM_I4` (0x9E).
- `lang37_call_closure_emits_call_dispatch`: `call_closure` emits `CALL` (0x28)
  targeting `__callClosure`.
- `lang37_dispatch_method_generated`: `CILProgramArtifact.methods` contains a
  method named `__callClosure` when the module contains `alloc_closure`.
- `lang37_dispatch_method_contains_ldelem_i4`: the `__callClosure` body contains
  `LDELEM_I4` (0x94).

---

## Non-Goals

- i64/float closure captures — deferred to LANG38.
- WASM closure lowering — LANG38.
- Real-dotnet round-trip test — no `dotnet` gate in existing CLR test suite;
  deferred to LANG39.
- Tail-call optimisation across closures.
- Multi-arity currying or partial application.
