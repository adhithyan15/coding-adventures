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

/// Is `name` bound *anywhere* on the chain (current frame or any enclosing one)?
/// This is the engine behind R's `exists(x)`: a cheap presence test that does not
/// clone the value. Like [`lookup`] it walks outward to the global frame.
pub fn exists(env: &Env, name: &str) -> bool {
    if env.borrow().vars.contains_key(name) {
        return true;
    }
    let parent = env.borrow().parent.clone();
    match parent {
        Some(p) => exists(&p, name),
        None => false,
    }
}

/// Remove `name` from the **current** frame's bindings, returning whether a
/// binding was actually present. This is R's `rm(x)`: it only ever deletes from
/// the frame it is called in (it does not reach into enclosing scopes), matching
/// R's `rm(..., envir = environment())` default.
pub fn remove(env: &Env, name: &str) -> bool {
    env.borrow_mut().vars.remove(name).is_some()
}

/// Super-assignment (`x <<- value`). R's rule: search the chain of **enclosing**
/// environments — i.e. start at the *parent*, skipping the current frame — for an
/// existing binding of `name`, and rebind the **nearest** one found. If no
/// enclosing frame binds the name, create it in the **global** (root) environment.
///
/// Why skip the current frame? `<<-` exists precisely to reach *past* the local
/// scope: inside a counter closure, `n <<- n + 1` must mutate the `n` captured in
/// the enclosing function frame, not shadow it with a fresh local `n`. (If `<<-`
/// looked at the current frame first it would behave like `<-` whenever a local
/// of the same name existed — the opposite of what it is for.)
///
/// ## Termination
///
/// The walk follows `parent` links only. Every [`Scope`] is created by
/// [`Scope::global`] (no parent) or [`Scope::child`] (parent is an *already
/// existing* env), so the chain is a finite, acyclic list rooted at the global
/// frame — there is no way to construct a cycle from S/R source. The loop below
/// therefore always reaches a parent-less frame and stops. We walk iteratively
/// (not recursively) so even a pathologically deep chain cannot overflow the
/// native stack here; the chain depth is in turn bounded by `MAX_EVAL_DEPTH`,
/// since each nested call frame costs eval-recursion to create.
pub fn super_assign(env: &Env, name: &str, value: SValue) {
    // Start from the enclosing frame (skip the current one).
    let mut cursor = env.borrow().parent.clone();
    let mut last: Option<Env> = None;
    while let Some(frame) = cursor {
        if frame.borrow().vars.contains_key(name) {
            // Found an existing enclosing binding — rebind it in place.
            frame.borrow_mut().vars.insert(name.to_string(), value);
            return;
        }
        last = Some(Rc::clone(&frame));
        cursor = frame.borrow().parent.clone();
    }
    // No enclosing frame had the name. R creates it in the global environment,
    // which is the root we just walked to (`last`). If there was no enclosing
    // frame at all (we were already at global), bind here.
    match last {
        Some(global) => {
            // `last` is the final frame the walk reached, so by the acyclic-chain
            // invariant it must be the parent-less root (the global env). Assert
            // that invariant in debug builds — if it ever fails, the scope chain
            // has been wired into a cycle or a non-root terminus, which would be a
            // construction bug elsewhere.
            debug_assert!(
                global.borrow().parent.is_none(),
                "super_assign reached a non-root frame; scope chain is not rooted at global"
            );
            global.borrow_mut().vars.insert(name.to_string(), value);
        }
        None => define(env, name, value),
    }
}
