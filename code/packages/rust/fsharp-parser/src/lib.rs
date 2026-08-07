//! F# parser backed by compiled versioned parser grammars.

use coding_adventures_fsharp_lexer::{tokenize_fsharp, DEFAULT_VERSION};
use parser::grammar_parser::{GrammarASTNode, GrammarParser};

mod _grammar;

/// Recursion-depth cap for the F# [`GrammarParser`] — see
/// [`GrammarParser::with_max_depth`] for why this guard exists at all (deep
/// recursion through `parse_rule` can overflow the *native* thread stack —
/// an uncatchable process abort — before this crate's own callers get a
/// chance to report anything). Before this constant was applied,
/// `create_fsharp_parser` never called `with_max_depth` at all, leaving
/// every caller exposed to a native-stack-overflow DoS from adversarial
/// deeply-nested input (e.g. `let x = (((...1...)))`).
///
/// **Not the shared engine's bare default** (see `csharp-parser`'s own
/// identically-named constant for why a blind `DEFAULT_MAX_RULE_DEPTH`
/// (128) is unsafe-for-usability on a rich general-purpose-language
/// grammar: it can trip after only a handful of levels of ordinary
/// parenthesised nesting). Measured directly instead (binary search over
/// candidate `with_max_depth` values against a fixed 5000-level
/// adversarial `let x = (((...1...)))` input — ordinary parenthesised
/// grouping, the shape universally present in every language — on a
/// default-~2MiB-stack worker thread in a debug build, no
/// `RUST_MIN_STACK` override or explicit `Builder::stack_size` present):
/// safe at **289**, crashes at **290**.
///
/// `MAX_RULE_DEPTH` is set to **200** — about 31% below that floor
/// (comparable margin to `apl-parser`'s own ~26.5%, `j-parser`'s ~30%,
/// `reduce-parser`'s ~28.5%). Measured real-input headroom at `200`: plain
/// parenthesised nesting parses cleanly to at least 20 levels — comfortably
/// beyond ordinary hand-written nesting depth.
///
/// This is measured against only **one** of F#'s recursion shapes (ordinary
/// paren grouping) — a full audit would also cover nested lambdas, nested
/// list/array/record literals, nested match expressions, and nested
/// computation expressions, the way `css-parser`/`toml-parser` measured
/// *every* shape in their own (much smaller) grammars. That fuller audit is
/// a tracked follow-up; this pass at minimum replaces an unmeasured,
/// silently-broken default with a properly-measured floor for the shape
/// most likely to bind.
const MAX_RULE_DEPTH: usize = 200;

fn resolve_version(version: &str) -> Result<&str, String> {
    let resolved = if version.is_empty() {
        DEFAULT_VERSION
    } else {
        version
    };

    if _grammar::SUPPORTED_VERSIONS.contains(&resolved) {
        Ok(resolved)
    } else {
        Err(format!(
            "Unknown F# version '{version}'. Valid values: {}",
            _grammar::SUPPORTED_VERSIONS
                .iter()
                .map(|value| format!("\"{}\"", value))
                .collect::<Vec<_>>()
                .join(", ")
        ))
    }
}

pub fn create_fsharp_parser(source: &str, version: &str) -> Result<GrammarParser, String> {
    let version = resolve_version(version)?;
    let tokens = tokenize_fsharp(source, version)?;
    let grammar = _grammar::parser_grammar(version)
        .expect("compiled F# parser grammar missing supported version");
    Ok(GrammarParser::new(tokens, grammar).with_max_depth(MAX_RULE_DEPTH))
}

pub fn parse_fsharp(source: &str, version: &str) -> Result<GrammarASTNode, String> {
    let mut parser = create_fsharp_parser(source, version)?;
    parser
        .parse()
        .map_err(|e| format!("F# parse failed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_let_binding() {
        let ast = parse_fsharp("let value = 1", "").unwrap();
        assert_eq!(ast.rule_name, "compilation_unit");
    }

    #[test]
    fn all_supported_versions_load() {
        for version in _grammar::SUPPORTED_VERSIONS {
            let ast = parse_fsharp("let value = 1", version).unwrap();
            assert_eq!(ast.rule_name, "compilation_unit", "version {version}");
        }
    }

    #[test]
    fn unknown_version_returns_error() {
        let error = parse_fsharp("let value = 1", "11").unwrap_err();
        assert!(error.contains("11"));
    }

    // -------------------------------------------------------------------
    // Recursion-depth guard (DoS hardening, interim DEFAULT_MAX_RULE_DEPTH
    // pass -- see MAX_RULE_DEPTH's own doc comment).
    // -------------------------------------------------------------------

    fn nested_paren_source(n: usize) -> String {
        format!("let x = {}1{}", "(".repeat(n), ")".repeat(n))
    }

    /// Deeply-nested input must not overflow the native stack on a
    /// default-stack thread -- the whole point of the guard.
    #[test]
    fn test_deeply_nested_input_does_not_overflow_on_default_stack() {
        let src = nested_paren_source(5000);
        let handle = std::thread::spawn(move || {
            let _ = parse_fsharp(&src, "");
        });
        handle
            .join()
            .expect("MAX_RULE_DEPTH must trip BEFORE native overflow on the default stack");
    }

    /// Reasonable, hand-writable nesting stays well under the cap.
    #[test]
    fn test_reasonable_nesting_stays_under_the_cap() {
        assert!(parse_fsharp(&nested_paren_source(10), "").is_ok());
    }
}
