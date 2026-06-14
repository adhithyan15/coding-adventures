use coding_adventures_macsyma_parser::parse_macsyma;
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};

fn count_rule(node: &GrammarASTNode, rule_name: &str) -> usize {
    let mut count = usize::from(node.rule_name == rule_name);
    for child in &node.children {
        if let ASTNodeOrToken::Node(child_node) = child {
            count += count_rule(child_node, rule_name);
        }
    }
    count
}

#[test]
fn parses_single_expression_statement() {
    let ast = parse_macsyma("x;");
    assert_eq!(ast.rule_name, "program");
    assert_eq!(count_rule(&ast, "statement"), 1);
}

#[test]
fn parses_precedence_bearing_arithmetic() {
    let ast = parse_macsyma("1 + 2 * 3;");
    assert!(count_rule(&ast, "additive") > 0);
    assert!(count_rule(&ast, "multiplicative") > 0);
}

#[test]
fn parses_function_definitions_and_calls() {
    let ast = parse_macsyma("f(x) := x^2; diff(f(x), x);");
    assert_eq!(count_rule(&ast, "statement"), 2);
    assert!(count_rule(&ast, "postfix") >= 2);
}

#[test]
fn parses_lists_comparisons_logic_and_dollar_terminators() {
    let ast = parse_macsyma("[1, 2, 3]$ a < b and not false;");
    assert!(count_rule(&ast, "list") > 0);
    assert!(count_rule(&ast, "comparison") > 0);
    assert!(count_rule(&ast, "logical_and") > 0);
}

#[test]
fn parses_control_flow_grammar_forms() {
    let ast = parse_macsyma("if x < 0 then -x else x; while x < 3 do x : x + 1;");
    assert_eq!(count_rule(&ast, "if_expr"), 1);
    assert_eq!(count_rule(&ast, "while_expr"), 1);
}
