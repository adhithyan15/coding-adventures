//! The local context: what the checker knows about the free variables it invents.
//!
//! # Why free variables
//!
//! To look inside a binder, `fun (x : A) => b`, the checker replaces the bound `#0` in
//! `b` by a brand-new free variable `x`, works on the body, and abstracts `x` back
//! afterwards (the "locally nameless" discipline). The local context records, for each
//! such variable, its type and, for `let`, its value:
//!
//! ```text
//!   x : A            (lambda / Pi binder)
//!   y : T := v       (let binder; whnf may replace y by v, which is rule zeta for fvars)
//! ```
//!
//! # Append-only, never reused
//!
//! Variable ids come from [`crate::ExprStore::fresh_fvar_id`], which never returns the
//! same id twice, and entries are never removed or changed. Consequently a fact about an
//! id ("its type is `A`") is true forever, which is what makes the checker's caches,
//! keyed by expression handle, sound across binders.

use crate::error::KernelError;
use crate::expr::{Expr, FVarId};
use crate::name::Name;
use std::collections::HashMap;

/// One entry of the local context.
#[derive(Clone, Debug)]
pub struct LocalDecl {
    /// The binder name (printing only).
    pub name: Name,
    /// The variable's type.
    pub ty: Expr,
    /// The value, for a `let`-bound variable.
    pub value: Option<Expr>,
}

/// A map from free variable to its declaration.
#[derive(Default, Debug)]
pub struct LocalContext {
    decls: HashMap<FVarId, LocalDecl>,
}

impl LocalContext {
    /// An empty context.
    pub fn new() -> LocalContext {
        LocalContext::default()
    }
    /// Look a variable up.
    pub fn get(&self, id: FVarId) -> Option<&LocalDecl> {
        self.decls.get(&id)
    }
    /// Number of variables.
    pub fn len(&self) -> usize {
        self.decls.len()
    }
    /// Is the context empty?
    pub fn is_empty(&self) -> bool {
        self.decls.is_empty()
    }
    /// Add a variable. An id that is already present is an error rather than an
    /// overwrite: replacing a type would invalidate every cache entry that used it.
    pub fn insert(&mut self, id: FVarId, decl: LocalDecl) -> Result<(), KernelError> {
        if self.decls.contains_key(&id) {
            return Err(KernelError::Internal("free variable id reused"));
        }
        self.decls.insert(id, decl);
        Ok(())
    }
}
