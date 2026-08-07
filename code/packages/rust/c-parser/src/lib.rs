//! # C parser — parsing the C integer-core subset (SIR27).
//!
//! The syntactic layer of the `c-to-semantic-ir` frontend
//! ([SIR27](../../../specs/SIR27-c-to-semantic-ir.md)).  It tokenizes with
//! [`coding_adventures_c_lexer`] and feeds the tokens to the generic
//! [`GrammarParser`] driving the compiled `c.grammar`.
//!
//! ```text
//! C source
//!    │  c_lexer::tokenize_c
//!    ▼
//! Vec<Token>
//!    │  parser::GrammarParser (c.grammar → CST)
//!    ▼
//! GrammarASTNode  { rule_name, children }
//! ```
//!
//! The result is the generic, uniform parse tree
//! [`parser::grammar_parser::GrammarASTNode`] — a consumer walks it by
//! `rule_name` (the lowering pass in `c-to-semantic-ir` does exactly that).
//! The root node's `rule_name` is `"translation_unit"`.

use coding_adventures_c_lexer::{tokenize_c, try_tokenize_c};
use parser::grammar_parser::{GrammarASTNode, GrammarParser, DEFAULT_MAX_RULE_DEPTH};

mod _grammar;

/// Recursion-depth cap for the C [`GrammarParser`] — see
/// [`GrammarParser::with_max_depth`] and [`DEFAULT_MAX_RULE_DEPTH`] for why
/// this guard exists at all (deep recursion through `parse_rule` can
/// overflow the *native* thread stack — an uncatchable process abort —
/// before this crate's own callers get a chance to report anything).
/// Before this constant was applied, neither `create_c_parser` nor
/// `try_parse_c` ever called `with_max_depth`, leaving every caller exposed
/// to a native-stack-overflow DoS from adversarial deeply-nested input
/// (e.g. `((((...1...))))`).
///
/// This is an interim, broad safety net: the shared engine's own
/// [`DEFAULT_MAX_RULE_DEPTH`] (128), not yet a bespoke value derived from
/// measuring `c.grammar`'s own specific recursion shapes the way
/// `css-parser`/`toml-parser`/`jsdoc-parser`/`reduce-parser` each did
/// (binary search over `with_max_depth` against a 5000-deep adversarial
/// input per shape, picking a value ~30% below the lowest floor found).
/// 128 is used here because every grammar measured that way in this repo
/// so far (7 crates, 12+ distinct shapes) has its *lowest* floor
/// comfortably above it — the tightest found being `reduce-parser`'s own
/// cons-chain shape at 179 (128 is ~28% below that) — so it is a
/// reasonable, evidence-backed interim value pending the full bespoke
/// measurement for this crate's own grammar (tracked as a follow-up).
const MAX_RULE_DEPTH: usize = DEFAULT_MAX_RULE_DEPTH;

/// Create a [`GrammarParser`] wired to the C grammar and tokens.  Ready to
/// call `.parse()`.
pub fn create_c_parser(source: &str) -> GrammarParser {
    let tokens = tokenize_c(source);
    GrammarParser::new(tokens, _grammar::parser_grammar()).with_max_depth(MAX_RULE_DEPTH)
}

/// Parse C `source` into a [`GrammarASTNode`] CST rooted at
/// `"translation_unit"`.  Panics on a parse error; use [`try_parse_c`] for the
/// fallible form.
pub fn parse_c(source: &str) -> GrammarASTNode {
    try_parse_c(source).unwrap_or_else(|e| panic!("C parse failed: {e}"))
}

/// Parse C `source`, returning a human-readable error string on failure.  The
/// truly fallible path — a lexical error becomes an `Err`, not a panic (so it
/// routes through [`try_tokenize_c`], not the panicking `tokenize_c` that
/// `create_c_parser` uses).
pub fn try_parse_c(source: &str) -> Result<GrammarASTNode, String> {
    let tokens = try_tokenize_c(source)?;
    GrammarParser::new(tokens, _grammar::parser_grammar())
        .with_max_depth(MAX_RULE_DEPTH)
        .parse()
        .map_err(|e| format!("{e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use parser::grammar_parser::ASTNodeOrToken;

    fn root(src: &str) -> GrammarASTNode {
        parse_c(src)
    }

    /// Does the tree contain a node with this `rule_name` anywhere?
    fn has_rule(node: &GrammarASTNode, target: &str) -> bool {
        if node.rule_name == target {
            return true;
        }
        node.children.iter().any(|c| match c {
            ASTNodeOrToken::Node(n) => has_rule(n, target),
            ASTNodeOrToken::Token(_) => false,
        })
    }

    fn count_rule(node: &GrammarASTNode, target: &str) -> usize {
        let here = usize::from(node.rule_name == target);
        here + node
            .children
            .iter()
            .map(|c| match c {
                ASTNodeOrToken::Node(n) => count_rule(n, target),
                ASTNodeOrToken::Token(_) => 0,
            })
            .sum::<usize>()
    }

    #[test]
    fn empty_main_parses() {
        let ast = root("int main(void) { }");
        assert_eq!(ast.rule_name, "translation_unit");
        assert!(has_rule(&ast, "function_def"));
    }

    #[test]
    fn declaration_with_init() {
        let ast = root("int main(void) { int32_t x = 5; return x; }");
        assert!(has_rule(&ast, "declaration"));
        assert!(has_rule(&ast, "return_stmt"));
    }

    #[test]
    fn arithmetic_precedence_is_structural() {
        // 2 + 3 * 4 must nest the multiply under the add (mult is tighter).
        let ast = root("int main(void) { return 2 + 3 * 4; }");
        assert!(has_rule(&ast, "additive"));
        assert!(has_rule(&ast, "multiplicative"));
    }

    #[test]
    fn cast_is_recognized() {
        // (uint8_t)x is a cast, not a parenthesised expression.
        let ast = root("int main(void) { uint8_t c = (uint8_t)300; return c; }");
        assert!(has_rule(&ast, "cast"), "cast not found:\n{ast:#?}");
    }

    #[test]
    fn control_flow_and_calls() {
        let src = "int f(int x) { if (x > 0) { return 1; } else { return 0; } }\n\
                   int main(void) { int i = 0; while (i < 10) { i = i + 1; } printf(\"%d\\n\", f(i)); return 0; }";
        let ast = root(src);
        assert_eq!(count_rule(&ast, "function_def"), 2);
        assert!(has_rule(&ast, "if_stmt"));
        assert!(has_rule(&ast, "while_stmt"));
        assert!(has_rule(&ast, "call_suffix")); // printf(...) and f(i)
    }

    #[test]
    fn for_loop_with_declaration_init() {
        let ast = root("int main(void) { for (int i = 0; i < 3; i = i + 1) { } return 0; }");
        assert!(has_rule(&ast, "for_stmt"));
        assert!(has_rule(&ast, "init_declarator"));
    }

    #[test]
    fn multiword_types_and_params() {
        let ast = root("unsigned long g(unsigned int a, long long b) { return a; }");
        assert!(has_rule(&ast, "function_def"));
        assert!(has_rule(&ast, "param"));
    }

    #[test]
    fn double_declaration_with_float_literal_parses() {
        // `double` is a type keyword and `3.14` a FLOAT_LIT primary — both new
        // in the float grammar slice.
        let ast = root("double area(double r) { double pi = 3.14; return pi; }");
        assert!(has_rule(&ast, "function_def"));
        assert!(has_rule(&ast, "declaration"));
    }

    #[test]
    fn array_declaration_with_brace_initializer_parses() {
        // `int a[3] = {1, 2, 3};` — an array dimension and a brace initializer
        // list, both new in the arrays grammar slice.
        let ast = root("int main(void) { int a[3] = {1, 2, 3}; return a[0]; }");
        assert!(has_rule(&ast, "init_declarator"));
        assert!(has_rule(&ast, "init_list"), "no init_list:\n{ast:#?}");
        assert!(has_rule(&ast, "index_suffix"), "no index_suffix:\n{ast:#?}");
    }

    #[test]
    fn array_sized_from_initializer_and_indexed_assignment_parse() {
        // `int a[] = {…}` (size inferred) and `a[i] = v` (indexed write).
        let ast = root("int main(void) { int a[] = {5, 6}; a[1] = 9; return a[1]; }");
        assert!(has_rule(&ast, "init_list"));
        assert_eq!(count_rule(&ast, "index_suffix"), 2, "a[1]=… and return a[1]");
    }

    // -------------------------------------------------------------------
    // Recursion-depth guard (DoS hardening, interim DEFAULT_MAX_RULE_DEPTH
    // pass -- see MAX_RULE_DEPTH's own doc comment).
    // -------------------------------------------------------------------

    fn nested_paren_source(n: usize) -> String {
        format!(
            "int main(void) {{ return {}1{}; }}",
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
            let _ = try_parse_c(&src);
        });
        handle
            .join()
            .expect("MAX_RULE_DEPTH must trip BEFORE native overflow on the default stack");
    }

    /// Reasonable, hand-writable nesting stays well under the cap.
    #[test]
    fn test_reasonable_nesting_stays_under_the_cap() {
        assert!(try_parse_c(&nested_paren_source(10)).is_ok());
    }
}
