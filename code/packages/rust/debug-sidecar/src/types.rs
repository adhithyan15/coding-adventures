//! Core data types for debug-sidecar.
//!
//! Two types are exported:
//!
//! - [`SourceLocation`] — a frozen (file, line, col) triple mapping one
//!   IIR instruction to a position in the original source file.
//!
//! - [`Variable`] — a register binding with a human name, type hint, and
//!   live range expressed as `[live_start, live_end)` instruction indices.
//!
//! Both types are cheaply cloneable (`String` fields) and implement `Eq`
//! and `Hash` so they can be used as `HashMap` keys.

use std::fmt;

// ---------------------------------------------------------------------------
// SourceLocation
// ---------------------------------------------------------------------------

/// Source position for one IIR instruction.
///
/// Lines and columns are 1-based, matching the convention used by editors,
/// debuggers, and the DWARF standard.
///
/// # Example
///
/// ```
/// use debug_sidecar::SourceLocation;
///
/// let loc = SourceLocation { file: "fib.tetrad".into(), line: 3, col: 5 };
/// assert_eq!(loc.to_string(), "fib.tetrad:3:5");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceLocation {
    /// Source file path as registered with [`crate::DebugSidecarWriter::add_source_file`].
    pub file: String,
    /// 1-based line number.
    pub line: u32,
    /// 1-based column number.
    pub col: u32,
}

impl fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}", self.file, self.line, self.col)
    }
}

// ---------------------------------------------------------------------------
// Variable
// ---------------------------------------------------------------------------

/// A named register binding valid over a range of instruction indices.
///
/// The live range is expressed as `[live_start, live_end)` — a variable is
/// live at instruction N when `live_start <= N < live_end`.
///
/// # Example
///
/// ```
/// use debug_sidecar::Variable;
///
/// let v = Variable {
///     reg_index: 0,
///     name: "n".into(),
///     type_hint: "u8".into(),
///     live_start: 0,
///     live_end: 12,
/// };
/// assert!(v.is_live_at(0));
/// assert!(v.is_live_at(11));
/// assert!(!v.is_live_at(12));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Variable {
    /// IIR register index (matches the register number in the VM's frame).
    pub reg_index: u32,
    /// Human-readable variable name from the source program.
    pub name: String,
    /// Declared type string (`"any"`, `"u8"`, `"Int"`, …).
    /// Empty string if no type annotation was given.
    pub type_hint: String,
    /// First instruction index at which this binding is valid (inclusive).
    pub live_start: usize,
    /// One-past-last instruction index (exclusive upper bound).
    pub live_end: usize,
}

impl Variable {
    /// Return `true` if this variable is live at `instr_index`.
    pub fn is_live_at(&self, instr_index: usize) -> bool {
        self.live_start <= instr_index && instr_index < self.live_end
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_location_display() {
        let loc = SourceLocation {
            file: "fibonacci.tetrad".into(),
            line: 3,
            col: 5,
        };
        assert_eq!(loc.to_string(), "fibonacci.tetrad:3:5");
    }

    #[test]
    fn source_location_eq() {
        let a = SourceLocation { file: "f.t".into(), line: 1, col: 2 };
        let b = SourceLocation { file: "f.t".into(), line: 1, col: 2 };
        assert_eq!(a, b);
    }

    #[test]
    fn source_location_ne_different_file() {
        let a = SourceLocation { file: "a.t".into(), line: 1, col: 1 };
        let b = SourceLocation { file: "b.t".into(), line: 1, col: 1 };
        assert_ne!(a, b);
    }

    #[test]
    fn variable_is_live_at_start() {
        let v = Variable {
            reg_index: 0, name: "x".into(), type_hint: "u8".into(),
            live_start: 5, live_end: 10,
        };
        assert!(v.is_live_at(5));
    }

    #[test]
    fn variable_is_live_at_before_end() {
        let v = Variable {
            reg_index: 0, name: "x".into(), type_hint: "u8".into(),
            live_start: 5, live_end: 10,
        };
        assert!(v.is_live_at(9));
    }

    #[test]
    fn variable_not_live_at_end() {
        let v = Variable {
            reg_index: 0, name: "x".into(), type_hint: "u8".into(),
            live_start: 5, live_end: 10,
        };
        assert!(!v.is_live_at(10));
    }

    #[test]
    fn variable_not_live_before_start() {
        let v = Variable {
            reg_index: 0, name: "x".into(), type_hint: "u8".into(),
            live_start: 5, live_end: 10,
        };
        assert!(!v.is_live_at(4));
    }

    #[test]
    fn variable_empty_range() {
        let v = Variable {
            reg_index: 0, name: "x".into(), type_hint: "any".into(),
            live_start: 3, live_end: 3,
        };
        assert!(!v.is_live_at(3));
    }

    #[test]
    fn variable_no_type_hint() {
        let v = Variable {
            reg_index: 2, name: "y".into(), type_hint: String::new(),
            live_start: 0, live_end: 1,
        };
        assert!(v.type_hint.is_empty());
    }
}
