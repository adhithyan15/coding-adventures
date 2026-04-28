//! Immutable name-to-IR mapping produced by [`super::match_pattern`].
//!
//! A successful match returns a [`Bindings`] containing every named
//! pattern's captured value.  Extending a `Bindings` always produces a
//! new value — the original is never mutated.  This makes the matcher
//! easy to reason about: each call's output depends only on its inputs.
//!
//! # Example
//!
//! ```rust
//! use symbolic_ir::int;
//! use cas_pattern_matching::Bindings;
//!
//! let b0 = Bindings::empty();
//! let b1 = b0.bind("x", int(1));
//! let b2 = b1.bind("y", int(2));
//!
//! assert_eq!(b2.get("x"), Some(&int(1)));
//! assert_eq!(b2.get("y"), Some(&int(2)));
//! assert_eq!(b2.len(), 2);
//! ```

use std::collections::HashMap;

use symbolic_ir::IRNode;

/// An immutable mapping from pattern-capture names to IR nodes.
///
/// The inner `HashMap` is shared via `Clone` (which copies the whole map
/// on each `bind`).  For pattern matching on typical CAS expressions —
/// which capture O(1–10) variables — this is fast enough and keeps the
/// ownership model simple.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bindings {
    data: HashMap<String, IRNode>,
}

impl Bindings {
    /// Create an empty `Bindings`.
    pub fn empty() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    /// Return a new `Bindings` with `name → value` added.
    ///
    /// If `name` is already bound to `value`, returns `self` cloned (the
    /// binding is idempotent).  The caller is responsible for checking
    /// consistency before calling `bind`.
    pub fn bind(&self, name: &str, value: IRNode) -> Self {
        // Idempotent: no change if already bound to same value.
        if let Some(existing) = self.data.get(name) {
            if existing == &value {
                return self.clone();
            }
        }
        let mut new_data = self.data.clone();
        new_data.insert(name.to_string(), value);
        Bindings { data: new_data }
    }

    /// Look up a captured name.  Returns `None` if not bound.
    pub fn get(&self, name: &str) -> Option<&IRNode> {
        self.data.get(name)
    }

    /// True if `name` is currently bound.
    pub fn contains(&self, name: &str) -> bool {
        self.data.contains_key(name)
    }

    /// Number of captured bindings.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// True if no names are captured.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Iterate over `(name, value)` pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &IRNode)> {
        self.data.iter().map(|(k, v)| (k.as_str(), v))
    }
}

impl std::fmt::Display for Bindings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Bindings{{")?;
        let mut first = true;
        for (k, v) in &self.data {
            if !first {
                write!(f, ", ")?;
            }
            write!(f, "{k}={v}")?;
            first = false;
        }
        write!(f, "}}")
    }
}
