// AUTO-GENERATED FILE - DO NOT EDIT
#![allow(clippy::all)]
use grammar_tools::parser_grammar::{ParserGrammar, GrammarRule, GrammarElement};

pub fn TomlGrammar() -> ParserGrammar {
    ParserGrammar {
        version: 1,
        rules: vec![
            GrammarRule {
                name: "document".to_string(),
                line_number: 38,
                body: GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "NEWLINE".to_string() }, GrammarElement::RuleReference { name: "expression".to_string() }] }) },
            },
            GrammarRule {
                name: "expression".to_string(),
                line_number: 49,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::RuleReference { name: "array_table_header".to_string() }, GrammarElement::RuleReference { name: "table_header".to_string() }, GrammarElement::RuleReference { name: "keyval".to_string() }] },
            },
            GrammarRule {
                name: "keyval".to_string(),
                line_number: 57,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "key".to_string() }, GrammarElement::TokenReference { name: "EQUALS".to_string() }, GrammarElement::RuleReference { name: "value".to_string() }] },
            },
            GrammarRule {
                name: "key".to_string(),
                line_number: 65,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "simple_key".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "DOT".to_string() }, GrammarElement::RuleReference { name: "simple_key".to_string() }] }) }] },
            },
            GrammarRule {
                name: "simple_key".to_string(),
                line_number: 82,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "BARE_KEY".to_string() }, GrammarElement::TokenReference { name: "BASIC_STRING".to_string() }, GrammarElement::TokenReference { name: "LITERAL_STRING".to_string() }, GrammarElement::TokenReference { name: "TRUE".to_string() }, GrammarElement::TokenReference { name: "FALSE".to_string() }, GrammarElement::TokenReference { name: "INTEGER".to_string() }, GrammarElement::TokenReference { name: "FLOAT".to_string() }, GrammarElement::TokenReference { name: "OFFSET_DATETIME".to_string() }, GrammarElement::TokenReference { name: "LOCAL_DATETIME".to_string() }, GrammarElement::TokenReference { name: "LOCAL_DATE".to_string() }, GrammarElement::TokenReference { name: "LOCAL_TIME".to_string() }] },
            },
            GrammarRule {
                name: "table_header".to_string(),
                line_number: 92,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "LBRACKET".to_string() }, GrammarElement::RuleReference { name: "key".to_string() }, GrammarElement::TokenReference { name: "RBRACKET".to_string() }] },
            },
            GrammarRule {
                name: "array_table_header".to_string(),
                line_number: 104,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "LBRACKET".to_string() }, GrammarElement::TokenReference { name: "LBRACKET".to_string() }, GrammarElement::RuleReference { name: "key".to_string() }, GrammarElement::TokenReference { name: "RBRACKET".to_string() }, GrammarElement::TokenReference { name: "RBRACKET".to_string() }] },
            },
            GrammarRule {
                name: "value".to_string(),
                line_number: 121,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "BASIC_STRING".to_string() }, GrammarElement::TokenReference { name: "ML_BASIC_STRING".to_string() }, GrammarElement::TokenReference { name: "LITERAL_STRING".to_string() }, GrammarElement::TokenReference { name: "ML_LITERAL_STRING".to_string() }, GrammarElement::TokenReference { name: "INTEGER".to_string() }, GrammarElement::TokenReference { name: "FLOAT".to_string() }, GrammarElement::TokenReference { name: "TRUE".to_string() }, GrammarElement::TokenReference { name: "FALSE".to_string() }, GrammarElement::TokenReference { name: "OFFSET_DATETIME".to_string() }, GrammarElement::TokenReference { name: "LOCAL_DATETIME".to_string() }, GrammarElement::TokenReference { name: "LOCAL_DATE".to_string() }, GrammarElement::TokenReference { name: "LOCAL_TIME".to_string() }, GrammarElement::RuleReference { name: "array".to_string() }, GrammarElement::RuleReference { name: "inline_table".to_string() }] },
            },
            GrammarRule {
                name: "array".to_string(),
                line_number: 140,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "LBRACKET".to_string() }, GrammarElement::RuleReference { name: "array_values".to_string() }, GrammarElement::TokenReference { name: "RBRACKET".to_string() }] },
            },
            GrammarRule {
                name: "array_values".to_string(),
                line_number: 142,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: "NEWLINE".to_string() }) }, GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "value".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: "NEWLINE".to_string() }) }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "COMMA".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: "NEWLINE".to_string() }) }, GrammarElement::RuleReference { name: "value".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: "NEWLINE".to_string() }) }] }) }, GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: "COMMA".to_string() }) }, GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: "NEWLINE".to_string() }) }] }) }] },
            },
            GrammarRule {
                name: "inline_table".to_string(),
                line_number: 162,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "LBRACE".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "keyval".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "COMMA".to_string() }, GrammarElement::RuleReference { name: "keyval".to_string() }] }) }] }) }, GrammarElement::TokenReference { name: "RBRACE".to_string() }] },
            },
        ],
    }
}
