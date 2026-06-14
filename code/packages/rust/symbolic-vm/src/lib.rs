//! # symbolic-vm
//!
//! A generic symbolic expression evaluator over [`symbolic_ir`] trees.
//!
//! The evaluator is policy-free: every language-specific decision
//! (what to do with an unbound name, which built-in operations exist,
//! whether to simplify or leave expressions symbolic) lives in a
//! [`Backend`] implementation.
//!
//! ## Architecture
//!
//! ```text
//!   IRNode (input)
//!      │
//!      ▼
//!   VM::eval
//!      │
//!      ├─ atom (Symbol) ──────→ Backend::lookup / on_unresolved
//!      │
//!      └─ Apply(head, args) ──→ evaluate args (unless held)
//!                                    │
//!                                    ├─ rewrite rules (Backend::rules)
//!                                    │
//!                                    ├─ head handler (Backend::handler_for)
//!                                    │
//!                                    ├─ user-defined function (stored Define)
//!                                    │
//!                                    └─ Backend::on_unknown_head
//!
//!   IRNode (output)
//! ```
//!
//! Two reference backends ship with this crate:
//!
//! - [`StrictBackend`] — every name must be bound, every head must have a
//!   handler, arithmetic must be fully numeric.  Raises on unknowns.
//! - [`SymbolicBackend`] — unbound names stay as free symbols; algebraic
//!   identities fold trivial cases; unknown heads pass through untouched.
//!
//! ## Example
//!
//! ```rust
//! use symbolic_ir::{apply, int, sym, ADD};
//! use symbolic_vm::{SymbolicBackend, VM};
//!
//! let mut vm = VM::new(Box::new(SymbolicBackend::new()));
//!
//! // Add(2, 3)  →  5
//! let expr = apply(sym(ADD), vec![int(2), int(3)]);
//! assert_eq!(vm.eval(expr), int(5));
//!
//! // Add(x, 0)  →  x  (identity fold)
//! let expr2 = apply(sym(ADD), vec![sym("x"), int(0)]);
//! assert_eq!(vm.eval(expr2), sym("x"));
//! ```

pub mod backend;
pub mod backends;
pub mod handlers;
pub mod vm;

pub use backend::{Backend, Handler};
pub use backends::{StrictBackend, SymbolicBackend};
pub use vm::VM;
