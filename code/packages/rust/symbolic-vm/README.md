# symbolic-vm (Rust)

A generic symbolic expression evaluator over the `symbolic-ir` tree
representation.  This is the Rust port of the Python `symbolic-vm` package.

## Architecture

```text
  IRNode (input)
     │
     ▼
  VM::eval
     │
     ├─ atom (Symbol) ──────→ Backend::lookup / on_unresolved
     │
     └─ Apply(head, args) ──→ evaluate args (unless held)
                                   │
                                   ├─ rewrite rules
                                   ├─ head handler (Backend::handler_for)
                                   ├─ user-defined function
                                   └─ Backend::on_unknown_head

  IRNode (output)
```

All policy decisions live in the `Backend` trait.  Two reference backends
are included:

| Backend | Unbound symbol | Unknown head | Symbolic arithmetic |
|---------|---------------|-------------|---------------------|
| `StrictBackend` | `panic!` | `panic!` | `panic!` |
| `SymbolicBackend` | returns symbol | returns expr | folds identities |

## Quick start

```rust
use symbolic_ir::{apply, int, sym, ADD, D, INTEGRATE, MUL, POW};
use symbolic_vm::{SymbolicBackend, VM};

let mut vm = VM::new(Box::new(SymbolicBackend::new()));

// Numeric fold
assert_eq!(vm.eval(apply(sym(ADD), vec![int(2), int(3)])), int(5));

// Identity fold: Add(x, 0) → x
let expr = apply(sym(ADD), vec![sym("x"), int(0)]);
assert_eq!(vm.eval(expr), sym("x"));

// Unbound free variable stays as-is
assert_eq!(vm.eval(sym("t")), sym("t"));

// Symbolic differentiation is installed only on SymbolicBackend.
let dx_x_sq = apply(sym(D), vec![apply(sym(POW), vec![sym("x"), int(2)]), sym("x")]);
assert_eq!(vm.eval(dx_x_sq), apply(sym(MUL), vec![int(2), sym("x")]));

// Symbolic integration has a small Phase 1 table on SymbolicBackend.
let int_x_sq = apply(sym(INTEGRATE), vec![apply(sym(POW), vec![sym("x"), int(2)]), sym("x")]);
assert_eq!(
    vm.eval(int_x_sq),
    apply(
        sym(MUL),
        vec![symbolic_ir::rat(1, 3), apply(sym(POW), vec![sym("x"), int(3)])],
    )
);
```

## Custom backend

Implement `Backend` to create a new CAS dialect:

```rust
use symbolic_vm::{Backend, Handler, VM};
use symbolic_ir::{IRApply, IRNode};
use std::collections::{HashMap, HashSet};

struct MyBackend { env: HashMap<String, IRNode>, held: HashSet<String> }

impl Backend for MyBackend {
    fn lookup(&self, name: &str) -> Option<IRNode> { self.env.get(name).cloned() }
    fn bind(&mut self, name: &str, value: IRNode) { self.env.insert(name.into(), value); }
    fn on_unresolved(&self, name: &str) -> IRNode { IRNode::Symbol(name.into()) }
    fn handler_for(&self, _name: &str) -> Option<&Handler> { None }
    fn hold_heads(&self) -> &HashSet<String> { &self.held }
}
```

## Exact arithmetic

The `handlers` module uses a `Numeric` enum (`Int(i64)`, `Rat(i64, i64)`,
`Float(f64)`) to preserve exactness:

- `Add(1/2, 1/3)` → `5/6` (not `0.8333…`)
- `Mul(2/3, 3/4)` → `1/2`
- `Pow(2, 10)` → `1024` (exact integer)
- `Div(1, 3)` → `1/3` (exact rational)

Integer overflow falls back to `Float`.

## Security: self-referential-reassignment DoS guard

`handlers::assign_handler` — the shared implementation behind every
consuming CAS runtime's `:=`/`=` — rejects a value before binding it if
either of two budgets is exceeded:

| Budget | Constant | Why |
|--------|----------|-----|
| Total `IRNode` count | `handlers::MAX_BOUND_VALUE_NODES` (100,000) | `a := a * a`, repeated, clones the whole current value into both operand positions, roughly *doubling* node count every step |
| Nesting depth | `handlers::MAX_BOUND_VALUE_DEPTH` (128) | `a := a + a`, repeated, additionally hits `Add`'s own flatten-then-left-associate canonicalization, whose rebuilt chain's depth equals its leaf count — also doubling, and dangerous *sooner* than node count because `VM::eval_symbol` natively re-walks a symbol's whole bound value on every lookup |

Both checks use an explicit-work-stack iterative walk
(`handlers::count_nodes_within_cap`/`handlers::depth_within_cap`, both
`pub`), never native recursion, so checking a pathological value can never
itself overflow the stack. On trip, `assign_handler` panics with a
descriptive message — the same failure shape it already uses for a
malformed (non-symbol) assignment target — which every consuming runtime's
own worker-thread `catch_unwind` boundary converts into a clean error.

A consuming runtime that bypasses `assign_handler` for its own plain
assignment (`axiom-runtime` is the one example in this repo — it binds
directly through `Backend::bind` rather than lowering to
`symbolic_ir::ASSIGN`) must apply the identical two checks at its own bind
site; both functions and constants are exported specifically for that.

## Stack position

```
symbolic-ir  ← symbolic-vm  ← macsyma-compiler (planned)
                             ← cas-simplify (planned)
                             ← cas-factor (planned)
```
