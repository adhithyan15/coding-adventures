// AUTO-GENERATED FILE — DO NOT EDIT
// Source: lattice.grammar
// Regenerate with: grammar-tools compile-grammar lattice.grammar
//
// This file embeds a ParserGrammar as native Rust data structures.
// Call `parser_grammar()` instead of reading and parsing the .grammar file.

use grammar_tools::parser_grammar::{GrammarElement, GrammarRule, ParserGrammar};

pub fn parser_grammar() -> ParserGrammar {
    ParserGrammar {
        rules: vec![
        GrammarRule {
            name: r#"stylesheet"#.to_string(),
            body: GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"rule"#.to_string() }) },
            line_number: 37,
        },
        GrammarRule {
            name: r#"rule"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"lattice_rule"#.to_string() },
                GrammarElement::RuleReference { name: r#"at_rule"#.to_string() },
                GrammarElement::RuleReference { name: r#"qualified_rule"#.to_string() },
            ] },
            line_number: 39,
        },
        GrammarRule {
            name: r#"lattice_rule"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"variable_declaration"#.to_string() },
                GrammarElement::RuleReference { name: r#"mixin_definition"#.to_string() },
                GrammarElement::RuleReference { name: r#"function_definition"#.to_string() },
                GrammarElement::RuleReference { name: r#"use_directive"#.to_string() },
                GrammarElement::RuleReference { name: r#"lattice_control"#.to_string() },
            ] },
            line_number: 51,
        },
        GrammarRule {
            name: r#"variable_declaration"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"VARIABLE"#.to_string() },
                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                GrammarElement::RuleReference { name: r#"value_list"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::TokenReference { name: r#"BANG_DEFAULT"#.to_string() },
                        GrammarElement::TokenReference { name: r#"BANG_GLOBAL"#.to_string() },
                    ] }) },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
            ] },
            line_number: 69,
        },
        GrammarRule {
            name: r#"mixin_definition"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"@mixin"#.to_string() },
                    GrammarElement::TokenReference { name: r#"FUNCTION"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"mixin_params"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"block"#.to_string() },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"@mixin"#.to_string() },
                    GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                    GrammarElement::RuleReference { name: r#"block"#.to_string() },
                ] },
            ] },
            line_number: 102,
        },
        GrammarRule {
            name: r#"mixin_params"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"mixin_param"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::RuleReference { name: r#"mixin_param"#.to_string() },
                    ] }) },
            ] },
            line_number: 105,
        },
        GrammarRule {
            name: r#"mixin_param"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"VARIABLE"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                        GrammarElement::RuleReference { name: r#"mixin_value_list"#.to_string() },
                    ] }) },
            ] },
            line_number: 112,
        },
        GrammarRule {
            name: r#"mixin_value_list"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"mixin_value"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"mixin_value"#.to_string() }) },
            ] },
            line_number: 117,
        },
        GrammarRule {
            name: r#"mixin_value"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"DIMENSION"#.to_string() },
                GrammarElement::TokenReference { name: r#"PERCENTAGE"#.to_string() },
                GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                GrammarElement::TokenReference { name: r#"HASH"#.to_string() },
                GrammarElement::TokenReference { name: r#"CUSTOM_PROPERTY"#.to_string() },
                GrammarElement::TokenReference { name: r#"UNICODE_RANGE"#.to_string() },
                GrammarElement::RuleReference { name: r#"function_call"#.to_string() },
                GrammarElement::TokenReference { name: r#"VARIABLE"#.to_string() },
                GrammarElement::TokenReference { name: r#"SLASH"#.to_string() },
                GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
            ] },
            line_number: 119,
        },
        GrammarRule {
            name: r#"include_directive"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"@include"#.to_string() },
                    GrammarElement::TokenReference { name: r#"FUNCTION"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"include_args"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                    GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"block"#.to_string() },
                        ] }) },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"@include"#.to_string() },
                    GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                    GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"block"#.to_string() },
                        ] }) },
                ] },
            ] },
            line_number: 130,
        },
        GrammarRule {
            name: r#"include_args"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"include_arg"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::RuleReference { name: r#"include_arg"#.to_string() },
                    ] }) },
            ] },
            line_number: 133,
        },
        GrammarRule {
            name: r#"include_arg"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"VARIABLE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"value_list"#.to_string() },
                ] },
                GrammarElement::RuleReference { name: r#"value_list"#.to_string() },
            ] },
            line_number: 137,
        },
        GrammarRule {
            name: r#"lattice_control"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"if_directive"#.to_string() },
                GrammarElement::RuleReference { name: r#"for_directive"#.to_string() },
                GrammarElement::RuleReference { name: r#"each_directive"#.to_string() },
                GrammarElement::RuleReference { name: r#"while_directive"#.to_string() },
            ] },
            line_number: 160,
        },
        GrammarRule {
            name: r#"if_directive"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"@if"#.to_string() },
                GrammarElement::RuleReference { name: r#"lattice_expression"#.to_string() },
                GrammarElement::RuleReference { name: r#"block"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"@else"#.to_string() },
                        GrammarElement::Literal { value: r#"if"#.to_string() },
                        GrammarElement::RuleReference { name: r#"lattice_expression"#.to_string() },
                        GrammarElement::RuleReference { name: r#"block"#.to_string() },
                    ] }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"@else"#.to_string() },
                        GrammarElement::RuleReference { name: r#"block"#.to_string() },
                    ] }) },
            ] },
            line_number: 164,
        },
        GrammarRule {
            name: r#"for_directive"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"@for"#.to_string() },
                GrammarElement::TokenReference { name: r#"VARIABLE"#.to_string() },
                GrammarElement::Literal { value: r#"from"#.to_string() },
                GrammarElement::RuleReference { name: r#"lattice_expression"#.to_string() },
                GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::Literal { value: r#"through"#.to_string() },
                        GrammarElement::Literal { value: r#"to"#.to_string() },
                    ] }) },
                GrammarElement::RuleReference { name: r#"lattice_expression"#.to_string() },
                GrammarElement::RuleReference { name: r#"block"#.to_string() },
            ] },
            line_number: 171,
        },
        GrammarRule {
            name: r#"each_directive"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"@each"#.to_string() },
                GrammarElement::TokenReference { name: r#"VARIABLE"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::TokenReference { name: r#"VARIABLE"#.to_string() },
                    ] }) },
                GrammarElement::Literal { value: r#"in"#.to_string() },
                GrammarElement::RuleReference { name: r#"each_list"#.to_string() },
                GrammarElement::RuleReference { name: r#"block"#.to_string() },
            ] },
            line_number: 176,
        },
        GrammarRule {
            name: r#"each_list"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"value"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::RuleReference { name: r#"value"#.to_string() },
                    ] }) },
            ] },
            line_number: 179,
        },
        GrammarRule {
            name: r#"while_directive"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"@while"#.to_string() },
                GrammarElement::RuleReference { name: r#"lattice_expression"#.to_string() },
                GrammarElement::RuleReference { name: r#"block"#.to_string() },
            ] },
            line_number: 184,
        },
        GrammarRule {
            name: r#"lattice_expression"#.to_string(),
            body: GrammarElement::RuleReference { name: r#"lattice_or_expr"#.to_string() },
            line_number: 203,
        },
        GrammarRule {
            name: r#"lattice_or_expr"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"lattice_and_expr"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"or"#.to_string() },
                        GrammarElement::RuleReference { name: r#"lattice_and_expr"#.to_string() },
                    ] }) },
            ] },
            line_number: 205,
        },
        GrammarRule {
            name: r#"lattice_and_expr"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"lattice_comparison"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"and"#.to_string() },
                        GrammarElement::RuleReference { name: r#"lattice_comparison"#.to_string() },
                    ] }) },
            ] },
            line_number: 207,
        },
        GrammarRule {
            name: r#"lattice_comparison"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"lattice_additive"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"comparison_op"#.to_string() },
                        GrammarElement::RuleReference { name: r#"lattice_additive"#.to_string() },
                    ] }) },
            ] },
            line_number: 209,
        },
        GrammarRule {
            name: r#"comparison_op"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"EQUALS_EQUALS"#.to_string() },
                GrammarElement::TokenReference { name: r#"NOT_EQUALS"#.to_string() },
                GrammarElement::TokenReference { name: r#"GREATER"#.to_string() },
                GrammarElement::TokenReference { name: r#"GREATER_EQUALS"#.to_string() },
                GrammarElement::TokenReference { name: r#"LESS"#.to_string() },
                GrammarElement::TokenReference { name: r#"LESS_EQUALS"#.to_string() },
            ] },
            line_number: 211,
        },
        GrammarRule {
            name: r#"lattice_additive"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"lattice_multiplicative"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                                GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"lattice_multiplicative"#.to_string() },
                    ] }) },
            ] },
            line_number: 214,
        },
        GrammarRule {
            name: r#"lattice_multiplicative"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"lattice_unary"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                                GrammarElement::TokenReference { name: r#"SLASH"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"lattice_unary"#.to_string() },
                    ] }) },
            ] },
            line_number: 219,
        },
        GrammarRule {
            name: r#"lattice_unary"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"lattice_unary"#.to_string() },
                ] },
                GrammarElement::RuleReference { name: r#"lattice_primary"#.to_string() },
            ] },
            line_number: 221,
        },
        GrammarRule {
            name: r#"lattice_primary"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"VARIABLE"#.to_string() },
                GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                GrammarElement::TokenReference { name: r#"DIMENSION"#.to_string() },
                GrammarElement::TokenReference { name: r#"PERCENTAGE"#.to_string() },
                GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                GrammarElement::TokenReference { name: r#"HASH"#.to_string() },
                GrammarElement::Literal { value: r#"true"#.to_string() },
                GrammarElement::Literal { value: r#"false"#.to_string() },
                GrammarElement::Literal { value: r#"null"#.to_string() },
                GrammarElement::RuleReference { name: r#"function_call"#.to_string() },
                GrammarElement::RuleReference { name: r#"map_literal"#.to_string() },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"lattice_expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
            ] },
            line_number: 224,
        },
        GrammarRule {
            name: r#"map_literal"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                GrammarElement::RuleReference { name: r#"map_entry"#.to_string() },
                GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                GrammarElement::RuleReference { name: r#"map_entry"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::RuleReference { name: r#"map_entry"#.to_string() },
                    ] }) },
                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
            ] },
            line_number: 235,
        },
        GrammarRule {
            name: r#"map_entry"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                        GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                    ] }) },
                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                GrammarElement::RuleReference { name: r#"lattice_expression"#.to_string() },
            ] },
            line_number: 237,
        },
        GrammarRule {
            name: r#"function_definition"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"@function"#.to_string() },
                    GrammarElement::TokenReference { name: r#"FUNCTION"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"mixin_params"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"function_body"#.to_string() },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"@function"#.to_string() },
                    GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                    GrammarElement::RuleReference { name: r#"function_body"#.to_string() },
                ] },
            ] },
            line_number: 261,
        },
        GrammarRule {
            name: r#"function_body"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"function_body_item"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
            ] },
            line_number: 264,
        },
        GrammarRule {
            name: r#"function_body_item"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"variable_declaration"#.to_string() },
                GrammarElement::RuleReference { name: r#"return_directive"#.to_string() },
                GrammarElement::RuleReference { name: r#"lattice_control"#.to_string() },
            ] },
            line_number: 266,
        },
        GrammarRule {
            name: r#"return_directive"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"@return"#.to_string() },
                GrammarElement::RuleReference { name: r#"lattice_expression"#.to_string() },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
            ] },
            line_number: 268,
        },
        GrammarRule {
            name: r#"use_directive"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"@use"#.to_string() },
                GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"as"#.to_string() },
                        GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                    ] }) },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
            ] },
            line_number: 281,
        },
        GrammarRule {
            name: r#"at_rule"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"AT_KEYWORD"#.to_string() },
                GrammarElement::RuleReference { name: r#"at_prelude"#.to_string() },
                GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                        GrammarElement::RuleReference { name: r#"block"#.to_string() },
                    ] }) },
            ] },
            line_number: 294,
        },
        GrammarRule {
            name: r#"at_prelude"#.to_string(),
            body: GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"at_prelude_token"#.to_string() }) },
            line_number: 296,
        },
        GrammarRule {
            name: r#"at_prelude_token"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                GrammarElement::TokenReference { name: r#"DIMENSION"#.to_string() },
                GrammarElement::TokenReference { name: r#"PERCENTAGE"#.to_string() },
                GrammarElement::TokenReference { name: r#"HASH"#.to_string() },
                GrammarElement::TokenReference { name: r#"CUSTOM_PROPERTY"#.to_string() },
                GrammarElement::TokenReference { name: r#"UNICODE_RANGE"#.to_string() },
                GrammarElement::TokenReference { name: r#"VARIABLE"#.to_string() },
                GrammarElement::RuleReference { name: r#"function_in_prelude"#.to_string() },
                GrammarElement::RuleReference { name: r#"paren_block"#.to_string() },
                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                GrammarElement::TokenReference { name: r#"SLASH"#.to_string() },
                GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                GrammarElement::TokenReference { name: r#"GREATER"#.to_string() },
                GrammarElement::TokenReference { name: r#"TILDE"#.to_string() },
                GrammarElement::TokenReference { name: r#"PIPE"#.to_string() },
                GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                GrammarElement::TokenReference { name: r#"AMPERSAND"#.to_string() },
                GrammarElement::TokenReference { name: r#"CDO"#.to_string() },
                GrammarElement::TokenReference { name: r#"CDC"#.to_string() },
            ] },
            line_number: 298,
        },
        GrammarRule {
            name: r#"function_in_prelude"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"FUNCTION"#.to_string() },
                GrammarElement::RuleReference { name: r#"at_prelude_tokens"#.to_string() },
                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
            ] },
            line_number: 306,
        },
        GrammarRule {
            name: r#"paren_block"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                GrammarElement::RuleReference { name: r#"at_prelude_tokens"#.to_string() },
                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
            ] },
            line_number: 307,
        },
        GrammarRule {
            name: r#"at_prelude_tokens"#.to_string(),
            body: GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"at_prelude_token"#.to_string() }) },
            line_number: 308,
        },
        GrammarRule {
            name: r#"qualified_rule"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"selector_list"#.to_string() },
                GrammarElement::RuleReference { name: r#"block"#.to_string() },
            ] },
            line_number: 314,
        },
        GrammarRule {
            name: r#"selector_list"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"complex_selector"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::RuleReference { name: r#"complex_selector"#.to_string() },
                    ] }) },
            ] },
            line_number: 320,
        },
        GrammarRule {
            name: r#"complex_selector"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"compound_selector"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"combinator"#.to_string() }) },
                        GrammarElement::RuleReference { name: r#"compound_selector"#.to_string() },
                    ] }) },
            ] },
            line_number: 322,
        },
        GrammarRule {
            name: r#"combinator"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"GREATER"#.to_string() },
                GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                GrammarElement::TokenReference { name: r#"TILDE"#.to_string() },
            ] },
            line_number: 324,
        },
        GrammarRule {
            name: r#"compound_selector"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"simple_selector"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"subclass_selector"#.to_string() }) },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"subclass_selector"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"subclass_selector"#.to_string() }) },
                ] },
            ] },
            line_number: 326,
        },
        GrammarRule {
            name: r#"simple_selector"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                GrammarElement::TokenReference { name: r#"AMPERSAND"#.to_string() },
                GrammarElement::TokenReference { name: r#"VARIABLE"#.to_string() },
                GrammarElement::TokenReference { name: r#"PERCENTAGE"#.to_string() },
            ] },
            line_number: 331,
        },
        GrammarRule {
            name: r#"subclass_selector"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"class_selector"#.to_string() },
                GrammarElement::RuleReference { name: r#"id_selector"#.to_string() },
                GrammarElement::RuleReference { name: r#"placeholder_selector"#.to_string() },
                GrammarElement::RuleReference { name: r#"attribute_selector"#.to_string() },
                GrammarElement::RuleReference { name: r#"pseudo_class"#.to_string() },
                GrammarElement::RuleReference { name: r#"pseudo_element"#.to_string() },
            ] },
            line_number: 334,
        },
        GrammarRule {
            name: r#"placeholder_selector"#.to_string(),
            body: GrammarElement::TokenReference { name: r#"PLACEHOLDER"#.to_string() },
            line_number: 338,
        },
        GrammarRule {
            name: r#"class_selector"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
            ] },
            line_number: 340,
        },
        GrammarRule {
            name: r#"id_selector"#.to_string(),
            body: GrammarElement::TokenReference { name: r#"HASH"#.to_string() },
            line_number: 342,
        },
        GrammarRule {
            name: r#"attribute_selector"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"attr_matcher"#.to_string() },
                        GrammarElement::RuleReference { name: r#"attr_value"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"IDENT"#.to_string() }) },
                    ] }) },
                GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
            ] },
            line_number: 344,
        },
        GrammarRule {
            name: r#"attr_matcher"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                GrammarElement::TokenReference { name: r#"TILDE_EQUALS"#.to_string() },
                GrammarElement::TokenReference { name: r#"PIPE_EQUALS"#.to_string() },
                GrammarElement::TokenReference { name: r#"CARET_EQUALS"#.to_string() },
                GrammarElement::TokenReference { name: r#"DOLLAR_EQUALS"#.to_string() },
                GrammarElement::TokenReference { name: r#"STAR_EQUALS"#.to_string() },
            ] },
            line_number: 346,
        },
        GrammarRule {
            name: r#"attr_value"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
            ] },
            line_number: 349,
        },
        GrammarRule {
            name: r#"pseudo_class"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"FUNCTION"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pseudo_class_args"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                ] },
            ] },
            line_number: 351,
        },
        GrammarRule {
            name: r#"pseudo_class_args"#.to_string(),
            body: GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"pseudo_class_arg"#.to_string() }) },
            line_number: 354,
        },
        GrammarRule {
            name: r#"pseudo_class_arg"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                GrammarElement::TokenReference { name: r#"DIMENSION"#.to_string() },
                GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                GrammarElement::TokenReference { name: r#"HASH"#.to_string() },
                GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                GrammarElement::TokenReference { name: r#"AMPERSAND"#.to_string() },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"FUNCTION"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pseudo_class_args"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pseudo_class_args"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                ] },
            ] },
            line_number: 356,
        },
        GrammarRule {
            name: r#"pseudo_element"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"COLON_COLON"#.to_string() },
                GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
            ] },
            line_number: 361,
        },
        GrammarRule {
            name: r#"block"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                GrammarElement::RuleReference { name: r#"block_contents"#.to_string() },
                GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
            ] },
            line_number: 371,
        },
        GrammarRule {
            name: r#"block_contents"#.to_string(),
            body: GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"block_item"#.to_string() }) },
            line_number: 373,
        },
        GrammarRule {
            name: r#"block_item"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"lattice_block_item"#.to_string() },
                GrammarElement::RuleReference { name: r#"at_rule"#.to_string() },
                GrammarElement::RuleReference { name: r#"declaration_or_nested"#.to_string() },
            ] },
            line_number: 375,
        },
        GrammarRule {
            name: r#"lattice_block_item"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"variable_declaration"#.to_string() },
                GrammarElement::RuleReference { name: r#"include_directive"#.to_string() },
                GrammarElement::RuleReference { name: r#"lattice_control"#.to_string() },
                GrammarElement::RuleReference { name: r#"content_directive"#.to_string() },
                GrammarElement::RuleReference { name: r#"extend_directive"#.to_string() },
                GrammarElement::RuleReference { name: r#"at_root_directive"#.to_string() },
            ] },
            line_number: 381,
        },
        GrammarRule {
            name: r#"content_directive"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"@content"#.to_string() },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
            ] },
            line_number: 391,
        },
        GrammarRule {
            name: r#"extend_directive"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"@extend"#.to_string() },
                GrammarElement::RuleReference { name: r#"selector_list"#.to_string() },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
            ] },
            line_number: 399,
        },
        GrammarRule {
            name: r#"at_root_directive"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"@at-root"#.to_string() },
                GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"selector_list"#.to_string() },
                            GrammarElement::RuleReference { name: r#"block"#.to_string() },
                        ] },
                        GrammarElement::RuleReference { name: r#"block"#.to_string() },
                    ] }) },
            ] },
            line_number: 404,
        },
        GrammarRule {
            name: r#"declaration_or_nested"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"declaration"#.to_string() },
                GrammarElement::RuleReference { name: r#"qualified_rule"#.to_string() },
            ] },
            line_number: 406,
        },
        GrammarRule {
            name: r#"declaration"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"property"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"value_list"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"priority"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"property"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"block"#.to_string() },
                ] },
            ] },
            line_number: 415,
        },
        GrammarRule {
            name: r#"property"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                GrammarElement::TokenReference { name: r#"CUSTOM_PROPERTY"#.to_string() },
            ] },
            line_number: 418,
        },
        GrammarRule {
            name: r#"priority"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"BANG"#.to_string() },
                GrammarElement::Literal { value: r#"important"#.to_string() },
            ] },
            line_number: 420,
        },
        GrammarRule {
            name: r#"value_list"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"value"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"value"#.to_string() }) },
            ] },
            line_number: 431,
        },
        GrammarRule {
            name: r#"value"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"DIMENSION"#.to_string() },
                GrammarElement::TokenReference { name: r#"PERCENTAGE"#.to_string() },
                GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                GrammarElement::TokenReference { name: r#"HASH"#.to_string() },
                GrammarElement::TokenReference { name: r#"CUSTOM_PROPERTY"#.to_string() },
                GrammarElement::TokenReference { name: r#"UNICODE_RANGE"#.to_string() },
                GrammarElement::RuleReference { name: r#"function_call"#.to_string() },
                GrammarElement::TokenReference { name: r#"VARIABLE"#.to_string() },
                GrammarElement::TokenReference { name: r#"SLASH"#.to_string() },
                GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                GrammarElement::RuleReference { name: r#"map_literal"#.to_string() },
            ] },
            line_number: 433,
        },
        GrammarRule {
            name: r#"function_call"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"FUNCTION"#.to_string() },
                    GrammarElement::RuleReference { name: r#"function_args"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                GrammarElement::TokenReference { name: r#"URL_TOKEN"#.to_string() },
            ] },
            line_number: 439,
        },
        GrammarRule {
            name: r#"function_args"#.to_string(),
            body: GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"function_arg"#.to_string() }) },
            line_number: 442,
        },
        GrammarRule {
            name: r#"function_arg"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"DIMENSION"#.to_string() },
                GrammarElement::TokenReference { name: r#"PERCENTAGE"#.to_string() },
                GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                GrammarElement::TokenReference { name: r#"HASH"#.to_string() },
                GrammarElement::TokenReference { name: r#"CUSTOM_PROPERTY"#.to_string() },
                GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                GrammarElement::TokenReference { name: r#"SLASH"#.to_string() },
                GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                GrammarElement::TokenReference { name: r#"VARIABLE"#.to_string() },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"FUNCTION"#.to_string() },
                    GrammarElement::RuleReference { name: r#"function_args"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
            ] },
            line_number: 444,
        },
    ],
        version: 1,
    }
}
