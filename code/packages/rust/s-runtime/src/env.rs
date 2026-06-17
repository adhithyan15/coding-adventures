//! Lexical environments — the scope chain.
//!
//! S has lexical (static) scoping: a function sees the variables that were in
//! scope where it was *defined*, not where it is *called*. An [`Env`] is a
//! reference-counted, mutable scope with an optional parent. Looking up a name
//! walks the parent chain; assignment binds in the current scope.
//!
//! We use `Rc<RefCell<Scope>>` so that several closures can share — and mutate
//! — the same captured environment, exactly as S closures do.

use crate::value::SValue;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// A shared, mutable scope handle.
pub type Env = Rc<RefCell<Scope>>;

/// One frame of the scope chain: its own bindings plus an optional parent.
pub struct Scope {
    vars: HashMap<String, SValue>,
    parent: Option<Env>,
}

impl Scope {
    /// Create the global (root) environment — no parent.
    pub fn global() -> Env {
        Rc::new(RefCell::new(Scope {
            vars: HashMap::new(),
            parent: None,
        }))
    }

    /// Create a child scope whose parent is `parent` (a function call frame).
    pub fn child(parent: &Env) -> Env {
        Rc::new(RefCell::new(Scope {
            vars: HashMap::new(),
            parent: Some(Rc::clone(parent)),
        }))
    }
}

/// Bind `name` to `value` in the current scope (ordinary `<-` assignment).
pub fn define(env: &Env, name: &str, value: SValue) {
    env.borrow_mut().vars.insert(name.to_string(), value);
}

/// Look up `name`, walking outward through parent scopes. Returns `None` if the
/// name is unbound anywhere on the chain.
pub fn lookup(env: &Env, name: &str) -> Option<SValue> {
    if let Some(v) = env.borrow().vars.get(name) {
        return Some(v.clone());
    }
    let parent = env.borrow().parent.clone();
    match parent {
        Some(p) => lookup(&p, name),
        None => None,
    }
}
