//! The one error type of the type checker.
//!
//! # Fail closed
//!
//! Every way a declaration or a term can be wrong has its own variant, and *every*
//! function of the checker returns `Result<_, KernelError>`. There is no panic, `unwrap`
//! or `expect` on a path an input can reach: an input that the checker cannot fully
//! judge (an unsupported construct, a resource limit) is an `Err`, never an acceptance.
//! The kernel is the trusted base, so "I could not tell" must always mean "rejected".
//!
//! "Cannot be reached from input" is backed by explicit limits, each reported as
//! [`KernelError::ResourceLimit`] (see `checker.rs`): call depth, term depth, total steps,
//! interned nodes (also inside every store traversal), and the depth (128) and tree size
//! (512) of every universe level in a `Sort` or constant. Level arithmetic is checked, and
//! handles from a foreign store are [`KernelError::ForeignExpr`], not a panic.
//!
//! Variants that mention terms carry [`Expr`] handles. A handle is only meaningful with
//! the store that made it (the environment's), so printing needs that store; `Display`
//! therefore prints the handle's debug form.

use crate::expr::Expr;
use crate::name::Name;
use std::fmt;

/// Why a term or a declaration was rejected.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KernelError {
    /// A constant that is not in the environment (and so, in particular, a declaration
    /// that mentions itself, because its own name is not added until it is accepted).
    UnknownConstant(Name),
    /// A declaration whose name already exists. Redeclaring would let a later
    /// declaration silently change the meaning of earlier ones.
    AlreadyDeclared(Name),
    /// A constant applied to the wrong number of universe levels.
    LevelArityMismatch {
        /// The constant.
        name: Name,
        /// How many universe parameters its declaration has.
        expected: usize,
        /// How many levels it was given.
        found: usize,
    },
    /// A universe parameter used but not declared by the enclosing declaration.
    UndeclaredLevelParam(Name),
    /// A declaration that lists the same universe parameter twice.
    DuplicateLevelParam(Name),
    /// A type was required to be a `Sort` (or to have one as its type) and is not.
    NotASort(Expr),
    /// A function was required to have a `Pi` type and does not.
    NotAPi(Expr),
    /// The inferred type is not definitionally equal to the expected one.
    TypeMismatch {
        /// The type that was demanded.
        expected: Expr,
        /// The type that was found.
        found: Expr,
    },
    /// A theorem whose statement is not a proposition (its sort is not `Prop`).
    TheoremNotProp(Expr),
    /// A loose bound variable `#i` reached the checker: the term is not closed.
    LooseBVar(u32),
    /// A declaration mentioned a free variable; declarations must be closed.
    FreeVarInDeclaration,
    /// A free variable that is not in the local context.
    UnknownFVar,
    /// A construct this slice does not check yet. Rejected, never accepted by default.
    Unsupported(&'static str),
    /// A documented resource limit was hit (depth, steps or term size).
    ResourceLimit(&'static str),
    /// An inductive declaration that is malformed in the way the message says (type is
    /// not a telescope ending in a sort, constructor does not return the type, ...).
    InvalidInductive(&'static str),
    /// A constructor argument mentions the inductive type in a non-strictly-positive
    /// position. Carries the constructor's name.
    NonPositiveOccurrence(Name),
    /// A constructor argument lives in a universe above the inductive type's. Carries
    /// the constructor's name.
    UniverseTooLarge(Name),
    /// An expression handle made by a different `ExprStore` than the environment's.
    ForeignExpr,
    /// An internal invariant was violated; reported instead of panicking.
    Internal(&'static str),
}

impl fmt::Display for KernelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for KernelError {}
