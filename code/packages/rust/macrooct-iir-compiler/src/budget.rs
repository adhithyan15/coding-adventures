//! # Pre-lex budgets: bounding the work spent *before* the engine counts it.
//!
//! The shared engine enforces [`Bounds`] on what it sees: tokens produced,
//! bytes read through a `SourceFs`, include depth. But it only ever sees
//! tokens a dialect has **already lexed**. `GrammarLexer` materialises a whole
//! file's tokens before returning, at roughly 260 bytes of memory per token,
//! so a dense 8 MB file of `;` used about 2 GB and 12 seconds before the
//! engine's `tokens_produced` check fired. A 17 MB primary file, over the
//! 16 MB `bytes_per_file` limit, compiled anyway, because the primary file is
//! lexed by this crate and never read through the engine's filesystem.
//!
//! MacroNib and the C dialect already closed this gap; MacroOct, the first
//! consumer, predated the rule. The fix is the same arithmetic they use:
//!
//! ```text
//!   every non-EOF token consumes at least one source byte
//!   ⇒ tokens(file) ≤ bytes(file)
//!   ⇒ capping bytes lexed at tokens_produced − 1 caps the peak token count
//!
//!   per file:   bytes ≤ min(bytes_per_file, total budget)
//!   aggregate:  Σ bytes lexed (primary + every include) ≤
//!               min(total_source_bytes, tokens_produced − 1)
//! ```
//!
//! Both checks run *before* the lexer is called, so a rejected input costs a
//! length comparison rather than gigabytes.
//!
//! ## Why a wrapper instead of state on `MacroOctDialect`
//!
//! [`MacroOctDialect`] is public, `Copy`, and documented as stateless: callers
//! use it as a plain value. The budget needs per-translation-unit state (bytes
//! already lexed), so it lives in [`BudgetedMacroOct`], a crate-private
//! [`Dialect`] that delegates every question to `MacroOctDialect` and only adds
//! the reservation in front of [`Dialect::lex`]. The pipeline in this crate
//! always uses the wrapper; an embedder driving `MacroOctDialect` directly owns
//! its own input limits, as the dialect's documentation says.

use std::cell::Cell;

use coding_adventures_source_preprocessor::macros::MacroTable;
use coding_adventures_source_preprocessor::{Bounds, Dialect, Directive, FileId, PpError};
use lexer::token::Token;

use crate::dialect::MacroOctDialect;

/// Source bytes a translation unit may hand to the lexer, per file and in
/// total.
#[derive(Debug)]
pub(crate) struct PrelexBudget {
    max_file_bytes: u64,
    max_total_bytes: u64,
    lexed_bytes: Cell<u64>,
}

fn byte_len(text: &str) -> u64 {
    u64::try_from(text.len()).unwrap_or(u64::MAX)
}

impl PrelexBudget {
    /// Derive both caps from the tightest of `bounds` and the defaults.
    pub(crate) fn new(bounds: Bounds) -> Self {
        let bounds = bounds.tighten(Bounds::default());
        let max_total_bytes = bounds
            .total_source_bytes
            .min(bounds.tokens_produced.saturating_sub(1));
        Self {
            max_file_bytes: bounds.bytes_per_file.min(max_total_bytes),
            max_total_bytes,
            lexed_bytes: Cell::new(0),
        }
    }

    /// Reject one file that could not be lexed within budget even alone.
    pub(crate) fn check_file(&self, text: &str) -> Result<(), PpError> {
        if byte_len(text) > self.max_file_bytes {
            return Err(PpError::new(
                "MacroOct source exceeds the pre-lex byte budget (including token limit)",
            ));
        }
        Ok(())
    }

    /// Charge `text` against the translation unit's aggregate budget, before
    /// it is lexed.
    pub(crate) fn reserve(&self, text: &str) -> Result<(), PpError> {
        self.check_file(text)?;
        let used = self.lexed_bytes.get().saturating_add(byte_len(text));
        if used > self.max_total_bytes {
            return Err(PpError::new(
                "MacroOct source exceeds the aggregate pre-lex token budget",
            ));
        }
        self.lexed_bytes.set(used);
        Ok(())
    }
}

/// [`MacroOctDialect`] plus one translation unit's [`PrelexBudget`].
#[derive(Debug)]
pub(crate) struct BudgetedMacroOct {
    pub(crate) budget: PrelexBudget,
}

impl BudgetedMacroOct {
    pub(crate) fn new(bounds: Bounds) -> Self {
        Self { budget: PrelexBudget::new(bounds) }
    }
}

impl Dialect for BudgetedMacroOct {
    fn classify(&self, line: &[Token]) -> Option<Result<Directive, PpError>> {
        MacroOctDialect.classify(line)
    }

    fn prepare_condition(
        &self,
        tokens: Vec<Token>,
        macros: &MacroTable,
    ) -> Result<Vec<Token>, PpError> {
        MacroOctDialect.prepare_condition(tokens, macros)
    }

    fn eval_condition(&self, tokens: &[Token]) -> Result<bool, PpError> {
        MacroOctDialect.eval_condition(tokens)
    }

    /// Reserve the included file's bytes, then lex it exactly as
    /// [`MacroOctDialect::lex`] does.
    fn lex(&self, text: &str, file: FileId) -> Result<Vec<Token>, PpError> {
        self.budget.reserve(text)?;
        MacroOctDialect.lex(text, file)
    }

    fn stringize(&self, tokens: &[Token]) -> Option<Token> {
        MacroOctDialect.stringize(tokens)
    }

    fn paste(&self, left: &Token, right: &Token) -> Option<Token> {
        MacroOctDialect.paste(left, right)
    }
}
