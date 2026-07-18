//! C# parser backed by compiled versioned parser grammars.

use coding_adventures_csharp_lexer::tokenize_csharp;
use parser::grammar_parser::{GrammarASTNode, GrammarParser};

mod _grammar;

/// Recursion-depth cap for the C# [`GrammarParser`] — see
/// [`GrammarParser::with_max_depth`] for why this guard exists at all (deep
/// recursion through `parse_rule` can overflow the *native* thread stack —
/// an uncatchable process abort — before this crate's own callers get a
/// chance to report anything). Before this constant was applied,
/// `create_csharp_parser` never called `with_max_depth` at all, leaving
/// every caller exposed to a native-stack-overflow DoS from adversarial
/// deeply-nested input.
///
/// **Not [`DEFAULT_MAX_RULE_DEPTH`](parser::grammar_parser::DEFAULT_MAX_RULE_DEPTH)** — an important lesson from measuring
/// this crate specifically: C#'s real expression precedence cascade
/// (`expression -> assignment_expression -> conditional_expression ->
/// null_coalescing_expression -> logical_or_expression -> ... ->
/// unary_expression -> postfix_expression -> primary_expression -> primary
/// -> LPAREN expression RPAREN -> ...`) is dozens of named rules deep per
/// nesting level — far longer than any CAS-family grammar (`reduce`,
/// `derive`) or the simpler `c-parser` subset measured elsewhere in this
/// repo. The shared engine's own [`DEFAULT_MAX_RULE_DEPTH`](parser::grammar_parser::DEFAULT_MAX_RULE_DEPTH) (128), applied
/// blindly, was confirmed by direct measurement to already refuse
/// ordinary, hand-writable nesting: a mere **5 levels** of plain
/// parenthesised grouping (`var x = (((((1)))));`) tripped it. A
/// technically crash-safe cap that rejects 5-deep nesting is not a
/// practically usable one.
///
/// Measured directly instead (binary search over candidate
/// `with_max_depth` values against a fixed 5000-level adversarial
/// `(((...1...)))` input — the ordinary parenthesised-grouping shape, the
/// one universally present in every language and the shape that has bound
/// for every CAS-family grammar measured in this repo so far — on a
/// default-~2MiB-stack worker thread in a debug build): safe at **269**,
/// crashes at **270**.
///
/// `MAX_RULE_DEPTH` is set to **190** — about 29% below that floor
/// (comparable margin to `apl-parser`'s own ~26.5%, `j-parser`'s ~30%,
/// `reduce-parser`'s ~28.5%). Measured real-input headroom at `190`: plain
/// parenthesised nesting parses cleanly to 8 levels (10 trips) — thinner
/// than the CAS-family grammars' own headroom, a direct consequence of how
/// much longer C#'s real precedence cascade is, but still comfortably
/// beyond ordinary hand-written nesting depth (style guides generally
/// discourage more than 3-4 levels for readability).
///
/// This is measured against only **one** of C#'s recursion shapes
/// (ordinary paren grouping) — a full audit would also cover nested
/// lambdas, nested collection expressions, nested method-call arguments,
/// and nested class/block bodies, the way `css-parser`/`toml-parser`
/// measured *every* shape in their own (much smaller) grammars. That
/// fuller audit is a tracked follow-up; this pass at minimum replaces an
/// unmeasured, silently-broken default with a properly-measured floor for
/// the shape most likely to bind.
const MAX_RULE_DEPTH: usize = 190;

fn validate_version(version: &str) -> Result<&str, String> {
    if _grammar::SUPPORTED_VERSIONS.contains(&version) {
        Ok(version)
    } else {
        Err(format!(
            "Unknown C# version '{version}'. Valid values: {}",
            _grammar::SUPPORTED_VERSIONS
                .iter()
                .map(|value| format!("\"{}\"", value))
                .collect::<Vec<_>>()
                .join(", ")
        ))
    }
}

pub fn create_csharp_parser(source: &str, version: &str) -> Result<GrammarParser, String> {
    let version = validate_version(version)?;
    let tokens = tokenize_csharp(source, version)?;
    let grammar = _grammar::parser_grammar(version)
        .expect("compiled C# parser grammar missing supported version");
    Ok(GrammarParser::new(tokens, grammar).with_max_depth(MAX_RULE_DEPTH))
}

pub fn parse_csharp(source: &str, version: &str) -> Result<GrammarASTNode, String> {
    let mut parser = create_csharp_parser(source, version)?;
    parser
        .parse()
        .map_err(|e| format!("C# parse failed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_class() {
        let ast = parse_csharp("class Hello {}", "12.0").unwrap();
        assert_eq!(ast.rule_name, "compilation_unit");
    }

    #[test]
    fn all_supported_versions_load() {
        for version in _grammar::SUPPORTED_VERSIONS {
            let ast = parse_csharp("public class Foo {}", version).unwrap();
            assert_eq!(ast.rule_name, "compilation_unit", "version {version}");
        }
    }

    #[test]
    fn unknown_version_returns_error() {
        let error = parse_csharp("class Hello {}", "99.0").unwrap_err();
        assert!(error.contains("99.0"));
    }

    // -------------------------------------------------------------------
    // Recursion-depth guard (DoS hardening) -- see MAX_RULE_DEPTH's own
    // doc comment for the measurement.
    // -------------------------------------------------------------------

    fn nested_paren_source(n: usize) -> String {
        format!(
            "class C {{ void M() {{ var x = {}1{}; }} }}",
            "(".repeat(n),
            ")".repeat(n)
        )
    }

    /// Deeply-nested input must not overflow the native stack on a
    /// default-stack thread -- the whole point of the guard.
    #[test]
    fn test_deeply_nested_input_does_not_overflow_on_default_stack() {
        let src = nested_paren_source(5000);
        let handle = std::thread::spawn(move || {
            let _ = parse_csharp(&src, "12.0");
        });
        handle
            .join()
            .expect("MAX_RULE_DEPTH must trip BEFORE native overflow on the default stack");
    }

    /// Reasonable, hand-writable nesting stays well under the cap. (8
    /// levels is the measured headroom at `MAX_RULE_DEPTH = 190` -- see
    /// that constant's own doc comment.)
    #[test]
    fn test_reasonable_nesting_stays_under_the_cap() {
        assert!(parse_csharp(&nested_paren_source(8), "12.0").is_ok());
    }
}
