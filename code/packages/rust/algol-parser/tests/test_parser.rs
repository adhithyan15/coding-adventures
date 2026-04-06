//! Integration tests for the ALGOL 60 parser.
//!
//! These tests exercise the full pipeline: grammar file → GrammarParser → AST.
//! Unlike the unit tests in lib.rs, these are compiled as a separate binary,
//! verifying the public API (`parse_algol`, `create_algol_parser`).
//!
//! # Test organization
//!
//! 1. Minimal programs — the simplest valid ALGOL 60 inputs
//! 2. Declarations — integer, real, boolean, array
//! 3. Assignments — simple and arithmetic
//! 4. Conditional statements — if/then/else
//! 5. For loops — step/until, while, and simple forms
//! 6. Boolean expressions
//! 7. Compound statements and nested blocks
//! 8. The factory function
//! 9. Complex programs

use coding_adventures_algol_parser::{create_algol_parser, parse_algol};
use parser::grammar_parser::{GrammarASTNode, ASTNodeOrToken};

// ===========================================================================
// Helper functions
// ===========================================================================

/// Assert that the root rule is "program" (the ALGOL 60 start symbol).
fn assert_program_root(ast: &GrammarASTNode) {
    assert_eq!(
        ast.rule_name, "program",
        "Expected root rule 'program', got '{}'",
        ast.rule_name
    );
}

/// Recursively search the AST for any node with the given rule name.
fn find_rule(node: &GrammarASTNode, target: &str) -> bool {
    if node.rule_name == target {
        return true;
    }
    for child in &node.children {
        if let ASTNodeOrToken::Node(child_node) = child {
            if find_rule(child_node, target) {
                return true;
            }
        }
    }
    false
}

/// Count the number of nodes with the given rule name in the entire tree.
fn count_rule(node: &GrammarASTNode, target: &str) -> usize {
    let mut count = if node.rule_name == target { 1 } else { 0 };
    for child in &node.children {
        if let ASTNodeOrToken::Node(child_node) = child {
            count += count_rule(child_node, target);
        }
    }
    count
}

// ===========================================================================
// 1. Minimal programs
// ===========================================================================

/// The absolute minimum: a block with one statement.
/// ALGOL 60 requires at least one statement after declarations.
#[test]
fn test_minimal_program() {
    let ast = parse_algol("begin integer x; x := 42 end");
    assert_program_root(&ast);
    assert!(find_rule(&ast, "block"), "Expected 'block' rule");
}

/// A block with no declarations (only statements) is also valid.
#[test]
fn test_program_no_declarations() {
    // This uses proc_stmt (calling a procedure named "halt" with no args).
    // The grammar allows blocks with zero declarations.
    let ast = parse_algol("begin halt end");
    assert_program_root(&ast);
    assert!(find_rule(&ast, "block"), "Expected 'block' rule");
}

/// A block with a single assignment using a real literal.
#[test]
fn test_program_real_assignment() {
    let ast = parse_algol("begin real pi; pi := 3.14 end");
    assert_program_root(&ast);
    assert!(find_rule(&ast, "assign_stmt"), "Expected 'assign_stmt'");
}

// ===========================================================================
// 2. Declarations
// ===========================================================================

/// Integer declaration with a single variable.
#[test]
fn test_integer_declaration() {
    let ast = parse_algol("begin integer n; n := 0 end");
    assert_program_root(&ast);
    assert!(find_rule(&ast, "type_decl"), "Expected 'type_decl'");
}

/// Integer declaration with multiple variables (comma-separated).
#[test]
fn test_integer_declaration_multiple() {
    let ast = parse_algol("begin integer x, y, z; x := 1; y := 2; z := 3 end");
    assert_program_root(&ast);
    assert!(find_rule(&ast, "type_decl"), "Expected 'type_decl'");
}

/// Real (floating-point) declaration.
#[test]
fn test_real_declaration() {
    let ast = parse_algol("begin real sum, average; sum := 0.0; average := 0.0 end");
    assert_program_root(&ast);
    assert!(find_rule(&ast, "type_decl"), "Expected 'type_decl' for real");
}

/// Boolean variable declaration.
#[test]
fn test_boolean_declaration() {
    let ast = parse_algol("begin boolean flag; flag := true end");
    assert_program_root(&ast);
    assert!(find_rule(&ast, "type_decl"), "Expected 'type_decl' for boolean");
}

/// Multiple declarations in one block.
#[test]
fn test_multiple_declarations() {
    let source = "begin integer i; real x; boolean flag; i := 0; x := 1.0; flag := false end";
    let ast = parse_algol(source);
    assert_program_root(&ast);

    // At least 3 type_decl nodes
    let decl_count = count_rule(&ast, "type_decl");
    assert!(decl_count >= 3,
        "Expected at least 3 type_decl nodes, got {decl_count}");
}

// ===========================================================================
// 3. Assignments
// ===========================================================================

/// Simple assignment: variable := integer literal.
#[test]
fn test_simple_assignment() {
    let ast = parse_algol("begin integer x; x := 42 end");
    assert_program_root(&ast);
    assert!(find_rule(&ast, "assign_stmt"), "Expected 'assign_stmt'");
}

/// Arithmetic assignment: x := 1 + 2 * 3
#[test]
fn test_arithmetic_assignment() {
    let ast = parse_algol("begin integer x; x := 1 + 2 * 3 end");
    assert_program_root(&ast);
    assert!(find_rule(&ast, "assign_stmt"), "Expected 'assign_stmt'");

    // The arithmetic expression rules should appear
    let has_arith = find_rule(&ast, "arith_expr")
        || find_rule(&ast, "simple_arith")
        || find_rule(&ast, "term");
    assert!(has_arith, "Expected arithmetic expression rules in AST");
}

/// Assignment with subtraction and division.
#[test]
fn test_assignment_sub_div() {
    let ast = parse_algol("begin real x; x := 10.0 / 2.0 - 1.0 end");
    assert_program_root(&ast);
    assert!(find_rule(&ast, "assign_stmt"), "Expected 'assign_stmt'");
}

/// Assignment with exponentiation (** operator).
#[test]
fn test_assignment_exponentiation_power() {
    let ast = parse_algol("begin real x; x := 2 ** 10 end");
    assert_program_root(&ast);
    assert!(find_rule(&ast, "assign_stmt"), "Expected 'assign_stmt'");
}

/// Assignment with exponentiation (^ operator).
#[test]
fn test_assignment_exponentiation_caret() {
    let ast = parse_algol("begin real x; x := 2 ^ 10 end");
    assert_program_root(&ast);
    assert!(find_rule(&ast, "assign_stmt"), "Expected 'assign_stmt'");
}

/// Assignment with integer division (div keyword operator).
#[test]
fn test_assignment_integer_div() {
    let ast = parse_algol("begin integer x; x := 10 div 3 end");
    assert_program_root(&ast);
    assert!(find_rule(&ast, "assign_stmt"), "Expected 'assign_stmt'");
}

/// Assignment with modulo (mod keyword operator).
#[test]
fn test_assignment_mod() {
    let ast = parse_algol("begin integer x; x := 10 mod 3 end");
    assert_program_root(&ast);
    assert!(find_rule(&ast, "assign_stmt"), "Expected 'assign_stmt'");
}

// ===========================================================================
// 4. Conditional statements
// ===========================================================================

/// If/then without else.
#[test]
fn test_if_then() {
    let source = "begin integer x; x := 0; if x = 0 then x := 1 end";
    let ast = parse_algol(source);
    assert_program_root(&ast);
    assert!(find_rule(&ast, "cond_stmt"), "Expected 'cond_stmt'");
}

/// If/then/else.
#[test]
fn test_if_then_else() {
    let source = "begin integer x; x := 0; if x = 0 then x := 1 else x := 2 end";
    let ast = parse_algol(source);
    assert_program_root(&ast);
    assert!(find_rule(&ast, "cond_stmt"), "Expected 'cond_stmt'");
}

/// If/then/else if chain.
/// The else-branch is itself a statement, so conditionals can chain.
#[test]
fn test_if_then_else_if_chain() {
    let source = "begin integer x, y; x := 5; if x < 0 then y := 0 else if x = 0 then y := 1 else y := 2 end";
    let ast = parse_algol(source);
    assert_program_root(&ast);

    // Should have at least 2 cond_stmt nodes (outer if and inner else-if)
    let cond_count = count_rule(&ast, "cond_stmt");
    assert!(cond_count >= 2,
        "Expected at least 2 cond_stmt nodes for if/else-if chain, got {cond_count}");
}

/// Comparison operators in conditions: <, <=, >, >=, =, !=
#[test]
fn test_if_with_relational_operators() {
    let comparisons = [
        ("x < 10",  "less-than"),
        ("x <= 10", "less-than-or-equal"),
        ("x > 10",  "greater-than"),
        ("x >= 10", "greater-than-or-equal"),
        ("x = 10",  "equal"),
        ("x != 10", "not-equal"),
    ];
    for (cond, label) in comparisons {
        let source = format!("begin integer x; x := 5; if {cond} then x := 0 end");
        let ast = parse_algol(&source);
        assert_program_root(&ast);
        assert!(find_rule(&ast, "cond_stmt"),
            "Expected 'cond_stmt' for {label} comparison");
    }
}

// ===========================================================================
// 5. For loops
// ===========================================================================

/// Step/until form: `for i := 1 step 1 until 10 do stmt`
/// This is the most common ALGOL 60 loop pattern.
#[test]
fn test_for_step_until() {
    let source = "begin integer i, sum; sum := 0; for i := 1 step 1 until 10 do sum := sum + i end";
    let ast = parse_algol(source);
    assert_program_root(&ast);
    assert!(find_rule(&ast, "for_stmt"), "Expected 'for_stmt'");
}

/// While form: `for x := expr while condition do stmt`
#[test]
fn test_for_while_form() {
    let source = "begin integer x; x := 10; for x := x while x > 0 do x := x - 1 end";
    let ast = parse_algol(source);
    assert_program_root(&ast);
    assert!(find_rule(&ast, "for_stmt"), "Expected 'for_stmt'");
}

/// Simple form: `for i := expr do stmt` (single value, loop executes once)
#[test]
fn test_for_simple_form() {
    let source = "begin integer i; for i := 42 do i := i + 1 end";
    let ast = parse_algol(source);
    assert_program_root(&ast);
    assert!(find_rule(&ast, "for_stmt"), "Expected 'for_stmt'");
}

// ===========================================================================
// 6. Boolean expressions
// ===========================================================================

/// Boolean AND expression.
#[test]
fn test_boolean_and() {
    let source = "begin boolean a, b, c; a := true; b := false; if a and b then c := true else c := false end";
    let ast = parse_algol(source);
    assert_program_root(&ast);
    assert!(find_rule(&ast, "cond_stmt"), "Expected cond_stmt with bool expr");
}

/// Boolean OR expression.
#[test]
fn test_boolean_or() {
    let source = "begin boolean a, b, c; a := true; b := false; if a or b then c := true end";
    let ast = parse_algol(source);
    assert_program_root(&ast);
    assert!(find_rule(&ast, "cond_stmt"), "Expected cond_stmt");
}

/// Boolean NOT expression.
#[test]
fn test_boolean_not() {
    let source = "begin boolean flag; flag := true; if not flag then flag := false end";
    let ast = parse_algol(source);
    assert_program_root(&ast);
    assert!(find_rule(&ast, "cond_stmt"), "Expected cond_stmt with NOT");
}

// ===========================================================================
// 7. Compound statements and nested blocks
// ===========================================================================

/// A compound statement (begin...end) as the body of an if branch.
/// This is how ALGOL resolves the dangling else — use begin...end for multi-statement then-branches.
#[test]
fn test_compound_statement() {
    let source = "begin integer x, y; x := 0; y := 0; if x = 0 then begin x := 1; y := 1 end end";
    let ast = parse_algol(source);
    assert_program_root(&ast);
    assert!(
        find_rule(&ast, "compound_stmt") || find_rule(&ast, "block"),
        "Expected compound_stmt or block for nested begin...end"
    );
}

/// A for loop body using a compound statement.
#[test]
fn test_for_with_compound_body() {
    let source = "begin integer i, x, y; x := 0; y := 0; for i := 1 step 1 until 5 do begin x := x + i; y := y + i end end";
    let ast = parse_algol(source);
    assert_program_root(&ast);
    assert!(find_rule(&ast, "for_stmt"), "Expected 'for_stmt'");
}

// ===========================================================================
// 8. Factory function
// ===========================================================================

/// The `create_algol_parser` factory function returns a working parser.
#[test]
fn test_create_parser_factory() {
    let mut parser = create_algol_parser("begin integer x; x := 0 end");
    let result = parser.parse();
    assert!(result.is_ok(), "Parser should succeed: {:?}", result.err());

    let ast = result.unwrap();
    assert_eq!(ast.rule_name, "program");
}

/// The factory function can be called multiple times independently.
#[test]
fn test_create_parser_multiple() {
    let mut p1 = create_algol_parser("begin integer a; a := 1 end");
    let mut p2 = create_algol_parser("begin real b; b := 2.5 end");

    let r1 = p1.parse();
    let r2 = p2.parse();

    assert!(r1.is_ok(), "First parse should succeed: {:?}", r1.err());
    assert!(r2.is_ok(), "Second parse should succeed: {:?}", r2.err());
}

// ===========================================================================
// 9. Complex programs
// ===========================================================================

/// A realistic ALGOL 60 program computing the sum of 1..10.
#[test]
fn test_summation_program() {
    let source = r#"
        begin
            integer i, sum;
            sum := 0;
            for i := 1 step 1 until 10 do
                sum := sum + i
        end
    "#;
    let ast = parse_algol(source);
    assert_program_root(&ast);
    assert!(find_rule(&ast, "for_stmt"), "Expected for loop");
    assert!(find_rule(&ast, "assign_stmt"), "Expected assignment");
}

/// A program with multiple statements and a conditional.
#[test]
fn test_conditional_sum_program() {
    let source = r#"
        begin
            integer n, result;
            n := 42;
            if n > 0 then result := n
            else result := 0
        end
    "#;
    let ast = parse_algol(source);
    assert_program_root(&ast);
    assert!(find_rule(&ast, "cond_stmt"), "Expected conditional");
}

/// A program using both arithmetic and boolean expressions.
#[test]
fn test_combined_arithmetic_boolean() {
    let source = r#"
        begin
            integer x, y;
            real z;
            x := 10;
            y := 3;
            z := x / y;
            if x > y and y > 0 then z := z + 1.0
        end
    "#;
    let ast = parse_algol(source);
    assert_program_root(&ast);

    assert!(find_rule(&ast, "assign_stmt"), "Expected assignment(s)");
    assert!(find_rule(&ast, "cond_stmt"), "Expected conditional");
}

/// ALGOL 60's precedence: exponentiation > multiplication > addition.
/// `2 + 3 * 4 ** 2` = `2 + (3 * (4 ** 2))` = `2 + 48` = `50`
#[test]
fn test_operator_precedence_parses() {
    let ast = parse_algol("begin real x; x := 2 + 3 * 4 ** 2 end");
    assert_program_root(&ast);
    assert!(find_rule(&ast, "assign_stmt"), "Expected assignment");
    assert!(find_rule(&ast, "factor"), "Expected factor rule (exponentiation)");
}

/// A program with a goto statement and a labeled statement.
#[test]
fn test_goto_program() {
    let source = r#"
        begin
            integer x;
            x := 0;
            loop: if x >= 10 then goto done;
            x := x + 1;
            goto loop;
            done: x := x
        end
    "#;
    let ast = parse_algol(source);
    assert_program_root(&ast);
    assert!(find_rule(&ast, "goto_stmt"), "Expected goto_stmt");
}
