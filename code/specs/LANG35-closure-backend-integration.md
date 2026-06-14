# LANG35 — Closure Backend Integration & Real-VM Tests

## Status

Spec → Implementation

---

## Motivation

LANG34 introduced two first-class IIR opcodes:

```
dest = alloc_closure(Str(fn_name), Var(cap0), …) : "closure"
dest = call_closure(Var(handle), Var(arg0), …)   : "any"
```

The LANG34 spec explicitly deferred backend support:

> *"Backend support is LANG35+"*
> — `LANG34-closures.md` §Non-Goals

Meanwhile, the LANG32/LANG33 roadmap table promised:

| LANG | Feature |
|------|---------|
| LANG32 | Global variables and IO |
| LANG33 | Module system |
| LANG34 | First-class closure opcodes |
| **LANG35** | **Real-VM integration tests (erl/java/dotnet/wasmtime)** |

LANG35 delivers on both commitments:

1. **BEAM backend**: Full `alloc_closure`/`call_closure` lowering.  The BEAM02
   spec already proved the cons-cell + `erlang:apply/3` strategy; LANG35 wires
   that strategy into the `iir-to-beam` direct-IIR backend.

2. **WASM/JVM/CLR backends**: Improved validator diagnostics — return a specific
   `ClosureOpcode` error (not the confusing `UntypedInstruction`) so callers
   understand these opcodes are valid but require a future lowering pass.
   Full WASM/JVM/CLR closure lowering is deferred to LANG36–LANG38 where each
   backend's closure representation can be designed cleanly.

3. **Real-VM integration tests**: Rust integration tests that invoke actual
   `erl`, `wasmtime`, `java`, and `dotnet` binaries.  Each test is gated by
   `which <binary>` so CI passes on machines without every runtime installed.

---

## Architecture

### BEAM closure representation (cons-cell approach, from BEAM02)

A closure is encoded as a BEAM cons cell:

```
[fn_atom | [cap0, cap1, …, capN]]
```

This representation uses only standard BEAM opcodes (`put_list`, `get_list`,
`call_ext`) and works on all Erlang/OTP versions without requiring the
contested `make_fun2`/`make_fun3` opcodes.

### `alloc_closure` → BEAM lowering

```
IIR:  %dest = alloc_closure(Str("__lambda_0"), %cap0, %cap1) : closure

BEAM:
  ; Step 1: load nil into scratch register r_scratch
  move    {a,nil}            {x, r_scratch}

  ; Step 2: prepend captures from right to left (builds [cap1, cap0])
  put_list {x, r_cap1}  {x, r_scratch}  {x, r_scratch}   % r_scratch = [cap1]
  put_list {x, r_cap0}  {x, r_scratch}  {x, r_scratch}   % r_scratch = [cap0, cap1]

  ; Step 3: prepend fn_atom to form the closure cons cell
  put_list {a, fn_atom} {x, r_scratch}  {x, r_dest}      % r_dest = [fn_atom | caps]
```

Scratch usage: **1 register** (`meta.next_reg`).

### `call_closure` → BEAM lowering

```
IIR:  %dest = call_closure(%handle, %arg0, %arg1) : any

BEAM:
  ; Step 1: extract fn_atom and caps from closure cons cell
  get_list  {x, r_handle}  {x, r_fn}    {x, r_caps}

  ; Step 2: build args list [arg0, arg1] from right to left
  move      {a, nil}       {x, r_args}
  put_list  {x, r_arg1}    {x, r_args}  {x, r_args}   % [arg1]
  put_list  {x, r_arg0}    {x, r_args}  {x, r_args}   % [arg0, arg1]

  ; Step 3: call erlang:'++'/2 to compute caps ++ args
  move      {x, r_caps}    {x, 0}       % x0 = caps
  move      {x, r_args}    {x, 1}       % x1 = args
  call_ext  2  {u, import_append}       % x0 = caps ++ args
  move      {x, 0}         {x, r_combined}

  ; Step 4: call erlang:apply/3 with (Module, FnAtom, Combined)
  move      {a, module_atom}  {x, 0}   % x0 = Module
  move      {x, r_fn}         {x, 1}   % x1 = FunctionName atom
  move      {x, r_combined}   {x, 2}   % x2 = CombinedArgs
  call_ext  3  {u, import_apply}       % x0 = return value
  move      {x, 0}         {x, r_dest}
```

Scratch usage: **4 registers** (`meta.next_reg + 0..3`).

#### Scratch register overflow guard

If `meta.next_reg > 251`, there are not enough scratch registers for
`call_closure`.  The lowerer returns `IIRBeamError::TooManyRegisters`.

### WASM/JVM/CLR: `ClosureOpcode` validator error

These three backends do not yet support closure opcodes.  The validators
are updated to detect `alloc_closure`/`call_closure` specifically and
return a `ClosureOpcode` error (e.g.,
`"ClosureOpcode: function "f" op "alloc_closure" — closure opcodes are not yet
supported by this backend; apply iir-builtin-lowering Phase 4 to downgrade to
call_builtin before lowering"`).

This replaces the confusing `UntypedInstruction` error that would previously
fire because `"closure"` and `"any"` are not concrete numeric types.

---

## Validator changes

### `iir-to-beam/src/validate.rs`

- `alloc_closure` with `type_hint == "closure"` → **accepted** (no longer
  falls into UntypedInstruction or UnsupportedOp).
- `call_closure` with `type_hint == "any"` → **accepted** (`"any"` is normally
  banned, but `call_closure` is a dynamic dispatch whose return type is
  necessarily unknown at compile time).
- Both ops are removed from the effective `UNSUPPORTED_OPS` list (or, more
  precisely, they are handled before the generic UnsupportedOp check).

### `iir-to-wasm/src/validate.rs`

New constant `CLOSURE_OPS: &[&str] = &["alloc_closure", "call_closure"]`.

New check in `validate_for_wasm`: if `CLOSURE_OPS.contains(&instr.op.as_str())`:
```
ClosureOpcode: function {:?}, op {:?} — closure opcodes require WASM closure
support (planned for a future LANG spec); apply iir-builtin-lowering Phase 4
to downgrade to call_builtin form before lowering to WASM
```

This fires **before** the existing UntypedInstruction check so callers get the
actionable message.

### `iir-to-jvm-class-file/src/validate.rs`

Same `ClosureOpcode` check pattern as WASM.

### `iir-to-cil-bytecode/src/validate.rs`

Same `ClosureOpcode` check pattern as WASM.

---

## New BEAM lowering: `OP_CALL_EXT`

BEAM opcode 6 (`call_ext`) calls an external (imported) function.

Format: `call_ext {u, Arity}, {u, ImportIdx}`

Two new imports are registered unconditionally (like the existing
`erlang:put/2`, `erlang:get/1`):

| Import | Erlang symbol | Usage |
|--------|---------------|-------|
| `import_append` | `erlang:'++'` / 2 | Concatenate captures + user args |
| `import_apply`  | `erlang:apply` / 3 | Dynamic dispatch by fn name |

New atom: `atom_append = atoms.intern("++")`.

`atom_apply` may already be interned by other code; if not, it is added:
`atom_apply = atoms.intern("apply")`.

---

## Real-VM integration tests

Each of the four backends gets an additional test file
`tests/test_real_vm.rs` (or section within `test_backend.rs`).

### `iir-to-beam` — real `erl` tests

```rust
#[test]
fn real_erl_arithmetic() { ... }   // (+ 5 3) → prints 8 via io_out

#[test]
fn real_erl_closure_adder() { ... } // (adder 5)(3) → prints 8 via alloc_closure / call_closure
```

Skip guard:
```rust
fn erl_available() -> bool {
    std::process::Command::new("erl").arg("-version")
        .output().map(|o| o.status.success()).unwrap_or(false)
}
```

Invocation:
```
erl -noshell -pa <tmpdir> -s <module> main -s init stop
```

### `iir-to-wasm` — real `wasmtime` tests

```rust
#[test]
fn real_wasmtime_arithmetic() { ... }  // (add 5 3) → exit code 8 (via halt)
```

Skip guard: `which wasmtime` succeeds.

Invocation: `wasmtime <tmpdir>/test.wasm`

### `iir-to-jvm-class-file` — real `java` tests

```rust
#[test]
fn real_java_arithmetic() { ... }  // (+ 5 3) → prints 8
```

Skip guard: `java -version` succeeds.

Invocation: `java -cp <tmpdir> TwigMain`

### `iir-to-cil-bytecode` — real `dotnet` tests

```rust
#[test]
fn real_dotnet_arithmetic() { ... }  // (+ 5 3) → prints 8
```

Skip guard: `dotnet --version` succeeds.

The `CILProgramArtifact` is written via the existing PE packager before
invocation.

---

## Files changed

| File | Change |
|------|--------|
| `code/specs/LANG35-closure-backend-integration.md` | **CREATE** (this file) |
| `iir-to-beam/src/validate.rs` | Accept `alloc_closure` / `call_closure` |
| `iir-to-beam/src/lower.rs` | Add `OP_CALL_EXT`; intern `++`/`apply`; new match arms |
| `iir-to-wasm/src/validate.rs` | Add `ClosureOpcode` error before UntypedInstruction |
| `iir-to-jvm-class-file/src/validate.rs` | Same |
| `iir-to-cil-bytecode/src/validate.rs` | Same |
| `iir-to-beam/tests/test_backend.rs` | Unit tests + real-erl section |
| `iir-to-wasm/tests/test_backend.rs` | `ClosureOpcode` test + real-wasmtime section |
| `iir-to-jvm-class-file/tests/test_backend.rs` | `ClosureOpcode` test + real-java section |
| `iir-to-cil-bytecode/tests/test_backend.rs` | `ClosureOpcode` test + real-dotnet section |
| `iir-to-beam/CHANGELOG.md` | v0.3.0 entry |
| `iir-to-wasm/CHANGELOG.md` | v0.3.0 entry |
| `iir-to-jvm-class-file/CHANGELOG.md` | v0.3.0 entry |
| `iir-to-cil-bytecode/CHANGELOG.md` | v0.3.0 entry |

---

## Non-goals for LANG35

- **JVM closure lowering**: Full `alloc_closure`/`call_closure` → JVM bytecode
  (dispatch table approach). Deferred to LANG36.
- **CLR closure lowering**: Deferred to LANG37.
- **WASM closure lowering**: Requires WasmGC function references (WasmGC
  `(type $Closure (struct ...))`) and `call_indirect`. Deferred to LANG38.
- **Tail-call optimisation across closures**: Out of scope for all backends.
- **Multi-arity currying or partial application**: Out of scope.

---

## Testing

### Unit tests added (`iir-to-beam`)

- `test_alloc_closure_no_captures_accepted` — validator allows type `"closure"`
- `test_alloc_closure_with_captures_accepted` — same with 2 captures
- `test_call_closure_accepted` — validator allows `"any"` type on `call_closure`
- `test_alloc_closure_no_captures_lowering` — emits correct `put_list` sequence
- `test_alloc_closure_with_two_captures_lowering` — correct list-building BEAM
- `test_call_closure_lowering` — emits `get_list`, `put_list`, `call_ext` ×2

### Unit tests added (WASM / JVM / CLR)

- `test_alloc_closure_closure_opcode_error` — returns `ClosureOpcode` error
- `test_call_closure_closure_opcode_error` — same
- `test_closure_opcode_error_not_untyped` — error text contains `ClosureOpcode`
  and does NOT contain `UntypedInstruction`

### Real-VM tests (gated)

All real-VM tests use `if !binary_available("erl") { return; }` as the first
statement so they are silently skipped when the VM binary is not on PATH.

---

## Backward compatibility

- The old `call_builtin "make_closure"` / `"apply_closure"` forms remain fully
  supported in `twig-vm`; `iir-builtin-lowering` Phase 4 continues to upgrade
  them.  LANG35 adds no breaking changes to any existing interface.
