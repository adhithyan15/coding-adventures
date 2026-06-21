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
use parser::grammar_parser::{GrammarASTNode, GrammarParser};
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

pub fn create_javascript_parser(source: &str, version: &str) -> Result<GrammarParser, String> {
    let version = validate_version(version)?;
    let tokens = tokenize_javascript(source, version)?;
    let grammar = _grammar::parser_grammar(version)
        .expect("compiled JavaScript parser grammar missing supported version");
    Ok(GrammarParser::new(tokens, grammar))
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
    Ok(GrammarParser::new(tokens, grammar))
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
    let tokens: Vec<lexer::token::Token> = cv_tokens.into_iter().map(|t| t.token).collect();

    // 2. Parse via the existing GrammarParser.
    let grammar = _grammar::parser_grammar(version.as_str())
        .expect("compiled JavaScript parser grammar missing supported version");
    let mut parser = GrammarParser::new(tokens, grammar);
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
}
