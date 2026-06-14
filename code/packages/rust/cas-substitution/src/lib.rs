//! # cas-substitution
//!
//! Two flavours of symbolic substitution over [`symbolic_ir`] trees.
//!
//! ## `subst` — structural substitution
//!
//! Walks an IR tree and replaces every node that is structurally equal to
//! `var` with `value`.  The search target can be any `IRNode`, not just a
//! symbol.
//!
//! ```rust
//! use symbolic_ir::{apply, int, sym, POW};
//! use cas_substitution::subst;
//!
//! // subst(2, x, x^2) → Pow(2, 2)  (un-simplified)
//! let expr = apply(sym(POW), vec![sym("x"), int(2)]);
//! let result = subst(int(2), &sym("x"), expr);
//! assert_eq!(result, apply(sym(POW), vec![int(2), int(2)]));
//! ```
//!
//! ## `replace_all` — pattern-aware substitution
//!
//! Applies a single `Rule(lhs, rhs)` (from `cas-pattern-matching`) at every
//! position in a tree where the LHS pattern matches.  The walk is top-down
//! and single-pass: once a node is rewritten, its replacement is not
//! searched again.
//!
//! ```rust
//! use symbolic_ir::{apply, int, sym, MUL, POW};
//! use cas_pattern_matching::{blank, named, rule};
//! use cas_substitution::replace_all;
//!
//! // Rule: Pow(a_, 2) → Mul(a_, a_)
//! let r = rule(
//!     apply(sym(POW), vec![named("a", blank()), int(2)]),
//!     apply(sym(MUL), vec![named("a", blank()), named("a", blank())]),
//! );
//! let expr = apply(sym(POW), vec![sym("y"), int(2)]);
//! let result = replace_all(expr, &r);
//! assert_eq!(result, apply(sym(MUL), vec![sym("y"), sym("y")]));
//! ```

pub mod replace_all;
pub mod subst;

pub use replace_all::{replace_all, replace_all_many};
pub use subst::{subst, subst_many};
