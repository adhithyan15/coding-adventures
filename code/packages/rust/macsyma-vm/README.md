# macsyma-vm

Macsyma's **own interpreter** for its v0 IIR subset, built directly on
`dynval-runtime`. It executes the `IIRModule` produced by
[`macsyma-iir-compiler`](../macsyma-iir-compiler) and returns a
`LispyValue`.

Part of the [`macsyma-iir-vm.md`](../../../specs/macsyma-iir-vm.md) spec —
the first bridge from a math language in this repo onto `interpreter_ir`
(IIR), the shared AOT/lang-vm chain, rather than only the Semantic IR
source-to-source translation pipeline `macsyma-to-semantic-ir` already
targets.

## Why a dedicated VM

Twig and McCarthy Lisp each already have their own small VM on top of
`dynval-runtime`'s tagged-`i64` value model (symbols, cons cells, nil),
deliberately *not* shared with one another — see `mccarthy-lisp-vm`'s own
README for the reasoning. Macsyma gets the same treatment: its own VM,
sharing only the `dynval-runtime` foundation.

## Instruction set (v0)

| Op             | Meaning                                                                 |
|----------------|--------------------------------------------------------------------------|
| `const`        | `Int(n)` → tagged int, `Int(0):ref<LispyPair>` → nil, `Var(name)` → interned symbol |
| `call_builtin` | `srcs[0]` is the builtin name (a `Var`), the rest are argument registers |
| `ret`          | return the value in `srcs[0]`                                            |

No branches, calls, or closures — v0's accepted grammar (literal
arithmetic, assignment, unevaluated symbolic expressions) needs none of
them. `call_builtin` covers both v0's real arithmetic (`+`/`-`/`*`/`/`,
genuinely executed here — not folded by the frontend) and its
unevaluated-symbolic-expression representation (`cons`, building the
`(head arg0 arg1 …)` chain `macsyma-iir-compiler` uses for a symbolic
`Apply` node, mirroring `mccarthy-lisp-iir-compiler`'s `QUOTE` lowering).

## Usage

```rust
use macsyma_vm::run;
// `module` is an IIRModule from macsyma-iir-compiler
let value = run(&module)?;        // a dynval_runtime::LispyValue
```

- `run(&IIRModule) -> Result<LispyValue, VmError>`
- `run_with_budget(&IIRModule, budget) -> Result<LispyValue, VmError>`

## Robustness

A flat dispatch loop with an instruction budget
(`DEFAULT_INSTRUCTION_BUDGET`) — no native recursion over the program.
Builtin traps (division by zero, overflow, wrong arity/type) surface as
`VmError::Runtime`, never panics.

## Known v0 gap: `/` truncates

`dynval-runtime` has no rational type; its `/` builtin is C-style
truncating integer division. `macsyma-iir-compiler` only ever emits a `/`
call when it has verified at compile time that the division is exact, so
this truncation is not reachable through the compiler — but the VM itself
does not re-check it (a hand-built module can still divide unevenly). See
[`macsyma-iir-vm.md`](../../../specs/macsyma-iir-vm.md) §3 and §6 (Wave 2:
bignum/rational support).
