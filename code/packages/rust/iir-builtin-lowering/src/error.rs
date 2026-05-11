//! # error — `BuiltinLoweringError`
//!
//! Errors that can arise when the builtin-lowering pass encounters a
//! `call_builtin` instruction that cannot be cleanly rewritten.
//!
//! ## When do these occur?
//!
//! The pipeline is designed so that `lower_builtins` runs **after**
//! `iir-type-checker`.  If the ordering is violated, the type hints may still
//! be `"any"` for arithmetic, which the downstream backends cannot handle.
//! `BuiltinLoweringError` surfaces that mis-ordering as a hard error rather
//! than silently passing a broken module to the backend.
//!
//! Similarly, if a numeric builtin is called with the wrong number of
//! arguments (e.g. `(+ a b c)` with three args), the pass emits
//! `WrongArity` instead of silently truncating or padding the operand list.

// ---------------------------------------------------------------------------
// BuiltinLoweringError
// ---------------------------------------------------------------------------

/// Errors produced when `lower_builtins` cannot rewrite a `call_builtin`
/// instruction.
#[derive(Debug, Clone, PartialEq)]
pub enum BuiltinLoweringError {
    /// A numeric builtin was called with the wrong number of arguments.
    ///
    /// # Fields
    /// - `builtin_name` — the name of the builtin (e.g. `"+"`).
    /// - `function_name` — the containing function (for diagnostic messages).
    /// - `expected` — the arity required by the lowering table.
    /// - `found` — how many argument operands were actually present.
    ///
    /// # Example scenario
    /// `(+ a b c)` compiles to a `call_builtin "+"` with 3 arg operands.
    /// The table expects exactly 2, so this error is emitted.
    WrongArity {
        builtin_name: String,
        function_name: String,
        expected: usize,
        found: usize,
    },

    /// A numeric builtin's type_hint is still `"any"` at lowering time.
    ///
    /// This is a **pipeline ordering bug**: `lower_builtins` must run after
    /// `iir-type-checker`.  Emitting this error makes the mis-ordering
    /// immediately visible instead of silently creating a module whose
    /// arithmetic instructions have `"any"` type — which the backends reject
    /// with a confusing message about unsupported types.
    ///
    /// # Fields
    /// - `builtin_name` — the name of the builtin.
    /// - `function_name` — the containing function.
    UntypedBuiltin {
        builtin_name: String,
        function_name: String,
    },
}

impl std::fmt::Display for BuiltinLoweringError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuiltinLoweringError::WrongArity {
                builtin_name,
                function_name,
                expected,
                found,
            } => write!(
                f,
                "builtin {builtin_name:?} in function {function_name:?} \
                 expects {expected} operand(s) but found {found}"
            ),
            BuiltinLoweringError::UntypedBuiltin {
                builtin_name,
                function_name,
            } => write!(
                f,
                "builtin {builtin_name:?} in function {function_name:?} \
                 has type_hint=\"any\" — run iir-type-checker before iir-builtin-lowering"
            ),
        }
    }
}

impl std::error::Error for BuiltinLoweringError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrong_arity_display() {
        let e = BuiltinLoweringError::WrongArity {
            builtin_name: "+".into(),
            function_name: "add".into(),
            expected: 2,
            found: 3,
        };
        let s = format!("{e}");
        assert!(s.contains('"' as char));
        assert!(s.contains("2"));
        assert!(s.contains("3"));
    }

    #[test]
    fn untyped_builtin_display() {
        let e = BuiltinLoweringError::UntypedBuiltin {
            builtin_name: "+".into(),
            function_name: "foo".into(),
        };
        let s = format!("{e}");
        assert!(s.contains("iir-type-checker"));
        assert!(s.contains("any"));
    }
}
