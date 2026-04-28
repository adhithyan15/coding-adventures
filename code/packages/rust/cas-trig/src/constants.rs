//! Head name constants for trigonometric functions and symbolic constants.
//!
//! All trig head names (`SIN`, `COS`, `TAN`, etc.) are re-exported from
//! `symbolic_ir` — collected here for convenience so callers only need
//! one `use cas_trig::constants::*`.
//!
//! The additional symbols are the canonical names for the mathematical
//! constants π and e as they appear in symbolic IR expressions (i.e. as
//! `Symbol("Pi")` and `Symbol("E")`).

/// Re-export all trig function heads from symbolic_ir.
pub use symbolic_ir::{ACOS, ASIN, ATAN, COS, SIN, SQRT, TAN};

/// The symbol name for the mathematical constant π.
///
/// Appears in the IR as `Symbol("Pi")`.
pub const PI_SYMBOL: &str = "Pi";

/// The symbol name for Euler's number *e*.
///
/// Appears in the IR as `Symbol("E")`.
pub const E_SYMBOL: &str = "E";
