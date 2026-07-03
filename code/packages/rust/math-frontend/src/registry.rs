//! The [`FrontendRegistry`]: look up a frontend by name and parse through it.
//!
//! Consumers hold one registry and call `registry.parse("latex", src)`. Adding support
//! for a new notation is `registry.register(Box::new(MyFrontend))` — nothing else in the
//! consumer changes.

use crate::expr::MathExpr;
use crate::frontend::{FrontendError, MathFrontend};
use std::collections::BTreeMap;

/// A name-keyed set of installed frontends.
#[derive(Default)]
pub struct FrontendRegistry {
    // BTreeMap so `names()` is stable/sorted (deterministic error messages and listings).
    frontends: BTreeMap<String, Box<dyn MathFrontend>>,
}

impl FrontendRegistry {
    /// An empty registry.
    pub fn new() -> Self {
        FrontendRegistry::default()
    }

    /// A registry pre-loaded with the built-in frontends.
    ///
    /// There are no built-in frontends yet — LaTeX (the first) lands as its own crate and
    /// will be registered here once it implements [`MathFrontend`]. Until then this is
    /// empty, by design, rather than pretending to support a notation it cannot parse.
    pub fn with_builtins() -> Self {
        FrontendRegistry::new()
    }

    /// Install a frontend, replacing any existing one with the same name. Returns the
    /// displaced frontend, if any.
    pub fn register(&mut self, frontend: Box<dyn MathFrontend>) -> Option<Box<dyn MathFrontend>> {
        self.frontends.insert(frontend.name().to_string(), frontend)
    }

    /// Look up a frontend by name.
    pub fn get(&self, name: &str) -> Option<&dyn MathFrontend> {
        self.frontends.get(name).map(|b| b.as_ref())
    }

    /// The names of all installed frontends, sorted.
    pub fn names(&self) -> Vec<&str> {
        self.frontends.keys().map(|s| s.as_str()).collect()
    }

    /// Parse `src` through the named frontend. If the name is unknown, returns a
    /// `FrontendError` (frontend `"<registry>"`) naming the unknown frontend and listing
    /// the installed ones — never a panic.
    pub fn parse(&self, name: &str, src: &str) -> Result<MathExpr, FrontendError> {
        match self.get(name) {
            Some(f) => f.parse(src),
            None => Err(FrontendError::new(
                "<registry>",
                format!(
                    "unknown frontend {:?}; installed: [{}]",
                    name,
                    self.names().join(", ")
                ),
                (0, src.len()),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expr::{MathExpr, Number};
    use crate::frontend::Capabilities;

    /// A trivial frontend used only to exercise the registry machinery: it parses a bare
    /// non-negative integer into `MathExpr::Number`, else errors.
    struct IntFrontend;
    impl MathFrontend for IntFrontend {
        fn name(&self) -> &str {
            "int"
        }
        fn parse(&self, src: &str) -> Result<MathExpr, FrontendError> {
            Number::parse(src)
                .map(MathExpr::Number)
                .ok_or_else(|| FrontendError::new("int", "not an integer", (0, src.len())))
        }
        fn capabilities(&self) -> Capabilities {
            Capabilities::none()
        }
    }

    #[test]
    fn register_get_and_names() {
        let mut r = FrontendRegistry::new();
        assert!(r.names().is_empty());
        assert!(r.register(Box::new(IntFrontend)).is_none());
        assert_eq!(r.names(), vec!["int"]);
        assert!(r.get("int").is_some());
        assert!(r.get("nope").is_none());
    }

    #[test]
    fn parse_through_named_frontend() {
        let mut r = FrontendRegistry::new();
        r.register(Box::new(IntFrontend));
        assert_eq!(r.parse("int", "42").unwrap(), MathExpr::Number(Number::from_i64(42)));
        assert!(r.parse("int", "x").is_err());
    }

    #[test]
    fn unknown_frontend_lists_installed_and_does_not_panic() {
        let mut r = FrontendRegistry::new();
        r.register(Box::new(IntFrontend));
        let err = r.parse("latex", "1").unwrap_err();
        assert_eq!(err.frontend, "<registry>");
        assert!(err.message.contains("unknown frontend"));
        assert!(err.message.contains("int")); // lists what IS installed
    }

    #[test]
    fn register_replaces_and_returns_displaced() {
        let mut r = FrontendRegistry::new();
        r.register(Box::new(IntFrontend));
        let displaced = r.register(Box::new(IntFrontend));
        assert!(displaced.is_some());
        assert_eq!(r.names().len(), 1);
    }

    #[test]
    fn with_builtins_is_currently_empty() {
        // Documented: no built-in frontends yet (latex arrives as its own crate).
        assert!(FrontendRegistry::with_builtins().names().is_empty());
    }
}
