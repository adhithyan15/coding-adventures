# mccarthy-lisp-vm

McCarthy 1960 Lisp's **own interpreter**, built directly on
`lispy-runtime`. It executes the `IIRModule` produced by
[`mccarthy-lisp-iir-compiler`](../mccarthy-lisp-iir-compiler) and
returns a `LispyValue`.

Part of **L2a** of the McCarthy Lisp implementation — see
[`MCCARTHY-LISP-PLAN.md`](../../../specs/MCCARTHY-LISP-PLAN.md).

## Why a dedicated VM

The interesting question for "running" McCarthy Lisp isn't *which VM* —
it's *what value model*. McCarthy programs return **symbols** and **cons
cells** (`(CAR '(A B C))` → `A`; `(CDR '(A B C))` → `(B C)`).

- **`vm-core`** is a *scalar* interpreter — its `Value` is only
  `Int / Float / Bool / Str / Null`. It has no symbol or cons
  representation, so it simply cannot run those programs.
- **`twig-vm`** *can* — but it is the VM for the **Twig** language
  (typed-CIR mnemonics, closures, module forms, …). McCarthy Lisp
  shouldn't ride on Twig's VM.

What both languages genuinely share is the **value model**:
`lispy-runtime`'s tagged-`i64` `LispyValue` (`int / nil / symbol / #t /
#f / heap-cons`), its interner, and its `cons / car / cdr / pair? / not
/ equal?` builtins. So McCarthy Lisp gets its **own** small VM on that
foundation — this crate. (Twig is a typed Lisp; McCarthy is its untyped
cousin. Same value model, separate VMs.)

## Instruction set (through L2b)

| Op             | Meaning                                                           |
|----------------|------------------------------------------------------------------|
| `const`        | `Int(n)`→int, `Int(0):ref<LispyPair>`→nil, `Var(name)`→interned symbol, `Bool(b)`→bool |
| `call_builtin` | `srcs[0]` is the builtin name (a `Var`), the rest are args; dispatched to a `lispy-runtime` builtin |
| `mov`          | copy a register (`dest ← srcs[0]`)                                |
| `jmp`          | unconditional branch to the label in `srcs[0]`                   |
| `jmp_if_false` | branch to the label in `srcs[1]` when `srcs[0]` is falsy (`#f`/`nil`); else fall through |
| `label`        | branch-target marker (`srcs[0]` is its name)                    |
| `ret`          | return the value in `srcs[0]`                                     |

`mov` / `jmp` / `jmp_if_false` / `label` are what `COND` lowers to
(L2b). User-function `call` (for `LAMBDA` / `LABEL`) lands with L2c — the
VM grows to match the compiler phase by phase.

## Usage

```rust
use mccarthy_lisp_vm::run;
// `module` is an IIRModule from mccarthy-lisp-iir-compiler
let value = run(&module)?;        // a lispy_runtime::LispyValue
```

- `run(&IIRModule) -> Result<LispyValue, VmError>`
- `run_with_budget(&IIRModule, budget) -> Result<LispyValue, VmError>` —
  bounds the number of instructions executed (a backstop against runaway
  loops once control flow exists).

## Robustness

The interpreter is a flat dispatch loop with an instruction budget
(`DEFAULT_INSTRUCTION_BUDGET`); there is no native recursion over the
program, so untrusted IIR cannot blow the stack. Builtin traps (wrong
arity / type) surface as `VmError::Runtime`, never panics.
