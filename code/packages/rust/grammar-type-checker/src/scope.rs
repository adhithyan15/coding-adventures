//! Lexical scope stack for the generic type checker.
//!
//! The scope stack is a chain of frames (innermost first).  Each frame maps
//! variable names to their inferred [`KindDecl`].  When a binder form is
//! entered (lambda / let), a new frame is pushed; when the body is fully
//! walked the frame is popped.
//!
//! Lookup walks the frame chain from innermost to outermost, matching Scheme's
//! lexical scoping semantics.  If no frame contains the name, the caller falls
//! back to `TypeDeclarations::globals` (populated from top-level defines
//! before the walk begins).

use std::collections::HashMap;

use type_declarations::KindDecl;

/// Lexical scope stack.
///
/// Created fresh for each `check()` call.  The initial global frame is seeded
/// from `TypeDeclarations::globals` before any expression is walked.
pub struct ScopeStack {
    /// Frames in push order (last = innermost / most recently pushed).
    frames: Vec<HashMap<String, KindDecl>>,
}

impl ScopeStack {
    /// Create a scope stack with one empty global frame.
    pub fn new() -> Self {
        Self {
            frames: vec![HashMap::new()],
        }
    }

    /// Push a new (empty) inner frame.
    pub fn push(&mut self) {
        self.frames.push(HashMap::new());
    }

    /// Pop the innermost frame.
    ///
    /// Never removes the outermost frame (the global frame stays for the
    /// lifetime of the check pass).
    pub fn pop(&mut self) {
        if self.frames.len() > 1 {
            self.frames.pop();
        }
    }

    /// Bind `name` → `kind` in the innermost frame.
    pub fn bind(&mut self, name: impl Into<String>, kind: KindDecl) {
        self.frames
            .last_mut()
            .expect("scope stack invariant: at least one frame")
            .insert(name.into(), kind);
    }

    /// Look up `name` in the scope chain (innermost first).
    ///
    /// Returns `None` if the name is not in any frame.  The caller should
    /// then check `TypeDeclarations::globals` before reporting an error.
    pub fn lookup(&self, name: &str) -> Option<&KindDecl> {
        // Iterate frames in reverse (innermost = last)
        for frame in self.frames.iter().rev() {
            if let Some(k) = frame.get(name) {
                return Some(k);
            }
        }
        None
    }

    /// Number of frames currently on the stack (≥ 1).
    #[cfg(test)]
    pub fn depth(&self) -> usize {
        self.frames.len()
    }
}

impl Default for ScopeStack {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_has_one_frame() {
        assert_eq!(ScopeStack::new().depth(), 1);
    }

    #[test]
    fn push_pop_symmetry() {
        let mut s = ScopeStack::new();
        s.push();
        assert_eq!(s.depth(), 2);
        s.pop();
        assert_eq!(s.depth(), 1);
    }

    #[test]
    fn pop_does_not_remove_global_frame() {
        let mut s = ScopeStack::new();
        s.pop(); // already at depth 1
        s.pop();
        assert_eq!(s.depth(), 1);
    }

    #[test]
    fn lookup_finds_in_innermost() {
        let mut s = ScopeStack::new();
        s.bind("x", KindDecl::Int);
        s.push();
        s.bind("x", KindDecl::Bool); // shadows outer
        assert_eq!(s.lookup("x"), Some(&KindDecl::Bool));
    }

    #[test]
    fn lookup_falls_through_to_outer() {
        let mut s = ScopeStack::new();
        s.bind("x", KindDecl::Int);
        s.push(); // inner frame, no binding for x
        assert_eq!(s.lookup("x"), Some(&KindDecl::Int));
    }

    #[test]
    fn lookup_returns_none_for_unbound() {
        let s = ScopeStack::new();
        assert_eq!(s.lookup("unbound"), None);
    }
}
