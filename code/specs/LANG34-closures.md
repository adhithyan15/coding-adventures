# LANG34 — First-Class Closure Opcodes at the IIR Level

## Status

Draft → Implementation

## Motivation

Before LANG34, closures in the Twig VM are handled entirely through two
`call_builtin` names:

```
%s0 = const("__lambda_0")               : any    ← string_arg indirection
%c0 = call_builtin("make_closure", %s0, %cap0, %cap1) : any
...
%r0 = call_builtin("apply_closure", %c0, %arg0) : any
```

The `%s0 = const(...)` instruction exists only to materialise the function name
as a runtime value so it can be passed as a `call_builtin` argument.  This
convention was introduced in LANG31 before `Operand::Str` existed.  LANG32
added `Operand::Str` — a compile-time string literal that backends must NOT look
up in the register file.  The `string_arg` / `const` indirection is therefore
now dead weight: the fn_name can be carried inline in the instruction.

Promoting closures to first-class IIR opcodes has three benefits:

1. **Cleaner IR** — no spurious `const` instructions cluttering the instruction
   stream before every `alloc_closure`.
2. **Static analysability** — the closure's target function name is statically
   visible in the instruction operands.  Passes can reason about it without
   chasing register definitions.
3. **Backend readiness** — `iir-to-*` backends (WASM, JVM, CLR, BEAM) need a
   well-typed first-class opcode, not a `call_builtin` string, to emit closure
   allocation into target bytecode.

---

## New Opcodes

### `alloc_closure`

```
dest = alloc_closure(Str(fn_name), Var(cap0), Var(cap1), …) : "closure"
```

**Purpose:** Allocate a closure for the named IIR function, capturing the
provided variables by value.

**Operands:**

| Index | Kind | Meaning |
|-------|------|---------|
| `srcs[0]` | `Operand::Str(fn_name)` | Compile-time name of the IIR function to close over.  Must be `Operand::Str`, NOT a register reference. |
| `srcs[1..]` | `Operand::Var(name)` | Captured variables, in the same order as the inner function's leading parameters. |

**Result:** A closure handle stored in `dest`.  The `type_hint` is `"closure"`.

**Allocation:** This instruction allocates a heap object.  It sets `may_alloc = true`.

**Semantics:** Equivalent to `lispy_runtime::heap::alloc_closure(intern(fn_name), captures)`.

---

### `call_closure`

```
dest = call_closure(Var(handle), Var(arg0), Var(arg1), …) : "any"
```

**Purpose:** Invoke a closure, passing explicit arguments.  The closure's captured
variables are prepended by the runtime before calling the underlying function.

**Operands:**

| Index | Kind | Meaning |
|-------|------|---------|
| `srcs[0]` | `Operand::Var(name)` | The closure handle produced by `alloc_closure`. |
| `srcs[1..]` | `Operand::Var(name)` | User-visible arguments, in order. |

**Result:** The return value of the closed-over function, stored in `dest`.
The `type_hint` is `"any"` because Twig is dynamically typed.

---

## Changes from LANG31 `call_builtin` Form

| Aspect | LANG31 `call_builtin` | LANG34 first-class opcode |
|--------|-----------------------|--------------------------|
| fn_name location | Register (%s0 = const("name")) | `Operand::Str` in `srcs[0]` |
| Extra `const` instruction | Always emitted | Not needed |
| Opcode | `"call_builtin"` | `"alloc_closure"` / `"call_closure"` |
| type_hint (alloc) | `"any"` | `"closure"` |
| Static analysability | Must chase const defs | Inline in instruction |

The old `call_builtin "make_closure"` and `"apply_closure"` forms are
**deprecated but not removed**.  They continue to work in the twig-vm dispatcher
for backward compatibility.  The `iir-builtin-lowering` Phase 4 pass rewrites
them to the new opcodes automatically.

---

## IIR Type System

A new type constant `CLOSURE_TYPE = "closure"` is added to `interpreter-ir::opcodes`.

`"closure"` is **not** in the `CONCRETE_TYPES` slice — it is not a scalar numeric
type and the `iir-to-*` numeric backends reject it (correct behaviour: closures
are not JVM primitives).

`is_closure_op(op)` returns `true` for `"alloc_closure"` and `"call_closure"`.

`is_value_producing(op)` returns `true` for both (both have a `dest`).

`is_allocating(op)` returns `true` for `"alloc_closure"` (sets `may_alloc`).

`is_known_op(op)` returns `true` for both.

---

## Affected Components

### `interpreter-ir/src/opcodes.rs`

- Add `pub const CLOSURE_TYPE: &str = "closure";`
- Add `pub fn is_closure_op(op: &str) -> bool`
- Update `is_value_producing` to include `"alloc_closure"` and `"call_closure"`
- Update `is_allocating` (if it exists) / document `may_alloc` on `alloc_closure`
- Update `is_known_op` to include both new opcodes

### `twig-vm/src/dispatch.rs`

New match arms in the main dispatch loop:

```rust
"alloc_closure" => {
    exec_alloc_closure(instr, &mut frame)?;
    pc += 1;
}
"call_closure" => {
    exec_call_closure(module, instr, &mut frame, depth, budget, globals, ic_table, profile, debug)?;
    pc += 1;
}
```

`exec_alloc_closure`: reads `Operand::Str(fn_name)` from `srcs[0]`, interns the
name, collects captures from `srcs[1..]`, calls
`lispy_runtime::heap::alloc_closure`.

`exec_call_closure`: reads closure handle from `srcs[0]` (unlike `apply_closure`
which reads from `srcs[1]`), reads user args from `srcs[1..]`, otherwise
identical logic to `exec_apply_closure`.

### `twig-ir-compiler/src/compiler.rs`

`compile_anonymous_lambda`:
- Replace `string_arg` + `call_builtin "make_closure"` with `alloc_closure`
- `srcs = [Operand::Str(fn_name)] ++ captures_as_var`
- `type_hint = "closure"`

`compile_apply` (indirect path):
- Replace `call_builtin "apply_closure" handle args...` with `call_closure handle args...`
- `srcs = [Operand::Var(fn_handle)] ++ args_as_var`
- `type_hint = "any"`

The `string_arg` helper is **retained** — still used by `global_set`/`global_get`
and `make_symbol` paths.

### `iir-builtin-lowering/src/closure.rs` (new)

Phase 4 of the lowering pipeline.  Rewrites legacy `call_builtin "make_closure"`
and `"apply_closure"` instructions to the new opcodes.

Algorithm per function:
1. Build a `const`-binding map: `HashMap<dest_name, literal_str>` from any
   `const` whose `srcs[0]` is `Operand::Var(literal)` or `Operand::Str(literal)`.
2. Walk instructions and collect rewrites:
   - `call_builtin "make_closure" fn_name_reg cap0...`:
     look up `fn_name_reg` in the const map, rewrite to `alloc_closure`.
     Mark the preceding `const` as dead (track by dest name in a removal set).
   - `call_builtin "apply_closure" handle arg0...`:
     rewrite to `call_closure` with `srcs[0] = Var(handle)`, `srcs[1..] = args`.
3. Build the new instruction list, skipping dead `const` instructions.

Public API: `pub fn lower_closure_builtins(module: &mut IIRModule)` — infallible.

### `iir-builtin-lowering/src/lib.rs`

- `pub mod closure;`
- `pub use closure::lower_closure_builtins;`
- Phase 4 call in `lower_builtins` after Phase 3 (global_io):

```rust
// ── Phase 4: closure builtins (LANG34) ───────────────────────────────────
closure::lower_closure_builtins(module);
```

---

## Backward Compatibility

- `call_builtin "make_closure"` and `"apply_closure"` remain valid in the
  twig-vm dispatcher via `exec_call_builtin`.  Existing compiled Twig programs
  (and hand-built tests using the old form) continue to work unchanged.
- `iir-builtin-lowering::lower_builtins` will now automatically upgrade them to
  the new opcodes as part of the standard pipeline.
- `iir-type-checker` does not need changes — it passes through `"any"` and
  `"closure"` type hints without error.

---

## Non-Goals for LANG34

- **`iir-to-*` backends (WASM, JVM, CLR, BEAM)**: these backends do not yet
  support `alloc_closure` / `call_closure`.  They will return a validation error
  for these opcodes.  Backend support is LANG35+.
- **Closure serialisation / deopt**: out of scope.
- **Multi-arity currying**: out of scope.
- **Tail-call elimination across closure boundaries**: out of scope.

---

## Example

Twig source:
```twig
(define adder (lambda (x) (lambda (y) (+ x y))))
(define add5 (adder 5))
(add5 3)
```

LANG31 IR (before LANG34):
```
; compile_anonymous_lambda for (lambda (y) (+ x y)):
%s0 = const("__lambda_1") : any          ← string_arg
%c0 = call_builtin("make_closure", %s0, %x) : any
ret(%c0) : any

; call site for (add5 3):
%r0 = call_builtin("apply_closure", %add5, 3) : any
```

LANG34 IR:
```
; compile_anonymous_lambda for (lambda (y) (+ x y)):
%c0 = alloc_closure(Str("__lambda_1"), %x) : closure   ← no const needed

; call site for (add5 3):
%r0 = call_closure(%add5, 3) : any
```

---

## Testing

- `interpreter-ir`: unit tests for `is_closure_op`, `CLOSURE_TYPE`
- `twig-vm`: hand-built `IIRModule` tests for `alloc_closure` and `call_closure` dispatch;
  end-to-end lambda programs via `twig_vm::run`
- `twig-ir-compiler`: assert compiler emits `alloc_closure`/`call_closure` (not `call_builtin`);
  assert no superfluous `const` before `alloc_closure`
- `iir-builtin-lowering`: Phase 4 unit tests; idempotency; mixed old+new forms
