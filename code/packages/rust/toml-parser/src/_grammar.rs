// AUTO-GENERATED FILE — DO NOT EDIT
// Source: toml.grammar
// Regenerate with: grammar-tools compile-grammar toml.grammar
//
// This file embeds a ParserGrammar as native Rust data structures.
// Call `parser_grammar()` instead of reading and parsing the .grammar file.

use grammar_tools::parser_grammar::{GrammarElement, GrammarRule, ParserGrammar};

pub fn parser_grammar() -> ParserGrammar {
    ParserGrammar {
        rules: vec![
        GrammarRule {
            name: r#"document"#.to_string(),
            body: GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] }) },
            line_number: 38,
        },
        GrammarRule {
            name: r#"expression"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"array_table_header"#.to_string() },
                GrammarElement::RuleReference { name: r#"table_header"#.to_string() },
                GrammarElement::RuleReference { name: r#"keyval"#.to_string() },
            ] },
            line_number: 49,
        },
        GrammarRule {
            name: r#"keyval"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"key"#.to_string() },
                GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                GrammarElement::RuleReference { name: r#"value"#.to_string() },
            ] },
            line_number: 57,
        },
        GrammarRule {
            name: r#"key"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"simple_key"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                        GrammarElement::RuleReference { name: r#"simple_key"#.to_string() },
                    ] }) },
            ] },
            line_number: 65,
        },
        GrammarRule {
            name: r#"simple_key"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"BARE_KEY"#.to_string() },
                GrammarElement::TokenReference { name: r#"BASIC_STRING"#.to_string() },
                GrammarElement::TokenReference { name: r#"LITERAL_STRING"#.to_string() },
                GrammarElement::TokenReference { name: r#"TRUE"#.to_string() },
                GrammarElement::TokenReference { name: r#"FALSE"#.to_string() },
                GrammarElement::TokenReference { name: r#"INTEGER"#.to_string() },
                GrammarElement::TokenReference { name: r#"FLOAT"#.to_string() },
                GrammarElement::TokenReference { name: r#"OFFSET_DATETIME"#.to_string() },
                GrammarElement::TokenReference { name: r#"LOCAL_DATETIME"#.to_string() },
                GrammarElement::TokenReference { name: r#"LOCAL_DATE"#.to_string() },
                GrammarElement::TokenReference { name: r#"LOCAL_TIME"#.to_string() },
            ] },
            line_number: 82,
        },
        GrammarRule {
            name: r#"table_header"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                GrammarElement::RuleReference { name: r#"key"#.to_string() },
                GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
            ] },
            line_number: 92,
        },
        GrammarRule {
            name: r#"array_table_header"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                GrammarElement::RuleReference { name: r#"key"#.to_string() },
                GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
            ] },
            line_number: 104,
        },
        GrammarRule {
            name: r#"value"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"BASIC_STRING"#.to_string() },
                GrammarElement::TokenReference { name: r#"ML_BASIC_STRING"#.to_string() },
                GrammarElement::TokenReference { name: r#"LITERAL_STRING"#.to_string() },
                GrammarElement::TokenReference { name: r#"ML_LITERAL_STRING"#.to_string() },
                GrammarElement::TokenReference { name: r#"INTEGER"#.to_string() },
                GrammarElement::TokenReference { name: r#"FLOAT"#.to_string() },
                GrammarElement::TokenReference { name: r#"TRUE"#.to_string() },
                GrammarElement::TokenReference { name: r#"FALSE"#.to_string() },
                GrammarElement::TokenReference { name: r#"OFFSET_DATETIME"#.to_string() },
                GrammarElement::TokenReference { name: r#"LOCAL_DATETIME"#.to_string() },
                GrammarElement::TokenReference { name: r#"LOCAL_DATE"#.to_string() },
                GrammarElement::TokenReference { name: r#"LOCAL_TIME"#.to_string() },
                GrammarElement::RuleReference { name: r#"array"#.to_string() },
                GrammarElement::RuleReference { name: r#"inline_table"#.to_string() },
            ] },
            line_number: 121,
        },
        GrammarRule {
            name: r#"array"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                GrammarElement::RuleReference { name: r#"array_values"#.to_string() },
                GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
            ] },
            line_number: 140,
        },
        GrammarRule {
            name: r#"array_values"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"value"#.to_string() },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                                GrammarElement::RuleReference { name: r#"value"#.to_string() },
                                GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                            ] }) },
                        GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    ] }) },
            ] },
            line_number: 142,
        },
        GrammarRule {
            name: r#"inline_table"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"keyval"#.to_string() },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                GrammarElement::RuleReference { name: r#"keyval"#.to_string() },
                            ] }) },
                    ] }) },
                GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
            ] },
            line_number: 162,
        },
    ],
        version: 1,
    }
}
