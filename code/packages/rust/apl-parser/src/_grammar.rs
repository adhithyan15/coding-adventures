// AUTO-GENERATED FILE — DO NOT EDIT
// Source: apl.grammar
// Regenerate with: grammar-tools compile-grammar apl.grammar
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
            line_number: 71,
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
            line_number: 73,
        },
        GrammarRule {
            name: r#"statement"#.to_string(),
            body: GrammarElement::RuleReference { name: r#"assignment"#.to_string() },
            line_number: 77,
        },
        GrammarRule {
            name: r#"assignment"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                    GrammarElement::RuleReference { name: r#"assignment"#.to_string() },
                ] },
                GrammarElement::RuleReference { name: r#"value_expr"#.to_string() },
            ] },
            line_number: 84,
        },
        GrammarRule {
            name: r#"value_expr"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"term"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"function_expr"#.to_string() },
                            GrammarElement::RuleReference { name: r#"value_expr"#.to_string() },
                        ] }) },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"function_expr"#.to_string() },
                    GrammarElement::RuleReference { name: r#"value_expr"#.to_string() },
                ] },
            ] },
            line_number: 95,
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
                    GrammarElement::RuleReference { name: r#"value_expr"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
            ] },
            line_number: 104,
        },
        GrammarRule {
            name: r#"function_atom"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                GrammarElement::TokenReference { name: r#"TIMES"#.to_string() },
                GrammarElement::TokenReference { name: r#"DIVIDE"#.to_string() },
                GrammarElement::TokenReference { name: r#"CEILING"#.to_string() },
                GrammarElement::TokenReference { name: r#"FLOOR"#.to_string() },
                GrammarElement::TokenReference { name: r#"RHO"#.to_string() },
                GrammarElement::TokenReference { name: r#"IOTA"#.to_string() },
                GrammarElement::TokenReference { name: r#"RAVEL"#.to_string() },
                GrammarElement::TokenReference { name: r#"EQ"#.to_string() },
                GrammarElement::TokenReference { name: r#"NE"#.to_string() },
                GrammarElement::TokenReference { name: r#"LT"#.to_string() },
                GrammarElement::TokenReference { name: r#"LE"#.to_string() },
                GrammarElement::TokenReference { name: r#"GE"#.to_string() },
                GrammarElement::TokenReference { name: r#"GT"#.to_string() },
            ] },
            line_number: 117,
        },
        GrammarRule {
            name: r#"function_expr"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"function_atom"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"REDUCE"#.to_string() },
                            GrammarElement::TokenReference { name: r#"SCAN"#.to_string() },
                        ] }) },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"OUTER"#.to_string() },
                    GrammarElement::RuleReference { name: r#"function_atom"#.to_string() },
                ] },
            ] },
            line_number: 129,
        },
    ],
        version: 1,
    }
}
