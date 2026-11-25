//! `lean-kernel`: a Lean 4 compatible proof kernel, written from Lean's documented type
//! theory (VER11a, `code/specs/VER11a-lean4-proof-kernel.md`).
//!
//! Slice 1 (KRN-002 part 1) is the **term layer** (VER11a section 4.2 "Expressions and
//! levels"). Slice 2 (version 0.2.0) added the **core of the type checker**: an
//! append-only environment of declarations, a local context, `infer` / `check` /
//! `ensure_sort` / `ensure_pi`, and `whnf` / `is_def_eq` with beta, zeta and delta.
//! Slice 3a (this one, version 0.3.0) adds **inductive types**: [`Environment::add_inductive`]
//! checks a single, non-mutual, non-nested inductive declaration (names, type telescope,
//! shared parameters, constructor results, universe constraint, strict positivity) and
//! *derives* its recursor; `whnf` gains **iota** (recursor on a constructor) and K-like
//! reduction; `is_def_eq` gains **proof irrelevance** and **eta for functions**. That is
//! enough for the first real proofs (Boolean truth tables, `Nat` arithmetic, `Eq.symm`).
//!
//! Version 0.4.0 adds the **trusted PIF reader** ([`pif`], VER32 section 5, backlog item
//! PIF-002): [`pif::read`] turns the text of a Proof Interchange Format file into calls on
//! [`Environment`], enforcing every reader rule of VER32 section 5.3 and failing closed on
//! everything the kernel does not implement yet.
//!
//! Still `Unsupported`, failing closed (slice 3b+): mutual and nested inductives, structure
//! projections and structure eta, unit-like types, quotients, literals (until the primitive
//! `Nat` / `String` definitions can be validated), lazy delta hints. Anything the kernel
//! cannot judge is *rejected*, never accepted. All recursion and work is bounded by
//! explicit [`Limits`] that report [`KernelError::ResourceLimit`].
//!
//! * [`name`]: hierarchical names (`Nat.add`, `_uniq.12`);
//! * [`level`]: universe levels with normalisation, equivalence and `>=` checks;
//! * [`expr`]: hash-consed expressions in a de Bruijn representation, with
//!   instantiate / abstract / lift / lower and universe-parameter instantiation;
//! * [`env`]: declarations (axiom, definition, theorem, opaque, and the kernel-produced
//!   inductive, constructor and recursor kinds) and the checked, append-only
//!   [`Environment`];
//! * [`inductive`]: inductive declarations, positivity, universe checks and recursor
//!   generation;
//! * [`pif`]: the PIF reader (the "Export reader" row of the line budget);
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
pub mod inductive;
pub mod infer;
pub mod lctx;
pub mod level;
pub mod name;
pub mod pif;
pub mod whnf;

pub use checker::{KResult, Limits, TypeChecker};
pub use env::{
    ConstructorInfo, DeclKind, Declaration, Environment, InductiveInfo, RecRule, RecursorInfo,
    ReducibilityHints,
};
pub use error::KernelError;
pub use inductive::{ConstructorDecl, InductiveDecl, InductiveTypeDecl, MAX_CONSTRUCTORS};
pub use lctx::{LocalContext, LocalDecl};

pub use expr::{
    checked_u32, BVarIndexTooLarge, Binder, BinderInfo, Expr, ExprKind, ExprStore, FVarId,
    FVarIdTaken, Literal, Meta, MAX_BVAR_INDEX,
};
pub use level::{
    levels_equiv, ArityMismatch, Level, LevelKind, LevelOffsetTooLarge, MAX_LEVEL_OFFSET,
};
pub use name::{Name, NamePart};
