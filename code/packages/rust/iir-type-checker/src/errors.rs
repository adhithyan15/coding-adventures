//! `TypeCheckError` and `TypeCheckWarning` — diagnostic types for IIR type checking.
//!
//! # Error vs warning
//!
//! | Kind | Category | Meaning |
//! |------|----------|---------|
//! | `InvalidType` | Error | `type_hint` is not `"any"`, not in `CONCRETE_TYPES`, and not a `ref<T>` |
//! | `TypeMismatch` | Error | Two source operands with different concrete types feed a binary op |
//! | `ConditionNotBool` | Error | `jmp_if_true` / `jmp_if_false` source has a non-`bool` concrete type |
//! | `UntypedOperation` | Warning | All operands are `"any"`; inference could not resolve the type |
//! | `ObservedTypeDivergence` | Warning | `type_hint` disagrees with `observed_type` from the profiler |
//!
//! Errors block specialised code-generation (the instruction must be treated
//! as generic / call-runtime).  Warnings are informational; the compiler can
//! still proceed.

/// The category of a type-check error.
///
/// Variants are ordered from most severe to least so that consumers can
/// compare `ErrorKind` values in match arms or ordering.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorKind {
    /// The `type_hint` string is not `"any"`, not in
    /// [`interpreter_ir::opcodes::CONCRETE_TYPES`], and not a `ref<T>` string.
    ///
    /// # Example trigger
    ///
    /// ```text
    /// IIRInstr { op: "add", type_hint: "integer", … }  // "integer" is invalid
    /// ```
    InvalidType,

    /// Two source operands carry different concrete types and the operation
    /// requires they agree (arithmetic, bitwise, most comparisons).
    ///
    /// # Example trigger
    ///
    /// ```text
    /// const a, 3          ; type_hint = "u8"
    /// const b, 3.0        ; type_hint = "f64"
    /// add   c, [a, b]     ; type_hint = "u8" — mismatches b's "f64"
    /// ```
    TypeMismatch,

    /// A `jmp_if_true` or `jmp_if_false` instruction has a source operand
    /// with a concrete type other than `"bool"`.
    ///
    /// # Example trigger
    ///
    /// ```text
    /// const x, 1          ; type_hint = "i64"
    /// jmp_if_true [x], L  ; x is "i64", not "bool"
    /// ```
    ConditionNotBool,
}

impl std::fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorKind::InvalidType => write!(f, "InvalidType"),
            ErrorKind::TypeMismatch => write!(f, "TypeMismatch"),
            ErrorKind::ConditionNotBool => write!(f, "ConditionNotBool"),
        }
    }
}

/// A fatal type-checking error attached to a specific instruction.
///
/// # Example
///
/// ```
/// use iir_type_checker::errors::{TypeCheckError, ErrorKind};
///
/// let e = TypeCheckError {
///     fn_name: "main".into(),
///     instr_idx: 2,
///     kind: ErrorKind::TypeMismatch,
///     message: "add: src types 'u8' and 'f64' disagree".into(),
/// };
/// assert_eq!(e.kind, ErrorKind::TypeMismatch);
/// println!("{}", e);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct TypeCheckError {
    /// Name of the function containing the faulty instruction.
    pub fn_name: String,

    /// Zero-based index of the instruction within the function's instruction list.
    pub instr_idx: usize,

    /// The category of the error.
    pub kind: ErrorKind,

    /// Human-readable explanation suitable for printing to a developer.
    pub message: String,
}

impl std::fmt::Display for TypeCheckError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{}] {}:{} — {}",
            self.kind, self.fn_name, self.instr_idx, self.message
        )
    }
}

/// A non-fatal type-check warning attached to a specific instruction.
///
/// Warnings do not block code generation.  The compiler may emit a
/// diagnostic and continue.
///
/// # Example
///
/// ```
/// use iir_type_checker::errors::TypeCheckWarning;
///
/// let w = TypeCheckWarning {
///     fn_name: "main".into(),
///     instr_idx: 0,
///     message: "all operands are 'any'; inference could not resolve type".into(),
/// };
/// println!("{}", w);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct TypeCheckWarning {
    /// Name of the function containing the instruction.
    pub fn_name: String,

    /// Zero-based index within the function's instruction list.
    pub instr_idx: usize,

    /// Human-readable description of the warning.
    pub message: String,
}

impl std::fmt::Display for TypeCheckWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[warn] {}:{} — {}", self.fn_name, self.instr_idx, self.message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_kind_display() {
        assert_eq!(ErrorKind::InvalidType.to_string(), "InvalidType");
        assert_eq!(ErrorKind::TypeMismatch.to_string(), "TypeMismatch");
        assert_eq!(ErrorKind::ConditionNotBool.to_string(), "ConditionNotBool");
    }

    #[test]
    fn error_kind_ordering() {
        // InvalidType is more severe than ConditionNotBool
        assert!(ErrorKind::InvalidType < ErrorKind::ConditionNotBool);
    }

    #[test]
    fn type_check_error_display() {
        let e = TypeCheckError {
            fn_name: "main".into(),
            instr_idx: 3,
            kind: ErrorKind::TypeMismatch,
            message: "add: 'u8' vs 'f64'".into(),
        };
        let s = e.to_string();
        assert!(s.contains("TypeMismatch"));
        assert!(s.contains("main"));
        assert!(s.contains("3"));
    }

    #[test]
    fn type_check_warning_display() {
        let w = TypeCheckWarning {
            fn_name: "loop_body".into(),
            instr_idx: 7,
            message: "unresolved".into(),
        };
        let s = w.to_string();
        assert!(s.contains("warn"));
        assert!(s.contains("loop_body"));
        assert!(s.contains("7"));
    }
}
