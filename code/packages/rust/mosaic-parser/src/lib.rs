//! # mosaic-parser — Parsing `.mosaic` source text into an AST.
//!
//! This crate is the second stage in the Mosaic compiler pipeline. Where
//! `mosaic-lexer` breaks source text into tokens, this crate arranges those
//! tokens into a grammar-driven AST that reflects the **structure** of the
//! Mosaic source — components, slots, node trees, property assignments, and
//! control-flow blocks.
//!
//! # The parsing pipeline
//!
//! ```text
//! Source text
//!       |
//!       v
//! mosaic-lexer          → Vec<Token>
//!       |
//!       v
//! mosaic.grammar        → ParserGrammar (rules: file, component_decl, slot_decl, …)
//!       |
//!       v
//! GrammarParser         → GrammarASTNode (rule_name = "file")
//! ```
//!
//! # Grammar rules (from mosaic.grammar)
//!
//! - `file` — `{ import_decl } component_decl`
//! - `import_decl` — `import NAME ["as" NAME] from STRING;`
//! - `component_decl` — `component NAME { { slot_decl } node_tree }`
//! - `slot_decl` — `slot NAME : slot_type [ = default_value ] ;`
//! - `slot_type` — `KEYWORD | NAME | list_type`
//! - `list_type` — `list < slot_type >`
//! - `node_element` — `NAME { { node_content } }`
//! - `node_content` — `property_assignment | child_node | slot_reference | when_block | each_block`
//! - `when_block` — `when @NAME { { node_content } }`
//! - `each_block` — `each @NAME as NAME { { node_content } }`

use mosaic_lexer::tokenize;
use parser::grammar_parser::{GrammarASTNode, GrammarParser};
mod _grammar;

// ===========================================================================
// Public API
// ===========================================================================

/// Recursion-depth cap for the Mosaic [`GrammarParser`] — see
/// [`GrammarParser::with_max_depth`] and
/// [`parser::grammar_parser::DEFAULT_MAX_RULE_DEPTH`] for why the underlying
/// guard exists at all (deep recursion through `parse_rule` can overflow the
/// *native* thread stack — an uncatchable process abort — before this
/// crate's own error-reporting paths ever get a chance to run).
/// `mosaic-parser` is reachable via the `mosaic` CLI on arbitrary `.mosaic`
/// files, a real attack surface.
///
/// # Four independent recursive shapes
///
/// This grammar has four *independent* recursion paths that must all be
/// measured, since a single `MAX_RULE_DEPTH` bounds the parser's internal
/// rule-invocation counter for any of them:
///
/// - **Element-tree nesting** — `node_element -> node_content -> child_node
///   -> node_element` (3 rule-frames per real nesting level).
/// - **`when`-block nesting** — `node_content -> when_block -> node_content`
///   (2 rule-frames per real nesting level).
/// - **`each`-block nesting** — `node_content -> each_block -> node_content`
///   (2 rule-frames per real nesting level).
/// - **List-type nesting** — `slot_type -> list_type -> slot_type` (2
///   rule-frames per real nesting level).
///
/// Measured (binary search, uncapped parser, on the true default-stack
/// per-test worker thread — no `RUST_MIN_STACK` override and no explicit
/// `Builder::stack_size`, matching what `cargo test` and a production
/// caller both actually get — debug build, adversarial 5000-level input):
/// element-tree safe through 288 rule-frames, crashes at 290; list-type
/// safe through 288, crashes at 290; `when`/`each`-block nesting (the
/// *binding*, lower floor) safe through 248, crashes at 249.
///
/// `MAX_RULE_DEPTH` is set to **170** — about 31% below the binding
/// 248-rule-frame floor (comparable margin to sibling crates' 25-45%
/// convention), independently confirmed not to crash a default-stack
/// thread even thousands of rule-frames past the cap for any of the four
/// shapes (see this crate's tests). Measured real-nesting headroom at 170
/// (capped parser, so no crash risk): element-tree nesting parses cleanly
/// up to 56 levels (57 trips the cap), `when`/`each`-block nesting up to 82
/// levels (83 trips the cap), list-type nesting up to 83 levels (84 trips
/// the cap) — comfortably past any hand-written Mosaic component's real
/// nesting.
const MAX_RULE_DEPTH: usize = 170;

/// Create a `GrammarParser` configured for Mosaic source text.
///
/// This function:
/// 1. Tokenizes `source` using `mosaic-lexer`.
/// 2. Reads and parses the `mosaic.grammar` file.
/// 3. Constructs a `GrammarParser` wired to those tokens and rules, with the
///    recursion-depth guard ([`MAX_RULE_DEPTH`]) enabled so pathologically
///    deep nesting fails cleanly instead of overflowing the native stack.
///
/// The returned parser is ready to call `.parse()` on. Use this for
/// custom error handling or incremental analysis.
///
/// # Panics
///
/// Panics if the grammar file is missing/invalid, or if tokenization fails.
pub fn create_mosaic_parser(source: &str) -> GrammarParser {
    let tokens = tokenize(source);
    let grammar = _grammar::parser_grammar();
    GrammarParser::new(tokens, grammar).with_max_depth(MAX_RULE_DEPTH)
}

/// Parse Mosaic source text into an AST.
///
/// The returned `GrammarASTNode` has `rule_name = "file"` and contains the
/// full structure of the Mosaic source: imports, the component declaration,
/// slot declarations, and the node tree.
///
/// # Panics
///
/// Panics if tokenization or parsing fails.
///
/// # Example
///
/// ```no_run
/// use mosaic_parser::parse;
///
/// let ast = parse(r#"
///   component Label {
///     slot text: text;
///     Text { content: @text; }
///   }
/// "#);
/// assert_eq!(ast.rule_name, "file");
/// ```
pub fn parse(source: &str) -> GrammarASTNode {
    let mut p = create_mosaic_parser(source);
    p.parse()
        .unwrap_or_else(|e| panic!("Mosaic parse failed: {e}"))
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use parser::grammar_parser::ASTNodeOrToken;

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    /// Verify the root rule is "file".
    fn assert_file_root(ast: &GrammarASTNode) {
        assert_eq!(
            ast.rule_name, "file",
            "Expected root rule 'file', got '{}'",
            ast.rule_name
        );
    }

    /// Recursively search the AST for a node with the given rule name.
    fn find_rule(node: &GrammarASTNode, target: &str) -> bool {
        if node.rule_name == target {
            return true;
        }
        for child in &node.children {
            if let ASTNodeOrToken::Node(n) = child {
                if find_rule(n, target) {
                    return true;
                }
            }
        }
        false
    }

    // -----------------------------------------------------------------------
    // Test 1: Minimal component — no slots, simple node
    // -----------------------------------------------------------------------

    /// The simplest possible Mosaic file: one component with no slots.
    #[test]
    fn test_parse_minimal_component() {
        let src = r#"component Empty { Box { } }"#;
        let ast = parse(src);
        assert_file_root(&ast);
        assert!(find_rule(&ast, "component_decl"), "Expected component_decl");
    }

    // -----------------------------------------------------------------------
    // Test 2: Component with a single text slot
    // -----------------------------------------------------------------------
    // NOTE: Slot names must not be reserved keywords. Use "title" instead of "text".

    #[test]
    fn test_parse_single_slot() {
        let src = r#"
          component Label {
            slot title: text;
            Text { content: @title; }
          }
        "#;
        let ast = parse(src);
        assert_file_root(&ast);
        assert!(find_rule(&ast, "slot_decl"), "Expected slot_decl");
    }

    // -----------------------------------------------------------------------
    // Test 3: Multiple slots of various primitive types
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_multiple_slot_types() {
        let src = r#"
          component Card {
            slot title: text;
            slot count: number;
            slot visible: bool;
            slot avatar: image;
            slot bg: color;
            Box { }
          }
        "#;
        let ast = parse(src);
        assert_file_root(&ast);
        assert!(find_rule(&ast, "slot_decl"), "Expected slot_decl");
    }

    // -----------------------------------------------------------------------
    // Test 4: Slot with default value
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_slot_with_default() {
        let src = r#"
          component Toggle {
            slot visible: bool = true;
            Box { }
          }
        "#;
        let ast = parse(src);
        assert_file_root(&ast);
        assert!(find_rule(&ast, "default_value"), "Expected default_value");
    }

    // -----------------------------------------------------------------------
    // Test 5: List type slot
    // -----------------------------------------------------------------------
    // list<T> parsing is now fixed by reordering slot_type alternation to try
    // list_type before KEYWORD.

    #[test]
    fn test_parse_list_slot() {
        let src = r#"
          component List {
            slot items: list<text>;
            Column { }
          }
        "#;
        let ast = parse(src);
        assert_file_root(&ast);
        assert!(find_rule(&ast, "list_type"), "Expected list_type");
    }

    // -----------------------------------------------------------------------
    // Test 6: Property assignment with dimension value
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_dimension_property() {
        let src = r#"
          component Padded {
            Box { padding: 16dp; }
          }
        "#;
        let ast = parse(src);
        assert_file_root(&ast);
        assert!(
            find_rule(&ast, "property_assignment"),
            "Expected property_assignment"
        );
    }

    // -----------------------------------------------------------------------
    // Test 7: Property assignment with color
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_color_property() {
        let src = r#"
          component Colored {
            Box { background: #2563eb; }
          }
        "#;
        let ast = parse(src);
        assert_file_root(&ast);
        assert!(
            find_rule(&ast, "property_assignment"),
            "Expected property_assignment"
        );
    }

    // -----------------------------------------------------------------------
    // Test 8: Slot reference as property value
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_slot_ref_property() {
        let src = r#"
          component Label {
            slot title: text;
            Text { content: @title; }
          }
        "#;
        let ast = parse(src);
        assert_file_root(&ast);
        assert!(find_rule(&ast, "slot_ref"), "Expected slot_ref");
    }

    // -----------------------------------------------------------------------
    // Test 9: Nested child nodes
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_nested_nodes() {
        let src = r#"
          component Layout {
            Column {
              Row {
                Text { content: "Hello"; }
              }
            }
          }
        "#;
        let ast = parse(src);
        assert_file_root(&ast);
        assert!(find_rule(&ast, "node_element"), "Expected node_element");
        assert!(find_rule(&ast, "child_node"), "Expected child_node");
    }

    // -----------------------------------------------------------------------
    // Test 10: when block
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_when_block() {
        let src = r#"
          component Conditional {
            slot show: bool;
            Column {
              when @show {
                Text { content: "Visible"; }
              }
            }
          }
        "#;
        let ast = parse(src);
        assert_file_root(&ast);
        assert!(find_rule(&ast, "when_block"), "Expected when_block");
    }

    // -----------------------------------------------------------------------
    // Test 11: each block
    // -----------------------------------------------------------------------
    // list<T> parsing is now fixed; each block test is fully active.

    #[test]
    fn test_parse_each_block() {
        let src = r#"
          component ItemList {
            slot items: list<text>;
            Column {
              each @items as item {
                Text { content: @item; }
              }
            }
          }
        "#;
        let ast = parse(src);
        assert_file_root(&ast);
        assert!(find_rule(&ast, "each_block"), "Expected each_block");
    }

    // -----------------------------------------------------------------------
    // Test 12: Slot reference as child (not property)
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_slot_reference_child() {
        let src = r#"
          component Container {
            slot header: node;
            Column {
              @header;
              Box { }
            }
          }
        "#;
        let ast = parse(src);
        assert_file_root(&ast);
        assert!(find_rule(&ast, "slot_reference"), "Expected slot_reference");
    }

    // -----------------------------------------------------------------------
    // Test 13: Import declaration
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_import() {
        let src = r#"
          import Button from "./button.mosaic";
          component Card {
            Box { }
          }
        "#;
        let ast = parse(src);
        assert_file_root(&ast);
        assert!(find_rule(&ast, "import_decl"), "Expected import_decl");
    }

    // -----------------------------------------------------------------------
    // Test 14: Import with alias
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_import_alias() {
        let src = r#"
          import Card as InfoCard from "./cards.mosaic";
          component Page {
            Box { }
          }
        "#;
        let ast = parse(src);
        assert_file_root(&ast);
        assert!(find_rule(&ast, "import_decl"), "Expected import_decl");
    }

    // -----------------------------------------------------------------------
    // Test 15: Enum property value (e.g., align.center)
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_enum_value() {
        let src = r#"
          component Centered {
            Box { align: center; }
          }
        "#;
        let ast = parse(src);
        assert_file_root(&ast);
        assert!(
            find_rule(&ast, "property_assignment"),
            "Expected property_assignment"
        );
    }

    // -----------------------------------------------------------------------
    // Test 16: Factory function returns working parser
    // -----------------------------------------------------------------------

    #[test]
    fn test_create_parser() {
        let src = r#"component X { Box { } }"#;
        let mut p = create_mosaic_parser(src);
        let result = p.parse();
        assert!(result.is_ok(), "Parser should succeed: {:?}", result.err());
        assert_eq!(result.unwrap().rule_name, "file");
    }
}

/// Regression tests for [`MAX_RULE_DEPTH`], one triple per independent
/// recursive shape (see that constant's doc comment). Uses
/// `create_mosaic_parser(..).parse()` directly (not the panicking [`parse`]
/// wrapper) since these tests need to observe the `Result` rather than
/// unwind through a panic.
#[cfg(test)]
mod depth_guard_tests {
    fn nested_element_source(n: usize) -> String {
        format!("component C {{ {}{} }}", "N{".repeat(n), "}".repeat(n))
    }

    fn nested_when_source(n: usize) -> String {
        format!(
            "component C {{ N {{ {}{} }} }}",
            "when @s {".repeat(n),
            "}".repeat(n)
        )
    }

    fn nested_each_source(n: usize) -> String {
        format!(
            "component C {{ N {{ {}{} }} }}",
            "each @s as it {".repeat(n),
            "}".repeat(n)
        )
    }

    fn nested_list_source(n: usize) -> String {
        format!(
            "component C {{ slot x: {}text{}; N{{}} }}",
            "list<".repeat(n),
            ">".repeat(n)
        )
    }

    macro_rules! depth_guard_triple {
        ($mod_name:ident, $source_fn:ident, $up_to_cap:expr, $one_past_cap:expr) => {
            mod $mod_name {
                use super::$source_fn as nested_source;

                /// Deeply-nested input must produce a recoverable error, not
                /// overflow the native stack. Parses 5000 levels — far past
                /// `MAX_RULE_DEPTH` — on a worker thread with a generous
                /// 32 MiB stack, so the *guard* is what stops the
                /// recursion, not the stack running out.
                #[test]
                fn test_deeply_nested_input_returns_error_not_overflow() {
                    let handle = std::thread::Builder::new()
                        .name(concat!(
                            "mosaic-parser-depth-guard-",
                            stringify!($mod_name),
                            "-regression"
                        ).to_string())
                        .stack_size(32 * 1024 * 1024)
                        .spawn(|| {
                            let result = super::super::create_mosaic_parser(&nested_source(5000)).parse();
                            assert!(
                                result.is_err(),
                                "deeply-nested input must fail with an error, not parse or crash"
                            );
                        })
                        .expect("failed to spawn worker thread");
                    handle
                        .join()
                        .expect("depth guard must keep the worker thread from crashing");
                }

                /// Input that nests *exactly up to* `MAX_RULE_DEPTH` still
                /// parses cleanly, and one layer deeper cleanly trips the
                /// guard. These exact boundary counts were found empirically
                /// by binary-searching against increasing nesting counts at
                /// the production cap — see `MAX_RULE_DEPTH`'s doc comment.
                #[test]
                fn test_nesting_up_to_cap_still_parses() {
                    assert!(
                        super::super::create_mosaic_parser(&nested_source($up_to_cap))
                            .parse()
                            .is_ok(),
                        "{} levels must stay under the cap",
                        $up_to_cap
                    );
                    assert!(
                        super::super::create_mosaic_parser(&nested_source($one_past_cap))
                            .parse()
                            .is_err(),
                        "one nesting level past the cap's measured limit must fail"
                    );
                }

                /// A caller relying on `MAX_RULE_DEPTH` must have the guard
                /// trip *before* the native stack overflows on a
                /// default-stack thread — otherwise a production caller
                /// (e.g. the `mosaic` CLI, or `cargo test`'s own per-test
                /// thread) would still crash. Parses far-too-deep input on a
                /// worker thread with **no** `stack_size` override (the same
                /// default a thread gets in this environment, unmodified by
                /// any `RUST_MIN_STACK` override). A clean `Err` (not a
                /// `join()` failure from a crashed thread) proves
                /// `MAX_RULE_DEPTH` sits safely below the native overflow
                /// point on the default stack.
                #[test]
                fn test_opt_in_cap_trips_before_overflow_on_default_stack() {
                    let handle = std::thread::spawn(|| {
                        let result = super::super::create_mosaic_parser(&nested_source(5000)).parse();
                        assert!(result.is_err(), "deeply-nested input must error, not crash");
                    });
                    handle.join().expect(
                        "MAX_RULE_DEPTH must trip BEFORE native overflow on the default stack",
                    );
                }
            }
        };
    }

    depth_guard_triple!(element_shape, nested_element_source, 56, 57);
    depth_guard_triple!(when_shape, nested_when_source, 82, 83);
    depth_guard_triple!(each_shape, nested_each_source, 82, 83);
    depth_guard_triple!(list_shape, nested_list_source, 83, 84);
}
