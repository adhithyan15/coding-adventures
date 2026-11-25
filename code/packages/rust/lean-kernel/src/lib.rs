//! `lean-kernel`: a Lean 4 compatible proof kernel, written from Lean's documented type
//! theory (VER11a, `code/specs/VER11a-lean4-proof-kernel.md`).
//!
//! Slice 1 (KRN-002 part 1) is the **term layer** (VER11a section 4.2 "Expressions and
//! levels"). Slice 2 (this one, version 0.2.0) adds the **core of the type checker**:
//! an append-only environment of declarations, a local context, `infer` / `check` /
//! `ensure_sort` / `ensure_pi`, and `whnf` / `is_def_eq` with beta, zeta and delta only.
//! It has no inductive types, recursors, projections, eta, proof irrelevance or literal
//! arithmetic yet (slice 3): anything it cannot judge is *rejected*, never accepted. All
//! recursion and work is bounded by explicit [`Limits`] that report
//! [`KernelError::ResourceLimit`].
//!
//! * [`name`]: hierarchical names (`Nat.add`, `_uniq.12`);
//! * [`level`]: universe levels with normalisation, equivalence and `>=` checks;
//! * [`expr`]: hash-consed expressions in a de Bruijn representation, with
//!   instantiate / abstract / lift / lower and universe-parameter instantiation;
//! * [`env`]: declarations (axiom, definition, theorem, opaque) and the checked,
//!   append-only [`Environment`];
//! * [`lctx`]: the local context of free variables;
//! * [`checker`], [`infer`], [`whnf`], [`defeq`]: the [`TypeChecker`];
//! * [`error`]: the single [`KernelError`] type.
//!
//! The crate has no third-party dependencies (only `bignum-core`, for `Nat` literals),
//! no I/O and no `unsafe`: the kernel is the trusted base of every proof checked with it.

#![forbid(unsafe_code)]

pub mod checker;
pub mod defeq;
pub mod env;
pub mod error;
pub mod expr;
pub mod infer;
pub mod lctx;
pub mod level;
pub mod name;
pub mod whnf;

pub use checker::{KResult, Limits, TypeChecker};
pub use env::{DeclKind, Declaration, Environment, ReducibilityHints};
pub use error::KernelError;
pub use lctx::{LocalContext, LocalDecl};

pub use expr::{
    checked_u32, BVarIndexTooLarge, Binder, BinderInfo, Expr, ExprKind, ExprStore, FVarId,
    FVarIdTaken, Literal, Meta, MAX_BVAR_INDEX,
};
pub use level::{
    levels_equiv, ArityMismatch, Level, LevelKind, LevelOffsetTooLarge, MAX_LEVEL_OFFSET,
};
pub use name::{Name, NamePart};
