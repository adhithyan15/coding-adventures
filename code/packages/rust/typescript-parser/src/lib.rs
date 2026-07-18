//! TypeScript parser backed by compiled versioned parser grammars (ts1.0 through ts5.8).

use coding_adventures_typescript_lexer::tokenize_typescript;
use parser::grammar_parser::{GrammarASTNode, GrammarParser};

mod _grammar;

/// Recursion-depth cap for the TypeScript [`GrammarParser`] — see
/// [`GrammarParser::with_max_depth`] for why this guard exists at all (deep
/// recursion through `parse_rule` can overflow the *native* thread stack —
/// an uncatchable process abort — before this crate's own callers get a
/// chance to report anything). Before this constant was applied,
/// `create_typescript_parser` never called `with_max_depth` at all,
/// leaving every caller — for *any* of the 6 embedded grammar versions
/// (ts1.0 through ts5.8) `_grammar::parser_grammar(version)` can select —
/// exposed to a native-stack-overflow DoS from adversarial deeply-nested
/// input (e.g. `let x = (((...1...)));`).
///
/// **Not the shared engine's bare default** (see `csharp-parser`'s own
/// identically-named constant for why a blind `DEFAULT_MAX_RULE_DEPTH`
/// (128) is unsafe-for-usability on a rich general-purpose-language
/// grammar). Measured directly instead (binary search over candidate
/// `with_max_depth` values against a fixed 5000-level adversarial
/// `let x = (((...1...)));` input — ordinary parenthesised grouping — on a
/// default-~2MiB-stack worker thread in a debug build, no
/// `RUST_MIN_STACK` override or explicit `Builder::stack_size` present),
/// against the **newest** embedded grammar (`"ts5.8"`, the most feature-complete,
/// expected to have the tightest floor): safe at **275**, crashes at
/// **276**. Spot-checked against the **oldest** embedded grammar
/// (`"ts1.0"`) too, since a single cap must be safe regardless of which
/// version a caller selects at runtime: `"ts1.0"`'s own floor is
/// dramatically higher (still safe at 500, not independently pinned down
/// further) — the much smaller TS 1.0 type-system surface has a
/// correspondingly shorter precedence cascade, so the newest version is
/// confirmed to be the binding one, not merely assumed. The 4 intermediate
/// versions were not independently spot-checked; this is a disclosed scope
/// limitation, not a verified guarantee across all 6.
///
/// `MAX_RULE_DEPTH` is set to **190** — about 31% below the binding 275
/// floor (comparable margin to `apl-parser`'s own ~26.5%, `j-parser`'s
/// ~30%, `reduce-parser`'s ~28.5%). Measured real-input headroom at `190`
/// (against `"ts5.8"`): plain parenthesised nesting parses cleanly to 7
/// levels (8 trips) — thinner than the simpler grammars measured
/// elsewhere in this repo, a direct consequence of how long TypeScript's
/// real type/expression precedence cascade is, but still enough for
/// ordinary hand-written nesting.
///
/// This is measured against only **one** of TypeScript's recursion shapes
/// (ordinary paren grouping) — a full audit would also cover nested
/// arrow functions, nested type literals/generics, nested array/object
/// literals, nested template literal types, and nested function-call
/// arguments, the way `css-parser`/`toml-parser` measured *every* shape in
/// their own (much smaller) grammars. That fuller audit — across all 6
/// embedded versions — is a tracked follow-up; this pass at minimum
/// replaces an unmeasured, silently-broken default with a
/// properly-measured floor for the shape most likely to bind.
const MAX_RULE_DEPTH: usize = 190;

fn validate_version(version: &str) -> Result<&str, String> {
    if _grammar::SUPPORTED_VERSIONS.contains(&version) {
        Ok(version)
    } else {
        Err(format!(
            "Unknown TypeScript version '{version}'. Valid values: {}",
            _grammar::SUPPORTED_VERSIONS
                .iter()
                .map(|value| format!("\"{}\"", value))
                .collect::<Vec<_>>()
                .join(", ")
        ))
    }
}

pub fn create_typescript_parser(source: &str, version: &str) -> Result<GrammarParser, String> {
    let version = validate_version(version)?;
    let tokens = tokenize_typescript(source, version)?;
    let grammar = _grammar::parser_grammar(version)
        .expect("compiled TypeScript parser grammar missing supported version");
    Ok(GrammarParser::new(tokens, grammar).with_max_depth(MAX_RULE_DEPTH))
}

pub fn parse_typescript(source: &str, version: &str) -> Result<GrammarASTNode, String> {
    let mut parser = create_typescript_parser(source, version)?;
    parser
        .parse()
        .map_err(|e| format!("TypeScript parse failed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ts1_0_typescript() {
        let ast = parse_typescript("var x = 1;", "ts1.0").unwrap();
        assert_eq!(ast.rule_name, "program");
    }

    #[test]
    fn parses_versioned_typescript() {
        let ast = parse_typescript("let x = 1;", "ts5.8").unwrap();
        assert_eq!(ast.rule_name, "program");
    }

    #[test]
    fn all_supported_versions_load() {
        for version in _grammar::SUPPORTED_VERSIONS {
            let ast = parse_typescript("", version).unwrap();
            assert_eq!(ast.rule_name, "program", "version {version:?}");
        }
    }

    // -------------------------------------------------------------------
    // Recursion-depth guard (DoS hardening) -- see MAX_RULE_DEPTH's own
    // doc comment for the measurement.
    // -------------------------------------------------------------------

    fn nested_paren_source(n: usize) -> String {
        format!("let x = {}1{};", "(".repeat(n), ")".repeat(n))
    }

    /// Deeply-nested input must not overflow the native stack on a
    /// default-stack thread -- the whole point of the guard.
    #[test]
    fn test_deeply_nested_input_does_not_overflow_on_default_stack() {
        let src = nested_paren_source(5000);
        let handle = std::thread::spawn(move || {
            let _ = parse_typescript(&src, "ts5.8");
        });
        handle
            .join()
            .expect("MAX_RULE_DEPTH must trip BEFORE native overflow on the default stack");
    }

    /// Reasonable, hand-writable nesting stays well under the cap.
    #[test]
    fn test_reasonable_nesting_stays_under_the_cap() {
        assert!(parse_typescript(&nested_paren_source(7), "ts5.8").is_ok());
    }
}
