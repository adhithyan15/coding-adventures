//! # Dialect — what a language plugs in.
//!
//! The engine owns everything that is the same in every preprocessor: source
//! mapping, include resolution, the conditional stack, and the resource
//! bounds. A [`Dialect`] owns everything that is not.
//!
//! The split is easy to state and easy to get wrong. The engine **never**:
//!
//! - lexes anything (each language already owns its `.tokens` grammar),
//! - decides what a directive means,
//! - knows what `@if 1 == 1` or `#if defined(X)` evaluates to.
//!
//! and a dialect **never**:
//!
//! - resolves a path or touches the filesystem,
//! - decides a resource bound,
//! - manages the include stack or conditional nesting.
//!
//! ## Why directive names are not in the engine
//!
//! It would be easy — and wrong — for the engine to look for `#if`/`#endif`
//! itself. Then every language would have to spell its directives the way C
//! does, and the "generic" engine would be a C preprocessor with extra steps.
//!
//! [`Dialect::classify`] is the only thing that decides whether a line is a
//! directive, which is why MacroOct can spell its terminator `@end` rather
//! than `@endif`. That divergence is deliberate: it breaks any engine that
//! quietly hardcoded C's vocabulary, so it fails loudly here instead of
//! silently constraining the next language.

use crate::diag::PpError;
use crate::fs::IncludeRequest;
use crate::source_map::FileId;
use lexer::token::Token;

/// What the engine should do with a directive line the dialect recognised.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Directive {
    /// Pull in another file.
    Include(IncludeRequest),
    /// Begin a conditional group; the tokens are its controlling expression.
    If(Vec<Token>),
    /// Alternative branch of the innermost conditional.
    Else,
    /// End the innermost conditional. Spelled `@end` in MacroOct, `#endif` in
    /// C — the engine does not care which.
    EndIf,
    /// Define a macro. Inert until macro expansion lands; carried here so the
    /// dialect interface does not have to change when it does.
    Define { name: String, body: Vec<Token> },
    /// A directive the dialect recognised but wants ignored (a no-op line).
    Ignore,
}

/// The per-language half of the preprocessor.
pub trait Dialect {
    /// Classify one logical line.
    ///
    /// `None` means "ordinary source" — the engine passes those tokens through
    /// untouched (or drops them, if they sit in a skipped conditional group).
    fn classify(&self, line: &[Token]) -> Option<Result<Directive, PpError>>;

    /// Evaluate a conditional's controlling expression.
    ///
    /// The engine has already checked this slice's grouping depth against
    /// [`crate::bounds::Bounds::condition_depth`] before calling, so an
    /// implementation does not need to defend itself against a million nested
    /// parentheses — and, more importantly, cannot accidentally reintroduce
    /// the unbounded-recursion abort by forgetting to.
    fn eval_condition(&self, tokens: &[Token]) -> Result<bool, PpError>;

    /// Lex an included file's text.
    ///
    /// The engine calls this rather than any lexer directly, so every language
    /// keeps its own grammar and the engine stays language-agnostic.
    fn lex(&self, text: &str, file: FileId) -> Result<Vec<Token>, PpError>;

    /// Stringize (C's `#`). `None` — the default — means the dialect has no
    /// such operator, which MacroOct and MacroNib both rely on.
    fn stringize(&self, _tokens: &[Token]) -> Option<Token> {
        None
    }

    /// Token paste (C's `##`). `None` by default, as above.
    fn paste(&self, _left: &Token, _right: &Token) -> Option<Token> {
        None
    }
}
