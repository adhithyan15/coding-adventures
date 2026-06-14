//! Parser-feedback contract.
//!
//! The lexer queries the parser via this trait to disambiguate
//! Ruby's context-sensitive constructs.  See
//! `code/specs/ruby-lexer-state-machine.md` §3 for the design.
//!
//! In Phase 2, the only query that actually wires through is
//! [`ParserOracle::is_local`] — which the lexer uses to decide
//! whether `/` after a name is a regex literal (method call with
//! regex arg) or binary division (local-variable division).
//!
//! The other methods (`in_def`, `in_block`, `in_lambda`,
//! `in_class_body`) are declared so future phases can plug in
//! without breaking the public API.  Default impls all return
//! `false` — the [`NoLocals`] oracle treats every name as a method
//! (the conservative choice for the paren-required v0 subset).

/// The lexer's view of the parser.
///
/// Implementors maintain whatever scope state they need to answer
/// the queries.  The lexer holds a `Box<dyn ParserOracle>` so the
/// concrete type can be swapped at construction.
pub trait ParserOracle {
    /// `true` if `name` is currently in scope as a local variable.
    /// Used to disambiguate `f /x/` — when `f` is local, the `/` is
    /// division; otherwise it opens a regex argument.
    fn is_local(&self, name: &str) -> bool {
        let _ = name;
        false
    }

    /// `true` if the lexer is currently inside a `def` body.
    /// Reserved for future use (some keyword behaviour differs
    /// inside vs outside `def`).
    fn in_def(&self) -> bool {
        false
    }

    /// `true` if the lexer is currently inside a block body
    /// (`do...end` or `{...}`).
    fn in_block(&self) -> bool {
        false
    }

    /// `true` if the lexer is currently inside a `lambda { ... }`.
    /// Reserved — `return` semantics differ inside lambdas.
    fn in_lambda(&self) -> bool {
        false
    }

    /// `true` if the lexer is currently inside a class body (not a
    /// method body within a class).
    fn in_class_body(&self) -> bool {
        false
    }
}

/// The default oracle: nothing is ever a local.  Treats every name
/// as a method.  Suitable for the paren-required v0 subset where
/// implicit-receiver calls are rare.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoLocals;

impl ParserOracle for NoLocals {}

/// A trivial oracle backed by an in-memory set of local-variable
/// names.  Useful for tests and for the parser to construct on the
/// fly as it walks a function body.
#[derive(Debug, Clone, Default)]
pub struct StaticLocals {
    locals: std::collections::HashSet<String>,
}

impl StaticLocals {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_locals<I, S>(locals: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            locals: locals.into_iter().map(Into::into).collect(),
        }
    }

    pub fn insert(&mut self, name: impl Into<String>) {
        self.locals.insert(name.into());
    }
}

impl ParserOracle for StaticLocals {
    fn is_local(&self, name: &str) -> bool {
        self.locals.contains(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_locals_says_no_to_everything() {
        let oracle = NoLocals;
        assert!(!oracle.is_local("foo"));
        assert!(!oracle.is_local("x"));
        assert!(!oracle.in_def());
        assert!(!oracle.in_block());
        assert!(!oracle.in_lambda());
        assert!(!oracle.in_class_body());
    }

    #[test]
    fn static_locals_round_trips() {
        let oracle = StaticLocals::with_locals(["x", "y", "total"]);
        assert!(oracle.is_local("x"));
        assert!(oracle.is_local("y"));
        assert!(oracle.is_local("total"));
        assert!(!oracle.is_local("foo"));
    }

    #[test]
    fn static_locals_insert_after_construction() {
        let mut oracle = StaticLocals::new();
        oracle.insert("counter");
        assert!(oracle.is_local("counter"));
        assert!(!oracle.is_local("other"));
    }
}
