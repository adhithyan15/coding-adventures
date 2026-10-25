//! # Release profiles: what an ECMAScript edition *means* for the subset
//!
//! LANG84 separates two questions about a release:
//!
//! ```text
//!   what does it LEX and PARSE?   its own es<edition>.tokens / .grammar pair
//!   what does the program MEAN?   its profile (this file)
//! ```
//!
//! For `es-number-log` every claiming edition means the same thing. A
//! JavaScript Number has been an IEEE 754 double since ES1, the arithmetic
//! operators the subset admits (`+ - * /`, unary `-`) are defined
//! identically in every edition, and `console.log` renders a Number with
//! `Number::toString`, which is also unchanged. So there is one profile.
//!
//! Editions still differ in what they *accept*: numeric separators
//! (`1_000`) are ES2021, so es2016–es2020 refuse that corpus case while
//! lexing it. That is a grammar difference, not a profile difference, and
//! the release sweep sees it as a second IIR-identity class.
//!
//! `release_inventory.py` reads the `name:` fields below: a `releases.csv`
//! row's `profile` cell must name one of them.

/// What a claiming release means for the subset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReleaseProfile {
    /// The `profile` cell of `code/grammars/ecmascript/releases.csv`.
    pub name: &'static str,
    /// The subset this profile's releases claim, as `subsets.csv` names it.
    pub subset: &'static str,
}

/// ES2016 through ES2025: IEEE doubles, `Number::toString` rendering.
pub const ECMASCRIPT: ReleaseProfile = ReleaseProfile {
    name: "ecmascript",
    subset: "es-number-log",
};

/// Every profile this crate defines, for tooling and tests.
pub const PROFILES: &[ReleaseProfile] = &[ECMASCRIPT];
