// AUTO-GENERATED FILE - DO NOT EDIT
#![allow(clippy::all)]
use grammar_tools::parser_grammar::{ParserGrammar, GrammarRule, GrammarElement};

pub fn SqlGrammar() -> ParserGrammar {
    ParserGrammar {
        version: 1,
        rules: vec![
            GrammarRule {
                name: "program".to_string(),
                line_number: 10,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "statement".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: ";".to_string() }, GrammarElement::RuleReference { name: "statement".to_string() }] }) }, GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: ";".to_string() }) }] },
            },
            GrammarRule {
                name: "statement".to_string(),
                line_number: 12,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::RuleReference { name: "select_stmt".to_string() }, GrammarElement::RuleReference { name: "insert_stmt".to_string() }, GrammarElement::RuleReference { name: "update_stmt".to_string() }, GrammarElement::RuleReference { name: "delete_stmt".to_string() }, GrammarElement::RuleReference { name: "create_table_stmt".to_string() }, GrammarElement::RuleReference { name: "drop_table_stmt".to_string() }] },
            },
            GrammarRule {
                name: "select_stmt".to_string(),
                line_number: 17,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "SELECT".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Alternation { choices: vec![GrammarElement::Literal { value: "DISTINCT".to_string() }, GrammarElement::Literal { value: "ALL".to_string() }] }) }, GrammarElement::RuleReference { name: "select_list".to_string() }, GrammarElement::Literal { value: "FROM".to_string() }, GrammarElement::RuleReference { name: "table_ref".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: "join_clause".to_string() }) }, GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: "where_clause".to_string() }) }, GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: "group_clause".to_string() }) }, GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: "having_clause".to_string() }) }, GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: "order_clause".to_string() }) }, GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: "limit_clause".to_string() }) }] },
            },
            GrammarRule {
                name: "select_list".to_string(),
                line_number: 22,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "STAR".to_string() }, GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "select_item".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: ",".to_string() }, GrammarElement::RuleReference { name: "select_item".to_string() }] }) }] }] },
            },
            GrammarRule {
                name: "select_item".to_string(),
                line_number: 23,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "expr".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "AS".to_string() }, GrammarElement::TokenReference { name: "NAME".to_string() }] }) }] },
            },
            GrammarRule {
                name: "table_ref".to_string(),
                line_number: 25,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "table_name".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "AS".to_string() }, GrammarElement::TokenReference { name: "NAME".to_string() }] }) }] },
            },
            GrammarRule {
                name: "table_name".to_string(),
                line_number: 26,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: ".".to_string() }, GrammarElement::TokenReference { name: "NAME".to_string() }] }) }] },
            },
            GrammarRule {
                name: "join_clause".to_string(),
                line_number: 28,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "join_type".to_string() }, GrammarElement::Literal { value: "JOIN".to_string() }, GrammarElement::RuleReference { name: "table_ref".to_string() }, GrammarElement::Literal { value: "ON".to_string() }, GrammarElement::RuleReference { name: "expr".to_string() }] },
            },
            GrammarRule {
                name: "join_type".to_string(),
                line_number: 29,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::Literal { value: "CROSS".to_string() }, GrammarElement::Literal { value: "INNER".to_string() }, GrammarElement::Group { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "LEFT".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: "OUTER".to_string() }) }] }) }, GrammarElement::Group { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "RIGHT".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: "OUTER".to_string() }) }] }) }, GrammarElement::Group { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "FULL".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: "OUTER".to_string() }) }] }) }] },
            },
            GrammarRule {
                name: "where_clause".to_string(),
                line_number: 32,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "WHERE".to_string() }, GrammarElement::RuleReference { name: "expr".to_string() }] },
            },
            GrammarRule {
                name: "group_clause".to_string(),
                line_number: 33,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "GROUP".to_string() }, GrammarElement::Literal { value: "BY".to_string() }, GrammarElement::RuleReference { name: "column_ref".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: ",".to_string() }, GrammarElement::RuleReference { name: "column_ref".to_string() }] }) }] },
            },
            GrammarRule {
                name: "having_clause".to_string(),
                line_number: 34,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "HAVING".to_string() }, GrammarElement::RuleReference { name: "expr".to_string() }] },
            },
            GrammarRule {
                name: "order_clause".to_string(),
                line_number: 35,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "ORDER".to_string() }, GrammarElement::Literal { value: "BY".to_string() }, GrammarElement::RuleReference { name: "order_item".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: ",".to_string() }, GrammarElement::RuleReference { name: "order_item".to_string() }] }) }] },
            },
            GrammarRule {
                name: "order_item".to_string(),
                line_number: 36,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "expr".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Alternation { choices: vec![GrammarElement::Literal { value: "ASC".to_string() }, GrammarElement::Literal { value: "DESC".to_string() }] }) }] },
            },
            GrammarRule {
                name: "limit_clause".to_string(),
                line_number: 37,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "LIMIT".to_string() }, GrammarElement::TokenReference { name: "NUMBER".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "OFFSET".to_string() }, GrammarElement::TokenReference { name: "NUMBER".to_string() }] }) }] },
            },
            GrammarRule {
                name: "insert_stmt".to_string(),
                line_number: 41,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "INSERT".to_string() }, GrammarElement::Literal { value: "INTO".to_string() }, GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "(".to_string() }, GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: ",".to_string() }, GrammarElement::TokenReference { name: "NAME".to_string() }] }) }, GrammarElement::Literal { value: ")".to_string() }] }) }, GrammarElement::Literal { value: "VALUES".to_string() }, GrammarElement::RuleReference { name: "row_value".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: ",".to_string() }, GrammarElement::RuleReference { name: "row_value".to_string() }] }) }] },
            },
            GrammarRule {
                name: "row_value".to_string(),
                line_number: 44,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "(".to_string() }, GrammarElement::RuleReference { name: "expr".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: ",".to_string() }, GrammarElement::RuleReference { name: "expr".to_string() }] }) }, GrammarElement::Literal { value: ")".to_string() }] },
            },
            GrammarRule {
                name: "update_stmt".to_string(),
                line_number: 46,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "UPDATE".to_string() }, GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::Literal { value: "SET".to_string() }, GrammarElement::RuleReference { name: "assignment".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: ",".to_string() }, GrammarElement::RuleReference { name: "assignment".to_string() }] }) }, GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: "where_clause".to_string() }) }] },
            },
            GrammarRule {
                name: "assignment".to_string(),
                line_number: 48,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::Literal { value: "=".to_string() }, GrammarElement::RuleReference { name: "expr".to_string() }] },
            },
            GrammarRule {
                name: "delete_stmt".to_string(),
                line_number: 50,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "DELETE".to_string() }, GrammarElement::Literal { value: "FROM".to_string() }, GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: "where_clause".to_string() }) }] },
            },
            GrammarRule {
                name: "create_table_stmt".to_string(),
                line_number: 54,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "CREATE".to_string() }, GrammarElement::Literal { value: "TABLE".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "IF".to_string() }, GrammarElement::Literal { value: "NOT".to_string() }, GrammarElement::Literal { value: "EXISTS".to_string() }] }) }, GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::Literal { value: "(".to_string() }, GrammarElement::RuleReference { name: "col_def".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: ",".to_string() }, GrammarElement::RuleReference { name: "col_def".to_string() }] }) }, GrammarElement::Literal { value: ")".to_string() }] },
            },
            GrammarRule {
                name: "col_def".to_string(),
                line_number: 56,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: "col_constraint".to_string() }) }] },
            },
            GrammarRule {
                name: "col_constraint".to_string(),
                line_number: 57,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::Group { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "NOT".to_string() }, GrammarElement::Literal { value: "NULL".to_string() }] }) }, GrammarElement::Literal { value: "NULL".to_string() }, GrammarElement::Group { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "PRIMARY".to_string() }, GrammarElement::Literal { value: "KEY".to_string() }] }) }, GrammarElement::Literal { value: "UNIQUE".to_string() }, GrammarElement::Group { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "DEFAULT".to_string() }, GrammarElement::RuleReference { name: "primary".to_string() }] }) }] },
            },
            GrammarRule {
                name: "drop_table_stmt".to_string(),
                line_number: 60,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "DROP".to_string() }, GrammarElement::Literal { value: "TABLE".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "IF".to_string() }, GrammarElement::Literal { value: "EXISTS".to_string() }] }) }, GrammarElement::TokenReference { name: "NAME".to_string() }] },
            },
            GrammarRule {
                name: "expr".to_string(),
                line_number: 64,
                body: GrammarElement::RuleReference { name: "or_expr".to_string() },
            },
            GrammarRule {
                name: "or_expr".to_string(),
                line_number: 65,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "and_expr".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "OR".to_string() }, GrammarElement::RuleReference { name: "and_expr".to_string() }] }) }] },
            },
            GrammarRule {
                name: "and_expr".to_string(),
                line_number: 66,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "not_expr".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "AND".to_string() }, GrammarElement::RuleReference { name: "not_expr".to_string() }] }) }] },
            },
            GrammarRule {
                name: "not_expr".to_string(),
                line_number: 67,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "NOT".to_string() }, GrammarElement::RuleReference { name: "not_expr".to_string() }] }, GrammarElement::RuleReference { name: "comparison".to_string() }] },
            },
            GrammarRule {
                name: "comparison".to_string(),
                line_number: 68,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "additive".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Alternation { choices: vec![GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "cmp_op".to_string() }, GrammarElement::RuleReference { name: "additive".to_string() }] }, GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "BETWEEN".to_string() }, GrammarElement::RuleReference { name: "additive".to_string() }, GrammarElement::Literal { value: "AND".to_string() }, GrammarElement::RuleReference { name: "additive".to_string() }] }, GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "NOT".to_string() }, GrammarElement::Literal { value: "BETWEEN".to_string() }, GrammarElement::RuleReference { name: "additive".to_string() }, GrammarElement::Literal { value: "AND".to_string() }, GrammarElement::RuleReference { name: "additive".to_string() }] }, GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "IN".to_string() }, GrammarElement::Literal { value: "(".to_string() }, GrammarElement::RuleReference { name: "value_list".to_string() }, GrammarElement::Literal { value: ")".to_string() }] }, GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "NOT".to_string() }, GrammarElement::Literal { value: "IN".to_string() }, GrammarElement::Literal { value: "(".to_string() }, GrammarElement::RuleReference { name: "value_list".to_string() }, GrammarElement::Literal { value: ")".to_string() }] }, GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "LIKE".to_string() }, GrammarElement::RuleReference { name: "additive".to_string() }] }, GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "NOT".to_string() }, GrammarElement::Literal { value: "LIKE".to_string() }, GrammarElement::RuleReference { name: "additive".to_string() }] }, GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "IS".to_string() }, GrammarElement::Literal { value: "NULL".to_string() }] }, GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "IS".to_string() }, GrammarElement::Literal { value: "NOT".to_string() }, GrammarElement::Literal { value: "NULL".to_string() }] }] }) }] },
            },
            GrammarRule {
                name: "cmp_op".to_string(),
                line_number: 78,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::Literal { value: "=".to_string() }, GrammarElement::TokenReference { name: "NOT_EQUALS".to_string() }, GrammarElement::Literal { value: "<".to_string() }, GrammarElement::Literal { value: ">".to_string() }, GrammarElement::Literal { value: "<=".to_string() }, GrammarElement::Literal { value: ">=".to_string() }] },
            },
            GrammarRule {
                name: "additive".to_string(),
                line_number: 79,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "multiplicative".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![GrammarElement::Literal { value: "+".to_string() }, GrammarElement::Literal { value: "-".to_string() }] }) }, GrammarElement::RuleReference { name: "multiplicative".to_string() }] }) }] },
            },
            GrammarRule {
                name: "multiplicative".to_string(),
                line_number: 80,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "unary".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "STAR".to_string() }, GrammarElement::Literal { value: "/".to_string() }, GrammarElement::Literal { value: "%".to_string() }] }) }, GrammarElement::RuleReference { name: "unary".to_string() }] }) }] },
            },
            GrammarRule {
                name: "unary".to_string(),
                line_number: 81,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "-".to_string() }, GrammarElement::RuleReference { name: "unary".to_string() }] }, GrammarElement::RuleReference { name: "primary".to_string() }] },
            },
            GrammarRule {
                name: "primary".to_string(),
                line_number: 82,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "NUMBER".to_string() }, GrammarElement::TokenReference { name: "STRING".to_string() }, GrammarElement::Literal { value: "NULL".to_string() }, GrammarElement::Literal { value: "TRUE".to_string() }, GrammarElement::Literal { value: "FALSE".to_string() }, GrammarElement::RuleReference { name: "function_call".to_string() }, GrammarElement::RuleReference { name: "column_ref".to_string() }, GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "(".to_string() }, GrammarElement::RuleReference { name: "expr".to_string() }, GrammarElement::Literal { value: ")".to_string() }] }] },
            },
            GrammarRule {
                name: "column_ref".to_string(),
                line_number: 85,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: ".".to_string() }, GrammarElement::TokenReference { name: "NAME".to_string() }] }) }] },
            },
            GrammarRule {
                name: "function_call".to_string(),
                line_number: 86,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::Literal { value: "(".to_string() }, GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "STAR".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: "value_list".to_string() }) }] }) }, GrammarElement::Literal { value: ")".to_string() }] },
            },
            GrammarRule {
                name: "value_list".to_string(),
                line_number: 87,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "expr".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: ",".to_string() }, GrammarElement::RuleReference { name: "expr".to_string() }] }) }] },
            },
        ],
    }
}
