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
use symbolic_ir::{apply, int, sym, ADD, MUL};
use symbolic_vm::{SymbolicBackend, VM};

let mut vm = VM::new(Box::new(SymbolicBackend::new()));

// Numeric fold
assert_eq!(vm.eval(apply(sym(ADD), vec![int(2), int(3)])), int(5));

// Identity fold: Add(x, 0) → x
let expr = apply(sym(ADD), vec![sym("x"), int(0)]);
assert_eq!(vm.eval(expr), sym("x"));

// Unbound free variable stays as-is
assert_eq!(vm.eval(sym("t")), sym("t"));
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

## Stack position

```
symbolic-ir  ← symbolic-vm  ← macsyma-compiler (planned)
                             ← cas-simplify (planned)
                             ← cas-factor (planned)
```
