// AUTO-GENERATED FILE - DO NOT EDIT
#![allow(clippy::all)]
use grammar_tools::parser_grammar::{ParserGrammar, GrammarRule, GrammarElement};

pub fn JsonGrammar() -> ParserGrammar {
    ParserGrammar {
        version: 1,
        rules: vec![
            GrammarRule {
                name: "value".to_string(),
                line_number: 28,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::RuleReference { name: "object".to_string() }, GrammarElement::RuleReference { name: "array".to_string() }, GrammarElement::TokenReference { name: "STRING".to_string() }, GrammarElement::TokenReference { name: "NUMBER".to_string() }, GrammarElement::TokenReference { name: "TRUE".to_string() }, GrammarElement::TokenReference { name: "FALSE".to_string() }, GrammarElement::TokenReference { name: "NULL".to_string() }] },
            },
            GrammarRule {
                name: "object".to_string(),
                line_number: 34,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "LBRACE".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "pair".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "COMMA".to_string() }, GrammarElement::RuleReference { name: "pair".to_string() }] }) }] }) }, GrammarElement::TokenReference { name: "RBRACE".to_string() }] },
            },
            GrammarRule {
                name: "pair".to_string(),
                line_number: 38,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "STRING".to_string() }, GrammarElement::TokenReference { name: "COLON".to_string() }, GrammarElement::RuleReference { name: "value".to_string() }] },
            },
            GrammarRule {
                name: "array".to_string(),
                line_number: 42,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "LBRACKET".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "value".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "COMMA".to_string() }, GrammarElement::RuleReference { name: "value".to_string() }] }) }] }) }, GrammarElement::TokenReference { name: "RBRACKET".to_string() }] },
            },
        ],
    }
}
