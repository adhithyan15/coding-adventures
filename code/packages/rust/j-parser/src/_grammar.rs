// AUTO-GENERATED FILE — DO NOT EDIT
// Source: j.grammar
// Regenerate with: grammar-tools compile-grammar j.grammar
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
            line_number: 88,
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
            line_number: 90,
        },
        GrammarRule {
            name: r#"statement"#.to_string(),
            body: GrammarElement::RuleReference { name: r#"assignment"#.to_string() },
            line_number: 94,
        },
        GrammarRule {
            name: r#"assignment"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"ASSIGN_LOCAL"#.to_string() },
                    GrammarElement::RuleReference { name: r#"assignment"#.to_string() },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"ASSIGN_GLOBAL"#.to_string() },
                    GrammarElement::RuleReference { name: r#"assignment"#.to_string() },
                ] },
                GrammarElement::RuleReference { name: r#"noun_expr"#.to_string() },
            ] },
            line_number: 100,
        },
        GrammarRule {
            name: r#"noun_expr"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"term"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"verb_expr"#.to_string() },
                            GrammarElement::RuleReference { name: r#"noun_expr"#.to_string() },
                        ] }) },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"verb_expr"#.to_string() },
                    GrammarElement::RuleReference { name: r#"noun_expr"#.to_string() },
                ] },
            ] },
            line_number: 111,
        },
        GrammarRule {
            name: r#"term"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() }) },
                ] },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"noun_expr"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
            ] },
            line_number: 117,
        },
        GrammarRule {
            name: r#"verb_expr"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"simple_verb"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"AT"#.to_string() },
                            GrammarElement::RuleReference { name: r#"verb_expr"#.to_string() },
                        ] }) },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"verb_train"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
            ] },
            line_number: 129,
        },
        GrammarRule {
            name: r#"simple_verb"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"verb_primitive"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::TokenReference { name: r#"REDUCE"#.to_string() },
                        GrammarElement::TokenReference { name: r#"SCAN"#.to_string() },
                    ] }) },
            ] },
            line_number: 138,
        },
        GrammarRule {
            name: r#"verb_primitive"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                GrammarElement::TokenReference { name: r#"PERCENT"#.to_string() },
                GrammarElement::TokenReference { name: r#"CARET"#.to_string() },
                GrammarElement::TokenReference { name: r#"FLOOR"#.to_string() },
                GrammarElement::TokenReference { name: r#"CEILING"#.to_string() },
                GrammarElement::TokenReference { name: r#"DOLLAR"#.to_string() },
                GrammarElement::TokenReference { name: r#"IDOT"#.to_string() },
                GrammarElement::TokenReference { name: r#"RAVEL"#.to_string() },
                GrammarElement::TokenReference { name: r#"HASH"#.to_string() },
                GrammarElement::TokenReference { name: r#"EQ"#.to_string() },
                GrammarElement::TokenReference { name: r#"NE"#.to_string() },
                GrammarElement::TokenReference { name: r#"LT"#.to_string() },
                GrammarElement::TokenReference { name: r#"GT"#.to_string() },
                GrammarElement::TokenReference { name: r#"LE"#.to_string() },
                GrammarElement::TokenReference { name: r#"GE"#.to_string() },
            ] },
            line_number: 140,
        },
        GrammarRule {
            name: r#"verb_train"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"train_tooth"#.to_string() },
                GrammarElement::RuleReference { name: r#"train_tooth"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"train_tooth"#.to_string() }) },
            ] },
            line_number: 148,
        },
        GrammarRule {
            name: r#"train_tooth"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"verb_expr"#.to_string() },
                GrammarElement::RuleReference { name: r#"term"#.to_string() },
            ] },
            line_number: 155,
        },
    ],
        version: 1,
    }
}
