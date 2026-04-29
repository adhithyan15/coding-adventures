//! `BackendRegistry` — look up backends by name.
//!
//! A `BackendRegistry` is a simple name-to-backend mapping.  Its purpose is to
//! decouple the *selection* of a backend from the *construction* of a
//! `CodegenPipeline`:
//!
//! ```rust
//! use codegen_core::registry::BackendRegistry;
//! use jit_core::backend::NullBackend;
//!
//! let mut registry = BackendRegistry::new();
//! registry.register("null", Box::new(NullBackend));
//!
//! assert!(registry.get("null").is_some());
//! assert!(registry.get("missing").is_none());
//! ```
//!
//! ## Why a registry?
//!
//! Without a registry, every call site that selects a backend must import and
//! instantiate the concrete backend type directly, coupling the call site to a
//! specific backend package.  The registry moves backend selection to
//! configuration time and makes it easy to enumerate available backends in
//! diagnostics or help output.
//!
//! ## Type erasure
//!
//! Because Rust does not support generic trait objects with multiple type
//! parameters directly, the registry stores backends as
//! `Box<dyn Any + Send + Sync>`.  Callers must downcast after retrieval.
//!
//! This is the same pattern used by `CodeGeneratorRegistry` (see
//! [`crate::codegen`]).

use std::any::Any;
use std::collections::HashMap;

// ── BackendRegistry ───────────────────────────────────────────────────────────

/// Name-to-backend mapping.
///
/// Backends are stored type-erased (`Box<dyn Any + Send + Sync>`) because
/// backends can be generic over different IR types.  Callers must downcast
/// after retrieval.
///
/// Registering a second backend with the same name silently replaces the first.
///
/// ## Example
///
/// ```rust
/// use codegen_core::registry::BackendRegistry;
/// use jit_core::backend::NullBackend;
///
/// let mut reg = BackendRegistry::new();
/// reg.register("null", Box::new(NullBackend));
/// assert_eq!(reg.len(), 1);
/// assert_eq!(reg.names(), vec!["null"]);
/// ```
pub struct BackendRegistry {
    backends: HashMap<String, Box<dyn Any + Send + Sync>>,
}

impl BackendRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            backends: HashMap::new(),
        }
    }

    /// Register a backend under `name`.
    ///
    /// If a backend with the same name already exists it is silently replaced.
    ///
    /// # Parameters
    ///
    /// - `name` — the lookup key (typically matches `Backend::name()`).
    /// - `backend` — type-erased backend; must implement `Any + Send + Sync`.
    pub fn register(&mut self, name: impl Into<String>, backend: Box<dyn Any + Send + Sync>) {
        self.backends.insert(name.into(), backend);
    }

    /// Return the backend registered under `name`, as an opaque `Any` reference.
    ///
    /// Returns `None` if no backend with that name has been registered.
    ///
    /// The caller must downcast to the concrete backend type:
    ///
    /// ```rust
    /// use codegen_core::registry::BackendRegistry;
    /// use jit_core::backend::NullBackend;
    ///
    /// let mut reg = BackendRegistry::new();
    /// reg.register("null", Box::new(NullBackend));
    ///
    /// let backend = reg.get("null").unwrap();
    /// let _concrete = backend.downcast_ref::<NullBackend>().unwrap();
    /// ```
    pub fn get(&self, name: &str) -> Option<&(dyn Any + Send + Sync)> {
        self.backends.get(name).map(|b| b.as_ref())
    }

    /// Return the backend registered under `name`, or an error message.
    ///
    /// Mirrors the Python `get_or_raise` method, adapted to return a `Result`
    /// rather than raising an exception.
    ///
    /// # Errors
    ///
    /// Returns `Err(String)` listing the available backend names when `name`
    /// is not found.
    ///
    /// ```rust
    /// use codegen_core::registry::BackendRegistry;
    /// use jit_core::backend::NullBackend;
    ///
    /// let mut reg = BackendRegistry::new();
    /// reg.register("null", Box::new(NullBackend));
    ///
    /// assert!(reg.get_or_raise("null").is_ok());
    /// assert!(reg.get_or_raise("missing").is_err());
    /// ```
    pub fn get_or_raise(&self, name: &str) -> Result<&(dyn Any + Send + Sync), String> {
        self.backends.get(name).map(|b| b.as_ref()).ok_or_else(|| {
            let available = if self.backends.is_empty() {
                "<none>".to_string()
            } else {
                self.names().join(", ")
            };
            format!(
                "No backend named {:?} in registry. Available: {}",
                name, available
            )
        })
    }

    /// Return all registered backend names, sorted alphabetically.
    ///
    /// ```rust
    /// use codegen_core::registry::BackendRegistry;
    /// use jit_core::backend::{NullBackend, EchoBackend};
    ///
    /// let mut reg = BackendRegistry::new();
    /// reg.register("zzz", Box::new(NullBackend));
    /// reg.register("aaa", Box::new(EchoBackend));
    /// assert_eq!(reg.names(), vec!["aaa", "zzz"]);
    /// ```
    pub fn names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.backends.keys().cloned().collect();
        names.sort();
        names
    }

    /// Return the number of registered backends.
    pub fn len(&self) -> usize {
        self.backends.len()
    }

    /// Return `true` if no backends are registered.
    pub fn is_empty(&self) -> bool {
        self.backends.is_empty()
    }
}

impl Default for BackendRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use jit_core::backend::{EchoBackend, NullBackend};

    // Test 1: registry starts empty
    #[test]
    fn registry_starts_empty() {
        let reg = BackendRegistry::new();
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);
        assert!(reg.names().is_empty());
    }

    // Test 2: default() == new()
    #[test]
    fn registry_default() {
        let reg: BackendRegistry = Default::default();
        assert!(reg.is_empty());
    }

    // Test 3: register + get by name
    #[test]
    fn registry_register_and_get() {
        let mut reg = BackendRegistry::new();
        reg.register("null", Box::new(NullBackend));
        assert!(!reg.is_empty());
        assert_eq!(reg.len(), 1);
        assert!(reg.get("null").is_some());
        assert!(reg.get("missing").is_none());
    }

    // Test 4: get() returns None for unknown names
    #[test]
    fn registry_get_unknown_returns_none() {
        let reg = BackendRegistry::new();
        assert!(reg.get("no-such-backend").is_none());
    }

    // Test 5: names() returns sorted names
    #[test]
    fn registry_names_sorted() {
        let mut reg = BackendRegistry::new();
        reg.register("zzz", Box::new(NullBackend));
        reg.register("aaa", Box::new(EchoBackend));
        reg.register("mmm", Box::new(NullBackend));
        assert_eq!(reg.names(), vec!["aaa", "mmm", "zzz"]);
    }

    // Test 6: registering the same name replaces the previous entry
    #[test]
    fn registry_register_replaces() {
        let mut reg = BackendRegistry::new();
        reg.register("b", Box::new(NullBackend));
        reg.register("b", Box::new(EchoBackend));
        assert_eq!(reg.len(), 1); // still one entry

        // Downcast confirms it's now the EchoBackend
        let stored = reg.get("b").unwrap();
        assert!(stored.downcast_ref::<EchoBackend>().is_some());
        assert!(stored.downcast_ref::<NullBackend>().is_none());
    }

    // Test 7: downcast_ref works after get()
    #[test]
    fn registry_downcast_after_get() {
        let mut reg = BackendRegistry::new();
        reg.register("null", Box::new(NullBackend));
        let any = reg.get("null").unwrap();
        let concrete = any.downcast_ref::<NullBackend>();
        assert!(concrete.is_some());
    }

    // Test 8: get_or_raise() returns Ok for a registered backend
    #[test]
    fn registry_get_or_raise_ok() {
        let mut reg = BackendRegistry::new();
        reg.register("null", Box::new(NullBackend));
        let result = reg.get_or_raise("null");
        assert!(result.is_ok());
    }

    // Test 9: get_or_raise() returns Err for an unregistered name
    #[test]
    fn registry_get_or_raise_err() {
        let mut reg = BackendRegistry::new();
        reg.register("null", Box::new(NullBackend));
        let result = reg.get_or_raise("unknown");
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(msg.contains("unknown"));
        assert!(msg.contains("null")); // lists the available backends
    }

    // Test 10: get_or_raise() error message says "<none>" when registry is empty
    #[test]
    fn registry_get_or_raise_empty_registry() {
        let reg = BackendRegistry::new();
        let result = reg.get_or_raise("anything");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("<none>"));
    }

    // Test 11: multiple backends, len() tracks correctly
    #[test]
    fn registry_len_multiple() {
        let mut reg = BackendRegistry::new();
        reg.register("a", Box::new(NullBackend));
        reg.register("b", Box::new(EchoBackend));
        reg.register("c", Box::new(NullBackend));
        assert_eq!(reg.len(), 3);
    }
}
