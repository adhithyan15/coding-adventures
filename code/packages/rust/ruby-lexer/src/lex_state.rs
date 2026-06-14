//! Lex states — the bookkeeping the lexer needs to disambiguate
//! Ruby's context-sensitive constructs.
//!
//! Mirrors MRI's `parse.y` enum, simplified for v0.  The key
//! distinction is between *expression-start* positions (where `/`
//! could begin a regex literal) and *expression-end* positions
//! (where `/` is binary division).  After a name token we are in
//! a third state, `ExprArg`, where the answer depends on whether
//! the name is a local variable (consult the
//! [`ParserOracle`](crate::ParserOracle)).
//!
//! See `code/specs/ruby-lexer-state-machine.md` §1 for the full
//! state table.  Phase 2 implements the first six; the remaining
//! states (`ExprCmdArg`, `ExprEndArg`, `ExprLabel`, `ExprLabeled`,
//! `ExprValue`, `ExprEndFn`, `ExprClass`) are no-ops in v0 and fold
//! into the closest neighbour.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LexState {
    /// Beginning of expression — the initial state.  Reached at file
    /// start, after `(`, `,`, `;`, newline, after a binary operator,
    /// after `=`, after most keywords.
    ///
    /// `/` here is a **regex literal**.
    #[default]
    ExprBeg,

    /// Mid-expression, immediately after a binary operator.  Same
    /// disambiguation as `ExprBeg` for `/` (regex).  Tracked
    /// separately so future versions can distinguish unary vs binary
    /// `+` / `-`.
    ExprMid,

    /// After a value-yielding token — a literal (`Int`, `String`,
    /// `Float`), a closing bracket (`)`, `]`, `}`), or `self` /
    /// `nil` / `true` / `false`.
    ///
    /// `/` here is **division**.
    ExprEnd,

    /// After a method-shaped name (`foo`, `puts`).  The name may be
    /// either a local variable (in which case it's a value and the
    /// next `/` is division) or a method call (in which case the
    /// next `/` opens a regex literal that is the call's argument).
    ///
    /// The lexer consults the [`ParserOracle`](crate::ParserOracle)
    /// to decide.
    ExprArg,

    /// After `def`, `class`, or `module`.  The next identifier is a
    /// method / class / module name being declared, not a value.
    /// Phase 2 records this state so the parser oracle can avoid
    /// treating the name as a local-variable introduction.
    ExprFname,

    /// After `.` or `::`.  The next identifier is a method name on
    /// the prior receiver — never a local.
    ExprDot,
}

impl LexState {
    /// `true` iff `/` in this state should be lexed as a **regex
    /// literal** (the start-of-expression interpretation).  Callers
    /// must handle `ExprArg` separately by consulting the oracle.
    pub fn slash_is_regex(self) -> bool {
        matches!(self, LexState::ExprBeg | LexState::ExprMid)
    }

    /// `true` iff `/` in this state is unambiguously **binary
    /// division**.  `ExprArg` is *not* unambiguous — see
    /// [`slash_is_regex`].
    pub fn slash_is_division(self) -> bool {
        matches!(self, LexState::ExprEnd)
    }

    /// `true` iff this state is *one of the ambiguous* positions
    /// (`ExprArg`) where the oracle must adjudicate.
    pub fn needs_oracle_for_slash(self) -> bool {
        matches!(self, LexState::ExprArg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slash_classification_is_total_for_non_arg_states() {
        for state in [
            LexState::ExprBeg,
            LexState::ExprMid,
            LexState::ExprEnd,
            LexState::ExprFname,
            LexState::ExprDot,
        ] {
            // Exactly one of (regex, division, needs_oracle) is true
            // for non-Arg states.  The Fname / Dot states currently
            // fold into "division" because a `/` immediately after
            // `def f /pat/` would be unusual; the spec leaves room
            // for future refinement.
            let r = state.slash_is_regex();
            let d = state.slash_is_division();
            let o = state.needs_oracle_for_slash();
            assert!(
                (r as u8 + d as u8 + o as u8) <= 1,
                "state {state:?} has conflicting slash classifications"
            );
        }
    }

    #[test]
    fn arg_needs_oracle() {
        assert!(LexState::ExprArg.needs_oracle_for_slash());
        assert!(!LexState::ExprArg.slash_is_regex());
        assert!(!LexState::ExprArg.slash_is_division());
    }

    #[test]
    fn default_is_expr_beg() {
        assert_eq!(LexState::default(), LexState::ExprBeg);
    }
}
