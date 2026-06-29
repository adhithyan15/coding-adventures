//! The frontend contract: the [`MathFrontend`] trait, its [`FrontendError`], and the
//! [`Capabilities`] a frontend advertises.
//!
//! A *frontend* is a parser for one input notation (LaTeX, AsciiMath, MathML, …) that
//! produces the neutral [`crate::MathExpr`]. Consumers depend on this trait, never on a
//! concrete notation — so supporting a new notation is "add one more frontend", with no
//! change to any consumer.

use crate::expr::MathExpr;

/// A parse failure from a frontend, carrying a byte span into the source so a caller can
/// underline the exact offending slice. `span` is half-open `[start, end)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrontendError {
    /// Which frontend raised the error (its [`MathFrontend::name`]).
    pub frontend: String,
    /// Human-readable description.
    pub message: String,
    /// Half-open byte span `[start, end)` into the source.
    pub span: (usize, usize),
}

impl FrontendError {
    pub fn new(frontend: impl Into<String>, message: impl Into<String>, span: (usize, usize)) -> Self {
        FrontendError {
            frontend: frontend.into(),
            message: message.into(),
            span,
        }
    }
}

impl std::fmt::Display for FrontendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} frontend: {} (bytes {}..{})",
            self.frontend, self.message, self.span.0, self.span.1
        )
    }
}

impl std::error::Error for FrontendError {}

/// What neutral constructs a frontend can currently emit. A consumer inspects this to
/// **gate gracefully** — e.g. "does this frontend support matrices yet?" — instead of
/// discovering a gap only when a parse unexpectedly fails. As a frontend matures it flips
/// more flags on; the shared conformance harness checks that the flags match reality.
///
/// All flags default to `false`; build with the `with_*` setters:
/// `Capabilities::none().with_fractions().with_powers()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Capabilities {
    pub fractions: bool,
    pub roots: bool,
    pub powers: bool,
    pub functions: bool,
    pub big_operators: bool,
    pub relations: bool,
    pub matrices: bool,
    pub implicit_mul: bool,
    pub text: bool,
    /// Emits the ± / ∓ operators ([`crate::BinOp::PlusMinus`] / [`crate::BinOp::MinusPlus`]).
    pub plusminus: bool,
    /// Emits binomial coefficients ([`crate::MathExpr::Binom`]).
    pub binomials: bool,
    /// Emits diacritical accents ([`crate::MathExpr::Accent`]: `\hat{x}`, `\bar{y}`, `\vec{v}`, …).
    pub accents: bool,
}

impl Capabilities {
    /// A frontend that supports nothing beyond the always-present core (numbers,
    /// symbols, the four arithmetic ops, grouping, unary minus).
    pub fn none() -> Self {
        Capabilities::default()
    }

    /// Everything on — handy for a fully-featured frontend's declaration.
    pub fn all() -> Self {
        Capabilities {
            fractions: true,
            roots: true,
            powers: true,
            functions: true,
            big_operators: true,
            relations: true,
            matrices: true,
            implicit_mul: true,
            text: true,
            plusminus: true,
            binomials: true,
            accents: true,
        }
    }

    pub fn with_fractions(mut self) -> Self { self.fractions = true; self }
    pub fn with_roots(mut self) -> Self { self.roots = true; self }
    pub fn with_powers(mut self) -> Self { self.powers = true; self }
    pub fn with_functions(mut self) -> Self { self.functions = true; self }
    pub fn with_big_operators(mut self) -> Self { self.big_operators = true; self }
    pub fn with_relations(mut self) -> Self { self.relations = true; self }
    pub fn with_matrices(mut self) -> Self { self.matrices = true; self }
    pub fn with_implicit_mul(mut self) -> Self { self.implicit_mul = true; self }
    pub fn with_text(mut self) -> Self { self.text = true; self }
    pub fn with_plusminus(mut self) -> Self { self.plusminus = true; self }
    pub fn with_binomials(mut self) -> Self { self.binomials = true; self }
    pub fn with_accents(mut self) -> Self { self.accents = true; self }
}

/// The contract every notation parser implements.
///
/// Implementors MUST be:
/// * **total & panic-free** — every input yields `Ok(MathExpr)` or `Err(FrontendError)`;
/// * **pure** — no I/O, no global mutable state, no network;
/// * **honest** — [`capabilities`](MathFrontend::capabilities) reflects what `parse`
///   actually emits (the conformance harness enforces this).
pub trait MathFrontend {
    /// A stable identifier, e.g. `"latex"`, `"asciimath"`, `"mathml"`.
    fn name(&self) -> &str;

    /// Parse one source string in this notation into the neutral AST.
    fn parse(&self, src: &str) -> Result<MathExpr, FrontendError>;

    /// Which neutral constructs this frontend can currently emit.
    fn capabilities(&self) -> Capabilities;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capabilities_builder_sets_only_named_flags() {
        let c = Capabilities::none().with_fractions().with_powers();
        assert!(c.fractions && c.powers);
        assert!(!c.matrices && !c.functions);
    }

    #[test]
    fn capabilities_all_and_none() {
        assert_eq!(Capabilities::none(), Capabilities::default());
        let a = Capabilities::all();
        assert!(a.fractions && a.matrices && a.text && a.relations);
        assert!(a.plusminus && a.binomials);
        // none() leaves the new flags off; builders flip exactly them.
        assert!(!Capabilities::none().plusminus && !Capabilities::none().binomials);
        let c = Capabilities::none().with_plusminus().with_binomials();
        assert!(c.plusminus && c.binomials && !c.fractions);
    }

    #[test]
    fn error_displays_with_frontend_and_span() {
        let e = FrontendError::new("latex", "unbalanced braces", (3, 5));
        let s = format!("{e}");
        assert!(s.contains("latex") && s.contains("unbalanced braces") && s.contains("3..5"));
    }
}
