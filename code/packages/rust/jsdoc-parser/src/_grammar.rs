// AUTO-GENERATED FILE — DO NOT EDIT
// Source: jsdoc.grammar
// Regenerate with: grammar-tools compile-grammar jsdoc.grammar
//
// This file embeds a ParserGrammar as native Rust data structures.
// Call `parser_grammar()` instead of reading and parsing the .grammar file.

use grammar_tools::parser_grammar::{GrammarElement, GrammarRule, ParserGrammar};

pub fn parser_grammar() -> ParserGrammar {
    ParserGrammar {
        rules: vec![
        GrammarRule {
            name: r#"document"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"description_line"#.to_string() }) },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"tag"#.to_string() }) },
            ] },
            line_number: 20,
        },
        GrammarRule {
            name: r#"description_line"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"DESCRIPTION_TEXT"#.to_string() },
                GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
            ] },
            line_number: 25,
        },
        GrammarRule {
            name: r#"tag"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"type_tag"#.to_string() },
                GrammarElement::RuleReference { name: r#"param_tag"#.to_string() },
                GrammarElement::RuleReference { name: r#"returns_tag"#.to_string() },
                GrammarElement::RuleReference { name: r#"unknown_tag"#.to_string() },
            ] },
            line_number: 43,
        },
        GrammarRule {
            name: r#"type_tag"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"AT_TAG"#.to_string() },
                GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
            ] },
            line_number: 51,
        },
        GrammarRule {
            name: r#"param_tag"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"AT_TAG"#.to_string() },
                GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                GrammarElement::RuleReference { name: r#"name_path"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"description_line_trailing"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
            ] },
            line_number: 57,
        },
        GrammarRule {
            name: r#"returns_tag"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"AT_TAG"#.to_string() },
                GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"description_line_trailing"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
            ] },
            line_number: 60,
        },
        GrammarRule {
            name: r#"unknown_tag"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"AT_TAG"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"tag_payload_token"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
            ] },
            line_number: 63,
        },
        GrammarRule {
            name: r#"tag_payload_token"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                GrammarElement::TokenReference { name: r#"ANGLE_OPEN"#.to_string() },
                GrammarElement::TokenReference { name: r#"ANGLE_CLOSE"#.to_string() },
                GrammarElement::TokenReference { name: r#"PIPE"#.to_string() },
                GrammarElement::TokenReference { name: r#"AMP"#.to_string() },
                GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                GrammarElement::TokenReference { name: r#"ELLIPSIS"#.to_string() },
                GrammarElement::TokenReference { name: r#"QUESTION"#.to_string() },
                GrammarElement::TokenReference { name: r#"BANG"#.to_string() },
                GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                GrammarElement::TokenReference { name: r#"DESCRIPTION_TEXT"#.to_string() },
            ] },
            line_number: 68,
        },
        GrammarRule {
            name: r#"description_line_trailing"#.to_string(),
            body: GrammarElement::TokenReference { name: r#"DESCRIPTION_TEXT"#.to_string() },
            line_number: 94,
        },
        GrammarRule {
            name: r#"type_expression"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                GrammarElement::RuleReference { name: r#"type"#.to_string() },
                GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
            ] },
            line_number: 106,
        },
        GrammarRule {
            name: r#"type"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"nullable_type"#.to_string() },
                GrammarElement::RuleReference { name: r#"non_nullable_type"#.to_string() },
                GrammarElement::RuleReference { name: r#"variadic_type"#.to_string() },
                GrammarElement::RuleReference { name: r#"optional_type"#.to_string() },
                GrammarElement::RuleReference { name: r#"primary_type"#.to_string() },
            ] },
            line_number: 111,
        },
        GrammarRule {
            name: r#"nullable_type"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"QUESTION"#.to_string() },
                GrammarElement::RuleReference { name: r#"primary_type"#.to_string() },
            ] },
            line_number: 118,
        },
        GrammarRule {
            name: r#"non_nullable_type"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"BANG"#.to_string() },
                GrammarElement::RuleReference { name: r#"primary_type"#.to_string() },
            ] },
            line_number: 121,
        },
        GrammarRule {
            name: r#"variadic_type"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"ELLIPSIS"#.to_string() },
                GrammarElement::RuleReference { name: r#"primary_type"#.to_string() },
            ] },
            line_number: 125,
        },
        GrammarRule {
            name: r#"optional_type"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"primary_type"#.to_string() },
                GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
            ] },
            line_number: 128,
        },
        GrammarRule {
            name: r#"primary_type"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"array_type"#.to_string() },
                GrammarElement::RuleReference { name: r#"nominal_type"#.to_string() },
                GrammarElement::RuleReference { name: r#"parenthesized_type"#.to_string() },
                GrammarElement::RuleReference { name: r#"wildcard_type"#.to_string() },
            ] },
            line_number: 132,
        },
        GrammarRule {
            name: r#"nominal_type"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    ] }) },
            ] },
            line_number: 138,
        },
        GrammarRule {
            name: r#"array_type"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"nominal_type"#.to_string() },
                GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                        GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                    ] }) },
            ] },
            line_number: 142,
        },
        GrammarRule {
            name: r#"parenthesized_type"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                GrammarElement::RuleReference { name: r#"type"#.to_string() },
                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
            ] },
            line_number: 146,
        },
        GrammarRule {
            name: r#"wildcard_type"#.to_string(),
            body: GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
            line_number: 149,
        },
        GrammarRule {
            name: r#"name_path"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"bracket_name"#.to_string() },
                GrammarElement::RuleReference { name: r#"dotted_name"#.to_string() },
            ] },
            line_number: 153,
        },
        GrammarRule {
            name: r#"dotted_name"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    ] }) },
            ] },
            line_number: 155,
        },
        GrammarRule {
            name: r#"bracket_name"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                        GrammarElement::RuleReference { name: r#"default_expr"#.to_string() },
                    ] }) },
                GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
            ] },
            line_number: 157,
        },
        GrammarRule {
            name: r#"default_expr"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
            ] },
            line_number: 161,
        },
    ],
        version: 1,
    }
}
