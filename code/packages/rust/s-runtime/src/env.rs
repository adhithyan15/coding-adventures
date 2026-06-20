//! Lexical environments — the scope chain.
//!
//! S has lexical (static) scoping: a function sees the variables that were in
//! scope where it was *defined*, not where it is *called*. An [`Env`] is a
//! reference-counted, mutable scope with an optional parent. Looking up a name
//! walks the parent chain; assignment binds in the current scope.
//!
//! We use `Rc<RefCell<Scope>>` so that several closures can share — and mutate
//! — the same captured environment, exactly as S closures do.
//!
//! ## Ownership model (R-22 — first-class environments)
//!
//! Before R-22 a scope was only ever reachable through interpreter / call-stack
//! ownership, so the parent link could safely be a *strong* `Rc`. R-22 reifies a
//! scope as a first-class value ([`SValue::Environment`]) that can be **stored in
//! another scope's bindings**. That opens a door to reference cycles:
//!
//! ```text
//!   e <- new.env()          # e's parent is the caller frame F
//!   assign("self", e, envir = e)   # e now holds a binding to itself
//! ```
//!
//! and, more subtly, a child's parent could (transitively) hold the child as a
//! value. If `parent` were a strong `Rc`, such a cycle of strong references
//! would be **uncollectable** — `Rc` never frees a cycle — leaking every binding
//! in it.
//!
//! **The fix: the `parent` link is a [`Weak`].** A `Weak` does not contribute to
//! the strong reference count, so it can never *close* a strong cycle. The strong
//! ownership of every live scope flows root→leaf:
//!
//! * the **global** environment is held by a strong `Rc` in `Interpreter::global`
//!   for the whole session;
//! * each **live call frame** is held by a strong `Rc` on the native call stack
//!   for the duration of the call;
//! * an environment **captured as a value** (returned from `new.env()` and bound
//!   to a variable) is kept alive by that strong binding.
//!
//! Parents are therefore only ever *referenced* (`Weak`), never *owned*, by their
//! children. No strong-`Rc` cycle is constructible from S/R source. A `Weak`
//! parent that fails to upgrade (its frame was already dropped) is treated as "no
//! parent" — every chain walk simply stops there, exactly as it would at the
//! root. In practice the global env keeps the whole ancestor chain of any
//! reachable scope alive, so a parent upgrade only fails for a frame that has
//! genuinely gone out of scope, which has no bindings anyone can still name.

use crate::value::SValue;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};

/// A shared, mutable scope handle.
pub type Env = Rc<RefCell<Scope>>;

/// One frame of the scope chain: its own bindings plus an optional **weak** link
/// to its parent. The parent is `Weak` (not `Rc`) so that an environment value
/// stored inside a scope can never form an uncollectable strong-reference cycle —
/// see the module-level ownership note.
pub struct Scope {
    vars: HashMap<String, SValue>,
    parent: Option<Weak<RefCell<Scope>>>,
}

impl Scope {
    /// Create the global (root) environment — no parent.
    pub fn global() -> Env {
        Rc::new(RefCell::new(Scope {
            vars: HashMap::new(),
            parent: None,
        }))
    }

    /// Create a child scope whose parent is `parent` (a function call frame, or a
    /// `new.env()` result). The parent is stored as a [`Weak`] so the child cannot
    /// keep an otherwise-dead parent alive and — crucially — cannot complete a
    /// strong-`Rc` cycle (see the module note).
    pub fn child(parent: &Env) -> Env {
        Rc::new(RefCell::new(Scope {
            vars: HashMap::new(),
            parent: Some(Rc::downgrade(parent)),
        }))
    }
}

/// Upgrade a scope's parent link to a strong handle, or `None` if the scope is a
/// root *or* its parent has already been dropped. Centralised so every walk
/// treats a non-upgradable `Weak` identically to "no parent" — the walk stops.
fn parent_of(env: &Env) -> Option<Env> {
    env.borrow().parent.as_ref().and_then(Weak::upgrade)
}

/// Bind `name` to `value` in the current scope (ordinary `<-` assignment).
pub fn define(env: &Env, name: &str, value: SValue) {
    env.borrow_mut().vars.insert(name.to_string(), value);
}

/// Look up `name`, walking outward through parent scopes. Returns `None` if the
/// name is unbound anywhere on the chain.
///
/// The walk is **iterative**, not recursive: a `RefCell` borrow is taken and
/// released for each frame before moving on, so we never hold two borrows of the
/// same scope at once (which would matter if a scope ever appeared twice on a
/// chain — it cannot, but the iterative shape makes re-entrancy a non-issue), and
/// a pathologically deep chain cannot overflow the native stack here.
pub fn lookup(env: &Env, name: &str) -> Option<SValue> {
    let mut cursor = Rc::clone(env);
    loop {
        if let Some(v) = cursor.borrow().vars.get(name) {
            return Some(v.clone());
        }
        match parent_of(&cursor) {
            Some(p) => cursor = p,
            None => return None,
        }
    }
}

/// Is `name` bound *anywhere* on the chain (current frame or any enclosing one)?
/// This is the engine behind R's `exists(x)`: a cheap presence test that does not
/// clone the value. Like [`lookup`] it walks outward (iteratively) to the root.
pub fn exists(env: &Env, name: &str) -> bool {
    let mut cursor = Rc::clone(env);
    loop {
        if cursor.borrow().vars.contains_key(name) {
            return true;
        }
        match parent_of(&cursor) {
            Some(p) => cursor = p,
            None => return false,
        }
    }
}

/// Remove `name` from the **current** frame's bindings, returning whether a
/// binding was actually present. This is R's `rm(x)`: it only ever deletes from
/// the frame it is called in (it does not reach into enclosing scopes), matching
/// R's `rm(..., envir = environment())` default.
pub fn remove(env: &Env, name: &str) -> bool {
    env.borrow_mut().vars.remove(name).is_some()
}

/// The names bound **directly** in `env`'s own frame (not the enclosing chain),
/// returned **sorted** as the engine behind R's `ls(envir = e)`. Only this frame
/// is inspected, matching `ls`'s default (it does not list inherited names).
pub fn names_in(env: &Env) -> Vec<String> {
    let mut names: Vec<String> = env.borrow().vars.keys().cloned().collect();
    names.sort();
    names
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
/// existing* env), and the parent link is a [`Weak`] that we *upgrade* on each
/// step, so the chain is a finite, acyclic list rooted at the global frame —
/// there is no way to construct a cycle from S/R source. The loop below therefore
/// always reaches a frame whose parent is absent (root) or no longer upgradable
/// (dropped) and stops. We walk iteratively (not recursively) so even a
/// pathologically deep chain cannot overflow the native stack here; the chain
/// depth is in turn bounded by `MAX_EVAL_DEPTH`, since each nested call frame
/// costs eval-recursion to create.
pub fn super_assign(env: &Env, name: &str, value: SValue) {
    // Start from the enclosing frame (skip the current one).
    let mut cursor = parent_of(env);
    let mut last: Option<Env> = None;
    while let Some(frame) = cursor {
        if frame.borrow().vars.contains_key(name) {
            // Found an existing enclosing binding — rebind it in place.
            frame.borrow_mut().vars.insert(name.to_string(), value);
            return;
        }
        let next = parent_of(&frame);
        last = Some(frame);
        cursor = next;
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
            // construction bug elsewhere. (`parent_of` returning `None` also
            // covers a dropped parent, but a reachable global is always kept alive
            // by the interpreter, so the terminus we reach is the global root.)
            debug_assert!(
                global.borrow().parent.is_none(),
                "super_assign reached a non-root frame; scope chain is not rooted at global"
            );
            global.borrow_mut().vars.insert(name.to_string(), value);
        }
        None => define(env, name, value),
    }
}
