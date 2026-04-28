//! # cas-list-operations
//!
//! Pure list operations over symbolic IR — the Rust port of the Python
//! `cas-list-operations` package.
//!
//! Every function accepts raw IR (an `IRNode` that should be a
//! `List(...)` application) and returns raw IR.  Errors raise
//! [`ListOperationError`] so callers can propagate or convert them to
//! user-facing messages.
//!
//! ## Quick start
//!
//! ```rust
//! use cas_list_operations::{make_list, length, first, rest, reverse, range_};
//! use symbolic_ir::int;
//!
//! // Build [1, 2, 3]
//! let lst = make_list(vec![int(1), int(2), int(3)]);
//!
//! assert_eq!(length(&lst).unwrap(), int(3));
//! assert_eq!(first(&lst).unwrap(), int(1));
//! assert_eq!(rest(&lst).unwrap(), make_list(vec![int(2), int(3)]));
//! assert_eq!(reverse(&lst).unwrap(), make_list(vec![int(3), int(2), int(1)]));
//!
//! // range_(5) → [1, 2, 3, 4, 5]  (MACSYMA convention)
//! let five = range_(5, None, 1).unwrap();
//! assert_eq!(five, make_list(vec![int(1), int(2), int(3), int(4), int(5)]));
//! ```
//!
//! ## Stack position
//!
//! ```text
//! symbolic-ir  ←  cas-list-operations
//! ```

pub mod operations;

pub use operations::{
    append, apply_, as_list, first, flatten, join, last, length, make_list,
    map_, part, range_, rest, reverse, select, sort_, ListOperationError, ListResult,
};

// ---------------------------------------------------------------------------
// IR head-name constants for list operations.
//
// These mirror the Python `heads.py` sentinels. The `LIST` head is already
// defined in `symbolic_ir`; the others are specific to list operations.
// ---------------------------------------------------------------------------

/// Head name for the `List(...)` container.
pub use symbolic_ir::LIST;

/// Head name for the `Length(lst)` operation.
pub const LENGTH: &str = "Length";

/// Head name for the `First(lst)` operation.
pub const FIRST: &str = "First";

/// Head name for the `Rest(lst)` operation.
pub const REST: &str = "Rest";

/// Head name for the `Last(lst)` operation.
pub const LAST: &str = "Last";

/// Head name for the `Append(lst1, lst2, ...)` operation.
pub const APPEND: &str = "Append";

/// Head name for the `Reverse(lst)` operation.
pub const REVERSE: &str = "Reverse";

/// Head name for the `Range(n)` / `Range(start, stop, step)` operation.
pub const RANGE: &str = "Range";

/// Head name for the `Map(f, lst)` operation.
pub const MAP: &str = "Map";

/// Head name for the `Apply(f, lst)` operation.
pub const APPLY_HEAD: &str = "Apply";

/// Head name for the `Select(pred, lst)` operation.
pub const SELECT: &str = "Select";

/// Head name for the `Sort(lst)` operation.
pub const SORT: &str = "Sort";

/// Head name for the `Part(lst, i)` operation.
pub const PART: &str = "Part";

/// Head name for the `Flatten(lst)` operation.
pub const FLATTEN: &str = "Flatten";

/// Head name for the `Join(lst1, lst2, ...)` operation (alias for Append).
pub const JOIN: &str = "Join";
