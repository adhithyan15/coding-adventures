//! JSDoc parser.
//!
//! Parses the **interior** of a `/** ... */` block comment into a tree of
//! tags and type expressions per CLOC05 §"`jsdoc.grammar` outline." The
//! enclosing markers are expected to be stripped by an upstream
//! comment-extractor stage (deferred follow-up).
//!
//! The parser is grammar-driven: the actual rules live in
//! [`code/grammars/jsdoc/jsdoc.grammar`](../../../grammars/jsdoc/jsdoc.grammar),
//! compiled to native Rust at build time via `grammar-tools compile-grammar`
//! and embedded as `mod _grammar`.

use coding_adventures_jsdoc_lexer::tokenize_jsdoc;
use parser::grammar_parser::{GrammarASTNode, GrammarParser};

mod _grammar;

/// Recursion-depth cap for the JSDoc [`GrammarParser`] — see
/// [`GrammarParser::with_max_depth`] and
/// [`parser::grammar_parser::DEFAULT_MAX_RULE_DEPTH`] for why the underlying
/// guard exists at all (deep recursion through `parse_rule` can overflow the
/// *native* thread stack — an uncatchable process abort — before this
/// crate's own callers ever get a chance to report anything). Before this
/// constant was added, `create_jsdoc_parser` never called `with_max_depth`
/// at all, leaving every caller exposed to a native-stack-overflow DoS from
/// an adversarial `@type {(((...)))}` payload.
///
/// # One recursion shape — and a graceful-degradation surprise
///
/// `jsdoc.grammar` has exactly one self-referential production: nested
/// parenthesised type expressions, `@type {(((Foo)))}` — `type ->
/// primary_type -> parenthesized_type -> type -> …`. Measured directly:
/// binary search over candidate `with_max_depth` values against a fixed
/// 5000-level adversarial input, on a default-~2MiB-stack worker thread in
/// a debug build. Safe at **289**, crashes at **290**.
///
/// `MAX_RULE_DEPTH` is set to **200** — about 31% below the 289 floor
/// (comparable margin to `apl-parser`'s own ~26.5%, `j-parser`'s ~30%,
/// `reduce-parser`'s ~28.5%).
///
/// The surprise: unlike every other `*-parser` crate audited alongside this
/// one, exceeding the cap here does **not** produce a parse *error*.
/// `tag = type_tag | param_tag | returns_tag | unknown_tag` — and
/// `unknown_tag = AT_TAG { tag_payload_token } NEWLINE` is a deliberate,
/// non-recursive catch-all (documented in `jsdoc.grammar`'s own header:
/// "unknown tags survive and round-trip") that sweeps up *any* sequence of
/// tokens up to the next `NEWLINE`, parens included. So once the depth cap
/// refuses `type_tag`'s attempt at a properly-nested `type_expression`,
/// the PEG ordered-choice in `tag` does not fail outright — it falls
/// through to `unknown_tag`, which happily accepts the same (still
/// well-formed, just too-deep) token sequence as an opaque, unstructured
/// payload. The overall parse still succeeds (`Ok`), just with a
/// *different* — degraded but harmless — tree shape than the "real" nested
/// type expression would have produced. This is arguably a *better*
/// outcome than a hard error (graceful degradation instead of rejection),
/// but it means this crate's own depth-guard regression tests assert "no
/// crash" rather than "returns `Err`", unlike every sibling `*-parser`
/// crate's identically-named tests.
///
/// Measured real-input headroom at `200` (using the CAPPED parser, so no
/// crash risk at all, and checking the resulting tree shape rather than
/// just success/failure): parenthesised-type nesting parses as a *genuine*
/// `parenthesized_type` node up to 48 levels; at 49 levels the tree
/// contains `unknown_tag` instead — comfortably beyond anything a
/// hand-written JSDoc comment needs, and confirmed not to crash a
/// default-stack thread even thousands of levels past the cap (see this
/// crate's tests).
const MAX_RULE_DEPTH: usize = 200;

/// Construct a JSDoc [`GrammarParser`] over `source`. The caller is
/// responsible for stripping the surrounding `/** */` markers; see the
/// crate-level docs.
pub fn create_jsdoc_parser(source: &str) -> GrammarParser {
    let tokens = tokenize_jsdoc(source);
    let grammar = _grammar::parser_grammar();
    GrammarParser::new(tokens, grammar).with_max_depth(MAX_RULE_DEPTH)
}

/// Parse `source` into a [`GrammarASTNode`] rooted at the `document`
/// rule. Returns `Err` on parse failure.
pub fn parse_jsdoc(source: &str) -> Result<GrammarASTNode, String> {
    let mut parser = create_jsdoc_parser(source);
    parser
        .parse()
        .map_err(|e| format!("JSDoc parse failed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_empty_document() {
        let ast = parse_jsdoc("").unwrap();
        assert_eq!(ast.rule_name, "document");
    }

    #[test]
    fn parses_type_tag() {
        let ast = parse_jsdoc("@type {number}\n").unwrap();
        assert_eq!(ast.rule_name, "document");
        // The tree should mention the AT_TAG token's text somewhere.
        let dumped = format!("{:?}", ast);
        assert!(dumped.contains("@type"));
    }

    #[test]
    fn parses_returns_tag() {
        let ast = parse_jsdoc("@returns {string}\n").unwrap();
        assert_eq!(ast.rule_name, "document");
    }

    #[test]
    fn parses_param_tag_with_name() {
        let ast = parse_jsdoc("@param {string} name\n").unwrap();
        assert_eq!(ast.rule_name, "document");
        let dumped = format!("{:?}", ast);
        assert!(dumped.contains("@param"));
        assert!(dumped.contains("name"));
    }

    #[test]
    fn parses_multiple_tags() {
        let src = "@param {string} name\n@returns {boolean}\n";
        let ast = parse_jsdoc(src).unwrap();
        assert_eq!(ast.rule_name, "document");
        let dumped = format!("{:?}", ast);
        assert!(dumped.contains("@param"));
        assert!(dumped.contains("@returns"));
    }

    #[test]
    fn parses_nullable_type() {
        let ast = parse_jsdoc("@type {?Foo}\n").unwrap();
        assert_eq!(ast.rule_name, "document");
    }

    #[test]
    fn parses_array_type() {
        let ast = parse_jsdoc("@type {string[]}\n").unwrap();
        assert_eq!(ast.rule_name, "document");
    }

    #[test]
    fn parses_dotted_nominal_type() {
        let ast = parse_jsdoc("@type {Foo.Bar.Baz}\n").unwrap();
        assert_eq!(ast.rule_name, "document");
    }

    #[test]
    fn unknown_tag_is_tolerated() {
        // @throws isn't in the v1 named-tag list, but the unknown_tag
        // rule should sweep it up so the parse still succeeds.
        let ast = parse_jsdoc("@throws {Error} when bad\n").unwrap();
        assert_eq!(ast.rule_name, "document");
        let dumped = format!("{:?}", ast);
        assert!(dumped.contains("@throws"));
    }

    #[test]
    fn create_jsdoc_parser_returns_working_parser() {
        let mut parser = create_jsdoc_parser("@type {string}\n");
        let ast = parser.parse().expect("parse should succeed");
        assert_eq!(ast.rule_name, "document");
    }

    // -----------------------------------------------------------------------
    // Recursion-depth guard (DoS hardening) -- see `MAX_RULE_DEPTH`'s own
    // doc comment for why these assert "does not crash" rather than
    // "returns Err", unlike every sibling `*-parser` crate's identically
    // named tests: `unknown_tag`'s deliberate catch-all fallback means an
    // over-cap `@type {(((...)))}` still parses successfully overall (as
    // an unstructured unknown tag), just with a different tree shape.
    // -----------------------------------------------------------------------

    fn nested_paren_type_source(n: usize) -> String {
        let mut s = String::from("@type {");
        for _ in 0..n {
            s.push('(');
        }
        s.push_str("Foo");
        for _ in 0..n {
            s.push(')');
        }
        s.push_str("}\n");
        s
    }

    /// Deeply-nested input must not overflow the native stack, regardless
    /// of whether the eventual parse result is `Ok` (via the `unknown_tag`
    /// fallback) or `Err`. Parses 5000 levels -- far past `MAX_RULE_DEPTH`
    /// -- on a worker thread with a generous 32 MiB stack, so the *guard*
    /// is what stops the recursion, not the stack running out.
    #[test]
    fn test_deeply_nested_input_does_not_overflow() {
        let src = nested_paren_type_source(5000);
        let handle = std::thread::Builder::new()
            .name("jsdoc-parser-depth-guard-regression".to_string())
            .stack_size(32 * 1024 * 1024)
            .spawn(move || {
                let _ = create_jsdoc_parser(&src).parse();
            })
            .expect("failed to spawn worker thread");
        handle
            .join()
            .expect("depth guard must keep the worker thread from crashing");
    }

    /// A caller relying on `MAX_RULE_DEPTH` must have the guard trip
    /// *before* the native stack overflows on a default-stack thread --
    /// otherwise a production caller (or `cargo test`'s own per-test
    /// thread) would still crash. Parses far-too-deep input on a worker
    /// thread with **no** `stack_size` override (the same ~2 MiB a default
    /// thread gets).
    #[test]
    fn test_cap_trips_before_overflow_on_default_stack() {
        let src = nested_paren_type_source(5000);
        let handle = std::thread::spawn(move || {
            let _ = create_jsdoc_parser(&src).parse();
        });
        handle
            .join()
            .expect("MAX_RULE_DEPTH must trip BEFORE native overflow on the default stack");
    }

    /// Reasonable, hand-writable nesting stays well under the cap and
    /// still produces a genuine `parenthesized_type` node, not the
    /// `unknown_tag` fallback.
    #[test]
    fn test_reasonable_nesting_stays_under_the_cap() {
        let ast = create_jsdoc_parser(&nested_paren_type_source(10))
            .parse()
            .expect("reasonable nesting should parse");

        fn contains_rule(node: &GrammarASTNode, name: &str) -> bool {
            use parser::grammar_parser::ASTNodeOrToken;
            node.rule_name == name
                || node.children.iter().any(|c| match c {
                    ASTNodeOrToken::Node(n) => contains_rule(n, name),
                    ASTNodeOrToken::Token(_) => false,
                })
        }
        assert!(contains_rule(&ast, "parenthesized_type"));
    }
}
