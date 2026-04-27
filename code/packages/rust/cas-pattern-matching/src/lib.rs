//! # cas-pattern-matching
//!
//! Structural pattern matching and rewriting over [`symbolic_ir`] trees.
//!
//! ## Overview
//!
//! Patterns are represented as ordinary `IRNode` trees using three sentinel
//! head symbols:
//!
//! | Head | Meaning |
//! |------|---------|
//! | `Blank()` | Matches any single expression |
//! | `Blank(T)` | Matches any expression whose effective head is `T` |
//! | `Pattern(name, inner)` | Named capture — records `name → target` in [`Bindings`] |
//!
//! Rules are `Rule(lhs, rhs)` or `RuleDelayed(lhs, rhs)` applies.
//!
//! ## Example
//!
//! ```rust
//! use symbolic_ir::{apply, int, sym, ADD};
//! use cas_pattern_matching::{blank, named, rule, match_pattern, Bindings};
//!
//! // Pattern:  Add(x_, y_)  →  matches any Add(a, b) and binds x, y
//! let pat = apply(sym(ADD), vec![named("x", blank()), named("y", blank())]);
//! let target = apply(sym(ADD), vec![int(2), int(3)]);
//!
//! let bindings = match_pattern(&pat, &target, Bindings::empty()).unwrap();
//! assert_eq!(bindings.get("x"), Some(&int(2)));
//! assert_eq!(bindings.get("y"), Some(&int(3)));
//! ```

pub mod bindings;
pub mod matcher;
pub mod nodes;
pub mod rewriter;

pub use bindings::Bindings;
pub use matcher::match_pattern;
pub use nodes::{blank, blank_typed, named, rule, rule_delayed};
pub use rewriter::{apply_rule, rewrite, RewriteCycleError};
