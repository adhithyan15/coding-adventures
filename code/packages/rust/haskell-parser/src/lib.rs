//! Haskell parser backed by compiled versioned parser grammars.

use coding_adventures_haskell_lexer::tokenize_haskell;
use parser::grammar_parser::{GrammarASTNode, GrammarParser};

mod _grammar;

/// Recursion-depth cap for the Haskell [`GrammarParser`] — see
/// [`GrammarParser::with_max_depth`] for why this guard exists at all (deep
/// recursion through `parse_rule` can overflow the *native* thread stack —
/// an uncatchable process abort — before this crate's own callers get a
/// chance to report anything). Before this constant was applied,
/// `create_haskell_parser` never called `with_max_depth` at all, leaving
/// every caller exposed to a native-stack-overflow DoS from adversarial
/// deeply-nested input (e.g. `(((...1...)))`).
///
/// **Not the shared engine's bare default** (see `csharp-parser`'s own
/// identically-named constant for why a blind `DEFAULT_MAX_RULE_DEPTH`
/// (128) is unsafe-for-usability on a rich general-purpose-language
/// grammar). Measured directly instead (binary search over candidate
/// `with_max_depth` values against a fixed 5000-level adversarial
/// `(((...1...)))` input — ordinary parenthesised grouping, the shape
/// universally present in every language — on a default-~2MiB-stack
/// worker thread in a debug build, no `RUST_MIN_STACK` override or
/// explicit `Builder::stack_size` present): safe at **260**, crashes at
/// **261**.
///
/// `MAX_RULE_DEPTH` is set to **180** — about 31% below that floor
/// (comparable margin to `apl-parser`'s own ~26.5%, `j-parser`'s ~30%,
/// `reduce-parser`'s ~28.5%). Measured real-input headroom at `180`: plain
/// parenthesised nesting parses cleanly to at least 15 levels — comfortably
/// beyond ordinary hand-written nesting depth.
///
/// This is measured against only **one** of Haskell's recursion shapes
/// (ordinary paren grouping) — a full audit would also cover nested
/// lambdas, nested list/tuple literals, nested `let`/`where`/`do` blocks,
/// and nested case expressions, the way `css-parser`/`toml-parser`
/// measured *every* shape in their own (much smaller) grammars. That
/// fuller audit is a tracked follow-up; this pass at minimum replaces an
/// unmeasured, silently-broken default with a properly-measured floor for
/// the shape most likely to bind.
const MAX_RULE_DEPTH: usize = 180;

fn validate_version(version: &str) -> Result<&str, String> {
    if _grammar::SUPPORTED_VERSIONS.contains(&version) {
        Ok(version)
    } else {
        Err(format!(
            "Unknown Haskell version '{version}'. Valid values: {}",
            _grammar::SUPPORTED_VERSIONS
                .iter()
                .map(|value| format!("\"{}\"", value))
                .collect::<Vec<_>>()
                .join(", ")
        ))
    }
}

pub fn create_haskell_parser(source: &str, version: &str) -> Result<GrammarParser, String> {
    let version = validate_version(version)?;
    let tokens = tokenize_haskell(source, version)?;
    let grammar = _grammar::parser_grammar(version)
        .expect("compiled Haskell parser grammar missing supported version");
    Ok(GrammarParser::new(tokens, grammar).with_max_depth(MAX_RULE_DEPTH))
}

pub fn parse_haskell(source: &str, version: &str) -> Result<GrammarASTNode, String> {
    let mut parser = create_haskell_parser(source, version)?;
    parser
        .parse()
        .map_err(|e| format!("Haskell parse failed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_name() {
        let ast = parse_haskell("x", "2010").unwrap();
        assert_eq!(ast.rule_name, "file");
    }

    #[test]
    fn all_supported_versions_load() {
        for version in _grammar::SUPPORTED_VERSIONS {
            let ast = parse_haskell("x", version).unwrap();
            assert_eq!(ast.rule_name, "file", "version {version}");
        }
    }

    #[test]
    fn unknown_version_returns_error() {
        let error = parse_haskell("x", "99").unwrap_err();
        assert!(error.contains("99"));
    }

    // -------------------------------------------------------------------
    // Recursion-depth guard (DoS hardening, interim DEFAULT_MAX_RULE_DEPTH
    // pass -- see MAX_RULE_DEPTH's own doc comment).
    // -------------------------------------------------------------------

    fn nested_paren_source(n: usize) -> String {
        format!("{}1{}", "(".repeat(n), ")".repeat(n))
    }

    /// Deeply-nested input must not overflow the native stack on a
    /// default-stack thread -- the whole point of the guard.
    #[test]
    fn test_deeply_nested_input_does_not_overflow_on_default_stack() {
        let src = nested_paren_source(5000);
        let handle = std::thread::spawn(move || {
            let _ = parse_haskell(&src, "2010");
        });
        handle
            .join()
            .expect("MAX_RULE_DEPTH must trip BEFORE native overflow on the default stack");
    }

    /// Reasonable, hand-writable nesting stays well under the cap.
    #[test]
    fn test_reasonable_nesting_stays_under_the_cap() {
        assert!(parse_haskell(&nested_paren_source(10), "2010").is_ok());
    }
}
