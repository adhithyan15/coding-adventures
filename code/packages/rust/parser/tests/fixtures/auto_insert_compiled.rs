// AUTO-GENERATED FILE — DO NOT EDIT
// Source: auto_insert.grammar
// Regenerate with: grammar-tools compile-grammar auto_insert.grammar
//
// This file embeds a ParserGrammar as native Rust data structures.
// Call `parser_grammar()` instead of reading and parsing the .grammar file.

use grammar_tools::parser_grammar::{GrammarElement, GrammarRule, ParserGrammar};

pub fn parser_grammar() -> ParserGrammar {
    ParserGrammar {
        rules: vec![
        GrammarRule {
            name: r#"program"#.to_string(),
            body: GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"statement"#.to_string() }) },
            line_number: 2,
        },
        GrammarRule {
            name: r#"statement"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"block"#.to_string() },
                GrammarElement::RuleReference { name: r#"return_statement"#.to_string() },
                GrammarElement::RuleReference { name: r#"empty_statement"#.to_string() },
                GrammarElement::RuleReference { name: r#"expression_statement"#.to_string() },
            ] },
            line_number: 3,
        },
        GrammarRule {
            name: r#"block"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"statement"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
            ] },
            line_number: 4,
        },
        GrammarRule {
            name: r#"empty_statement"#.to_string(),
            body: GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
            line_number: 5,
        },
        GrammarRule {
            name: r#"return_statement"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"return"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::AdjacencyPredicate { predicate: grammar_tools::parser_grammar::AdjacencyPredicate::NoNewlineBefore },
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    ] }) },
                GrammarElement::RuleReference { name: r#"statement_end"#.to_string() },
            ] },
            line_number: 6,
        },
        GrammarRule {
            name: r#"expression_statement"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                GrammarElement::RuleReference { name: r#"statement_end"#.to_string() },
            ] },
            line_number: 7,
        },
        GrammarRule {
            name: r#"statement_end"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                GrammarElement::AutoInsert { token: r#"SEMICOLON"#.to_string(), closers: vec![r#"RBRACE"#.to_string()] },
            ] },
            line_number: 8,
        },
        GrammarRule {
            name: r#"expression"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"postfix"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                        GrammarElement::RuleReference { name: r#"postfix"#.to_string() },
                    ] }) },
            ] },
            line_number: 9,
        },
        GrammarRule {
            name: r#"postfix"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"primary"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"call_args"#.to_string() }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::AdjacencyPredicate { predicate: grammar_tools::parser_grammar::AdjacencyPredicate::NoNewlineBefore },
                        GrammarElement::Literal { value: r#"++"#.to_string() },
                    ] }) },
            ] },
            line_number: 10,
        },
        GrammarRule {
            name: r#"call_args"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"expression"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
            ] },
            line_number: 11,
        },
        GrammarRule {
            name: r#"primary"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
            ] },
            line_number: 12,
        },
    ],
        version: 0,
    }
}
