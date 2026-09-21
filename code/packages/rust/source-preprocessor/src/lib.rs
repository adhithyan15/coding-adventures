//! # source-preprocessor — a generic preprocessor engine (PREP01)
//!
//! A preprocessor rewrites source *before* the parser sees it: pulling in
//! other files, selecting between alternatives, and (from slice 2) expanding
//! macros. Several languages in this repository need one and none have one.
//!
//! The obvious approach — write a preprocessor per language — duplicates the
//! genuinely hard parts four or five times, each with its own bugs. The hard
//! parts are not the directive syntax, which is easy and different everywhere.
//! They are:
//!
//! - keeping a token's true origin across inclusion and expansion,
//! - the non-recursive expansion algorithm,
//! - and making the whole thing safe to point at hostile input.
//!
//! So this crate is an **engine** plus per-language **dialect** plug-ins.
//!
//! ```text
//!     source text
//!         │
//!         ├─ pre_tokenize:  line splicing only
//!         ▼
//!     the language's own lexer  (its existing .tokens grammar)
//!         │
//!         ├─ post_tokenize:  THIS ENGINE, in one pass —
//!         │                  include ⟷ conditionals ⟷ macros
//!         ▼
//!     [Token] + SourceMap  →  the language's existing parser, unchanged
//! ```
//!
//! # What is shared and what is not
//!
//! | The engine owns | A [`Dialect`] owns |
//! |---|---|
//! | source map | directive syntax and names |
//! | include resolution, cycles, bounds | tokenization |
//! | the conditional stack | what a condition means |
//! | expansion algorithm (slice 2) | stringize / paste, replacement matching |
//!
//! Deliberately *not* unified: COBOL's `COPY … REPLACING` is pseudo-text
//! matching, a different algorithm from macro expansion. It will reuse the
//! include, source-map, dispatch and bounds layers and bring its own matcher.
//! A facility that tried to be universal at the *semantics* layer would serve
//! neither language well.
//!
//! # Three decisions worth knowing before reading the code
//!
//! **One pass, not include-then-conditionals.** An `@if` decides whether an
//! `@include` happens, and the included file defines names that later `@if`s
//! test. They interleave. See [`engine`].
//!
//! **A side table, not a wider `Token`.** `lexer::token::Token` has no file
//! identity, and 136 crates depend on it. Widening it would be a repo-wide
//! change to serve two frontends, so provenance lives beside the stream
//! instead. See [`source_map`].
//!
//! **Bound work and bytes, not only shape.** Counting nesting depth and
//! emitted tokens misses expansion consumed by a conditional, byte growth that
//! does not grow the token count, and acyclic fan-out. See [`bounds`].
//!
//! # Status
//!
//! Slice 1: includes, conditionals, source mapping, bounds. Macro expansion
//! (`@define` and friends) arrives in slice 2 and is refused with a diagnostic
//! until then, rather than silently ignored.
//!
//! # Example
//!
//! ```
//! use coding_adventures_source_preprocessor::{
//!     bounds::Bounds, fs::MemoryFs, preprocess,
//! };
//! # use coding_adventures_source_preprocessor::{dialect::{Dialect, Directive}, diag::PpError, source_map::FileId};
//! # use lexer::token::{Token, TokenType};
//! # struct D;
//! # fn tok(v: &str, line: usize) -> Token {
//! #     Token { type_: TokenType::Name, value: v.into(), line, column: 1,
//! #             type_name: None, flags: None, cv: None }
//! # }
//! # impl Dialect for D {
//! #     fn classify(&self, line: &[Token]) -> Option<Result<Directive, PpError>> {
//! #         match line.first()?.value.as_str() {
//! #             "@if" => Some(Ok(Directive::If(line[1..].to_vec()))),
//! #             "@else" => Some(Ok(Directive::Else)),
//! #             "@end" => Some(Ok(Directive::EndIf)),
//! #             _ => None,
//! #         }
//! #     }
//! #     fn eval_condition(&self, t: &[Token]) -> Result<bool, PpError> {
//! #         Ok(t.first().map(|t| t.value != "0").unwrap_or(false))
//! #     }
//! #     fn lex(&self, _t: &str, _f: FileId) -> Result<Vec<Token>, PpError> { Ok(vec![]) }
//! # }
//! # let mut fs = MemoryFs::new();
//! # let file = fs.insert("main", "");
//! // @if 1 / kept / @else / dropped / @end
//! let tokens = vec![
//!     tok("@if", 1), tok("1", 1),
//!     tok("kept", 2),
//!     tok("@else", 3),
//!     tok("dropped", 4),
//!     tok("@end", 5),
//! ];
//!
//! let result = preprocess(tokens, file, &D, &mut fs, Bounds::default()).unwrap();
//!
//! let kept: Vec<_> = result.tokens.iter().map(|t| t.value.as_str()).collect();
//! assert_eq!(kept, ["kept"]);
//! // Every surviving token still knows where it came from.
//! assert_eq!(result.map.len(), result.tokens.len());
//! ```

pub mod bounds;
pub mod diag;
pub mod dialect;
pub mod engine;
pub mod fs;
pub mod source_map;

pub use bounds::Bounds;
pub use diag::PpError;
pub use dialect::{Dialect, Directive};
pub use engine::{preprocess, Preprocessed};
pub use fs::{IncludeRequest, MemoryFs, RootedFs, SourceFs, SourceText};
pub use source_map::{FileId, Locus, Position, SourceMap};
