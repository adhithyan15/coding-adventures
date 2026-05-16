//! # cas-bridge — Layer-1 cores ↔ symbolic-vm IR.
//!
//! Every Layer-1 numerical / statistical / financial core in the repo
//! ships a clean Rust API. This crate exposes the *same* functions to
//! the symbolic VM as IR handlers, so an `Apply(Symbol("Mean"), [...])`
//! evaluates to the same answer the direct Rust call would produce.
//!
//! That gives us four things in one library:
//!
//! 1. **A function registry the symbolic VM can dispatch through.** The
//!    same engine that evaluates Mathematica-style symbolic expressions
//!    (MACSYMA, the existing `cas-*` stack) now evaluates spreadsheet
//!    formulas and R-style expressions, without re-implementing the
//!    math.
//! 2. **Exact arithmetic when possible, float when not.** Inputs that
//!    are all `IRNode::Integer` flow through the cores as exact
//!    integers (where the core supports it); mixed integer/float
//!    propagates to float; symbolic inputs stay symbolic and pass
//!    through.
//! 3. **A single dispatch table** that's introspectable: callers can
//!    list the registered function names, query which core a name
//!    belongs to, and unregister at will.
//! 4. **A path to the future:** when `math-core`, `financial-core`,
//!    `lookup-core`, `text-core`, `datetime-core`, `engineering-core`
//!    land on `main`, this crate adds `register_<domain>_handlers`
//!    functions in lock-step, so a spreadsheet or R runtime gets
//!    every Layer-1 function via a single backend hookup.
//!
//! ## Where it fits
//!
//! ```text
//!   visicalc-modern / r-runtime / s-runtime / macsyma frontend
//!                              │
//!                              │  IRNode (Apply(Mean, [1, 2, 3]))
//!                              ▼
//!                   ┌──────────────────────┐
//!                   │      symbolic-vm     │
//!                   │   Backend / Handler  │
//!                   └─────────┬────────────┘
//!                             │  registered via:
//!                             ▼
//!                   ┌──────────────────────┐
//!                   │      cas-bridge      │ ← THIS CRATE
//!                   │  IRNode <-> Number   │
//!                   │  IRNode <-> Double   │
//!                   │  handler factories   │
//!                   └─────────┬────────────┘
//!                             │  delegates math to:
//!                             ▼
//!     statistics-core · math-core · financial-core · ...
//! ```
//!
//! ## Portability bar
//!
//! Per `backend-crate-catalog.md` §1: `forbid(unsafe_code)`, no
//! `#[cfg(target_os)]`, no I/O, no globals, WASM-compatible.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod convert;
pub mod registry;
pub mod statistics_handlers;

pub use registry::{HandlerRegistry, register_statistics_handlers};

// Re-export the IR types so downstream crates only pull in `cas-bridge`.
pub use symbolic_ir::{apply, flt, int, rat, sym, IRApply, IRNode};
pub use symbolic_vm::{Backend, Handler, VM};
