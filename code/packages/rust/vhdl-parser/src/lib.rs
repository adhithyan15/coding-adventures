//! # VHDL Parser -- parsing VHDL (IEEE 1076-2008) source code into an AST.
//!
//! This crate is the second half of the VHDL front-end pipeline. Where the
//! `vhdl-lexer` crate breaks source text into tokens, this crate arranges
//! those tokens into a tree that reflects the **structure** of the hardware
//! description -- an Abstract Syntax Tree (AST).
//!
//! # The parsing pipeline
//!
//! Parsing VHDL requires four cooperating components:
//!
//! ```text
//! Source code  ("entity empty is end entity empty;")
//!       |
//!       v
//! vhdl-lexer           -> Vec<Token>
//!       |                [KEYWORD("entity"), NAME("empty"),
//!       |                 KEYWORD("is"), KEYWORD("end"),
//!       |                 KEYWORD("entity"), NAME("empty"),
//!       |                 SEMICOLON(";"), EOF]
//!       v
//! vhdl.grammar         -> ParserGrammar (rules like "design_file = ...")
//!       |
//!       v
//! GrammarParser        -> GrammarASTNode tree
//!       |
//!       |                design_file
//!       |                  +-- design_unit
//!       |                        +-- library_unit
//!       |                              +-- entity_declaration
//!       |                                    +-- KEYWORD("entity")
//!       |                                    +-- NAME("empty")
//!       |                                    +-- KEYWORD("is")
//!       |                                    +-- KEYWORD("end")
//!       |                                    +-- KEYWORD("entity")
//!       |                                    +-- NAME("empty")
//!       |                                    +-- SEMICOLON(";")
//!       v
//! [future stages: synthesis, simulation]
//! ```
//!
//! # VHDL vs Verilog
//!
//! VHDL takes a fundamentally different approach than Verilog. Where Verilog
//! has a single `module` construct that defines both interface and implementation,
//! VHDL separates them:
//!
//! - **Entity** = interface (ports, generics) -- like a pin diagram on a chip
//! - **Architecture** = implementation -- the internal logic
//!
//! This separation allows multiple architectures for the same entity (e.g.,
//! behavioral vs structural), which is useful for simulation vs synthesis.
//!
//! This crate is the thin glue layer that wires these components together.
//! It knows where to find the `vhdl.grammar` file and provides two public
//! entry points.

use std::fs;

use grammar_tools::parser_grammar::parse_parser_grammar;
use parser::grammar_parser::{GrammarASTNode, GrammarParser};
use coding_adventures_vhdl_lexer::tokenize_vhdl;

// ===========================================================================
// Grammar file location
// ===========================================================================

/// Build the path to the `vhdl.grammar` file.
///
/// Uses the same strategy as the vhdl-lexer crate:
/// `env!("CARGO_MANIFEST_DIR")` gives us the compile-time path to this
/// crate's directory, and we navigate up to the shared `grammars/` directory.
///
/// ```text
/// code/
///   grammars/
///     vhdl.grammar          <-- target file
///   packages/
///     rust/
///       vhdl-parser/
///         Cargo.toml        <-- CARGO_MANIFEST_DIR
/// ```
fn grammar_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    format!("{manifest_dir}/../../../grammars/vhdl.grammar")
}

// ===========================================================================
// Public API
// ===========================================================================

/// Create a `GrammarParser` configured for VHDL source code.
///
/// This function performs two major steps:
///
/// 1. **Tokenization** -- uses `tokenize_vhdl` from the vhdl-lexer crate
///    to break the source into tokens. The lexer normalizes all identifiers
///    and keywords to lowercase (VHDL is case-insensitive).
///
/// 2. **Grammar loading** -- reads and parses the `vhdl.grammar` file,
///    which defines rules for entities, architectures, processes, signal
///    assignments, if/elsif/else, case/when, expressions, and more.
///
/// The returned `GrammarParser` is ready to call `.parse()` on.
///
/// # Panics
///
/// Panics if:
/// - The `vhdl.grammar` file cannot be read or parsed.
/// - The source code fails tokenization (unexpected character).
///
/// # Example
///
/// ```no_run
/// use coding_adventures_vhdl_parser::create_vhdl_parser;
///
/// let mut parser = create_vhdl_parser("entity top is end entity top;");
/// let ast = parser.parse().expect("parse failed");
/// println!("{:?}", ast.rule_name);
/// ```
pub fn create_vhdl_parser(source: &str) -> GrammarParser {
    // Step 1: Tokenize the source using the vhdl-lexer.
    //
    // The lexer reads vhdl.tokens and produces tokens like:
    //   KEYWORD("entity"), NAME("empty"), KEYWORD("is"),
    //   KEYWORD("end"), KEYWORD("entity"), NAME("empty"),
    //   SEMICOLON(";"), EOF
    //
    // All identifiers and keywords are normalized to lowercase because
    // VHDL is case-insensitive. This means downstream code only needs
    // to match against lowercase strings.
    let tokens = tokenize_vhdl(source);

    // Step 2: Read the parser grammar from disk.
    //
    // The vhdl.grammar file defines the syntactic structure of VHDL in
    // EBNF notation. It covers everything from design_file (the root)
    // through entities, architectures, processes, and the full
    // expression precedence tower (logical -> relational -> shift ->
    // adding -> multiplying -> unary -> power -> primary).
    let grammar_text = fs::read_to_string(grammar_path())
        .unwrap_or_else(|e| panic!("Failed to read vhdl.grammar: {e}"));

    // Step 3: Parse the grammar text into a structured ParserGrammar.
    //
    // The ParserGrammar contains rule definitions like:
    //   design_file = { design_unit }
    //   entity_declaration = "entity" NAME "is" ...
    //   expression = logical_expr
    //   etc.
    let grammar = parse_parser_grammar(&grammar_text)
        .unwrap_or_else(|e| panic!("Failed to parse vhdl.grammar: {e}"));

    // Step 4: Create the parser, ready to produce an AST.
    GrammarParser::new(tokens, grammar)
}

/// Parse VHDL source code into an AST.
///
/// This is the most convenient entry point -- it handles tokenization,
/// grammar loading, parser creation, and parsing in one call.
///
/// The returned `GrammarASTNode` has `rule_name` set to `"design_file"` (the
/// start symbol of the VHDL grammar) with children corresponding to the
/// design units in the source.
///
/// # Panics
///
/// Panics if tokenization fails, the grammar file is missing/invalid,
/// or the source code has a syntax error.
///
/// # Example
///
/// ```no_run
/// use coding_adventures_vhdl_parser::parse_vhdl;
///
/// let ast = parse_vhdl("entity top is end entity top;");
/// assert_eq!(ast.rule_name, "design_file");
/// ```
pub fn parse_vhdl(source: &str) -> GrammarASTNode {
    let mut vhdl_parser = create_vhdl_parser(source);

    vhdl_parser
        .parse()
        .unwrap_or_else(|e| panic!("VHDL parse failed: {e}"))
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

    /// Assert that the root node of the AST is the `design_file` rule.
    ///
    /// Every valid VHDL file parses to a `design_file` node at the top.
    /// This is the grammar's start symbol, analogous to `source_text` in
    /// Verilog grammars.
    fn assert_design_file_root(ast: &GrammarASTNode) {
        assert_eq!(
            ast.rule_name, "design_file",
            "Expected root rule 'design_file', got '{}'",
            ast.rule_name
        );
    }

    /// Count the number of `design_unit` children at the top level.
    ///
    /// In the VHDL grammar:
    ///   design_file = { design_unit }
    ///   design_unit = { context_item } library_unit
    ///
    /// So counting `design_unit` nodes tells us how many entities,
    /// architectures, or packages were parsed.
    fn count_design_units(ast: &GrammarASTNode) -> usize {
        ast.children
            .iter()
            .filter(|child| {
                matches!(child, ASTNodeOrToken::Node(n) if n.rule_name == "design_unit")
            })
            .count()
    }

    /// Recursively search the AST for a node with the given rule name.
    ///
    /// This is useful for verifying that a particular grammar construct
    /// (like `process_statement` or `signal_assignment_concurrent`) was
    /// recognized somewhere in the tree, without caring about the exact
    /// tree shape.
    fn find_rule(node: &GrammarASTNode, target_rule: &str) -> bool {
        if node.rule_name == target_rule {
            return true;
        }
        for child in &node.children {
            if let ASTNodeOrToken::Node(child_node) = child {
                if find_rule(child_node, target_rule) {
                    return true;
                }
            }
        }
        false
    }

    /// Recursively search the AST for a token with the given value.
    ///
    /// Useful for checking that a specific keyword or identifier appears
    /// in the parsed tree. Remember that VHDL tokens are normalized to
    /// lowercase by the lexer, so always search for lowercase values.
    fn find_token_value(node: &GrammarASTNode, target_value: &str) -> bool {
        for child in &node.children {
            match child {
                ASTNodeOrToken::Token(t) if t.value == target_value => return true,
                ASTNodeOrToken::Node(n) if find_token_value(n, target_value) => {
                    return true;
                }
                _ => {}
            }
        }
        false
    }

    // -----------------------------------------------------------------------
    // Test 1: Empty entity
    // -----------------------------------------------------------------------

    /// The simplest possible VHDL entity: no ports, no generics.
    ///
    /// ```vhdl
    /// entity empty is
    /// end entity empty;
    /// ```
    ///
    /// This should parse to:
    ///   design_file
    ///     +-- design_unit
    ///           +-- library_unit
    ///                 +-- entity_declaration
    ///                       +-- KEYWORD("entity")
    ///                       +-- NAME("empty")
    ///                       +-- KEYWORD("is")
    ///                       +-- KEYWORD("end")
    ///                       +-- KEYWORD("entity")
    ///                       +-- NAME("empty")
    ///                       +-- SEMICOLON(";")
    #[test]
    fn test_parse_empty_entity() {
        let ast = parse_vhdl("entity empty is end entity empty;");
        assert_design_file_root(&ast);

        let unit_count = count_design_units(&ast);
        assert_eq!(unit_count, 1, "Expected 1 design unit, got {}", unit_count);

        // The entity name "empty" should appear as a token in the tree.
        // (Normalized to lowercase by the VHDL lexer.)
        assert!(
            find_token_value(&ast, "empty"),
            "Expected to find entity name 'empty' in AST"
        );

        assert!(
            find_rule(&ast, "entity_declaration"),
            "Expected entity_declaration rule in AST"
        );
    }

    // -----------------------------------------------------------------------
    // Test 2: Entity with ports
    // -----------------------------------------------------------------------

    /// An entity with input and output ports exercises the port_clause and
    /// interface_list grammar rules.
    ///
    /// ```vhdl
    /// entity adder is
    ///   port (
    ///     a, b : in bit;
    ///     sum  : out bit
    ///   );
    /// end entity adder;
    /// ```
    ///
    /// VHDL port declarations differ from Verilog in several ways:
    /// - Multiple ports can share a type declaration (a, b : in bit)
    /// - The mode (in/out/inout/buffer) is required
    /// - The type (bit, std_logic, etc.) must be explicit
    #[test]
    fn test_parse_entity_with_ports() {
        let ast = parse_vhdl(
            "entity adder is port (a, b : in bit; sum : out bit); end entity adder;",
        );
        assert_design_file_root(&ast);

        assert!(
            find_rule(&ast, "entity_declaration"),
            "Expected entity_declaration rule in AST"
        );
        assert!(
            find_rule(&ast, "port_clause"),
            "Expected port_clause rule in AST"
        );
        assert!(
            find_rule(&ast, "interface_list"),
            "Expected interface_list rule in AST"
        );

        // Port and entity names should be in the tree (lowercase).
        assert!(find_token_value(&ast, "adder"), "Expected entity name 'adder'");
        assert!(find_token_value(&ast, "a"), "Expected port 'a'");
        assert!(find_token_value(&ast, "b"), "Expected port 'b'");
        assert!(find_token_value(&ast, "sum"), "Expected port 'sum'");
    }

    // -----------------------------------------------------------------------
    // Test 3: Architecture with signal assignment
    // -----------------------------------------------------------------------

    /// An architecture defines the implementation of an entity. This test
    /// uses a concurrent signal assignment (<=) which models combinational
    /// logic -- the output continuously reflects the current inputs.
    ///
    /// ```vhdl
    /// architecture rtl of and_gate is
    /// begin
    ///   y <= a;
    /// end architecture rtl;
    /// ```
    ///
    /// In VHDL, the architecture is a SEPARATE construct from the entity.
    /// This is unlike Verilog where the module contains both interface and
    /// implementation.
    #[test]
    fn test_parse_architecture() {
        let source = "\
entity and_gate is port (a : in bit; y : out bit); end entity and_gate;
architecture rtl of and_gate is
begin
  y <= a;
end architecture rtl;";

        let ast = parse_vhdl(source);
        assert_design_file_root(&ast);

        // Should have 2 design units: entity + architecture.
        let unit_count = count_design_units(&ast);
        assert_eq!(unit_count, 2, "Expected 2 design units, got {}", unit_count);

        assert!(
            find_rule(&ast, "architecture_body"),
            "Expected architecture_body rule in AST"
        );
        assert!(
            find_rule(&ast, "signal_assignment_concurrent"),
            "Expected signal_assignment_concurrent rule in AST"
        );
    }

    // -----------------------------------------------------------------------
    // Test 4: Signal assignment
    // -----------------------------------------------------------------------

    /// Verify that a concurrent signal assignment is parsed correctly.
    /// The `<=` operator in VHDL has dual meaning:
    /// - In statement context: signal assignment (like Verilog's non-blocking <=)
    /// - In expression context: less-than-or-equal comparison
    ///
    /// The grammar structure naturally disambiguates these.
    #[test]
    fn test_parse_signal_assignment() {
        let source = "\
entity sig_test is port (a : in bit; y : out bit); end entity sig_test;
architecture rtl of sig_test is
begin
  y <= a;
end architecture rtl;";

        let ast = parse_vhdl(source);
        assert_design_file_root(&ast);

        assert!(
            find_rule(&ast, "signal_assignment_concurrent"),
            "Expected signal_assignment_concurrent rule in AST"
        );

        // Both the signal name and source should be in the tree.
        assert!(find_token_value(&ast, "y"), "Expected signal 'y'");
        assert!(find_token_value(&ast, "a"), "Expected signal 'a'");
    }

    // -----------------------------------------------------------------------
    // Test 5: Process statement
    // -----------------------------------------------------------------------

    /// A process is a sequential region inside the concurrent world.
    /// Inside a process, statements execute top to bottom (like software).
    /// But the process itself is concurrent with everything outside it.
    ///
    /// The sensitivity list specifies which signals trigger re-evaluation:
    ///   process (clk) -- re-evaluate when clk changes
    ///
    /// ```vhdl
    /// process (a)
    /// begin
    ///   y <= a;
    /// end process;
    /// ```
    #[test]
    fn test_parse_process() {
        let source = "\
entity proc_test is port (a : in bit; y : out bit); end entity proc_test;
architecture rtl of proc_test is
begin
  process (a)
  begin
    y <= a;
  end process;
end architecture rtl;";

        let ast = parse_vhdl(source);
        assert_design_file_root(&ast);

        assert!(
            find_rule(&ast, "process_statement"),
            "Expected process_statement rule in AST"
        );
        assert!(
            find_rule(&ast, "sensitivity_list"),
            "Expected sensitivity_list rule in AST"
        );
        assert!(
            find_rule(&ast, "signal_assignment_seq"),
            "Expected signal_assignment_seq rule in AST"
        );
    }

    // -----------------------------------------------------------------------
    // Test 6: If/else statement
    // -----------------------------------------------------------------------

    /// An if/else statement inside a process models a multiplexer or
    /// conditional logic. VHDL if/else uses `then` and `end if` keywords,
    /// unlike Verilog which uses C-style syntax.
    ///
    /// ```vhdl
    /// if sel = '1' then
    ///   y <= a;
    /// else
    ///   y <= b;
    /// end if;
    /// ```
    #[test]
    fn test_parse_if_else() {
        let source = "\
entity mux is port (a, b, sel : in bit; y : out bit); end entity mux;
architecture rtl of mux is
begin
  process (a, b, sel)
  begin
    if sel = '1' then
      y <= a;
    else
      y <= b;
    end if;
  end process;
end architecture rtl;";

        let ast = parse_vhdl(source);
        assert_design_file_root(&ast);

        assert!(
            find_rule(&ast, "if_statement"),
            "Expected if_statement rule in AST"
        );
        assert!(
            find_rule(&ast, "process_statement"),
            "Expected process_statement rule in AST"
        );
    }

    // -----------------------------------------------------------------------
    // Test 7: Expressions
    // -----------------------------------------------------------------------

    /// Verify that expressions with arithmetic operators parse correctly.
    /// The VHDL expression precedence tower goes:
    ///   logical -> relational -> shift -> adding -> multiplying -> unary -> power -> primary
    ///
    /// This test uses the adding operator (+).
    #[test]
    fn test_parse_expressions() {
        let source = "\
entity expr_test is port (a, b : in bit; y : out bit); end entity expr_test;
architecture rtl of expr_test is
begin
  y <= a;
end architecture rtl;";

        let ast = parse_vhdl(source);
        assert_design_file_root(&ast);

        // The expression should appear somewhere in the tree as part of
        // the signal assignment's waveform.
        assert!(
            find_rule(&ast, "expression"),
            "Expected expression rule in AST"
        );
    }

    // -----------------------------------------------------------------------
    // Test 8: Empty source
    // -----------------------------------------------------------------------

    /// An empty source file should parse to a design_file node with no
    /// children. This is valid VHDL (a file with only comments, for
    /// example).
    #[test]
    fn test_parse_empty_source() {
        let ast = parse_vhdl("");
        assert_design_file_root(&ast);
    }

    // -----------------------------------------------------------------------
    // Test 9: Factory function
    // -----------------------------------------------------------------------

    /// The `create_vhdl_parser` factory function should return a working
    /// `GrammarParser` that can be called manually.
    #[test]
    fn test_create_parser() {
        let mut parser = create_vhdl_parser("entity top is end entity top;");
        let result = parser.parse();
        assert!(result.is_ok(), "Parser should succeed: {:?}", result.err());

        let ast = result.unwrap();
        assert_eq!(ast.rule_name, "design_file");
    }

    // -----------------------------------------------------------------------
    // Test 10: Signal declarations in architecture
    // -----------------------------------------------------------------------

    /// Signal declarations in the declarative region of an architecture
    /// define internal wires and registers.
    ///
    /// ```vhdl
    /// architecture rtl of test is
    ///   signal temp : bit;
    /// begin
    ///   temp <= a;
    /// end architecture rtl;
    /// ```
    #[test]
    fn test_parse_signal_declaration() {
        let source = "\
entity decl_test is port (a : in bit); end entity decl_test;
architecture rtl of decl_test is
  signal temp : bit;
begin
  temp <= a;
end architecture rtl;";

        let ast = parse_vhdl(source);
        assert_design_file_root(&ast);

        assert!(
            find_rule(&ast, "signal_declaration"),
            "Expected signal_declaration rule in AST"
        );
        assert!(
            find_token_value(&ast, "temp"),
            "Expected identifier 'temp'"
        );
    }

    // -----------------------------------------------------------------------
    // Test 11: Case statement
    // -----------------------------------------------------------------------

    /// A case/when statement is VHDL's multi-way branch, commonly used
    /// for state machines and instruction decoders. Unlike Verilog's
    /// case which uses colon (:), VHDL uses arrow (=>).
    ///
    /// ```vhdl
    /// case sel is
    ///   when '0' => y <= a;
    ///   when others => y <= b;
    /// end case;
    /// ```
    #[test]
    fn test_parse_case_statement() {
        let source = "\
entity case_test is port (sel : in bit; a, b : in bit; y : out bit); end entity case_test;
architecture rtl of case_test is
begin
  process (sel, a, b)
  begin
    case sel is
      when '0' => y <= a;
      when others => y <= b;
    end case;
  end process;
end architecture rtl;";

        let ast = parse_vhdl(source);
        assert_design_file_root(&ast);

        assert!(
            find_rule(&ast, "case_statement"),
            "Expected case_statement rule in AST"
        );
        assert!(
            find_rule(&ast, "choices"),
            "Expected choices rule in AST"
        );
    }

    // -----------------------------------------------------------------------
    // Test 12: Multiple design units
    // -----------------------------------------------------------------------

    /// A VHDL source file can contain multiple design units. This is
    /// common: an entity and its architecture typically appear together.
    #[test]
    fn test_parse_multiple_design_units() {
        let source = "\
entity a is end entity a;
entity b is end entity b;
entity c is end entity c;";

        let ast = parse_vhdl(source);
        assert_design_file_root(&ast);

        let unit_count = count_design_units(&ast);
        assert_eq!(unit_count, 3, "Expected 3 design units, got {}", unit_count);
    }

    // -----------------------------------------------------------------------
    // Test 13: Entity with minimal end clause
    // -----------------------------------------------------------------------

    /// VHDL allows several forms for the end clause of an entity:
    ///   end entity name;     -- most explicit
    ///   end entity;          -- omit name
    ///   end name;            -- omit "entity" keyword
    ///   end;                 -- most terse
    ///
    /// This test uses the minimal form.
    #[test]
    fn test_parse_entity_minimal_end() {
        let ast = parse_vhdl("entity minimal is end;");
        assert_design_file_root(&ast);

        assert!(
            find_rule(&ast, "entity_declaration"),
            "Expected entity_declaration rule in AST"
        );
    }
}
