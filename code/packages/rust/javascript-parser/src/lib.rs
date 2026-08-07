//! JavaScript parser backed by compiled ECMAScript parser grammars (es1 through es2025).
//!
//! # Correlation-vector plumbing
//!
//! Per [CLOC03](../../../specs/CLOC03-correlation-vector-plumbing.md)
//! §"Stage 2 — Parser", when called via [`parse_javascript_with_cv`] the
//! parser inherits the lexer's per-token CV IDs onto the AST via
//! `CVLog::merge(token_ids, Origin{...})`.
//!
//! **v1 (this version):** stamps a single CV ID onto the `Program` root
//! built from the merge of *all* token IDs, and appends one
//! `Contribution { source: "parser", tag: "constructed", meta: { version, ... } }`.
//! Per-AST-node CV propagation requires deeper plumbing into the
//! underlying `GrammarParser` (which produces a generic `GrammarASTNode`
//! tree, not the typed `javascript-ast::Program` yet) and is deferred to
//! follow-up PRs alongside the AST-typed parser output.

use coding_adventures_correlation_vector::{CVLog, Origin};
use coding_adventures_javascript_lexer::{
    tokenize_javascript, tokenize_javascript_typed, tokenize_javascript_with_cv,
};
use coding_adventures_javascript_tokens::EsVersion;
use parser::grammar_parser::{GrammarASTNode, GrammarParser, DEFAULT_MAX_RULE_DEPTH};
use std::collections::HashMap;

/// Typed default version. New code should prefer this over the string form.
pub const DEFAULT_ES_VERSION: EsVersion = EsVersion::Es2025;

mod _grammar;
pub mod asi;
pub mod bridge;

/// Parse JavaScript source and return a fully typed [`Program`] AST
/// (CLOC12.136 bridge).
///
/// Returns `Err` for:
/// - Lexer / parser failures (malformed JavaScript).
/// - Phase 2+ syntax not yet in the typed AST (async, generators, classes,
///   for-in/of, try-catch, destructuring, template literals …).  Callers
///   that handle this gracefully should match on
///   [`bridge::BridgeError::UnsupportedSyntax`] and fall back to
///   WHITESPACE_ONLY / identity output.
pub fn parse_javascript_program(
    source: &str,
    version: EsVersion,
) -> Result<coding_adventures_javascript_ast::Program, String> {
    let node = parse_javascript_typed(source, version)?;
    bridge::grammar_to_program(&node, version).map_err(|e| e.to_string())
}

fn validate_version(version: &str) -> Result<&str, String> {
    if _grammar::SUPPORTED_VERSIONS.contains(&version) {
        Ok(version)
    } else {
        Err(format!(
            "Unknown JavaScript/ECMAScript version '{version}'. Valid values: {}",
            _grammar::SUPPORTED_VERSIONS
                .iter()
                .map(|value| format!("\"{}\"", value))
                .collect::<Vec<_>>()
                .join(", ")
        ))
    }
}

// SECURITY: `GrammarParser::new` defaults to an *unbounded* recursion depth,
// and `javascript-to-semantic-ir::compile_source` (and any other caller
// reachable with untrusted JS on an ordinary ~2 MiB stack) calls this
// function's parse path directly, so pathologically deep nesting
// (`((((…))))`, `1+1+…`) would otherwise overflow the native thread stack —
// an uncatchable process abort — before either `Result`-returning entry
// point below ever got a chance to report anything. `asi.rs`'s own
// `parse_with_asi` already measured `DEFAULT_MAX_RULE_DEPTH` (128) safe for
// this exact grammar ("trips a clean, recoverable parse error well below
// the ~200-frame overflow point... real JS never nests grouping this
// deep") — opted in here too, for the untyped/typed factories that
// `parse_with_asi` does not sit in front of.
pub fn create_javascript_parser(source: &str, version: &str) -> Result<GrammarParser, String> {
    let version = validate_version(version)?;
    let tokens = tokenize_javascript(source, version)?;
    let grammar = _grammar::parser_grammar(version)
        .expect("compiled JavaScript parser grammar missing supported version");
    Ok(GrammarParser::new(tokens, grammar).with_max_depth(DEFAULT_MAX_RULE_DEPTH))
}

pub fn parse_javascript(source: &str, version: &str) -> Result<GrammarASTNode, String> {
    let mut parser = create_javascript_parser(source, version)?;
    parser
        .parse()
        .map_err(|e| format!("JavaScript parse failed: {e}"))
}

/// Typed version of [`create_javascript_parser`]. Takes an [`EsVersion`]
/// directly; cannot fail with an unknown-version error.
pub fn create_javascript_parser_typed(
    source: &str,
    version: EsVersion,
) -> Result<GrammarParser, String> {
    let tokens = tokenize_javascript_typed(source, version)?;
    let grammar = _grammar::parser_grammar(version.as_str())
        .expect("compiled JavaScript parser grammar missing supported version");
    Ok(GrammarParser::new(tokens, grammar).with_max_depth(DEFAULT_MAX_RULE_DEPTH))
}

/// Typed version of [`parse_javascript`]. Takes an [`EsVersion`] directly.
pub fn parse_javascript_typed(
    source: &str,
    version: EsVersion,
) -> Result<GrammarASTNode, String> {
    // Apply Phase-1 ASI: the parser requires explicit `SEMICOLON` terminals, so
    // semicolon-light source (`{ a() }`, `return 1}`) would otherwise fail and
    // closurec would degrade the whole program to WHITESPACE_ONLY. `parse_with_asi`
    // only inserts a `;` before a `}`/EOF when parsing genuinely failed for lack
    // of one, so it is a no-op on any input that already parses. See [`asi`].
    let tokens = tokenize_javascript_typed(source, version)?;
    asi::parse_with_asi(tokens, version).map_err(|e| format!("JavaScript parse failed: {e}"))
}

/// CV-carrying twin of [`parse_javascript_typed`] (CLOC27 D3).
///
/// Routes through the correlation-vector tokenizer
/// ([`tokenize_javascript_with_cv`]) so every token carries its own CvId
/// (CLOC27 D2), then runs the identical Phase-1 ASI parse. The returned
/// `GrammarASTNode` therefore holds tokens whose `cv` is `Some(..)`, and the
/// bridge's leaf factory (`convert_primary_token`) stamps that id onto each
/// leaf literal — giving the SIMPLE optimization path real per-token source
/// provenance for its constant folds.
///
/// This is the entry the SIMPLE `--correlation_vector` path uses (CLOC27 D5);
/// the plain [`parse_javascript_typed`] stays the zero-overhead default and is
/// byte-identical to today on the non-CV path.
///
/// Unlike [`parse_javascript_with_cv`], this does *not* mint a program-root CV
/// or append a "constructed" contribution: it is the typed-AST feeder for the
/// optimizer, whose own passes (constant-fold) own the downstream CV records.
pub fn parse_javascript_typed_with_cv(
    source: &str,
    source_file: &str,
    version: EsVersion,
    cv: &mut CVLog,
) -> Result<GrammarASTNode, String> {
    // Tokenize with CV plumbing, then stamp each token's CvId onto the token
    // (CLOC27 D2) so it rides through the parser to the bridge unchanged.
    let cv_tokens = tokenize_javascript_with_cv(source, source_file, version, cv)?;
    let tokens: Vec<lexer::token::Token> = cv_tokens
        .into_iter()
        .map(|t| lexer::token::Token {
            cv: Some(t.cv),
            ..t.token
        })
        .collect();
    // Identical Phase-1 ASI parse to `parse_javascript_typed` — only the token
    // source differs (CV-stamped). See [`asi`].
    asi::parse_with_asi(tokens, version).map_err(|e| format!("JavaScript parse failed: {e}"))
}

/// A parsed program paired with the correlation-vector ID assigned to its
/// root by [`parse_javascript_with_cv`].
///
/// The CV ID is in `CVLog`'s standard string format (e.g. `"a3f1.1.4"`).
/// Use it to look up the parser's contributions, the root `Origin`, or
/// the merged parent token IDs.
#[derive(Debug, Clone)]
pub struct ProgramWithCv {
    /// The parsed AST. v1 still uses the generic `GrammarASTNode` —
    /// switching to `javascript-ast::Program` is a follow-up.
    pub ast: GrammarASTNode,
    /// The CV ID assigned to the program root.
    pub cv: String,
}

/// Parse JavaScript source and plumb correlation vectors per CLOC03
/// §"Stage 2 — Parser" (v1: root-only).
///
/// Behavior:
///
/// 1. Tokenize via [`tokenize_javascript_with_cv`] so every token gets
///    its own CV ID anchored to the source file.
/// 2. Run the existing `GrammarParser` over the tokens (stripping the
///    CV-pairing wrapper, since the underlying parser doesn't carry CV
///    information per-node yet — a follow-up PR plumbs CV deeper).
/// 3. Compute the program-root CV via `cv.merge(all_token_cv_ids,
///    Origin{ source: source_file, location: "0:0", ... })`. The
///    `Origin` is the *program-as-a-whole* origin; individual tokens'
///    own origins are still reachable via their per-token CVs.
/// 4. Append a `Contribution { source: "parser", tag: "constructed",
///    meta: { rule, version } }` to the program-root CV.
///
/// Returns `(ast, program_cv_id)` packaged in a [`ProgramWithCv`].
pub fn parse_javascript_with_cv(
    source: &str,
    source_file: &str,
    version: EsVersion,
    cv: &mut CVLog,
) -> Result<ProgramWithCv, String> {
    // 1. Tokenize with CV plumbing.
    let cv_tokens = tokenize_javascript_with_cv(source, source_file, version, cv)?;
    let token_cv_ids: Vec<String> = cv_tokens.iter().map(|t| t.cv.clone()).collect();
    // CLOC27 D2: stamp each token's CvId onto the token itself instead of
    // discarding it. The parser copies tokens into `GrammarASTNode` children
    // unchanged, so the id rides through to the bridge's leaf factory
    // (`convert_primary_token`), where it becomes the leaf literal's `cv`.
    // Previously this stripped the id (`.map(|t| t.token)`), leaving every leaf
    // with `cv: None` and dead-ending fold provenance at the bridge boundary.
    let tokens: Vec<lexer::token::Token> = cv_tokens
        .into_iter()
        .map(|t| lexer::token::Token {
            cv: Some(t.cv),
            ..t.token
        })
        .collect();

    // 2. Parse via the existing GrammarParser. Opt into the depth guard —
    //    see `create_javascript_parser`'s SECURITY comment.
    let grammar = _grammar::parser_grammar(version.as_str())
        .expect("compiled JavaScript parser grammar missing supported version");
    let mut parser = GrammarParser::new(tokens, grammar).with_max_depth(DEFAULT_MAX_RULE_DEPTH);
    let ast = parser
        .parse()
        .map_err(|e| format!("JavaScript parse failed: {e}"))?;

    // 3. Mint the program-root CV by merging every token's CV.
    let parent_refs: Vec<&str> = token_cv_ids.iter().map(|s| s.as_str()).collect();
    let program_cv = cv.merge(
        &parent_refs,
        Some(Origin {
            source: source_file.to_string(),
            location: "0:0".to_string(),
            timestamp: None,
            meta: HashMap::new(),
        }),
    );

    // 4. Append the "constructed" contribution per CLOC03 §Stage 2.
    let mut meta = HashMap::new();
    meta.insert(
        "rule".to_string(),
        serde_json::Value::String(ast.rule_name.clone()),
    );
    meta.insert(
        "version".to_string(),
        serde_json::Value::String(version.as_str().to_string()),
    );
    // Ignore the Err path on `contribute` — the only error it can return is
    // "contributing to a deleted entity," which can't happen here (we just
    // created the entity).
    let _ = cv.contribute(&program_cv, "parser", "constructed", meta);

    Ok(ProgramWithCv {
        ast,
        cv: program_cv,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_es5_javascript() {
        let ast = parse_javascript("var x = 1;", "es5").unwrap();
        assert_eq!(ast.rule_name, "program");
    }

    #[test]
    fn parses_versioned_ecmascript() {
        let ast = parse_javascript("let x = 1;", "es2015").unwrap();
        assert_eq!(ast.rule_name, "program");
    }

    #[test]
    fn all_supported_versions_load() {
        for version in _grammar::SUPPORTED_VERSIONS {
            let ast = parse_javascript("", version).unwrap();
            assert_eq!(ast.rule_name, "program", "version {version:?}");
        }
    }

    #[test]
    fn parse_typed_es2015() {
        let ast = parse_javascript_typed("let x = 1;", EsVersion::Es2015).unwrap();
        assert_eq!(ast.rule_name, "program");
    }

    #[test]
    fn default_es_version_constant_is_es2025() {
        assert_eq!(DEFAULT_ES_VERSION, EsVersion::Es2025);
    }

    #[test]
    fn all_typed_versions_load() {
        for &version in EsVersion::ALL {
            let ast = parse_javascript_typed("", version).unwrap();
            assert_eq!(ast.rule_name, "program", "version {version}");
        }
    }

    #[test]
    fn create_parser_typed() {
        let _parser = create_javascript_parser_typed("var x = 1;", EsVersion::Es5).unwrap();
    }

    // ----- CV-plumbed parser (CLOC03 Stage 2 v1) -----

    #[test]
    fn parse_with_cv_assigns_a_program_id() {
        let mut cv = CVLog::new(true);
        let pwc = parse_javascript_with_cv("var x = 1;", "src/test.js", EsVersion::Es5, &mut cv)
            .unwrap();
        assert_eq!(pwc.ast.rule_name, "program");
        assert!(!pwc.cv.is_empty(), "expected a non-empty program CV id");
    }

    #[test]
    fn parse_with_cv_program_id_resolves_in_log() {
        let mut cv = CVLog::new(true);
        let pwc = parse_javascript_with_cv("var x = 1;", "lookup.js", EsVersion::Es5, &mut cv)
            .unwrap();
        let entry = cv
            .get(&pwc.cv)
            .unwrap_or_else(|| panic!("program CV {:?} not found in log", pwc.cv));
        let origin = entry
            .origin
            .as_ref()
            .expect("program CV must have an Origin");
        assert_eq!(origin.source, "lookup.js");
        assert_eq!(origin.location, "0:0");
    }

    #[test]
    fn parse_with_cv_appends_constructed_contribution() {
        let mut cv = CVLog::new(true);
        let pwc = parse_javascript_with_cv("var x = 1;", "src.js", EsVersion::Es5, &mut cv)
            .unwrap();
        let history = cv.history(&pwc.cv);
        let constructed = history
            .iter()
            .find(|c| c.source == "parser" && c.tag == "constructed")
            .expect("expected one parser/constructed contribution");
        assert_eq!(
            constructed.meta.get("rule").and_then(|v| v.as_str()),
            Some("program")
        );
        assert_eq!(
            constructed.meta.get("version").and_then(|v| v.as_str()),
            Some("es5")
        );
    }

    #[test]
    fn parse_with_cv_program_has_token_ancestors() {
        let mut cv = CVLog::new(true);
        let pwc = parse_javascript_with_cv("var x = 1;", "anc.js", EsVersion::Es5, &mut cv)
            .unwrap();
        // The program CV was minted via cv.merge(token_ids, ...) so it
        // should have at least one ancestor.
        let ancestors = cv.ancestors(&pwc.cv);
        assert!(
            !ancestors.is_empty(),
            "expected program CV to have token ancestors"
        );
    }

    #[test]
    fn parse_with_cv_disabled_log_still_returns_ast() {
        // Disabled log keeps API shape but skips storage. The parser must
        // not panic and must still return a valid AST.
        let mut cv = CVLog::new(false);
        let pwc = parse_javascript_with_cv("var x = 1;", "off.js", EsVersion::Es5, &mut cv)
            .unwrap();
        assert_eq!(pwc.ast.rule_name, "program");
        assert!(!pwc.cv.is_empty());
    }

    // ===================================================================
    // CLOC17 — assignment-expression parsing regression
    // ===================================================================
    //
    // The `assignment_expression` PEG rule used to list `conditional_expression`
    // BEFORE the `left_hand_side_expression assignment_operator
    // assignment_expression` alternative. Ordered-choice PEG is first-match-
    // wins: a bare identifier `a` is itself a valid `conditional_expression`,
    // so the parser committed to that alternative, consumed only `a`, and left
    // the `=` unconsumed — the assign-target alternative was never reached and
    // `a = 1;` failed to parse. closurec then fell back to whitespace-only
    // minification for the WHOLE program (see spec CLOC17). The fix reorders
    // every es*.grammar so the assign-target alternative is tried first
    // (function-likes ahead of it, `conditional_expression` last); when no
    // assignment operator follows the left-hand side the sequence fails fast
    // and falls through to `conditional_expression` exactly as before.
    //
    // The bug was identical across all 14 grammars, so these tests sweep
    // `EsVersion::ALL` for the version-independent forms, and check the
    // arrow/yield alternatives (which must stay AHEAD of assign-target) only
    // on the versions that have them.

    /// Assert `src` parses (grammar stage) under `version` and yields a
    /// `program` root.
    fn assert_parses(src: &str, version: EsVersion) {
        let node = parse_javascript_typed(src, version)
            .unwrap_or_else(|e| panic!("[{version:?}] expected `{src}` to parse, got: {e}"));
        assert_eq!(node.rule_name, "program", "[{version:?}] `{src}`");
    }

    #[test]
    fn cloc17_assignment_statements_parse_every_version() {
        // The canonical forms the reorder unblocks: simple assignment,
        // compound assignment, member-target assignment, a right-associative
        // chain, and a ternary right-hand side. All 14 grammars shared the
        // bug, so all 14 must now accept them.
        for &version in EsVersion::ALL {
            assert_parses("a = 1;", version);
            assert_parses("a += 1;", version);
            assert_parses("a.b = 1;", version);
            assert_parses("a = b = c;", version); // right-associative chain
            assert_parses("a = b ? c : d;", version); // ternary RHS
        }
    }

    #[test]
    fn cloc17_compound_assignment_operators_parse() {
        // A representative spread of compound operators on a modern grammar —
        // each is a distinct `assignment_operator` token, all of which now sit
        // behind the reachable assign-target alternative.
        for src in [
            "a += 1;",
            "a -= 1;",
            "a *= 1;",
            "a /= 1;",
            "a %= 1;",
            "a <<= 1;",
            "a >>= 1;",
            "a >>>= 1;",
            "a &= 1;",
            "a |= 1;",
            "a ^= 1;",
        ] {
            assert_parses(src, EsVersion::Es2025);
        }
    }

    #[test]
    fn cloc17_non_assignment_forms_still_parse_every_version() {
        // The reorder tries the assign-target alternative FIRST; when no
        // assignment operator follows the left-hand side it must fail fast and
        // fall through to `conditional_expression`. Pin that every common
        // non-assignment expression statement (and the separate declarator
        // path) still parses, on every version — i.e. the reorder is purely
        // additive.
        for &version in EsVersion::ALL {
            assert_parses("a;", version); // bare identifier
            assert_parses("a.b;", version); // member
            assert_parses("f();", version); // call
            assert_parses("a + b;", version); // binary
            assert_parses("a ? b : c;", version); // ternary
            assert_parses("var x = 1;", version); // declarator init (separate path)
        }
    }

    #[test]
    fn cloc17_arrow_and_yield_unaffected_on_modern_versions() {
        // Arrow / yield alternatives stay AHEAD of the assign-target
        // alternative in the reordered rule, so they must still parse where
        // the version supports them (es2015+).
        for &version in &[EsVersion::Es2015, EsVersion::Es2025] {
            assert_parses("var f = x => x;", version);
        }
        // `yield` is only meaningful inside a generator body.
        assert_parses("function* g() { yield 1; }", EsVersion::Es2025);
    }

    #[test]
    fn cloc17_assignment_bridges_to_assignment_expression() {
        // End-to-end through the bridge: the parsed assignment must produce an
        // `AssignmentExpression` typed node (not a parse error / fallback),
        // proving the fix unblocks the downstream optimization pipeline and
        // not merely the parser.
        use coding_adventures_javascript_ast::statement::TaggedStatement;
        use coding_adventures_javascript_ast::{Expression, ProgramItem, Statement};
        let program = parse_javascript_program("a = 1;", EsVersion::Es2025)
            .expect("assignment must bridge to a typed Program");
        let item = program.body.first().expect("one program item");
        match item {
            ProgramItem::Statement(Statement::Tagged(TaggedStatement::ExpressionStatement(es))) => {
                assert!(
                    matches!(es.expression, Expression::AssignmentExpression(_)),
                    "expected AssignmentExpression, got {:?}",
                    es.expression
                );
            }
            other => panic!("expected an expression statement, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // MAX_RULE_DEPTH recursion-depth guard (create_javascript_parser /
    // create_javascript_parser_typed / parse_javascript_with_cv)
    //
    // `asi.rs`'s own `parse_with_asi` already opts into `DEFAULT_MAX_RULE_DEPTH`
    // and documents it safe for this grammar; these three entry points did
    // not. Measured directly (binary search against `create_javascript_parser`,
    // the untyped factory `javascript-to-semantic-ir::compile_source` calls):
    // safe up to 17 real nesting levels, trips at 18 — comfortably below
    // `asi.rs`'s own documented ~200-frame native overflow point.
    // -----------------------------------------------------------------------

    fn nested_paren_source(n: usize) -> String {
        format!("var x = {}1{};", "(".repeat(n), ")".repeat(n))
    }

    /// Deeply-nested input must produce a recoverable error, not overflow the
    /// native stack. We parse 5000 levels — far past `DEFAULT_MAX_RULE_DEPTH`
    /// — on a worker thread with a generous 32 MiB stack, so the *guard* is
    /// what stops the recursion, not the stack running out.
    #[test]
    fn test_deeply_nested_input_returns_error_not_overflow() {
        let handle = std::thread::Builder::new()
            .name("javascript-parser-depth-guard-regression".to_string())
            .stack_size(32 * 1024 * 1024)
            .spawn(|| {
                let result = parse_javascript(&nested_paren_source(5000), "es2020");
                assert!(
                    result.is_err(),
                    "deeply-nested input must fail with an error, not parse or crash"
                );
            })
            .expect("failed to spawn worker thread");
        handle
            .join()
            .expect("depth guard must keep the worker thread from crashing");
    }

    /// Input that nests *exactly up to* `DEFAULT_MAX_RULE_DEPTH` still
    /// parses cleanly, and one layer deeper cleanly trips the guard. These
    /// exact boundary counts (17 legitimate levels) were found empirically
    /// by binary-searching against increasing nesting counts at the
    /// production cap.
    #[test]
    fn test_nesting_up_to_cap_still_parses() {
        assert!(
            parse_javascript(&nested_paren_source(17), "es2020").is_ok(),
            "17 levels must stay under the cap"
        );
        assert!(
            parse_javascript(&nested_paren_source(18), "es2020").is_err(),
            "one nesting level past the cap's measured limit must fail"
        );
    }

    /// A caller relying on the depth guard must have it trip *before* the
    /// native stack overflows on a default-stack thread — otherwise a
    /// production caller (e.g. `javascript-to-semantic-ir::compile_source`,
    /// or `cargo test`'s own per-test thread) would still crash. We parse
    /// far-too-deep input on a worker thread with **no** `stack_size`
    /// override (the same ~2 MiB a default thread gets). A clean `Err` (not
    /// a `join()` failure from a crashed thread) proves the cap sits safely
    /// below the native overflow point on the default stack.
    #[test]
    fn test_opt_in_cap_trips_before_overflow_on_default_stack() {
        let handle = std::thread::spawn(|| {
            let result = parse_javascript(&nested_paren_source(5000), "es2020");
            assert!(result.is_err(), "deeply-nested input must error, not crash");
        });
        handle
            .join()
            .expect("the depth guard must trip BEFORE native overflow on the default stack");
    }

    /// `create_javascript_parser_typed` shares the same guard: constructing
    /// the parser always succeeds (it does no parsing yet), but calling
    /// `.parse()` on deeply-nested input must fail cleanly.
    #[test]
    fn test_typed_entry_point_also_rejects_deep_nesting() {
        let mut parser =
            create_javascript_parser_typed(&nested_paren_source(5000), EsVersion::Es2020)
                .expect("constructing the parser never fails");
        assert!(parser.parse().is_err(), "deeply-nested input must fail to parse");
    }

    /// `parse_javascript_with_cv` shares the same guard.
    #[test]
    fn test_cv_entry_point_also_rejects_deep_nesting() {
        let mut cv = CVLog::new(false);
        assert!(
            parse_javascript_with_cv(&nested_paren_source(5000), "test.js", EsVersion::Es2020, &mut cv)
                .is_err()
        );
    }
}
