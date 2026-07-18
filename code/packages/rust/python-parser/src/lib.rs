//! Python parser backed by compiled versioned parser grammars.

use coding_adventures_python_lexer::{tokenize_python, DEFAULT_VERSION};
use parser::grammar_parser::{GrammarASTNode, GrammarParser};

mod _grammar;

/// Recursion-depth cap for the Python [`GrammarParser`] — see
/// [`GrammarParser::with_max_depth`] for why this guard exists at all (deep
/// recursion through `parse_rule` can overflow the *native* thread stack —
/// an uncatchable process abort — before this crate's own callers get a
/// chance to report anything). Before this constant was applied,
/// `create_python_parser` never called `with_max_depth` at all, leaving
/// every caller — for *any* of the 6 embedded grammar versions (2.7, 3.0,
/// 3.6, 3.8, 3.10, 3.12) `_grammar::parser_grammar(version)` can select —
/// exposed to a native-stack-overflow DoS from adversarial deeply-nested
/// input (e.g. `x = (((...1...)))`).
///
/// **Not the shared engine's bare default** (see `csharp-parser`'s own
/// identically-named constant for why a blind `DEFAULT_MAX_RULE_DEPTH`
/// (128) is unsafe-for-usability on a rich general-purpose-language
/// grammar). Measured directly instead (binary search over candidate
/// `with_max_depth` values against a fixed 5000-level adversarial
/// `x = (((...1...)))` input — ordinary parenthesised grouping, the shape
/// universally present in every language — on a default-~2MiB-stack
/// worker thread in a debug build), against the **newest** embedded
/// grammar (`"3.12"`, the most feature-complete and therefore most likely
/// to have the tightest floor): safe at **273**, crashes at **274**. Spot-
/// checked against the **oldest** embedded grammar (`"2.7"`) too, since a
/// single cap must be safe regardless of which version a caller selects
/// at runtime: identical floor, 273/274 — the ordinary parenthesised-
/// expression recursion path is unchanged between Python 2.7 and 3.12 (new
/// syntax across that span, e.g. `async`/`await`/walrus/`match`, is
/// additive, not a change to this shape). The other 4 embedded versions
/// were not independently spot-checked; this is a disclosed scope
/// limitation, not a verified guarantee across all 6.
///
/// `MAX_RULE_DEPTH` is set to **190** — about 30% below the 273 floor
/// (comparable margin to `apl-parser`'s own ~26.5%, `j-parser`'s ~30%,
/// `reduce-parser`'s ~28.5%). Measured real-input headroom at `190`
/// (against `"3.12"`): plain parenthesised nesting parses cleanly to 9
/// levels (10 trips) — comfortably beyond ordinary hand-written nesting
/// depth.
///
/// This is measured against only **one** of Python's recursion shapes
/// (ordinary paren grouping) — a full audit would also cover nested
/// lambdas, nested list/dict/set/tuple literals and comprehensions, nested
/// function calls, nested `match`/`case` structural patterns (3.10+), and
/// nested `if`/`for`/`while`/`def`/`class` blocks, the way `css-parser`/
/// `toml-parser` measured *every* shape in their own (much smaller)
/// grammars. That fuller audit — across all 6 embedded versions — is a
/// tracked follow-up; this pass at minimum replaces an unmeasured,
/// silently-broken default with a properly-measured floor for the shape
/// most likely to bind.
const MAX_RULE_DEPTH: usize = 190;

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
            "Unsupported Python version '{version}'. Supported versions: {}",
            _grammar::SUPPORTED_VERSIONS
                .iter()
                .map(|value| format!("\"{}\"", value))
                .collect::<Vec<_>>()
                .join(", ")
        ))
    }
}

pub fn create_python_parser(source: &str, version: &str) -> Result<GrammarParser, String> {
    let version = resolve_version(version)?;
    let tokens = tokenize_python(source, version)?;
    let grammar = _grammar::parser_grammar(version)
        .expect("compiled Python parser grammar missing supported version");
    Ok(GrammarParser::new(tokens, grammar).with_max_depth(MAX_RULE_DEPTH))
}

pub fn parse_python(source: &str, version: &str) -> Result<GrammarASTNode, String> {
    let mut parser = create_python_parser(source, version)?;
    parser
        .parse()
        .map_err(|e| format!("Python parse failed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_with_default_version() {
        let ast = parse_python("x = 1\n", "").unwrap();
        assert_eq!(ast.rule_name, "file");
    }

    #[test]
    fn parses_indented_block() {
        let ast = parse_python("def f():\n    return 1\n", "3.12").unwrap();
        assert_eq!(ast.rule_name, "file");
    }

    #[test]
    fn all_supported_versions_load() {
        for version in _grammar::SUPPORTED_VERSIONS {
            let ast = parse_python("", version).unwrap();
            assert!(
                !ast.rule_name.is_empty(),
                "version {version} should produce a root rule"
            );
        }
    }

    #[test]
    fn unsupported_version_returns_error() {
        let error = parse_python("x = 1\n", "4.0").unwrap_err();
        assert!(error.contains("4.0"));
    }

    // -------------------------------------------------------------------
    // Recursion-depth guard (DoS hardening) -- see MAX_RULE_DEPTH's own
    // doc comment for the measurement.
    // -------------------------------------------------------------------

    fn nested_paren_source(n: usize) -> String {
        format!("x = {}1{}\n", "(".repeat(n), ")".repeat(n))
    }

    /// Deeply-nested input must not overflow the native stack on a
    /// default-stack thread -- the whole point of the guard.
    #[test]
    fn test_deeply_nested_input_does_not_overflow_on_default_stack() {
        let src = nested_paren_source(5000);
        let handle = std::thread::spawn(move || {
            let _ = parse_python(&src, "3.12");
        });
        handle
            .join()
            .expect("MAX_RULE_DEPTH must trip BEFORE native overflow on the default stack");
    }

    /// Reasonable, hand-writable nesting stays well under the cap.
    #[test]
    fn test_reasonable_nesting_stays_under_the_cap() {
        assert!(parse_python(&nested_paren_source(9), "3.12").is_ok());
    }
}
