# mccarthy-lisp-vm

McCarthy 1960 Lisp's **own interpreter**, built directly on
`dynval-runtime`. It executes the `IIRModule` produced by
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
`dynval-runtime`'s tagged-`i64` `LispyValue` (`int / nil / symbol / #t /
#f / heap-cons`), its interner, and its `cons / car / cdr / pair? / not
/ equal?` builtins. So McCarthy Lisp gets its **own** small VM on that
foundation — this crate. (Twig is a typed Lisp; McCarthy is its untyped
cousin. Same value model, separate VMs.)

## Instruction set (through L2c-3c)

| Op             | Meaning                                                           |
|----------------|------------------------------------------------------------------|
| `const`        | `Int(n)`→int, `Int(0):ref<LispyPair>`→nil, `Var(name)`→interned symbol, `Bool(b)`→bool |
| `call_builtin` | `srcs[0]` is the builtin name (a `Var`), the rest are args; dispatched to a `dynval-runtime` builtin |
| `call`         | `srcs[0]` is the callee function *name*; the rest are arguments. Runs the callee in a fresh frame (params bound to args) and returns its value into `dest` |
| `apply`        | `srcs[0]` is a register holding a *closure value* `(*CLOSURE* fn-name . env)`; the rest are arguments. Destructures the closure, flattens the captured `env` into the **leading** call args (then appends the supplied args), looks the function up by name, runs it in a fresh frame — **dynamic** dispatch with captured-variable binding |
| `mov`          | copy a register (`dest ← srcs[0]`)                                |
| `jmp`          | unconditional branch to the label in `srcs[0]`                   |
| `jmp_if_false` | branch to the label in `srcs[1]` when `srcs[0]` is falsy (`#f`/`nil`); else fall through |
| `label`        | branch-target marker (`srcs[0]` is its name)                    |
| `ret`          | return the value in `srcs[0]`                                     |

`mov`/`jmp`/`jmp_if_false`/`label` are what `COND` lowers to (L2b);
`call` is what `LAMBDA` application lowers to (L2c-1). Call nesting is
bounded by `MAX_CALL_DEPTH` (256) and the shared instruction budget, so
an untrusted self-recursive module errors cleanly rather than
overflowing the stack.

**`LABEL` recursion (L2c-2) needed no new opcode.** A named recursive
function `(LABEL F (LAMBDA … (F …) …))` compiles to a function whose body
simply `call`s itself by name — and `call` already resolves the callee
from the module and runs it in a fresh frame. So recursion "just works",
with the same `MAX_CALL_DEPTH` + instruction-budget guards turning a
non-terminating recursion into a clean `CallDepthExceeded` rather than a
native stack overflow.

**`apply` (L2c-3a) is the one new opcode for closures.** A `LAMBDA` used
as a value compiles to a closure `(*CLOSURE* fn-name . env)`; applying it
dispatches *dynamically* (the callee isn't a static name). The tag
`*CLOSURE*` is not a lexable McCarthy symbol, so a value the VM accepts as
a closure can only have been built by the compiler — never forged via
`QUOTE`; applying anything else is a clean `NotAClosure`. `apply` shares
`call`'s depth/budget guards, so the Ω combinator
`((LAMBDA (X) (X X)) (LAMBDA (X) (X X)))` terminates with
`CallDepthExceeded` rather than a stack overflow.

**Captured variables (L2c-3b).** A closure's `env = (v1 … vk)` holds the
values of the captured free variables. `apply` flattens `env` into the
leading call arguments (the lifted function's parameters are
`captured ∪ own`, captured first), so a closure built in one scope and
applied in another still sees the values it closed over —
`(((LAMBDA (X) (LAMBDA (Y) (CONS X Y))) 'A) 'B)` ⇒ `(A . B)`.

**Recursive closures (L2c-3c) need no VM change.** A `LABEL` used as a
value is a closure whose body recurses through an ordinary static `call`
to its own name, with the captured `env` supplied as leading `apply` args.
So `((LAMBDA (G) (G '(A B C))) (LABEL LAST (LAMBDA (L) (COND ((ATOM (CDR L))
(CAR L)) ('T (LAST (CDR L)))))))` ⇒ `C`, and a non-terminating recursive
closure still hits `CallDepthExceeded`.

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
