// AUTO-GENERATED FILE — DO NOT EDIT
// Source: q.grammar
// Regenerate with: grammar-tools compile-grammar q.grammar
//
// This file embeds a ParserGrammar as native Rust data structures.
// Call `parser_grammar()` instead of reading and parsing the .grammar file.

use grammar_tools::parser_grammar::{GrammarElement, GrammarRule, ParserGrammar};

pub fn parser_grammar() -> ParserGrammar {
    ParserGrammar {
        rules: vec![
        GrammarRule {
            name: r#"program"#.to_string(),
            body: GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"line"#.to_string() }) },
            line_number: 150,
        },
        GrammarRule {
            name: r#"line"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"statement"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                ] },
                GrammarElement::RuleReference { name: r#"statement"#.to_string() },
                GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
            ] },
            line_number: 152,
        },
        GrammarRule {
            name: r#"statement"#.to_string(),
            body: GrammarElement::RuleReference { name: r#"assignment"#.to_string() },
            line_number: 156,
        },
        GrammarRule {
            name: r#"assignment"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"assignment"#.to_string() },
                ] },
                GrammarElement::RuleReference { name: r#"noun_expr"#.to_string() },
            ] },
            line_number: 165,
        },
        GrammarRule {
            name: r#"noun_expr"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"term"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::Sequence { elements: vec![
                                GrammarElement::RuleReference { name: r#"verb_expr"#.to_string() },
                                GrammarElement::RuleReference { name: r#"noun_expr"#.to_string() },
                            ] },
                            GrammarElement::RuleReference { name: r#"noun_expr"#.to_string() },
                        ] }) },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"verb_expr"#.to_string() },
                    GrammarElement::RuleReference { name: r#"noun_expr"#.to_string() },
                ] },
            ] },
            line_number: 179,
        },
        GrammarRule {
            name: r#"term"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() }) },
                ] },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::RuleReference { name: r#"function_literal"#.to_string() },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"noun_expr"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                GrammarElement::RuleReference { name: r#"list_literal"#.to_string() },
            ] },
            line_number: 190,
        },
        GrammarRule {
            name: r#"list_literal"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                GrammarElement::RuleReference { name: r#"noun_expr"#.to_string() },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                GrammarElement::RuleReference { name: r#"noun_expr"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                        GrammarElement::RuleReference { name: r#"noun_expr"#.to_string() },
                    ] }) },
                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
            ] },
            line_number: 216,
        },
        GrammarRule {
            name: r#"function_literal"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                        GrammarElement::RuleReference { name: r#"param_list"#.to_string() },
                        GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                    ] }) },
                GrammarElement::RuleReference { name: r#"stmt_seq"#.to_string() },
                GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
            ] },
            line_number: 226,
        },
        GrammarRule {
            name: r#"param_list"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    ] }) },
            ] },
            line_number: 234,
        },
        GrammarRule {
            name: r#"stmt_seq"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"statement"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                        GrammarElement::RuleReference { name: r#"statement"#.to_string() },
                    ] }) },
            ] },
            line_number: 243,
        },
        GrammarRule {
            name: r#"verb_expr"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"verb_primitive"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"EACH"#.to_string() },
                            GrammarElement::TokenReference { name: r#"REDUCE"#.to_string() },
                            GrammarElement::TokenReference { name: r#"SCAN"#.to_string() },
                        ] }) },
                ] },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::RuleReference { name: r#"function_literal"#.to_string() },
            ] },
            line_number: 257,
        },
        GrammarRule {
            name: r#"verb_primitive"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                GrammarElement::TokenReference { name: r#"PERCENT"#.to_string() },
                GrammarElement::TokenReference { name: r#"BANG"#.to_string() },
                GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                GrammarElement::TokenReference { name: r#"HASH"#.to_string() },
                GrammarElement::TokenReference { name: r#"UNDERSCORE"#.to_string() },
                GrammarElement::TokenReference { name: r#"AMP"#.to_string() },
                GrammarElement::TokenReference { name: r#"PIPE"#.to_string() },
                GrammarElement::TokenReference { name: r#"TILDE"#.to_string() },
                GrammarElement::TokenReference { name: r#"EQ"#.to_string() },
                GrammarElement::TokenReference { name: r#"LT"#.to_string() },
                GrammarElement::TokenReference { name: r#"GT"#.to_string() },
                GrammarElement::TokenReference { name: r#"LE"#.to_string() },
                GrammarElement::TokenReference { name: r#"GE"#.to_string() },
                GrammarElement::TokenReference { name: r#"NE"#.to_string() },
            ] },
            line_number: 267,
        },
    ],
        version: 1,
    }
}
