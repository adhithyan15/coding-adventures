# iir-to-beam

**IIR → BEAM bytecode backend** — lowers an `IIRModule` (from the
`interpreter-ir` crate) directly to a `BEAMModule` (from the `ir-to-beam`
encoder) **without going through the deprecated `compiler-ir` layer**.

## Overview

```
IIRModule                              ← interpreter-ir crate
   │
   ▼  validate_for_beam()
   │  Returns Vec<String> of errors; empty = safe to lower.
   │
   ▼  lower_iir_to_beam()
BEAMModule                             ← ir-to-beam encoder types
   │
   ▼  encode_beam()
Vec<u8>   (.beam file ready to load into Erlang/OTP)
```

The `compiler-ir` crate represents a flat, low-level machine IR (registers are
plain integers, there is only one function, types are not tracked).
`interpreter-ir` (IIR) is richer: it carries named variables, static type hints,
multiple functions with parameters, labels, and comparison operators.  This crate
bridges that richer world to BEAM without any loss of information through a
deprecated intermediate.

## Why IIR → BEAM directly?

BEAM is a **register machine** like IIR, not a stack machine like JVM or WASM.
The mapping is natural:

| IIR concept | BEAM concept |
|-------------|-------------|
| Named variable | x-register (allocated by a simple scan) |
| `const` instruction | `move {i,val} {x,reg}` |
| `add`/`sub`/etc. | `gc_bif2 erlang:+/2 …` |
| `cmp_eq` | synthesized via `is_eq_exact` + two `move` + label |
| `label` | `{label, {u,N}}` |
| `jmp_if_true` | `is_eq_exact` (branch if false) + `jump` |
| `call fn` | move args to x0…xN, `call`, move x0 to dest |

## Quick start

```rust
use interpreter_ir::{IIRModule, IIRFunction, IIRInstr, Operand};
use iir_to_beam::{validate_for_beam, lower_iir_to_beam, IIRBeamConfig, encode_beam};

// Build a module with one function: add(a: i32, b: i32) -> i32
let fn_ = IIRFunction::new(
    "add",
    vec![("a".into(), "i32".into()), ("b".into(), "i32".into())],
    "i32",
    vec![
        IIRInstr::new("add", Some("result".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())], "i32"),
        IIRInstr::new("ret", None,
            vec![Operand::Var("result".into())], "i32"),
    ],
);
let module = IIRModule {
    name: "mymod".into(),
    functions: vec![fn_],
    entry_point: Some("add".into()),
    language: "tetrad".into(),
};

// Step 1 — validate
let errors = validate_for_beam(&module);
assert!(errors.is_empty(), "validation errors: {:?}", errors);

// Step 2 — lower
let config = IIRBeamConfig::new("mymod");
let beam_module = lower_iir_to_beam(&module, &config).unwrap();

// Step 3 — encode to bytes
let bytes = encode_beam(&beam_module);
assert_eq!(&bytes[0..4], b"FOR1");
```

## Supported opcodes

| IIR op | BEAM emission |
|--------|---------------|
| `const` (Int) | `move {i,val} {x,rd}` |
| `const` (Bool) | `move {i,0or1} {x,rd}` |
| `add` | `gc_bif2 erlang:+/2` |
| `sub` | `gc_bif2 erlang:-/2` |
| `mul` | `gc_bif2 erlang:*/2` |
| `div` | `gc_bif2 erlang:div/2` |
| `mod` | `gc_bif2 erlang:rem/2` |
| `neg` | `gc_bif1 erlang:-/1` |
| `and` | `gc_bif2 erlang:band/2` |
| `or`  | `gc_bif2 erlang:bor/2` |
| `xor` | `gc_bif2 erlang:bxor/2` |
| `not` | `gc_bif1 erlang:bnot/1` |
| `shl` | `gc_bif2 erlang:bsl/2` |
| `shr` | `gc_bif2 erlang:bsr/2` |
| `cmp_eq` | `move 0 rd; is_eq_exact → synth; move 1 rd; label synth` |
| `cmp_ne` | `move 0 rd; is_ne_exact → synth; move 1 rd; label synth` |
| `cmp_lt` | `move 0 rd; is_lt(r1,r2) → synth; move 1 rd; label synth` |
| `cmp_le` | `move 0 rd; is_ge(r2,r1) → synth; move 1 rd; label synth` |
| `cmp_gt` | `move 0 rd; is_lt(r2,r1) → synth; move 1 rd; label synth` |
| `cmp_ge` | `move 0 rd; is_ge(r1,r2) → synth; move 1 rd; label synth` |
| `label` | `{label {u,N}}` |
| `jmp` | `{jump {f,N}}` |
| `jmp_if_true` | `is_eq_exact(cond,0) → fall; jump target; label fall` |
| `jmp_if_false`| `is_ne_exact(cond,0) → fall; jump target; label fall` |
| `ret` | `move {x,r} {x,0}; return` |
| `ret_void` | `return` |
| `call` | move args to x0…; `call {u,arity} {f,entry}`; move x0 → dest |
| `load_reg` | `move {x,v} {x,rd}` |
| `store_reg` | `move {x,src} {x,v}` |
| `type_assert` | nop (erased at lowering time) |

## Unsupported (validation rejects)

`call_builtin`, `io_in`, `io_out`, `cast`, `load_mem`, `store_mem`, `alloc`,
`box`, `unbox`, `field_load`, `field_store`, `is_null`, `safepoint`, and any
instruction with `type_hint` of `"any"`, `"polymorphic"`, `"str"`, or `"ref<…>"`.
Float constant operands are also rejected — BEAM integer arithmetic cannot hold
IEEE-754 doubles without boxing.

## Module structure

```
{label, L_fi}.
{func_info, {a,ModAtom}, {a,FnAtom}, {u,Arity}}.
{label, L_entry}.          ← referenced by ExpT
  … translated IIR instructions …
{label, L_fi2}.            ← next function starts here
{func_info, …}
…
{int_code_end}.
```

## Register allocation

Variables are allocated to x-registers via a deterministic two-pass scan:

1. Function parameters → x0, x1, x2, … (in order)
2. All `dest` names and `Var` src operands → next available register,
   or the existing register if the name was already seen.

The same variable name always maps to the same register within one function.
This is a simple linear scan — no liveness analysis, no spilling.

## BEAM label numbering

A global counter (starting at 1) increments across all functions.  Each
function consumes two labels for its preamble (func_info label + entry label),
then one per `label` IIR instruction, plus extra synthetic labels for
comparison synthesis and conditional-branch synthesis.

## Crate layout

| Module | Responsibility |
|--------|----------------|
| `validate` | Pre-flight checks on `IIRModule` |
| `lower` | Two-pass IIR → BEAM lowering; error types; config |
| `codegen` | `IIRBeamCodeGenerator` — thin adapter (`name` / `validate` / `generate`) |
| `lib` | Re-exports; crate entry point |
