//! `lean-kernel`: a Lean 4 compatible proof kernel, written from Lean's documented type
//! theory (VER11a, `code/specs/VER11a-lean4-proof-kernel.md`).
//!
//! This first slice is the **term layer** only (backlog item KRN-002, part 1, VER11a
//! section 4.2 "Expressions and levels"): there is no type checker yet.
//!
//! * [`name`]: hierarchical names (`Nat.add`, `_uniq.12`);
//! * [`level`]: universe levels with normalisation, equivalence and `>=` checks;
//! * [`expr`]: hash-consed expressions in a de Bruijn representation, with
//!   instantiate / abstract / lift / lower and universe-parameter instantiation.
//!
//! The crate has no third-party dependencies (only `bignum-core`, for `Nat` literals),
//! no I/O and no `unsafe`: the kernel is the trusted base of every proof checked with it.

#![forbid(unsafe_code)]

pub mod expr;
pub mod level;
pub mod name;

pub use expr::{
    checked_u32, BVarIndexTooLarge, Binder, BinderInfo, Expr, ExprKind, ExprStore, FVarId,
    FVarIdTaken, Literal, Meta, MAX_BVAR_INDEX,
};
pub use level::{
    levels_equiv, ArityMismatch, Level, LevelKind, LevelOffsetTooLarge, MAX_LEVEL_OFFSET,
};
pub use name::{Name, NamePart};
