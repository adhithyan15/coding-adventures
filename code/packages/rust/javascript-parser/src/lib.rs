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
    let mut parser = create_javascript_parser_typed(source, version)?;
    parser
        .parse()
        .map_err(|e| format!("JavaScript parse failed: {e}"))
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
}
