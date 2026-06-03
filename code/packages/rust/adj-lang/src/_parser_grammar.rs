// AUTO-GENERATED FILE — DO NOT EDIT
// Source: adj_lang.grammar
// Regenerate with: grammar-tools compile-grammar adj_lang.grammar
//
// This file embeds a ParserGrammar as native Rust data structures.
// Call `parser_grammar()` instead of reading and parsing the .grammar file.

use grammar_tools::parser_grammar::{GrammarElement, GrammarRule, ParserGrammar};

pub fn parser_grammar() -> ParserGrammar {
    ParserGrammar {
        rules: vec![
        GrammarRule {
            name: r#"program"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"statement"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"EOF"#.to_string() },
            ] },
            line_number: 20,
        },
        GrammarRule {
            name: r#"statement"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"prior_decl"#.to_string() },
                GrammarElement::RuleReference { name: r#"contributes_decl"#.to_string() },
                GrammarElement::RuleReference { name: r#"interacts_decl"#.to_string() },
                GrammarElement::RuleReference { name: r#"uncertain_decl"#.to_string() },
                GrammarElement::RuleReference { name: r#"observe_decl"#.to_string() },
                GrammarElement::RuleReference { name: r#"query_decl"#.to_string() },
            ] },
            line_number: 22,
        },
        GrammarRule {
            name: r#"prior_decl"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"prior"#.to_string() },
                GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                GrammarElement::Literal { value: r#"for"#.to_string() },
                GrammarElement::RuleReference { name: r#"term"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"annotation"#.to_string() }) },
            ] },
            line_number: 35,
        },
        GrammarRule {
            name: r#"contributes_decl"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"contributes"#.to_string() },
                GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                GrammarElement::Literal { value: r#"from"#.to_string() },
                GrammarElement::RuleReference { name: r#"term"#.to_string() },
                GrammarElement::Literal { value: r#"to"#.to_string() },
                GrammarElement::RuleReference { name: r#"term"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"annotation"#.to_string() }) },
            ] },
            line_number: 37,
        },
        GrammarRule {
            name: r#"interacts_decl"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"interacts"#.to_string() },
                GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                GrammarElement::Literal { value: r#"when"#.to_string() },
                GrammarElement::RuleReference { name: r#"term"#.to_string() },
                GrammarElement::Literal { value: r#"and"#.to_string() },
                GrammarElement::RuleReference { name: r#"term"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"and"#.to_string() },
                        GrammarElement::RuleReference { name: r#"term"#.to_string() },
                    ] }) },
                GrammarElement::Literal { value: r#"for"#.to_string() },
                GrammarElement::RuleReference { name: r#"term"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"annotation"#.to_string() }) },
            ] },
            line_number: 39,
        },
        GrammarRule {
            name: r#"uncertain_decl"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"uncertain"#.to_string() },
                GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                GrammarElement::RuleReference { name: r#"term"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::RuleReference { name: r#"term"#.to_string() },
                    ] }) },
                GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                GrammarElement::Literal { value: r#"for"#.to_string() },
                GrammarElement::RuleReference { name: r#"term"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"annotation"#.to_string() }) },
            ] },
            line_number: 41,
        },
        GrammarRule {
            name: r#"observe_decl"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"observe"#.to_string() },
                GrammarElement::RuleReference { name: r#"term"#.to_string() },
            ] },
            line_number: 43,
        },
        GrammarRule {
            name: r#"query_decl"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"QUESTION"#.to_string() },
                GrammarElement::RuleReference { name: r#"term"#.to_string() },
            ] },
            line_number: 45,
        },
        GrammarRule {
            name: r#"annotation"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"source_annotation"#.to_string() },
                GrammarElement::RuleReference { name: r#"locator_annotation"#.to_string() },
                GrammarElement::RuleReference { name: r#"trust_annotation"#.to_string() },
            ] },
            line_number: 53,
        },
        GrammarRule {
            name: r#"source_annotation"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"source"#.to_string() },
                GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
            ] },
            line_number: 58,
        },
        GrammarRule {
            name: r#"locator_annotation"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"locator"#.to_string() },
                GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
            ] },
            line_number: 59,
        },
        GrammarRule {
            name: r#"trust_annotation"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"trust"#.to_string() },
                GrammarElement::RuleReference { name: r#"trust_tier"#.to_string() },
            ] },
            line_number: 60,
        },
        GrammarRule {
            name: r#"trust_tier"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Literal { value: r#"consensus"#.to_string() },
                GrammarElement::Literal { value: r#"authoritative"#.to_string() },
                GrammarElement::Literal { value: r#"empirical"#.to_string() },
                GrammarElement::Literal { value: r#"inferred"#.to_string() },
                GrammarElement::Literal { value: r#"unattributed"#.to_string() },
            ] },
            line_number: 62,
        },
        GrammarRule {
            name: r#"term"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                        GrammarElement::RuleReference { name: r#"term"#.to_string() },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                GrammarElement::RuleReference { name: r#"term"#.to_string() },
                            ] }) },
                        GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                    ] }) },
            ] },
            line_number: 79,
        },
    ],
        version: 1,
    }
}
