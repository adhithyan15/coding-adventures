//! The function registry — the table the symbolic VM's `Backend`
//! reads when dispatching on a head.
//!
//! The registry is just a `HashMap<String, Handler>` plus convenience
//! methods. Downstream consumers (visicalc-modern, r-runtime,
//! macsyma) build a backend and pass its handler map through here to
//! populate it with every Layer-1 core function.

use std::collections::HashMap;
use std::sync::Arc;

use symbolic_vm::Handler;

/// A collection of named handlers backed by a hash map.
///
/// Use [`register_statistics_handlers`] (and the future
/// `register_math_handlers`, `register_financial_handlers`, …) to
/// populate it, then `into_inner` to hand the map to a backend
/// constructor.
#[derive(Default)]
pub struct HandlerRegistry {
    handlers: HashMap<String, Handler>,
}

impl HandlerRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a single handler by name. Replaces any existing
    /// binding (later registrations win).
    pub fn register(&mut self, name: impl Into<String>, handler: Handler) {
        self.handlers.insert(name.into(), handler);
    }

    /// Register the same handler under multiple aliases (e.g.
    /// `"Mean"`, `"AVERAGE"`, `"AVG"` all route to the same function).
    pub fn register_aliases<I, S>(&mut self, names: I, handler: Handler)
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for name in names {
            self.handlers.insert(name.into(), Arc::clone(&handler));
        }
    }

    /// Number of registered handlers.
    pub fn len(&self) -> usize {
        self.handlers.len()
    }

    /// `true` if no handlers are registered.
    pub fn is_empty(&self) -> bool {
        self.handlers.is_empty()
    }

    /// Borrow a handler by name.
    pub fn get(&self, name: &str) -> Option<&Handler> {
        self.handlers.get(name)
    }

    /// All registered names, in arbitrary order.
    pub fn names(&self) -> impl Iterator<Item = &String> {
        self.handlers.keys()
    }

    /// Consume the registry and return the underlying map.
    pub fn into_inner(self) -> HashMap<String, Handler> {
        self.handlers
    }

    /// Merge another registry into this one. Later registrations win
    /// on name conflicts.
    pub fn merge(&mut self, other: HandlerRegistry) {
        for (name, handler) in other.into_inner() {
            self.handlers.insert(name, handler);
        }
    }
}

/// Populate `registry` with statistics-core Phase-1 handlers
/// (descriptive, counting, rank). Each function is exposed under its
/// canonical R name *and* its Excel name(s), so a spreadsheet calling
/// `AVERAGE` and an R script calling `mean` resolve to the same
/// handler.
pub fn register_statistics_handlers(registry: &mut HandlerRegistry) {
    use crate::statistics_handlers as h;

    registry.register_aliases(["Sum", "SUM"], h::sum_handler());
    registry.register_aliases(["Prod", "Product", "PRODUCT"], h::prod_handler());
    registry.register_aliases(["Mean", "AVERAGE", "AVG"], h::mean_handler());
    registry.register_aliases(["Median", "MEDIAN"], h::median_handler());
    registry.register_aliases(["Var", "VAR.S", "VAR"], h::var_handler());
    registry.register_aliases(["Sd", "STDEV.S", "STDEV"], h::sd_handler());
    registry.register_aliases(["VarP", "VAR.P", "VARP"], h::var_pop_handler());
    registry.register_aliases(["SdP", "STDEV.P", "STDEVP"], h::sd_pop_handler());
    registry.register_aliases(["Min", "MIN"], h::min_handler());
    registry.register_aliases(["Max", "MAX"], h::max_handler());
    registry.register_aliases(["Count", "COUNT"], h::count_handler());
    registry.register_aliases(["CountA", "COUNTA"], h::count_a_handler());
    registry.register_aliases(["Length"], h::length_handler());
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_starts_empty() {
        let r = HandlerRegistry::new();
        assert!(r.is_empty());
        assert_eq!(r.len(), 0);
    }

    #[test]
    fn register_aliases_shares_handler() {
        let mut r = HandlerRegistry::new();
        register_statistics_handlers(&mut r);
        // "Mean", "AVERAGE", "AVG" all resolve.
        assert!(r.get("Mean").is_some());
        assert!(r.get("AVERAGE").is_some());
        assert!(r.get("AVG").is_some());
        // And to the same Arc (pointer identity).
        let mean_ptr = Arc::as_ptr(r.get("Mean").unwrap());
        let avg_ptr = Arc::as_ptr(r.get("AVG").unwrap());
        assert_eq!(mean_ptr, avg_ptr);
    }

    #[test]
    fn statistics_registry_includes_all_phase_1() {
        let mut r = HandlerRegistry::new();
        register_statistics_handlers(&mut r);
        // Phase 1 functions, canonical R names.
        for name in [
            "Sum", "Prod", "Mean", "Median", "Var", "Sd", "VarP", "SdP", "Min", "Max",
            "Count", "CountA", "Length",
        ] {
            assert!(r.get(name).is_some(), "{name} missing");
        }
    }

    #[test]
    fn merge_overwrites_on_conflict() {
        let mut a = HandlerRegistry::new();
        register_statistics_handlers(&mut a);
        let mut b = HandlerRegistry::new();
        register_statistics_handlers(&mut b);
        a.merge(b);
        // Same set of names.
        for name in ["Mean", "AVERAGE"] {
            assert!(a.get(name).is_some());
        }
    }
}
