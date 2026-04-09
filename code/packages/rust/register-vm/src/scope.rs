//! # Lexical Scope Contexts
//!
//! JavaScript uses *lexical scoping* — a function can access variables from
//! the enclosing function's scope even after the enclosing function has
//! returned.  The classic example is a closure:
//!
//! ```javascript
//! function makeCounter() {
//!     let count = 0;           // lives in a Context slot
//!     return function() {
//!         count += 1;          // captured from parent Context
//!         return count;
//!     };
//! }
//! ```
//!
//! V8 Ignition implements this with *Context* objects arranged in a chain.
//! Each function invocation has a pointer to its own Context, which in turn
//! points to its parent Context, and so on up to the global Context.
//!
//! ## Implementation
//!
//! A [`Context`] is a `Vec<VMValue>` of *slots* plus an optional `parent`
//! pointer.  Opcodes [`LDA_CONTEXT_SLOT`](crate::opcodes::LDA_CONTEXT_SLOT)
//! and [`STA_CONTEXT_SLOT`](crate::opcodes::STA_CONTEXT_SLOT) navigate the
//! chain by walking up `depth` levels before reading or writing a slot.
//!
//! We use `Rc<RefCell<Context>>` so that closures can share the same
//! `Context` object without duplicating it — the closure and its creating
//! frame both hold a reference-counted pointer to the same allocation.

use std::rc::Rc;
use std::cell::RefCell;
use crate::types::VMValue;

/// `Context` — one level of the lexical scope chain.
///
/// Each function call may create its own Context (if it has variables that
/// need to be captured by inner functions).  The `parent` link points to
/// the enclosing function's Context.
///
/// ```text
/// global Context { slots: [print, …] }
///     ▲
///     │ parent
/// outer Context { slots: [count = 0] }
///     ▲
///     │ parent
/// inner Context { slots: [] }   ← current frame
/// ```
#[derive(Debug)]
pub struct Context {
    /// Storage for all variables declared in this scope.
    /// Each variable is assigned a compile-time slot index.
    pub slots: Vec<VMValue>,

    /// The enclosing scope's Context, or `None` for the global scope.
    pub parent: Option<Rc<RefCell<Context>>>,
}

/// Creates a new Context with the given number of variable slots.
///
/// All slots are initialised to [`VMValue::Undefined`], matching the
/// JavaScript behaviour where `let x;` leaves `x` as `undefined` until it
/// is first assigned.
///
/// # Arguments
/// * `parent` — the enclosing scope, or `None` for a top-level context.
/// * `slot_count` — number of variable slots to allocate.
///
/// # Returns
/// A reference-counted, interior-mutable pointer to the new Context.
pub fn new_context(
    parent: Option<Rc<RefCell<Context>>>,
    slot_count: usize,
) -> Rc<RefCell<Context>> {
    Rc::new(RefCell::new(Context {
        slots: vec![VMValue::Undefined; slot_count],
        parent,
    }))
}

/// Loads a value from the scope chain.
///
/// Walk up `depth` levels (0 = current, 1 = parent, …) then read slot `idx`.
///
/// Returns [`VMValue::Undefined`] if the depth or index is out of range, to
/// avoid panicking on malformed bytecode.
///
/// # Arguments
/// * `ctx` — starting context (the current frame's context).
/// * `depth` — number of parent hops to take before reading.
/// * `idx` — slot index within the resolved context.
pub fn get_slot(ctx: &Rc<RefCell<Context>>, depth: usize, idx: usize) -> VMValue {
    let resolved = walk_up(ctx, depth);
    match resolved {
        None => VMValue::Undefined,
        Some(c) => {
            let c = c.borrow();
            c.slots.get(idx).cloned().unwrap_or(VMValue::Undefined)
        }
    }
}

/// Stores a value into the scope chain.
///
/// Walk up `depth` levels then write `value` to slot `idx`.
/// Silently ignores out-of-range depth or index.
///
/// # Arguments
/// * `ctx` — starting context.
/// * `depth` — parent-hop count.
/// * `idx` — slot index.
/// * `value` — value to store.
pub fn set_slot(
    ctx: &Rc<RefCell<Context>>,
    depth: usize,
    idx: usize,
    value: VMValue,
) {
    let resolved = walk_up(ctx, depth);
    if let Some(c) = resolved {
        let mut c = c.borrow_mut();
        if idx < c.slots.len() {
            c.slots[idx] = value;
        }
    }
}

/// Helper: walk up the parent chain `depth` times and return the resulting
/// context, or `None` if the chain is shorter than `depth`.
fn walk_up(ctx: &Rc<RefCell<Context>>, depth: usize) -> Option<Rc<RefCell<Context>>> {
    if depth == 0 {
        return Some(Rc::clone(ctx));
    }
    let parent = ctx.borrow().parent.as_ref().map(Rc::clone);
    match parent {
        None => None,
        Some(p) => walk_up(&p, depth - 1),
    }
}
