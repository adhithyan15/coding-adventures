//! # `macrooct-iir-compiler` — the composition, and the point of PREP01 slice 1.
//!
//! MacroOct is **Oct, plus a preprocessor, and nothing else.** This crate is
//! where that sentence is cashed out, and the amount of code it takes is the
//! architectural claim:
//!
//! ```text
//!     MacroOct source
//!         │
//!         ▼  macrooct-lexer          ← macrooct.tokens = oct.tokens + @directives
//!     [Token]  with AT_INCLUDE / AT_IF / AT_ELSE / AT_END / AT_DEFINE
//!         │
//!         ▼  source_preprocessor::preprocess(…, &MacroOctDialect, …)
//!     [Token]  pure Oct — every directive token consumed
//!         │
//!         ▼  Oct's parser grammar          UNCHANGED
//!         ▼  oct_type_checker::check_ast   UNCHANGED
//!         ▼  oct_iir_compiler::compile_ast UNCHANGED
//!     IIRModule  →  all eight LANG VM backends, none of which learns anything
//! ```
//!
//! A preprocessor runs *before* the parser, so a dialect that gains one needs
//! no new parser, no new type checker and no new backend. If that composition
//! did not work, the engine would not be a clean layer — and PREP01 §7 exists
//! to learn that here, on a small language, rather than in C.
//!
//! ## The oracle: identical IIR, not merely "it also works"
//!
//! Because MacroOct is a dialect of Oct, a MacroOct program's hand-expanded
//! equivalent is a **valid Oct program**. So the acceptance criterion is not
//! "the MacroOct program runs and prints the right thing" — it is that the two
//! lower to the *same IIR*, which is a far stronger statement and costs zero
//! backend work to check. See `tests/iir_identity.rs`.
//!
//! ## Oct is not modified. Which costs this crate two copies.
//!
//! PREP01 holds Oct fixed: a reference you are free to edit is not a
//! reference. Note what that does NOT mean, because the distinction is the
//! whole design:
//!
//! - Oct's **language** is untouched — grammar, semantics, type rules,
//!   corpus, matrix rows, specs. MacroOct is checked against it.
//! - Oct's **crate** gained exactly one additive entry point,
//!   `create_oct_parser_from_tokens`, which hands back the parser Oct already
//!   builds — its own compiled `oct.grammar` and its own `MAX_RULE_DEPTH`
//!   guard — over tokens the caller supplies.
//!
//! An earlier draft of this crate avoided even that by embedding a second
//! compiled copy of `oct.grammar` and restating `MAX_RULE_DEPTH`. That was
//! worse, and worse in a way that a byte-identity test only half covered: the
//! copied *grammar* was guarded, but the copied *constant* was not, so a
//! retuned parser depth in Oct would have diverged silently. Reusing Oct's
//! builder removes both copies, and makes "MacroOct reuses Oct's parser
//! unchanged" literally true rather than aspirational.
//!
//! ## Status: slice 1
//!
//! `@include`, `@if`, `@else` and `@end` work. `@define` is *recognised* and
//! then refused by the engine with a located diagnostic, because slice 1 has
//! no macro table; slice 2 lands it.
//!
//! ## Example
//!
//! ```
//! use coding_adventures_macrooct_iir_compiler::compile_source;
//!
//! let module = compile_source(
//!     "@if 1\nfn main() { out(1, 42); }\n@else\nfn main() { out(1, 7); }\n@end\n",
//!     "demo",
//! ).unwrap();
//!
//! // One `main`, and it is the taken branch's.
//! assert_eq!(module.functions.len(), 1);
//! assert_eq!(module.entry_point.as_deref(), Some("main"));
//! ```

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

pub mod dialect;


pub use dialect::MacroOctDialect;

use coding_adventures_macrooct_lexer::try_tokenize_macrooct;
use coding_adventures_source_preprocessor::{
    preprocess, Bounds, FileId, MemoryFs, PpError, SourceMap,
};
use interpreter_ir::IIRModule;
use lexer::token::Token;
use oct_iir_compiler::OctError;
use oct_type_checker::check_ast;

/// Anything that can go wrong compiling MacroOct.
///
/// The split is the point of the crate: [`MacroOctError::Preprocess`] is the
/// only variant MacroOct owns. Everything downstream of preprocessing is Oct's
/// and arrives as [`MacroOctError::Oct`] unchanged, because after the
/// directives are consumed the stream is Oct's problem, parsed by Oct's
/// grammar and checked by Oct's type checker.
#[derive(Debug)]
pub enum MacroOctError {
    /// The source could not be tokenized.
    Lex(String),
    /// The preprocessor refused: a bad directive, an unresolvable include, an
    /// unterminated conditional, or a resource bound. Carries the engine's
    /// located diagnostic.
    Preprocess(PpError),
    /// Oct's parser, type checker or IIR lowering rejected the *preprocessed*
    /// program. Note what this means: the directives are already gone, so this
    /// is a complaint about Oct code — which may be code the author never
    /// typed, because it arrived through an `@include` or was selected by an
    /// `@if`.
    Oct(OctError),
}

impl std::fmt::Display for MacroOctError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MacroOctError::Lex(m) => write!(f, "MacroOct lex error: {m}"),
            MacroOctError::Preprocess(e) => write!(f, "MacroOct preprocessor error: {e}"),
            MacroOctError::Oct(e) => write!(f, "MacroOct (Oct stage) error: {e}"),
        }
    }
}

impl std::error::Error for MacroOctError {}

/// A preprocessed MacroOct translation unit: the token stream Oct's parser
/// will see, and where each of its tokens really came from.
///
/// Returned by [`preprocess_source`] for callers that want the intermediate —
/// tests that assert a branch was dropped, or a future LSP that needs to map a
/// diagnostic back through an `@include`. [`compile_source`] uses it and
/// discards it.
pub struct PreprocessedUnit {
    /// The Oct token stream, directive tokens consumed, with the lexer's
    /// end-of-stream sentinel restored at the end.
    pub tokens: Vec<Token>,
    /// One entry per token in [`Self::tokens`] **except** the restored
    /// sentinel, resolving to the file, line and column it came from.
    ///
    /// The off-by-one is deliberate and is documented on
    /// [`preprocess_source`]: the sentinel is not a token the engine ever saw,
    /// so inventing a provenance for it would be a lie in the one table whose
    /// whole job is to tell the truth about provenance.
    pub map: SourceMap,
}

/// Preprocess MacroOct source into an Oct token stream.
///
/// `fs` supplies `@include` targets and `file` names the primary unit.
///
/// ## The end-of-stream sentinel, and why it is handled here
///
/// `GrammarLexer` appends an `Eof` token to every stream. The engine knows
/// nothing about that — "does this language's lexer append a sentinel?" is
/// exactly the sort of per-language fact a generic engine must not carry — so
/// this function strips it before preprocessing and restores it after. Both
/// halves matter, and for different reasons:
///
/// - **Stripping.** The engine groups tokens into logical lines by
///   `Token::line`. A sentinel sharing a line with a closing `@end` (which a
///   file with no final newline produces every time) would join that
///   directive's line and make it look like `@end` with trailing junk.
///   `MacroOctDialect::lex` strips it from *included* files for a sharper
///   reason still: spliced into the middle of the stream, an `Eof` makes Oct's
///   parser stop early and silently compile a truncated program.
/// - **Restoring.** Oct's parser expects a stream that ends in one.
///
/// The restored sentinel carries the *original* source's final position,
/// which is the honest answer: it marks the end of the translation unit, and
/// the translation unit is the file the compiler was pointed at.
pub fn preprocess_source(
    source: &str,
    file: FileId,
    fs: &mut dyn coding_adventures_source_preprocessor::SourceFs,
    bounds: Bounds,
) -> Result<PreprocessedUnit, MacroOctError> {
    let mut tokens = try_tokenize_macrooct(source).map_err(MacroOctError::Lex)?;

    // Keep a copy of the sentinel rather than synthesising one, so its line and
    // column are the real end of the real file.
    let sentinel = tokens.last().cloned();
    dialect::strip_eof(&mut tokens);

    let result = preprocess(tokens, file, &MacroOctDialect, fs, bounds)
        .map_err(MacroOctError::Preprocess)?;

    let mut tokens = result.tokens;
    if let Some(sentinel) = sentinel {
        tokens.push(sentinel);
    }
    Ok(PreprocessedUnit { tokens, map: result.map })
}

/// Compile a MacroOct source string to IIR.
///
/// This is the entry point `lang-aot` dispatches to, and it mirrors
/// `oct_iir_compiler::compile_source`'s signature exactly so the
/// `Language::MacroOct` arm looks like the `Language::Oct` one.
///
/// ## `@include` and this entry point
///
/// A source *string* has no directory, so there is nowhere for a relative
/// include to resolve against: the only file this function knows about is the
/// string itself. An `@include` therefore fails with "no such included file",
/// which is honest — the file genuinely is not reachable — but it does mean
/// includes are not exercised by a string-entry caller. Use
/// [`compile_source_with_includes`] for an in-memory include set (tests, and
/// any embedder with its own virtual filesystem), or [`preprocess_source`]
/// directly with a `RootedFs` for real files under a declared search root.
///
/// ## The module's `language` field says `"oct"`, deliberately
///
/// `compile_ast` stamps `IIRModule::language` with `"oct"`, and MacroOct does
/// not override it. That is not an oversight: the identity oracle compares a
/// MacroOct module against an Oct one field for field, and a different string
/// there would be a difference with no semantic content that nevertheless
/// broke the comparison. It is also *true* — after preprocessing, the program
/// being lowered is Oct.
pub fn compile_source(source: &str, module_name: &str) -> Result<IIRModule, MacroOctError> {
    compile_source_with_includes(source, module_name, &[])
}

/// Compile MacroOct source with an in-memory set of includable files.
///
/// `includes` is `(name, text)` pairs; an `@include "name"` resolves against
/// them by exact spelling.
///
/// ## Why `MemoryFs` and not `RootedFs`
///
/// `RootedFs` is the *only* sanctioned implementation for real paths, and this
/// function does not use it because it does not touch real paths: nothing here
/// reaches the disk, so there is no containment property to enforce and no
/// TOCTOU window to close. A caller that wants to include real files should
/// build a `RootedFs` over its declared search roots and call
/// [`preprocess_source`] — and must not write its own `SourceFs`, which is the
/// engine's stated security contract.
///
/// ```
/// use coding_adventures_macrooct_iir_compiler::compile_source_with_includes;
///
/// let module = compile_source_with_includes(
///     "@include \"helper.macrooct\"\nfn main() { out(1, helper()); }\n",
///     "demo",
///     &[("helper.macrooct", "fn helper() -> u8 { return 42; }")],
/// ).unwrap();
///
/// assert_eq!(module.functions.len(), 2);
/// ```
pub fn compile_source_with_includes(
    source: &str,
    module_name: &str,
    includes: &[(&str, &str)],
) -> Result<IIRModule, MacroOctError> {
    let mut fs = MemoryFs::new();
    for (name, text) in includes {
        fs.insert(*name, *text);
    }
    // The primary unit needs a `FileId`, and a `FileId` is deliberately opaque
    // and constructible only by a `SourceFs` — otherwise a dialect could mint
    // one naming a different file and misattribute provenance across the whole
    // source map. So the primary source is inserted into the same filesystem
    // to get its id.
    //
    // Naming it `<main>` in angle brackets keeps it from colliding with any
    // real include spelling: `RootedFs::screen_spelling` and this crate's own
    // include set both work on path-shaped names, and no path is spelled with
    // the brackets. (The name is only ever used in diagnostics.)
    let file = fs.insert("<main>", source);

    let unit = preprocess_source(source, file, &mut fs, Bounds::default())?;
    compile_tokens(unit.tokens, module_name)
}

/// Parse, type-check and lower an already-preprocessed Oct token stream.
///
/// Every line below is Oct's, unchanged, and that is the whole point of the
/// slice. The only thing this crate contributes is the stream.
fn compile_tokens(tokens: Vec<Token>, module_name: &str) -> Result<IIRModule, MacroOctError> {
    let mut parser = coding_adventures_oct_parser::create_oct_parser_from_tokens(tokens);
    let ast = parser
        .parse()
        .map_err(|e| MacroOctError::Oct(OctError::Parse(format!("{e}"))))?;

    let type_result = check_ast(&ast);
    if !type_result.ok {
        return Err(MacroOctError::Oct(OctError::Type(
            type_result.errors.into_iter().map(|e| e.message).collect(),
        )));
    }

    oct_iir_compiler::compile_ast(&ast, module_name).map_err(MacroOctError::Oct)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The instruction stream, as comparable text. Used by the tests below to
    /// say "these two programs lowered to the same thing" without depending on
    /// `IIRModule` gaining a `PartialEq` it does not have.
    fn ops(module: &IIRModule) -> Vec<String> {
        module
            .functions
            .iter()
            .flat_map(|f| f.instructions.iter().map(|i| format!("{i:?}")))
            .collect()
    }

    #[test]
    fn a_program_with_no_directives_compiles_exactly_like_oct() {
        let src = "fn main() { out(1, 42); }";
        let macrooct = compile_source(src, "m").expect("MacroOct compiles");
        let oct = oct_iir_compiler::compile_source(src, "m").expect("Oct compiles");
        assert_eq!(ops(&macrooct), ops(&oct));
    }

    #[test]
    fn a_taken_branch_is_kept_and_the_other_is_dropped() {
        let module = compile_source(
            "@if 1\nfn main() { out(1, 42); }\n@else\nfn main() { out(1, 7); }\n@end\n",
            "m",
        )
        .expect("compiles");
        // One `main`, not two: if the untaken branch survived, Oct's own
        // duplicate-definition handling would be what caught it, not us.
        assert_eq!(module.functions.len(), 1);
        let text = ops(&module).join(" ");
        assert!(text.contains("42"), "{text}");
        assert!(!text.contains('7'), "the untaken branch leaked: {text}");
    }

    #[test]
    fn a_false_condition_selects_the_else_branch() {
        let module = compile_source(
            "@if 0\nfn main() { out(1, 42); }\n@else\nfn main() { out(1, 7); }\n@end\n",
            "m",
        )
        .expect("compiles");
        let text = ops(&module).join(" ");
        assert!(text.contains('7'), "{text}");
        assert!(!text.contains("42"), "{text}");
    }

    #[test]
    fn an_include_splices_a_whole_function_in() {
        let module = compile_source_with_includes(
            "@include \"helper.macrooct\"\nfn main() { out(1, helper()); }\n",
            "m",
            &[("helper.macrooct", "fn helper() -> u8 { return 42; }")],
        )
        .expect("compiles");
        let names: Vec<&str> = module.functions.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"helper"), "{names:?}");
        assert!(names.contains(&"main"), "{names:?}");
    }

    #[test]
    fn an_included_file_may_itself_include_and_use_directives() {
        // The composition is not one level deep. The engine walks includes on
        // an explicit stack, and each included file is lexed by MacroOct's own
        // grammar -- so directives inside an include are directives.
        let module = compile_source_with_includes(
            "@include \"outer.macrooct\"\nfn main() { out(1, helper()); }\n",
            "m",
            &[
                ("outer.macrooct", "@include \"inner.macrooct\"\n"),
                (
                    "inner.macrooct",
                    "@if 1\nfn helper() -> u8 { return 42; }\n@else\nfn helper() -> u8 { return 7; }\n@end\n",
                ),
            ],
        )
        .expect("compiles");
        assert_eq!(module.functions.len(), 2);
        assert!(ops(&module).join(" ").contains("42"));
    }

    #[test]
    fn an_include_inside_a_skipped_group_is_never_resolved() {
        // The engine's rule: inside a group being skipped it does directive
        // recognition and nesting tracking only. Proven here by naming a file
        // that does not exist -- resolving it would be an error, so compiling
        // cleanly is positive evidence the include was never attempted.
        let module = compile_source(
            "@if 0\n@include \"does-not-exist.macrooct\"\n@end\nfn main() { out(1, 42); }\n",
            "m",
        )
        .expect("a skipped include must not be resolved");
        assert_eq!(module.functions.len(), 1);
    }

    #[test]
    fn a_condition_inside_a_skipped_group_is_never_evaluated() {
        // Same rule, the other half: `@if 1 +` is a malformed condition. If
        // the engine evaluated it despite sitting in a skipped group, this
        // would be a diagnostic rather than a clean compile.
        let module = compile_source(
            "@if 0\n@if 1 +\nfn main() { out(1, 7); }\n@end\n@end\nfn main() { out(1, 42); }\n",
            "m",
        )
        .expect("a skipped condition must not be evaluated");
        assert!(ops(&module).join(" ").contains("42"));
    }

    #[test]
    fn nested_conditionals_select_the_inner_branch_of_the_outer_one() {
        let module = compile_source(
            "@if 1\n@if 0\nfn main() { out(1, 7); }\n@else\nfn main() { out(1, 42); }\n@end\n@end\n",
            "m",
        )
        .expect("compiles");
        let text = ops(&module).join(" ");
        assert!(text.contains("42"), "{text}");
        assert!(!text.contains('7'), "{text}");
    }

    // --- refusals ----------------------------------------------------------

    #[test]
    fn define_is_refused_with_a_located_diagnostic_in_slice_one() {
        // The engine refuses `Directive::Define` because slice 1 has no macro
        // table. That refusal is the CORRECT slice-1 behaviour: a `@define`
        // that silently did nothing would be far more confusing.
        let err = compile_source("@define LED 1\nfn main() { }\n", "m")
            .expect_err("slice 1 must refuse @define");
        let text = err.to_string();
        assert!(matches!(err, MacroOctError::Preprocess(_)), "{text}");
        assert!(text.contains("slice 2"), "{text}");
    }

    #[test]
    fn an_unterminated_conditional_is_refused() {
        let err = compile_source("@if 1\nfn main() { }\n", "m")
            .expect_err("an unclosed @if must be refused");
        assert!(matches!(err, MacroOctError::Preprocess(_)), "{err}");
    }

    #[test]
    fn an_end_without_an_if_is_refused() {
        let err = compile_source("fn main() { }\n@end\n", "m")
            .expect_err("a stray @end must be refused");
        assert!(matches!(err, MacroOctError::Preprocess(_)), "{err}");
    }

    #[test]
    fn an_unresolvable_include_is_refused() {
        let err = compile_source("@include \"nope.macrooct\"\nfn main() { }\n", "m")
            .expect_err("an unresolvable include must be refused");
        assert!(matches!(err, MacroOctError::Preprocess(_)), "{err}");
    }

    #[test]
    fn an_include_cycle_is_refused_rather_than_looping_forever() {
        let err = compile_source_with_includes(
            "@include \"a.macrooct\"\nfn main() { }\n",
            "m",
            &[
                ("a.macrooct", "@include \"b.macrooct\"\n"),
                ("b.macrooct", "@include \"a.macrooct\"\n"),
            ],
        )
        .expect_err("an include cycle must be refused");
        assert!(err.to_string().contains("cycle"), "{err}");
    }

    #[test]
    fn a_syntax_error_in_the_selected_branch_is_reported_as_an_oct_error() {
        // The layering, made observable: the preprocessor succeeded (it has no
        // opinion about Oct syntax), and Oct rejected what it produced.
        let err = compile_source("@if 1\nfn main( {\n@end\n", "m").expect_err("must fail");
        assert!(matches!(err, MacroOctError::Oct(_)), "{err}");
    }

    #[test]
    fn a_syntax_error_in_the_dropped_branch_is_not_reported() {
        // Because the branch is dropped before the parser ever sees it. This
        // is ordinary preprocessor behaviour and the reason `@if 0` is where
        // hostile input would hide -- which is exactly why the engine refuses
        // to evaluate, resolve or expand anything inside a skipped group.
        compile_source("@if 1\nfn main() { }\n@else\nfn main( {\n@end\n", "m")
            .expect("a dropped branch is never parsed");
    }

    #[test]
    fn a_lex_error_is_reported_as_a_lex_error() {
        let err = compile_source("fn main() { $ }", "m").expect_err("must fail");
        assert!(matches!(err, MacroOctError::Lex(_)), "{err}");
    }

    // --- the sentinel ------------------------------------------------------

    #[test]
    fn a_file_with_no_trailing_newline_still_compiles() {
        // The `Eof` sentinel shares the last line here. Without the strip, it
        // would join `@end`'s logical line and the directive would look like
        // `@end` plus trailing junk.
        compile_source("@if 1\nfn main() { out(1, 42); }\n@end", "m")
            .expect("no trailing newline must be fine");
    }

    #[test]
    fn an_included_files_sentinel_does_not_truncate_the_program() {
        // The sharper half. An `Eof` spliced into the middle of the stream
        // makes Oct's parser stop early and silently compile a TRUNCATED
        // program -- so `main` would simply vanish, with no diagnostic.
        let module = compile_source_with_includes(
            "@include \"helper.macrooct\"\nfn main() { out(1, helper()); }\n",
            "m",
            &[("helper.macrooct", "fn helper() -> u8 { return 42; }")],
        )
        .expect("compiles");
        assert_eq!(
            module.functions.len(),
            2,
            "the program after an include was truncated"
        );
    }

    #[test]
    fn the_preprocessed_stream_carries_a_locus_per_token() {
        let unit = {
            let mut fs = MemoryFs::new();
            let src = "@if 1\nfn main() { out(1, 42); }\n@end\n";
            let file = fs.insert("<main>", src);
            preprocess_source(src, file, &mut fs, Bounds::default()).unwrap()
        };
        // Every token except the restored sentinel has provenance. The
        // sentinel is excluded deliberately: the engine never saw it, so
        // inventing a locus for it would be a lie in the one table whose job
        // is to tell the truth about where tokens came from.
        assert_eq!(unit.map.len(), unit.tokens.len() - 1);
        assert!(unit.tokens.iter().all(|t| t.value != "@if"));
    }

}
