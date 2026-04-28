//! Pure list operations over symbolic IR.
//!
//! ## Design
//!
//! Every function borrows `&IRNode` for its list argument and returns a new
//! `IRNode` (owned).  Cloning is necessary because the tree is immutable and
//! each returned list is a freshly built `Apply(List, [...])`.
//!
//! ### Error handling
//!
//! All fallible operations return `ListResult<IRNode>`.  The error carries a
//! human-readable message and implements `std::error::Error` so callers can
//! use `?` freely.
//!
//! ### Head-identity check
//!
//! A node is recognised as a List when:
//! ```text
//! node == Apply { head: Symbol("List"), args: [...] }
//! ```
//! We compare by value, not pointer, so freshly constructed `sym(LIST)` heads
//! match existing ones correctly.
//!
//! ### Sort key
//!
//! `sort_` orders elements by their `Debug` representation (`format!("{:?}",
//! node)`).  This is the same key that `cas_simplify::canonical` uses for
//! arg-list sorting, so the two orderings are consistent without introducing
//! a dependency on `cas-simplify`.

use std::fmt;

use symbolic_ir::{apply, int, sym, IRNode, LIST};

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Error raised when a list operation is given a non-List node or an
/// out-of-range index.
///
/// Mirrors Python's `ListOperationError(ValueError)`.
#[derive(Debug, Clone, PartialEq)]
pub struct ListOperationError(pub String);

impl fmt::Display for ListOperationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ListOperationError: {}", self.0)
    }
}

impl std::error::Error for ListOperationError {}

/// Shorthand result type for list operations.
pub type ListResult<T> = Result<T, ListOperationError>;

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Return the argument slice of a `List(...)` node, or an error.
///
/// The check is: the node must be an `Apply` whose head is the symbol `"List"`.
///
/// ```rust
/// use cas_list_operations::{as_list, make_list};
/// use symbolic_ir::int;
///
/// let lst = make_list(vec![int(1), int(2)]);
/// assert_eq!(as_list(&lst).unwrap().len(), 2);
/// ```
pub fn as_list(node: &IRNode) -> ListResult<&[IRNode]> {
    if let IRNode::Apply(a) = node {
        if a.head == sym(LIST) {
            return Ok(&a.args);
        }
    }
    Err(ListOperationError(format!(
        "expected a List, got {node:?}"
    )))
}

/// Build a `List(...)` node from a `Vec<IRNode>`.
///
/// ```rust
/// use cas_list_operations::make_list;
/// use symbolic_ir::{int, sym, LIST};
///
/// let lst = make_list(vec![int(1), int(2)]);
/// if let symbolic_ir::IRNode::Apply(a) = &lst {
///     assert_eq!(a.head, sym(LIST));
///     assert_eq!(a.args.len(), 2);
/// } else { panic!("expected Apply"); }
/// ```
pub fn make_list(args: Vec<IRNode>) -> IRNode {
    apply(sym(LIST), args)
}

// ---------------------------------------------------------------------------
// Public operations
// ---------------------------------------------------------------------------

/// Number of elements in the list.
///
/// ```rust
/// use cas_list_operations::{make_list, length};
/// use symbolic_ir::int;
///
/// let lst = make_list(vec![int(1), int(2), int(3)]);
/// assert_eq!(length(&lst).unwrap(), int(3));
///
/// assert_eq!(length(&make_list(vec![])).unwrap(), int(0));
/// ```
pub fn length(lst: &IRNode) -> ListResult<IRNode> {
    let args = as_list(lst)?;
    Ok(int(args.len() as i64))
}

/// First element of the list.  Raises on empty list.
///
/// ```rust
/// use cas_list_operations::{make_list, first};
/// use symbolic_ir::int;
///
/// assert_eq!(first(&make_list(vec![int(7), int(8)])).unwrap(), int(7));
/// assert!(first(&make_list(vec![])).is_err());
/// ```
pub fn first(lst: &IRNode) -> ListResult<IRNode> {
    let args = as_list(lst)?;
    if args.is_empty() {
        return Err(ListOperationError("first() of empty list".into()));
    }
    Ok(args[0].clone())
}

/// Everything but the first element.  Raises on empty list.
///
/// ```rust
/// use cas_list_operations::{make_list, rest};
/// use symbolic_ir::int;
///
/// let lst = make_list(vec![int(1), int(2), int(3)]);
/// assert_eq!(rest(&lst).unwrap(), make_list(vec![int(2), int(3)]));
/// assert!(rest(&make_list(vec![])).is_err());
/// ```
pub fn rest(lst: &IRNode) -> ListResult<IRNode> {
    let args = as_list(lst)?;
    if args.is_empty() {
        return Err(ListOperationError("rest() of empty list".into()));
    }
    Ok(make_list(args[1..].to_vec()))
}

/// Last element of the list.  Raises on empty list.
///
/// ```rust
/// use cas_list_operations::{make_list, last};
/// use symbolic_ir::int;
///
/// let lst = make_list(vec![int(1), int(2), int(3)]);
/// assert_eq!(last(&lst).unwrap(), int(3));
/// assert!(last(&make_list(vec![])).is_err());
/// ```
pub fn last(lst: &IRNode) -> ListResult<IRNode> {
    let args = as_list(lst)?;
    if args.is_empty() {
        return Err(ListOperationError("last() of empty list".into()));
    }
    Ok(args[args.len() - 1].clone())
}

/// Reverse the list.
///
/// ```rust
/// use cas_list_operations::{make_list, reverse};
/// use symbolic_ir::int;
///
/// let lst = make_list(vec![int(1), int(2), int(3)]);
/// assert_eq!(reverse(&lst).unwrap(), make_list(vec![int(3), int(2), int(1)]));
/// ```
pub fn reverse(lst: &IRNode) -> ListResult<IRNode> {
    let args = as_list(lst)?;
    let mut rev = args.to_vec();
    rev.reverse();
    Ok(make_list(rev))
}

/// Concatenate a slice of List nodes.
///
/// Mirrors Python's `append(*lsts)`.  Pass all lists as a slice:
///
/// ```rust
/// use cas_list_operations::{make_list, append};
/// use symbolic_ir::int;
///
/// let a = make_list(vec![int(1)]);
/// let b = make_list(vec![int(2), int(3)]);
/// assert_eq!(
///     append(&[a, b]).unwrap(),
///     make_list(vec![int(1), int(2), int(3)])
/// );
/// ```
pub fn append(lsts: &[IRNode]) -> ListResult<IRNode> {
    let mut out: Vec<IRNode> = Vec::new();
    for lst in lsts {
        let args = as_list(lst)?;
        out.extend_from_slice(args);
    }
    Ok(make_list(out))
}

/// Alias for [`append`].  Mathematica spelling.
pub fn join(lsts: &[IRNode]) -> ListResult<IRNode> {
    append(lsts)
}

/// 1-based indexed access.  Negative indices count from the end.
///
/// `part(lst, 1)` is the first element; `part(lst, -1)` is the last.
/// Index `0` is always an error (1-based convention).
///
/// ```rust
/// use cas_list_operations::{make_list, part};
/// use symbolic_ir::int;
///
/// let lst = make_list(vec![int(10), int(20), int(30)]);
/// assert_eq!(part(&lst, 1).unwrap(), int(10));
/// assert_eq!(part(&lst, -1).unwrap(), int(30));
/// assert!(part(&lst, 0).is_err());
/// assert!(part(&lst, 5).is_err());
/// ```
pub fn part(lst: &IRNode, index: i64) -> ListResult<IRNode> {
    let args = as_list(lst)?;
    if index == 0 {
        return Err(ListOperationError(
            "Part: index 0 is invalid (1-based)".into(),
        ));
    }
    let len = args.len() as i64;
    // Convert 1-based or negative index to a 0-based Python-style index.
    // Positive: py_index = index - 1  (e.g., 1 → 0, 3 → 2)
    // Negative: py_index = len + index (e.g., -1 → len-1)
    let py_index = if index > 0 { index - 1 } else { len + index };
    if py_index < 0 || py_index >= len {
        return Err(ListOperationError(format!(
            "Part: index {index} out of range"
        )));
    }
    Ok(args[py_index as usize].clone())
}

/// Generate a list of consecutive integers.
///
/// ## Single-argument form
///
/// `range_(n, None, 1)` → `[1, 2, ..., n]` (MACSYMA convention, inclusive).
/// `range_(0, None, 1)` → empty list.
///
/// ## Two/three-argument form
///
/// `range_(start, Some(stop), step)` generates integers from `start` to
/// `stop` (inclusive), incrementing by `step`.  Negative `step` counts
/// downward.  `step == 0` is an error.
///
/// ```rust
/// use cas_list_operations::{make_list, range_};
/// use symbolic_ir::int;
///
/// // range_(5) → [1, 2, 3, 4, 5]
/// assert_eq!(
///     range_(5, None, 1).unwrap(),
///     make_list(vec![int(1), int(2), int(3), int(4), int(5)])
/// );
///
/// // range_(3, Some(7), 1) → [3, 4, 5, 6, 7]
/// let r = range_(3, Some(7), 1).unwrap();
/// assert_eq!(r, make_list(vec![int(3), int(4), int(5), int(6), int(7)]));
///
/// // range_(10, Some(1), -2) → [10, 8, 6, 4, 2]
/// let r2 = range_(10, Some(1), -2).unwrap();
/// assert_eq!(r2, make_list(vec![int(10), int(8), int(6), int(4), int(2)]));
/// ```
pub fn range_(start: i64, stop: Option<i64>, step: i64) -> ListResult<IRNode> {
    match stop {
        None => {
            // Single-arg form: [1..start] inclusive.
            let values: Vec<IRNode> = (1..=start).map(int).collect();
            Ok(make_list(values))
        }
        Some(stop) => {
            if step == 0 {
                return Err(ListOperationError("Range: step cannot be 0".into()));
            }
            let mut values: Vec<IRNode> = Vec::new();
            if step > 0 {
                let mut i = start;
                while i <= stop {
                    values.push(int(i));
                    i += step;
                }
            } else {
                let mut i = start;
                while i >= stop {
                    values.push(int(i));
                    i += step;
                }
            }
            Ok(make_list(values))
        }
    }
}

/// Apply `f` to each element, returning a list of unevaluated `f(a)` nodes.
///
/// `f` is the IR head — any `IRNode` (typically a `Symbol`).  The caller's
/// VM is responsible for evaluating the resulting applies.
///
/// ```rust
/// use cas_list_operations::{make_list, map_};
/// use symbolic_ir::{apply, int, sym};
///
/// let f = sym("f");
/// let lst = make_list(vec![int(1), int(2)]);
/// let out = map_(f.clone(), &lst).unwrap();
/// assert_eq!(
///     out,
///     make_list(vec![
///         apply(f.clone(), vec![int(1)]),
///         apply(f.clone(), vec![int(2)]),
///     ])
/// );
/// ```
pub fn map_(f: IRNode, lst: &IRNode) -> ListResult<IRNode> {
    let args = as_list(lst)?;
    let out: Vec<IRNode> = args
        .iter()
        .map(|a| apply(f.clone(), vec![a.clone()]))
        .collect();
    Ok(make_list(out))
}

/// Replace the list's head.
///
/// `apply_(Add, [a, b, c])` → `Add(a, b, c)`.
///
/// This is how Mathematica's `Apply[f, list]` works: the outer `List` head
/// is discarded and `f` becomes the new head over the same args.
///
/// ```rust
/// use cas_list_operations::{make_list, apply_};
/// use symbolic_ir::{apply, int, sym, ADD};
///
/// let lst = make_list(vec![int(1), int(2), int(3)]);
/// let out = apply_(sym(ADD), &lst).unwrap();
/// assert_eq!(out, apply(sym(ADD), vec![int(1), int(2), int(3)]));
/// ```
pub fn apply_(f: IRNode, lst: &IRNode) -> ListResult<IRNode> {
    let args = as_list(lst)?;
    Ok(apply(f, args.to_vec()))
}

/// Keep elements where `pred` returns `true`.
///
/// ```rust
/// use cas_list_operations::{make_list, select};
/// use symbolic_ir::{int, IRNode};
///
/// let lst = make_list(vec![int(1), int(2), int(3), int(4)]);
/// let evens = select(&lst, |n| matches!(n, IRNode::Integer(v) if v % 2 == 0)).unwrap();
/// assert_eq!(evens, make_list(vec![int(2), int(4)]));
/// ```
pub fn select<F: Fn(&IRNode) -> bool>(lst: &IRNode, pred: F) -> ListResult<IRNode> {
    let args = as_list(lst)?;
    let out: Vec<IRNode> = args.iter().filter(|a| pred(a)).cloned().collect();
    Ok(make_list(out))
}

/// Stable sort by `Debug` representation (matches canonical-form ordering).
///
/// Using `format!("{:?}", node)` as the key is consistent with how
/// `cas_simplify::canonical` orders arg lists, without introducing a
/// dependency on `cas-simplify`.
///
/// ```rust
/// use cas_list_operations::{make_list, sort_};
/// use symbolic_ir::int;
///
/// let lst = make_list(vec![int(3), int(1), int(2)]);
/// assert_eq!(sort_(&lst).unwrap(), make_list(vec![int(1), int(2), int(3)]));
/// ```
pub fn sort_(lst: &IRNode) -> ListResult<IRNode> {
    let args = as_list(lst)?;
    let mut sorted = args.to_vec();
    sorted.sort_by_key(|n| format!("{n:?}"));
    Ok(make_list(sorted))
}

/// Flatten `depth` levels of nested lists.
///
/// - `depth = 0`: no change.
/// - `depth = 1`: one level of unwrapping (default).
/// - `depth = -1`: completely flat (sentinel for unlimited depth).
///
/// ```rust
/// use cas_list_operations::{make_list, flatten};
/// use symbolic_ir::int;
///
/// // [1, [2, 3], 4]  depth=1  →  [1, 2, 3, 4]
/// let nested = make_list(vec![
///     int(1),
///     make_list(vec![int(2), int(3)]),
///     int(4),
/// ]);
/// assert_eq!(
///     flatten(&nested, 1).unwrap(),
///     make_list(vec![int(1), int(2), int(3), int(4)])
/// );
///
/// // flatten with -1 → fully flat
/// let deeply = make_list(vec![int(1), make_list(vec![int(2), make_list(vec![int(3)])])]);
/// assert_eq!(
///     flatten(&deeply, -1).unwrap(),
///     make_list(vec![int(1), int(2), int(3)])
/// );
/// ```
pub fn flatten(lst: &IRNode, depth: i64) -> ListResult<IRNode> {
    let args = as_list(lst)?;
    let effective_depth = if depth < 0 { i64::MAX } else { depth };
    let out = flatten_args(args, effective_depth);
    Ok(make_list(out))
}

/// Recursive helper for `flatten`.
///
/// Walks the argument list and, for each element that is itself a `List`,
/// recursively flattens it up to `depth` levels deep.
fn flatten_args(args: &[IRNode], depth: i64) -> Vec<IRNode> {
    if depth == 0 {
        return args.to_vec();
    }
    let mut out: Vec<IRNode> = Vec::with_capacity(args.len());
    for a in args {
        if let IRNode::Apply(inner) = a {
            if inner.head == sym(LIST) {
                // This element is itself a List — unwrap one level and recurse.
                let inner_flat = flatten_args(&inner.args, depth - 1);
                out.extend(inner_flat);
                continue;
            }
        }
        // Not a List, or depth reached 0 — keep as-is.
        out.push(a.clone());
    }
    out
}
