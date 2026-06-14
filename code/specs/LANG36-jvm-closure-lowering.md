# LANG36 — JVM Closure Lowering

**Status:** In progress  
**Depends on:** LANG34 (first-class closure opcodes), LANG35 (BEAM closure lowering)  
**Crate:** `iir-to-jvm-class-file` v0.4.0

---

## Context

LANG35 added `alloc_closure` / `call_closure` support to the BEAM backend and
added `ClosureOpcode` validation errors to the JVM, CLR, and WASM backends
(rejecting closures with an actionable error message).  LANG36 promotes the
JVM backend from "reject with ClosureOpcode" to "lower closures to a
`long[]`-based dispatch-table approach."

---

## Closure Representation

A JVM closure is a **`long[]` array**:

```text
closure[0]  = function dispatch index (u32 as long)
closure[1]  = first captured value (as long)
closure[2]  = second captured value (as long)
…
```

All captured values and call-time arguments are represented as `long`:

| IIR type       | JVM representation         |
|----------------|---------------------------|
| `i32`, `u32`, `bool` | sign-extended to `long` |
| `i64`, `u64`   | direct `long`             |
| `f32`, `f64`   | deferred — `ClosureOpcode` error in v1 |

This mirrors the BEAM backend where everything is an Erlang term.  Float
closure captures are deferred to LANG38 (requires boxing to `Object[]`).

---

## Dispatch Method

For any module that contains `alloc_closure` or `call_closure` instructions,
the lowering pass generates a synthetic **`__callClosure`** static method:

```java
// Signature: static long __callClosure([J, [J) -> J
// [J = long[], J = long
static long __callClosure(long[] closure, long[] args) {
    long fn_idx = closure[0];
    // case 0: __lambda_0  — 1 capture, 1 arg
    if (fn_idx == 0L) {
        return __lambda_0(closure[1], args[0]);
    }
    // case 1: __add_fn    — 0 captures, 1 arg
    if (fn_idx == 1L) {
        return __add_fn(args[0]);
    }
    // … one branch per closure-eligible function
    return 0L;  // unreachable default
}
```

The dispatch table is built from the IIR module:

1. **Pre-pass**: collect every function name that appears as `srcs[0]` of any
   `alloc_closure` instruction.
2. Assign each collected name a stable integer index (alphabetical order for
   determinism).
3. In each `if (fn_idx == N)` branch, reconstruct the static call using:
   - `closure[1..]` for the captured variables (in declaration order)
   - `args[0..]` for the call-time arguments

The method descriptor is `([J[J)J` ("takes two long arrays, returns long").

---

## New JVM Opcodes

LANG36 adds four opcodes to `iir-to-jvm-class-file/src/lower.rs`:

| Opcode | Byte | Description |
|--------|------|-------------|
| `NEWARRAY` | `0xBC` | Allocate primitive array; operand `0x0B` = T_LONG |
| `LALOAD`   | `0x2F` | Load `long` from `long[]` |
| `LASTORE`  | `0x50` | Store `long` into `long[]` |
| `LCMP`     | `0x94` | Compare two longs; result: `-1`, `0`, or `1` |

`DUP` (0x59), `IFEQ` (0x99), `ICONST_N`, `LCONST_N` are already present.

`LASTORE` requires stack state: `..., arrayref, index (int), value (long)`.
`LALOAD` requires: `..., arrayref, index (int)` → `long`.

---

## `alloc_closure` Lowering

```text
IIR:  dest = alloc_closure(Str("fn_name"), Var(cap0), Var(cap1)) : "closure"

JVM bytecode sequence:
  iconst_{n+1}            ; array size = 1 (idx) + n (captures)
  newarray T_LONG (0x0B)  ; long[] closure_arr = new long[n+1]
  dup
  iconst_0
  ldc2_w <fn_idx>         ; push function dispatch index as long
  lastore                 ; closure_arr[0] = fn_idx
  dup
  iconst_1
  lload  cap0_slot        ; or lconst/bipush for i32 → sign-extend
  lastore                 ; closure_arr[1] = cap0
  dup
  iconst_2
  lload  cap1_slot
  lastore                 ; closure_arr[2] = cap1
  astore dest_slot        ; dest = closure_arr
```

Notes:
- For i32 captures: `iload slot; i2l` (sign-extend int to long)  
- For i64 captures: `lload slot` directly
- `ldc2_w` is already supported by the backend for long constants

---

## `call_closure` Lowering

```text
IIR:  dest = call_closure(Var(handle), Var(arg0), Var(arg1)) : "any"

JVM bytecode sequence:
  aload handle_slot        ; push closure handle (long[])
  iconst_2                 ; args array size = 2
  newarray T_LONG (0x0B)   ; long[] args_arr = new long[2]
  dup
  iconst_0
  lload arg0_slot          ; (or i2l for i32)
  lastore                  ; args_arr[0] = arg0
  dup
  iconst_1
  lload arg1_slot
  lastore                  ; args_arr[1] = arg1
  invokestatic ClassName.__callClosure([J[J)J
  lstore dest_slot         ; dest = result
```

For `dest` of non-long type (i32/bool): `lstore dest` then load as `l2i`.

---

## Validator Change

`validate_for_jvm` changes in `validate.rs`:
- Remove `alloc_closure` and `call_closure` from the `ClosureOpcode` reject list.
- Add early-accept block (matching the BEAM validator's Check 2.5).
- Reject `alloc_closure` instructions whose captures have `f32`/`f64` type hints
  (float closure captures are deferred).

The updated ClosureOpcode note in the validator becomes:
```text
"[fn_name] ClosureOpcode: alloc_closure/call_closure with float captures
 require the BEAM backend in v1; use integer types or upgrade to LANG38"
```

---

## Dispatch Index Assignment

Functions are eligible for the dispatch table if they appear as `srcs[0]`
(the `Str` fn_name operand) in any `alloc_closure` instruction in the module.

Index assignment: sort the eligible function names alphabetically, assign
indices 0..N-1.  Deterministic ordering ensures byte-identical class files
from identical modules.

For the `__callClosure` method, the lowering pass must know — for each
eligible function:
1. How many captures it was allocated with (from `alloc_closure` instruction's
   `srcs[1..]` length)
2. Its full parameter list (from `func.params` in the `IIRModule`)

The number of captures + number of call-time args must equal the function's
total arity.  Validated at lowering time: if `captures + call_args ≠ func.params.len()`,
return `IIRJvmError::InvalidOperand`.

---

## Files Changed

| File | Change |
|------|--------|
| `iir-to-jvm-class-file/src/validate.rs` | Remove `alloc_closure`/`call_closure` from ClosureOpcode; add early-accept |
| `iir-to-jvm-class-file/src/lower.rs` | Add 4 opcodes; add closure pre-pass; lower `alloc_closure`/`call_closure`; generate `__callClosure` |
| `iir-to-jvm-class-file/tests/test_backend.rs` | Add tests 4–7: validator accept, dispatch generation, alloc lowering, call lowering; test_8 optional real-JVM round-trip |
| `iir-to-jvm-class-file/CHANGELOG.md` | Add v0.4.0 entry |
| `iir-to-jvm-class-file/Cargo.toml` | Bump to 0.4.0 |

---

## Tests

### Validator tests

- `lang36_alloc_closure_accepted_by_jvm_validator`: `alloc_closure` with integer
  captures no longer returns `ClosureOpcode`.
- `lang36_call_closure_accepted_by_jvm_validator`: `call_closure` with `"any"`
  type_hint passes validation.
- `lang36_float_closure_still_rejected`: `alloc_closure` with a `f32` capture
  still returns a `ClosureOpcode` error.

### Lowering tests

- `lang36_alloc_closure_emits_newarray`: module with `alloc_closure` emits
  `NEWARRAY` (0xBC) in the bytecode stream.
- `lang36_alloc_closure_emits_lastore`: `alloc_closure` emits `LASTORE`(0x50).
- `lang36_call_closure_emits_invokestatic_dispatch`: `call_closure` emits
  `INVOKESTATIC` (0xB8) pointing to `__callClosure`.
- `lang36_dispatch_method_generated`: the `JvmClassFile` has a method named
  `__callClosure` when the module contains `alloc_closure`.
- `lang36_dispatch_method_contains_lcmp`: the `__callClosure` method's Code
  attribute contains `LCMP` (0x94).

### Real-JVM round-trip (gated)

- `lang36_real_jvm_closure_adder`: equivalent of BEAM's test_66 — compile a
  two-function module with `alloc_closure` + `call_closure`, write the `.class`
  file, run with `java`, assert the output is `7`.  Gated by `java_available()`.

---

## Non-Goals

- Float closures — deferred to LANG38.
- CLR closure lowering — LANG37.
- WASM closure lowering — LANG38.
- Tail-call optimisation across closures.
- Multi-arity currying or partial application.
