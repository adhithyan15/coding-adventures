//! Java parser backed by compiled versioned parser grammars.

use coding_adventures_java_lexer::tokenize_java;
use parser::grammar_parser::{GrammarASTNode, GrammarParser};

mod _grammar;

/// Recursion-depth cap for the Java [`GrammarParser`] — see
/// [`GrammarParser::with_max_depth`] for why this guard exists at all (deep
/// recursion through `parse_rule` can overflow the *native* thread stack —
/// an uncatchable process abort — before this crate's own callers get a
/// chance to report anything). Before this constant was applied,
/// `create_java_parser` never called `with_max_depth` at all, leaving
/// every caller exposed to a native-stack-overflow DoS from adversarial
/// deeply-nested input (e.g. `int x = (((...1...)));`).
///
/// **Not the shared engine's bare default** (see `csharp-parser`'s own
/// identically-named constant for why a blind `DEFAULT_MAX_RULE_DEPTH`
/// (128) is unsafe-for-usability on a rich general-purpose-language
/// grammar). Measured directly instead (binary search over candidate
/// `with_max_depth` values against a fixed 5000-level adversarial
/// `(((...1...)))` input — ordinary parenthesised grouping, the shape
/// universally present in every language — on a default-~2MiB-stack
/// worker thread in a debug build, no `RUST_MIN_STACK` override or
/// explicit `Builder::stack_size` present): safe at **264**, crashes at
/// **265**.
///
/// `MAX_RULE_DEPTH` is set to **180** — about 32% below that floor
/// (comparable margin to `apl-parser`'s own ~26.5%, `j-parser`'s ~30%,
/// `reduce-parser`'s ~28.5%). Measured real-input headroom at `180`: plain
/// parenthesised nesting parses cleanly to at least 10 levels — comfortably
/// beyond ordinary hand-written nesting depth.
///
/// This is measured against only **one** of Java's recursion shapes
/// (ordinary paren grouping) — a full audit would also cover nested
/// lambdas, nested array/collection initializers, nested method-call
/// arguments, and nested class/block bodies, the way `css-parser`/
/// `toml-parser` measured *every* shape in their own (much smaller)
/// grammars. That fuller audit is a tracked follow-up; this pass at
/// minimum replaces an unmeasured, silently-broken default with a
/// properly-measured floor for the shape most likely to bind.
const MAX_RULE_DEPTH: usize = 180;

fn validate_version(version: &str) -> Result<&str, String> {
    if _grammar::SUPPORTED_VERSIONS.contains(&version) {
        Ok(version)
    } else {
        Err(format!(
            "Unknown Java version '{version}'. Valid values: {}",
            _grammar::SUPPORTED_VERSIONS
                .iter()
                .map(|value| format!("\"{}\"", value))
                .collect::<Vec<_>>()
                .join(", ")
        ))
    }
}

pub fn create_java_parser(source: &str, version: &str) -> Result<GrammarParser, String> {
    let version = validate_version(version)?;
    let tokens = tokenize_java(source, version)?;
    let grammar = _grammar::parser_grammar(version)
        .expect("compiled Java parser grammar missing supported version");
    Ok(GrammarParser::new(tokens, grammar).with_max_depth(MAX_RULE_DEPTH))
}

pub fn parse_java(source: &str, version: &str) -> Result<GrammarASTNode, String> {
    let mut parser = create_java_parser(source, version)?;
    parser
        .parse()
        .map_err(|e| format!("Java parse failed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_class() {
        let ast = parse_java("class Hello { }", "21").unwrap();
        assert_eq!(ast.rule_name, "program");
    }

    #[test]
    fn all_supported_versions_load() {
        for version in _grammar::SUPPORTED_VERSIONS {
            let ast = parse_java("", version).unwrap();
            assert_eq!(ast.rule_name, "program", "version {version}");
        }
    }

    #[test]
    fn unknown_version_returns_error() {
        let error = parse_java("class Hello { }", "99").unwrap_err();
        assert!(error.contains("99"));
    }

    // -------------------------------------------------------------------
    // Recursion-depth guard (DoS hardening, interim DEFAULT_MAX_RULE_DEPTH
    // pass -- see MAX_RULE_DEPTH's own doc comment).
    // -------------------------------------------------------------------

    fn nested_paren_source(n: usize) -> String {
        format!(
            "class C {{ void m() {{ int x = {}1{}; }} }}",
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
            let _ = parse_java(&src, "21");
        });
        handle
            .join()
            .expect("MAX_RULE_DEPTH must trip BEFORE native overflow on the default stack");
    }

    /// Reasonable, hand-writable nesting stays well under the cap.
    #[test]
    fn test_reasonable_nesting_stays_under_the_cap() {
        assert!(parse_java(&nested_paren_source(10), "21").is_ok());
    }

    // -------------------------------------------------------------------
    // Real construct coverage (JV02 M0). `parses_basic_class` above only
    // ever checked `ast.rule_name == "program"` on a bare `class Hello {
    // }` -- it proves the parser didn't error, not that any interesting
    // construct actually produced the AST shape a future lowering pass
    // will need to match on. These tests use the engine's own
    // `find_nodes`/`collect_tokens` walkers (parser::grammar_parser) to
    // assert a named rule genuinely appears in the parsed tree, for every
    // construct this frontend's own milestones (JV02 M1-M9) will consume.
    // -------------------------------------------------------------------

    use parser::grammar_parser::find_nodes;

    #[test]
    fn interface_with_generic_type_param_and_method_signature() {
        let ast = parse_java("interface Shape<T> { T area(); }", "21").unwrap();
        assert!(
            !find_nodes(&ast, "interface_declaration").is_empty(),
            "expected an interface_declaration node"
        );
        assert!(
            !find_nodes(&ast, "type_parameters").is_empty(),
            "expected a type_parameters node for <T>"
        );
        assert!(
            !find_nodes(&ast, "interface_method_declaration").is_empty()
                || !find_nodes(&ast, "method_declaration").is_empty(),
            "expected a method-declaration-shaped node for `T area();`"
        );
    }

    #[test]
    fn class_with_extends_and_implements() {
        let ast =
            parse_java("class Dog extends Animal implements Comparable { }", "21").unwrap();
        assert!(!find_nodes(&ast, "class_declaration").is_empty());
        let tokens = parser::grammar_parser::collect_tokens(&ast, None);
        let values: Vec<&str> = tokens.iter().map(|t| t.value.as_str()).collect();
        assert!(values.contains(&"extends"), "expected `extends` keyword");
        assert!(
            values.contains(&"implements"),
            "expected `implements` keyword"
        );
    }

    #[test]
    fn generic_class_declaration_with_field() {
        let ast = parse_java("class Box<T> { T value; }", "21").unwrap();
        assert!(!find_nodes(&ast, "class_declaration").is_empty());
        assert!(
            !find_nodes(&ast, "type_parameters").is_empty(),
            "expected a type_parameters node for <T>"
        );
        assert!(!find_nodes(&ast, "field_declaration").is_empty());
    }

    #[test]
    fn lambda_expression_assigned_to_a_variable() {
        let ast = parse_java(
            "class C { void m() { Runnable r = () -> System.out.println(1); } }",
            "21",
        )
        .unwrap();
        assert!(
            !find_nodes(&ast, "lambda_expression").is_empty(),
            "expected a lambda_expression node"
        );
    }

    #[test]
    fn try_catch_finally_with_throws_clause() {
        let ast = parse_java(
            "class C { void m() throws java.io.IOException { try { m(); } catch (java.io.IOException e) { } finally { } } }",
            "21",
        )
        .unwrap();
        assert!(!find_nodes(&ast, "try_statement").is_empty());
        assert!(!find_nodes(&ast, "catch_clause").is_empty());
        assert!(
            !find_nodes(&ast, "throws_clause").is_empty(),
            "expected a throws_clause node on the method signature"
        );
    }

    #[test]
    fn annotation_on_a_method_declaration() {
        let ast = parse_java(
            "class C { @Override public String toString() { return \"\"; } }",
            "21",
        )
        .unwrap();
        assert!(
            !find_nodes(&ast, "annotation").is_empty(),
            "expected an annotation node for @Override"
        );
        assert!(!find_nodes(&ast, "method_declaration").is_empty());
    }

    #[test]
    fn enum_declaration_with_constants() {
        let ast = parse_java("enum Color { RED, GREEN, BLUE }", "21").unwrap();
        assert!(
            !find_nodes(&ast, "enum_declaration").is_empty(),
            "expected an enum_declaration node"
        );
    }

    #[test]
    fn varargs_and_method_reference() {
        let ast = parse_java(
            "class C { void f(int... xs) { java.util.Arrays.stream(xs).forEach(System.out::println); } }",
            "21",
        )
        .unwrap();
        // Real construct, no crash / parse error is the load-bearing
        // assertion here -- `unwrap()` above already enforces that.
        assert!(!find_nodes(&ast, "method_declaration").is_empty());
    }

    // -------------------------------------------------------------------
    // Nested-generics `>>`/`>>>` token-splitting (shared `parser` crate
    // engine, see `parser::grammar_parser::split_angle_bracket_run`).
    // The lexer merges consecutive `>` characters into a single
    // `RIGHT_SHIFT`/`UNSIGNED_RIGHT_SHIFT`-typed token (same shape as a
    // real shift operator -- see `coding_adventures_java_lexer`'s own
    // `nested_generic_closing_angle_brackets_merge_into_one_token`
    // test), and the parser now contextually re-splits it into separate
    // `GREATER_THAN` closers whenever it specifically expects one, one
    // `>` at a time.
    // -------------------------------------------------------------------

    #[test]
    fn two_level_nested_generic_closes_from_a_merged_right_shift_token() {
        let ast = parse_java(
            "class C { void f() { Map<String, List<Integer>> m; } }",
            "21",
        )
        .unwrap();
        // Two distinct `type_arguments` nodes: the outer `Map<...>` and
        // the inner `List<Integer>`.
        assert_eq!(find_nodes(&ast, "type_arguments").len(), 2);
    }

    #[test]
    fn three_level_nested_generic_closes_from_a_merged_unsigned_right_shift_token() {
        let ast = parse_java(
            "class C { void f() { Box<Box<Box<Integer>>> b; } }",
            "21",
        )
        .unwrap();
        assert_eq!(find_nodes(&ast, "type_arguments").len(), 3);
    }

    #[test]
    fn real_right_shift_expression_still_parses_as_a_shift_after_the_splitting_fix() {
        // `x >> 2` must still tokenize/parse as an actual shift operator,
        // not be mistaken for a stray generic closer -- the split only
        // ever fires when the grammar specifically expects a bare
        // `GREATER_THAN`, which a shift-expression's own right-hand side
        // never does.
        let ast = parse_java("class C { void f() { int y = x >> 2; } }", "21").unwrap();
        let tokens = parser::grammar_parser::collect_tokens(&ast, None);
        assert!(
            tokens.iter().any(|t| t.value == ">>"),
            "expected the real `>>` shift operator to survive as a single token"
        );
    }

    #[test]
    fn nested_generic_and_real_shift_expression_coexist_in_one_file() {
        // Regression guard for the packrat-memoization invalidation this
        // fix also needed: a real shift expression parsed earlier in the
        // same file must not leave a stale memo entry that corrupts a
        // later nested-generic split, and vice versa.
        let ast = parse_java(
            "class C { void f() { int y = x >> 2; Map<String, List<Integer>> m; int z = x >>> 1; } }",
            "21",
        )
        .unwrap();
        assert_eq!(find_nodes(&ast, "type_arguments").len(), 2);
        let tokens = parser::grammar_parser::collect_tokens(&ast, None);
        let shift_tokens: Vec<&str> = tokens
            .iter()
            .filter(|t| t.value == ">>" || t.value == ">>>")
            .map(|t| t.value.as_str())
            .collect();
        assert_eq!(shift_tokens, vec![">>", ">>>"]);
    }

    /// Regression guard for a review finding on the fix above: the first
    /// version of the memo-invalidation logic did a full `self.memo
    /// .clear()` on every split, which -- since the memo table grows with
    /// how much of the file has already been parsed -- turns *ordinary*
    /// Java source with many scattered (non-adversarial, idiomatic)
    /// nested-generic field declarations into O(fileLength^2) parsing.
    /// The fix tightened this to `retain`, dropping only entries whose
    /// recorded `end_pos` actually reached the mutated position. This
    /// test doesn't assert a specific time bound (flaky on shared CI
    /// hardware) -- it asserts the parse of a few hundred scattered
    /// occurrences still completes and is correct, and stands as the
    /// place a future reader would drop a `#[bench]`/timing check if this
    /// ever regresses back to a full clear.
    #[test]
    fn many_scattered_nested_generics_in_one_large_file_all_parse_correctly() {
        let mut body = String::new();
        for i in 0..300 {
            body.push_str(&format!("Map<String, List<Integer>> field{i};\n"));
        }
        let src = format!("class C {{ {body} }}");
        let ast = parse_java(&src, "21").unwrap();
        assert_eq!(find_nodes(&ast, "type_arguments").len(), 600);
    }
}
