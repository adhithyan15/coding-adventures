// AUTO-GENERATED FILE — DO NOT EDIT
// Source: excel.grammar
// Regenerate with: grammar-tools compile-grammar excel.grammar
//
// This file embeds a ParserGrammar as native Rust data structures.
// Call `parser_grammar()` instead of reading and parsing the .grammar file.

use grammar_tools::parser_grammar::{GrammarElement, GrammarRule, ParserGrammar};

pub fn parser_grammar() -> ParserGrammar {
    ParserGrammar {
        rules: vec![
        GrammarRule {
            name: r#"formula"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"ws"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                        GrammarElement::RuleReference { name: r#"ws"#.to_string() },
                    ] }) },
                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                GrammarElement::RuleReference { name: r#"ws"#.to_string() },
            ] },
            line_number: 15,
        },
        GrammarRule {
            name: r#"ws"#.to_string(),
            body: GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"SPACE"#.to_string() }) },
            line_number: 17,
        },
        GrammarRule {
            name: r#"req_space"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"SPACE"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"SPACE"#.to_string() }) },
            ] },
            line_number: 18,
        },
        GrammarRule {
            name: r#"expression"#.to_string(),
            body: GrammarElement::RuleReference { name: r#"comparison_expr"#.to_string() },
            line_number: 20,
        },
        GrammarRule {
            name: r#"comparison_expr"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"concat_expr"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"ws"#.to_string() },
                        GrammarElement::RuleReference { name: r#"comparison_op"#.to_string() },
                        GrammarElement::RuleReference { name: r#"ws"#.to_string() },
                        GrammarElement::RuleReference { name: r#"concat_expr"#.to_string() },
                    ] }) },
            ] },
            line_number: 22,
        },
        GrammarRule {
            name: r#"comparison_op"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                GrammarElement::TokenReference { name: r#"NOT_EQUALS"#.to_string() },
                GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                GrammarElement::TokenReference { name: r#"LESS_EQUALS"#.to_string() },
                GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                GrammarElement::TokenReference { name: r#"GREATER_EQUALS"#.to_string() },
            ] },
            line_number: 23,
        },
        GrammarRule {
            name: r#"concat_expr"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"additive_expr"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"ws"#.to_string() },
                        GrammarElement::TokenReference { name: r#"AMP"#.to_string() },
                        GrammarElement::RuleReference { name: r#"ws"#.to_string() },
                        GrammarElement::RuleReference { name: r#"additive_expr"#.to_string() },
                    ] }) },
            ] },
            line_number: 26,
        },
        GrammarRule {
            name: r#"additive_expr"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"multiplicative_expr"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"ws"#.to_string() },
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                                GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"ws"#.to_string() },
                        GrammarElement::RuleReference { name: r#"multiplicative_expr"#.to_string() },
                    ] }) },
            ] },
            line_number: 27,
        },
        GrammarRule {
            name: r#"multiplicative_expr"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"power_expr"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"ws"#.to_string() },
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                                GrammarElement::TokenReference { name: r#"SLASH"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"ws"#.to_string() },
                        GrammarElement::RuleReference { name: r#"power_expr"#.to_string() },
                    ] }) },
            ] },
            line_number: 28,
        },
        GrammarRule {
            name: r#"power_expr"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"unary_expr"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"ws"#.to_string() },
                        GrammarElement::TokenReference { name: r#"CARET"#.to_string() },
                        GrammarElement::RuleReference { name: r#"ws"#.to_string() },
                        GrammarElement::RuleReference { name: r#"unary_expr"#.to_string() },
                    ] }) },
            ] },
            line_number: 29,
        },
        GrammarRule {
            name: r#"unary_expr"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"prefix_op"#.to_string() },
                        GrammarElement::RuleReference { name: r#"ws"#.to_string() },
                    ] }) },
                GrammarElement::RuleReference { name: r#"postfix_expr"#.to_string() },
            ] },
            line_number: 30,
        },
        GrammarRule {
            name: r#"prefix_op"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
            ] },
            line_number: 31,
        },
        GrammarRule {
            name: r#"postfix_expr"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"primary"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"ws"#.to_string() },
                        GrammarElement::TokenReference { name: r#"PERCENT"#.to_string() },
                    ] }) },
            ] },
            line_number: 32,
        },
        GrammarRule {
            name: r#"primary"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"parenthesized_expression"#.to_string() },
                GrammarElement::RuleReference { name: r#"constant"#.to_string() },
                GrammarElement::RuleReference { name: r#"function_call"#.to_string() },
                GrammarElement::RuleReference { name: r#"structure_reference"#.to_string() },
                GrammarElement::RuleReference { name: r#"reference_expression"#.to_string() },
                GrammarElement::RuleReference { name: r#"bang_reference"#.to_string() },
                GrammarElement::RuleReference { name: r#"bang_name"#.to_string() },
                GrammarElement::RuleReference { name: r#"name_reference"#.to_string() },
            ] },
            line_number: 34,
        },
        GrammarRule {
            name: r#"parenthesized_expression"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                GrammarElement::RuleReference { name: r#"ws"#.to_string() },
                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                GrammarElement::RuleReference { name: r#"ws"#.to_string() },
                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
            ] },
            line_number: 43,
        },
        GrammarRule {
            name: r#"constant"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                GrammarElement::TokenReference { name: r#"KEYWORD"#.to_string() },
                GrammarElement::TokenReference { name: r#"ERROR_CONSTANT"#.to_string() },
                GrammarElement::RuleReference { name: r#"array_constant"#.to_string() },
            ] },
            line_number: 45,
        },
        GrammarRule {
            name: r#"array_constant"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                GrammarElement::RuleReference { name: r#"ws"#.to_string() },
                GrammarElement::RuleReference { name: r#"array_row"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"ws"#.to_string() },
                        GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                        GrammarElement::RuleReference { name: r#"ws"#.to_string() },
                        GrammarElement::RuleReference { name: r#"array_row"#.to_string() },
                    ] }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"ws"#.to_string() },
                        GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                    ] }) },
                GrammarElement::RuleReference { name: r#"ws"#.to_string() },
                GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
            ] },
            line_number: 47,
        },
        GrammarRule {
            name: r#"array_row"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"array_item"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"ws"#.to_string() },
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::RuleReference { name: r#"ws"#.to_string() },
                        GrammarElement::RuleReference { name: r#"array_item"#.to_string() },
                    ] }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"ws"#.to_string() },
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                    ] }) },
            ] },
            line_number: 48,
        },
        GrammarRule {
            name: r#"array_item"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                GrammarElement::TokenReference { name: r#"KEYWORD"#.to_string() },
                GrammarElement::TokenReference { name: r#"ERROR_CONSTANT"#.to_string() },
            ] },
            line_number: 49,
        },
        GrammarRule {
            name: r#"function_call"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"function_name"#.to_string() },
                GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                GrammarElement::RuleReference { name: r#"ws"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"function_argument_list"#.to_string() }) },
                GrammarElement::RuleReference { name: r#"ws"#.to_string() },
                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
            ] },
            line_number: 51,
        },
        GrammarRule {
            name: r#"function_name"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"FUNCTION_NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
            ] },
            line_number: 52,
        },
        GrammarRule {
            name: r#"function_argument_list"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"function_argument"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"ws"#.to_string() },
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::RuleReference { name: r#"ws"#.to_string() },
                        GrammarElement::RuleReference { name: r#"function_argument"#.to_string() },
                    ] }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"ws"#.to_string() },
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                    ] }) },
            ] },
            line_number: 53,
        },
        GrammarRule {
            name: r#"function_argument"#.to_string(),
            body: GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"expression"#.to_string() }) },
            line_number: 54,
        },
        GrammarRule {
            name: r#"reference_expression"#.to_string(),
            body: GrammarElement::RuleReference { name: r#"union_reference"#.to_string() },
            line_number: 56,
        },
        GrammarRule {
            name: r#"union_reference"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"intersection_reference"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"ws"#.to_string() },
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::RuleReference { name: r#"ws"#.to_string() },
                        GrammarElement::RuleReference { name: r#"intersection_reference"#.to_string() },
                    ] }) },
            ] },
            line_number: 57,
        },
        GrammarRule {
            name: r#"intersection_reference"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"range_reference"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"req_space"#.to_string() },
                        GrammarElement::RuleReference { name: r#"range_reference"#.to_string() },
                    ] }) },
            ] },
            line_number: 58,
        },
        GrammarRule {
            name: r#"range_reference"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"reference_primary"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"ws"#.to_string() },
                        GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                        GrammarElement::RuleReference { name: r#"ws"#.to_string() },
                        GrammarElement::RuleReference { name: r#"reference_primary"#.to_string() },
                    ] }) },
            ] },
            line_number: 59,
        },
        GrammarRule {
            name: r#"reference_primary"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"parenthesized_reference"#.to_string() },
                GrammarElement::RuleReference { name: r#"prefixed_reference"#.to_string() },
                GrammarElement::RuleReference { name: r#"external_reference"#.to_string() },
                GrammarElement::RuleReference { name: r#"structure_reference"#.to_string() },
                GrammarElement::RuleReference { name: r#"a1_reference"#.to_string() },
                GrammarElement::RuleReference { name: r#"bang_reference"#.to_string() },
                GrammarElement::RuleReference { name: r#"bang_name"#.to_string() },
                GrammarElement::RuleReference { name: r#"name_reference"#.to_string() },
            ] },
            line_number: 61,
        },
        GrammarRule {
            name: r#"parenthesized_reference"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                GrammarElement::RuleReference { name: r#"ws"#.to_string() },
                GrammarElement::RuleReference { name: r#"reference_expression"#.to_string() },
                GrammarElement::RuleReference { name: r#"ws"#.to_string() },
                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
            ] },
            line_number: 70,
        },
        GrammarRule {
            name: r#"prefixed_reference"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"REF_PREFIX"#.to_string() },
                GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::RuleReference { name: r#"a1_reference"#.to_string() },
                        GrammarElement::RuleReference { name: r#"name_reference"#.to_string() },
                        GrammarElement::RuleReference { name: r#"structure_reference"#.to_string() },
                    ] }) },
            ] },
            line_number: 71,
        },
        GrammarRule {
            name: r#"external_reference"#.to_string(),
            body: GrammarElement::TokenReference { name: r#"REF_PREFIX"#.to_string() },
            line_number: 72,
        },
        GrammarRule {
            name: r#"bang_reference"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"BANG"#.to_string() },
                GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::TokenReference { name: r#"CELL"#.to_string() },
                        GrammarElement::TokenReference { name: r#"COLUMN_REF"#.to_string() },
                        GrammarElement::TokenReference { name: r#"ROW_REF"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                    ] }) },
            ] },
            line_number: 73,
        },
        GrammarRule {
            name: r#"bang_name"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"BANG"#.to_string() },
                GrammarElement::RuleReference { name: r#"name_reference"#.to_string() },
            ] },
            line_number: 74,
        },
        GrammarRule {
            name: r#"name_reference"#.to_string(),
            body: GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
            line_number: 75,
        },
        GrammarRule {
            name: r#"column_reference"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"DOLLAR"#.to_string() }) },
                GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::TokenReference { name: r#"COLUMN_REF"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    ] }) },
            ] },
            line_number: 77,
        },
        GrammarRule {
            name: r#"row_reference"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"DOLLAR"#.to_string() }) },
                GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::TokenReference { name: r#"ROW_REF"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                    ] }) },
            ] },
            line_number: 78,
        },
        GrammarRule {
            name: r#"a1_reference"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"CELL"#.to_string() },
                GrammarElement::RuleReference { name: r#"column_reference"#.to_string() },
                GrammarElement::RuleReference { name: r#"row_reference"#.to_string() },
                GrammarElement::TokenReference { name: r#"COLUMN_REF"#.to_string() },
                GrammarElement::TokenReference { name: r#"ROW_REF"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
            ] },
            line_number: 80,
        },
        GrammarRule {
            name: r#"structure_reference"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"table_name"#.to_string() }) },
                GrammarElement::RuleReference { name: r#"intra_table_reference"#.to_string() },
            ] },
            line_number: 82,
        },
        GrammarRule {
            name: r#"table_name"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"TABLE_NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
            ] },
            line_number: 83,
        },
        GrammarRule {
            name: r#"intra_table_reference"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"STRUCTURED_KEYWORD"#.to_string() },
                GrammarElement::RuleReference { name: r#"structured_column_range"#.to_string() },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                    GrammarElement::RuleReference { name: r#"ws"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"inner_structure_reference"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"ws"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                ] },
            ] },
            line_number: 84,
        },
        GrammarRule {
            name: r#"inner_structure_reference"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"structured_keyword_list"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"ws"#.to_string() },
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"ws"#.to_string() },
                            GrammarElement::RuleReference { name: r#"structured_column_range"#.to_string() },
                        ] }) },
                ] },
                GrammarElement::RuleReference { name: r#"structured_column_range"#.to_string() },
            ] },
            line_number: 87,
        },
        GrammarRule {
            name: r#"structured_keyword_list"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"STRUCTURED_KEYWORD"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"ws"#.to_string() },
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::RuleReference { name: r#"ws"#.to_string() },
                        GrammarElement::TokenReference { name: r#"STRUCTURED_KEYWORD"#.to_string() },
                    ] }) },
            ] },
            line_number: 89,
        },
        GrammarRule {
            name: r#"structured_column_range"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"structured_column"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"ws"#.to_string() },
                        GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                        GrammarElement::RuleReference { name: r#"ws"#.to_string() },
                        GrammarElement::RuleReference { name: r#"structured_column"#.to_string() },
                    ] }) },
            ] },
            line_number: 90,
        },
        GrammarRule {
            name: r#"structured_column"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"STRUCTURED_COLUMN"#.to_string() },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"AT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STRUCTURED_COLUMN"#.to_string() },
                ] },
            ] },
            line_number: 91,
        },
    ],
        version: 1,
    }
}
