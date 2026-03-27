// AUTO-GENERATED FILE — DO NOT EDIT
// Source: lisp.grammar
// Regenerate with: grammar-tools compile-grammar lisp.grammar
//
// This file embeds a ParserGrammar as native Rust data structures.
// Call `parser_grammar()` instead of reading and parsing the .grammar file.

use grammar_tools::parser_grammar::{GrammarElement, GrammarRule, ParserGrammar};

pub fn parser_grammar() -> ParserGrammar {
    ParserGrammar {
        rules: vec![
        GrammarRule {
            name: r#"program"#.to_string(),
            body: GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"sexpr"#.to_string() }) },
            line_number: 2,
        },
        GrammarRule {
            name: r#"sexpr"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"atom"#.to_string() },
                GrammarElement::RuleReference { name: r#"list"#.to_string() },
                GrammarElement::RuleReference { name: r#"quoted"#.to_string() },
            ] },
            line_number: 3,
        },
        GrammarRule {
            name: r#"atom"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                GrammarElement::TokenReference { name: r#"SYMBOL"#.to_string() },
                GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
            ] },
            line_number: 4,
        },
        GrammarRule {
            name: r#"list"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                GrammarElement::RuleReference { name: r#"list_body"#.to_string() },
                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
            ] },
            line_number: 5,
        },
        GrammarRule {
            name: r#"list_body"#.to_string(),
            body: GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"sexpr"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"sexpr"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                            GrammarElement::RuleReference { name: r#"sexpr"#.to_string() },
                        ] }) },
                ] }) },
            line_number: 6,
        },
        GrammarRule {
            name: r#"quoted"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"QUOTE"#.to_string() },
                GrammarElement::RuleReference { name: r#"sexpr"#.to_string() },
            ] },
            line_number: 7,
        },
    ],
        version: 1,
    }
}
