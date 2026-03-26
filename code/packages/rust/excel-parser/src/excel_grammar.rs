// AUTO-GENERATED FILE - DO NOT EDIT
#![allow(clippy::all)]
use grammar_tools::parser_grammar::{ParserGrammar, GrammarRule, GrammarElement};

pub fn ExcelGrammar() -> ParserGrammar {
    ParserGrammar {
        version: 1,
        rules: vec![
            GrammarRule {
                name: "formula".to_string(),
                line_number: 15,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "ws".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "EQUALS".to_string() }, GrammarElement::RuleReference { name: "ws".to_string() }] }) }, GrammarElement::RuleReference { name: "expression".to_string() }, GrammarElement::RuleReference { name: "ws".to_string() }] },
            },
            GrammarRule {
                name: "ws".to_string(),
                line_number: 17,
                body: GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: "SPACE".to_string() }) },
            },
            GrammarRule {
                name: "req_space".to_string(),
                line_number: 18,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "SPACE".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: "SPACE".to_string() }) }] },
            },
            GrammarRule {
                name: "expression".to_string(),
                line_number: 20,
                body: GrammarElement::RuleReference { name: "comparison_expr".to_string() },
            },
            GrammarRule {
                name: "comparison_expr".to_string(),
                line_number: 22,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "concat_expr".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "ws".to_string() }, GrammarElement::RuleReference { name: "comparison_op".to_string() }, GrammarElement::RuleReference { name: "ws".to_string() }, GrammarElement::RuleReference { name: "concat_expr".to_string() }] }) }] },
            },
            GrammarRule {
                name: "comparison_op".to_string(),
                line_number: 23,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "EQUALS".to_string() }, GrammarElement::TokenReference { name: "NOT_EQUALS".to_string() }, GrammarElement::TokenReference { name: "LESS_THAN".to_string() }, GrammarElement::TokenReference { name: "LESS_EQUALS".to_string() }, GrammarElement::TokenReference { name: "GREATER_THAN".to_string() }, GrammarElement::TokenReference { name: "GREATER_EQUALS".to_string() }] },
            },
            GrammarRule {
                name: "concat_expr".to_string(),
                line_number: 26,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "additive_expr".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "ws".to_string() }, GrammarElement::TokenReference { name: "AMP".to_string() }, GrammarElement::RuleReference { name: "ws".to_string() }, GrammarElement::RuleReference { name: "additive_expr".to_string() }] }) }] },
            },
            GrammarRule {
                name: "additive_expr".to_string(),
                line_number: 27,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "multiplicative_expr".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "ws".to_string() }, GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "PLUS".to_string() }, GrammarElement::TokenReference { name: "MINUS".to_string() }] }) }, GrammarElement::RuleReference { name: "ws".to_string() }, GrammarElement::RuleReference { name: "multiplicative_expr".to_string() }] }) }] },
            },
            GrammarRule {
                name: "multiplicative_expr".to_string(),
                line_number: 28,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "power_expr".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "ws".to_string() }, GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "STAR".to_string() }, GrammarElement::TokenReference { name: "SLASH".to_string() }] }) }, GrammarElement::RuleReference { name: "ws".to_string() }, GrammarElement::RuleReference { name: "power_expr".to_string() }] }) }] },
            },
            GrammarRule {
                name: "power_expr".to_string(),
                line_number: 29,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "unary_expr".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "ws".to_string() }, GrammarElement::TokenReference { name: "CARET".to_string() }, GrammarElement::RuleReference { name: "ws".to_string() }, GrammarElement::RuleReference { name: "unary_expr".to_string() }] }) }] },
            },
            GrammarRule {
                name: "unary_expr".to_string(),
                line_number: 30,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "prefix_op".to_string() }, GrammarElement::RuleReference { name: "ws".to_string() }] }) }, GrammarElement::RuleReference { name: "postfix_expr".to_string() }] },
            },
            GrammarRule {
                name: "prefix_op".to_string(),
                line_number: 31,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "PLUS".to_string() }, GrammarElement::TokenReference { name: "MINUS".to_string() }] },
            },
            GrammarRule {
                name: "postfix_expr".to_string(),
                line_number: 32,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "primary".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "ws".to_string() }, GrammarElement::TokenReference { name: "PERCENT".to_string() }] }) }] },
            },
            GrammarRule {
                name: "primary".to_string(),
                line_number: 34,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::RuleReference { name: "parenthesized_expression".to_string() }, GrammarElement::RuleReference { name: "constant".to_string() }, GrammarElement::RuleReference { name: "function_call".to_string() }, GrammarElement::RuleReference { name: "structure_reference".to_string() }, GrammarElement::RuleReference { name: "reference_expression".to_string() }, GrammarElement::RuleReference { name: "bang_reference".to_string() }, GrammarElement::RuleReference { name: "bang_name".to_string() }, GrammarElement::RuleReference { name: "name_reference".to_string() }] },
            },
            GrammarRule {
                name: "parenthesized_expression".to_string(),
                line_number: 43,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "LPAREN".to_string() }, GrammarElement::RuleReference { name: "ws".to_string() }, GrammarElement::RuleReference { name: "expression".to_string() }, GrammarElement::RuleReference { name: "ws".to_string() }, GrammarElement::TokenReference { name: "RPAREN".to_string() }] },
            },
            GrammarRule {
                name: "constant".to_string(),
                line_number: 45,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "NUMBER".to_string() }, GrammarElement::TokenReference { name: "STRING".to_string() }, GrammarElement::TokenReference { name: "KEYWORD".to_string() }, GrammarElement::TokenReference { name: "ERROR_CONSTANT".to_string() }, GrammarElement::RuleReference { name: "array_constant".to_string() }] },
            },
            GrammarRule {
                name: "array_constant".to_string(),
                line_number: 47,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "LBRACE".to_string() }, GrammarElement::RuleReference { name: "ws".to_string() }, GrammarElement::RuleReference { name: "array_row".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "ws".to_string() }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }, GrammarElement::RuleReference { name: "ws".to_string() }, GrammarElement::RuleReference { name: "array_row".to_string() }] }) }, GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "ws".to_string() }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }] }) }, GrammarElement::RuleReference { name: "ws".to_string() }, GrammarElement::TokenReference { name: "RBRACE".to_string() }] },
            },
            GrammarRule {
                name: "array_row".to_string(),
                line_number: 48,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "array_item".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "ws".to_string() }, GrammarElement::TokenReference { name: "COMMA".to_string() }, GrammarElement::RuleReference { name: "ws".to_string() }, GrammarElement::RuleReference { name: "array_item".to_string() }] }) }, GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "ws".to_string() }, GrammarElement::TokenReference { name: "COMMA".to_string() }] }) }] },
            },
            GrammarRule {
                name: "array_item".to_string(),
                line_number: 49,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "NUMBER".to_string() }, GrammarElement::TokenReference { name: "STRING".to_string() }, GrammarElement::TokenReference { name: "KEYWORD".to_string() }, GrammarElement::TokenReference { name: "ERROR_CONSTANT".to_string() }] },
            },
            GrammarRule {
                name: "function_call".to_string(),
                line_number: 51,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "function_name".to_string() }, GrammarElement::TokenReference { name: "LPAREN".to_string() }, GrammarElement::RuleReference { name: "ws".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: "function_argument_list".to_string() }) }, GrammarElement::RuleReference { name: "ws".to_string() }, GrammarElement::TokenReference { name: "RPAREN".to_string() }] },
            },
            GrammarRule {
                name: "function_name".to_string(),
                line_number: 52,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "FUNCTION_NAME".to_string() }, GrammarElement::TokenReference { name: "NAME".to_string() }] },
            },
            GrammarRule {
                name: "function_argument_list".to_string(),
                line_number: 53,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "function_argument".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "ws".to_string() }, GrammarElement::TokenReference { name: "COMMA".to_string() }, GrammarElement::RuleReference { name: "ws".to_string() }, GrammarElement::RuleReference { name: "function_argument".to_string() }] }) }, GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "ws".to_string() }, GrammarElement::TokenReference { name: "COMMA".to_string() }] }) }] },
            },
            GrammarRule {
                name: "function_argument".to_string(),
                line_number: 54,
                body: GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: "expression".to_string() }) },
            },
            GrammarRule {
                name: "reference_expression".to_string(),
                line_number: 56,
                body: GrammarElement::RuleReference { name: "union_reference".to_string() },
            },
            GrammarRule {
                name: "union_reference".to_string(),
                line_number: 57,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "intersection_reference".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "ws".to_string() }, GrammarElement::TokenReference { name: "COMMA".to_string() }, GrammarElement::RuleReference { name: "ws".to_string() }, GrammarElement::RuleReference { name: "intersection_reference".to_string() }] }) }] },
            },
            GrammarRule {
                name: "intersection_reference".to_string(),
                line_number: 58,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "range_reference".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "req_space".to_string() }, GrammarElement::RuleReference { name: "range_reference".to_string() }] }) }] },
            },
            GrammarRule {
                name: "range_reference".to_string(),
                line_number: 59,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "reference_primary".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "ws".to_string() }, GrammarElement::TokenReference { name: "COLON".to_string() }, GrammarElement::RuleReference { name: "ws".to_string() }, GrammarElement::RuleReference { name: "reference_primary".to_string() }] }) }] },
            },
            GrammarRule {
                name: "reference_primary".to_string(),
                line_number: 61,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::RuleReference { name: "parenthesized_reference".to_string() }, GrammarElement::RuleReference { name: "prefixed_reference".to_string() }, GrammarElement::RuleReference { name: "external_reference".to_string() }, GrammarElement::RuleReference { name: "structure_reference".to_string() }, GrammarElement::RuleReference { name: "a1_reference".to_string() }, GrammarElement::RuleReference { name: "bang_reference".to_string() }, GrammarElement::RuleReference { name: "bang_name".to_string() }, GrammarElement::RuleReference { name: "name_reference".to_string() }] },
            },
            GrammarRule {
                name: "parenthesized_reference".to_string(),
                line_number: 70,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "LPAREN".to_string() }, GrammarElement::RuleReference { name: "ws".to_string() }, GrammarElement::RuleReference { name: "reference_expression".to_string() }, GrammarElement::RuleReference { name: "ws".to_string() }, GrammarElement::TokenReference { name: "RPAREN".to_string() }] },
            },
            GrammarRule {
                name: "prefixed_reference".to_string(),
                line_number: 71,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "REF_PREFIX".to_string() }, GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![GrammarElement::RuleReference { name: "a1_reference".to_string() }, GrammarElement::RuleReference { name: "name_reference".to_string() }, GrammarElement::RuleReference { name: "structure_reference".to_string() }] }) }] },
            },
            GrammarRule {
                name: "external_reference".to_string(),
                line_number: 72,
                body: GrammarElement::TokenReference { name: "REF_PREFIX".to_string() },
            },
            GrammarRule {
                name: "bang_reference".to_string(),
                line_number: 73,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "BANG".to_string() }, GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "CELL".to_string() }, GrammarElement::TokenReference { name: "COLUMN_REF".to_string() }, GrammarElement::TokenReference { name: "ROW_REF".to_string() }, GrammarElement::TokenReference { name: "NUMBER".to_string() }] }) }] },
            },
            GrammarRule {
                name: "bang_name".to_string(),
                line_number: 74,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "BANG".to_string() }, GrammarElement::RuleReference { name: "name_reference".to_string() }] },
            },
            GrammarRule {
                name: "name_reference".to_string(),
                line_number: 75,
                body: GrammarElement::TokenReference { name: "NAME".to_string() },
            },
            GrammarRule {
                name: "column_reference".to_string(),
                line_number: 77,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: "DOLLAR".to_string() }) }, GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "COLUMN_REF".to_string() }, GrammarElement::TokenReference { name: "NAME".to_string() }] }) }] },
            },
            GrammarRule {
                name: "row_reference".to_string(),
                line_number: 78,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: "DOLLAR".to_string() }) }, GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "ROW_REF".to_string() }, GrammarElement::TokenReference { name: "NUMBER".to_string() }] }) }] },
            },
            GrammarRule {
                name: "a1_reference".to_string(),
                line_number: 80,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "CELL".to_string() }, GrammarElement::RuleReference { name: "column_reference".to_string() }, GrammarElement::RuleReference { name: "row_reference".to_string() }, GrammarElement::TokenReference { name: "COLUMN_REF".to_string() }, GrammarElement::TokenReference { name: "ROW_REF".to_string() }, GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::TokenReference { name: "NUMBER".to_string() }] },
            },
            GrammarRule {
                name: "structure_reference".to_string(),
                line_number: 82,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: "table_name".to_string() }) }, GrammarElement::RuleReference { name: "intra_table_reference".to_string() }] },
            },
            GrammarRule {
                name: "table_name".to_string(),
                line_number: 83,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "TABLE_NAME".to_string() }, GrammarElement::TokenReference { name: "NAME".to_string() }] },
            },
            GrammarRule {
                name: "intra_table_reference".to_string(),
                line_number: 84,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "STRUCTURED_KEYWORD".to_string() }, GrammarElement::RuleReference { name: "structured_column_range".to_string() }, GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "LBRACKET".to_string() }, GrammarElement::RuleReference { name: "ws".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: "inner_structure_reference".to_string() }) }, GrammarElement::RuleReference { name: "ws".to_string() }, GrammarElement::TokenReference { name: "RBRACKET".to_string() }] }] },
            },
            GrammarRule {
                name: "inner_structure_reference".to_string(),
                line_number: 87,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "structured_keyword_list".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "ws".to_string() }, GrammarElement::TokenReference { name: "COMMA".to_string() }, GrammarElement::RuleReference { name: "ws".to_string() }, GrammarElement::RuleReference { name: "structured_column_range".to_string() }] }) }] }, GrammarElement::RuleReference { name: "structured_column_range".to_string() }] },
            },
            GrammarRule {
                name: "structured_keyword_list".to_string(),
                line_number: 89,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "STRUCTURED_KEYWORD".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "ws".to_string() }, GrammarElement::TokenReference { name: "COMMA".to_string() }, GrammarElement::RuleReference { name: "ws".to_string() }, GrammarElement::TokenReference { name: "STRUCTURED_KEYWORD".to_string() }] }) }] },
            },
            GrammarRule {
                name: "structured_column_range".to_string(),
                line_number: 90,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "structured_column".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "ws".to_string() }, GrammarElement::TokenReference { name: "COLON".to_string() }, GrammarElement::RuleReference { name: "ws".to_string() }, GrammarElement::RuleReference { name: "structured_column".to_string() }] }) }] },
            },
            GrammarRule {
                name: "structured_column".to_string(),
                line_number: 91,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "STRUCTURED_COLUMN".to_string() }, GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "AT".to_string() }, GrammarElement::TokenReference { name: "STRUCTURED_COLUMN".to_string() }] }] },
            },
        ],
    }
}
