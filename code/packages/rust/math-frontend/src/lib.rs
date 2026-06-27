//! # math-frontend — a pluggable parser-frontend framework
//!
//! Reasoning and adjudication systems must accept mathematics in the **many notations**
//! people and models actually write: LaTeX, AsciiMath, MathML, Unicode/plain math, … We
//! must not hard-code each notation into every consumer. Instead, a *frontend* is a parser
//! for one notation that produces **one common neutral AST** — [`MathExpr`] — and every
//! consumer (a rule engine, a computer-algebra system, a renderer) lowers that single
//! tree. Adding a notation is "register one more frontend"; consumers don't change.
//!
//! This crate is the framework all frontends and consumers share:
//!
//! * [`MathExpr`] (+ [`Number`], [`BinOp`], …) — the notation-agnostic AST. Two source
//!   strings that mean the same math produce the same tree (`a \times b` ≡ `a \cdot b`);
//!   numbers are **exact-preserving** ([`Number`] never silently rounds to `f64`).
//! * [`MathFrontend`] — the contract a notation parser implements (total, panic-free,
//!   pure), with [`FrontendError`] (spanned) and [`Capabilities`] (what it can emit).
//! * [`FrontendRegistry`] — look a frontend up by name and parse through it.
//! * [`check_frontend`] — a shared conformance harness enforcing the contract.
//!
//! Parsing only: evaluation, simplification, and rendering belong to consumers, never to a
//! frontend. The first frontend (LaTeX) lands as its own crate and registers here.
//!
//! ## Example
//!
//! ```
//! use math_frontend::{FrontendRegistry, MathFrontend, MathExpr, Number, Capabilities, FrontendError};
//!
//! struct Int;                              // a toy frontend: parse a bare integer
//! impl MathFrontend for Int {
//!     fn name(&self) -> &str { "int" }
//!     fn parse(&self, s: &str) -> Result<MathExpr, FrontendError> {
//!         Number::parse(s).map(MathExpr::Number)
//!             .ok_or_else(|| FrontendError::new("int", "not an integer", (0, s.len())))
//!     }
//!     fn capabilities(&self) -> Capabilities { Capabilities::none() }
//! }
//!
//! let mut reg = FrontendRegistry::new();
//! reg.register(Box::new(Int));
//! assert_eq!(reg.parse("int", "42").unwrap(), MathExpr::Number(Number::from_i64(42)));
//! ```

mod conformance;
mod expr;
mod frontend;
mod registry;

pub use conformance::{check_frontend, ConformanceReport};
pub use expr::{BigOp, BinOp, Func, MathExpr, Number, RelOp, UnaryOp};
pub use frontend::{Capabilities, FrontendError, MathFrontend};
pub use registry::FrontendRegistry;
