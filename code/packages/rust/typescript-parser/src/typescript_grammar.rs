// AUTO-GENERATED FILE - DO NOT EDIT
#![allow(clippy::all)]
use grammar_tools::parser_grammar::{ParserGrammar, GrammarRule, GrammarElement};

pub fn TypescriptGrammar() -> ParserGrammar {
    ParserGrammar {
        version: 1,
        rules: vec![
            GrammarRule {
                name: "program".to_string(),
                line_number: 29,
                body: GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: "statement".to_string() }) },
            },
            GrammarRule {
                name: "statement".to_string(),
                line_number: 30,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::RuleReference { name: "var_declaration".to_string() }, GrammarElement::RuleReference { name: "assignment".to_string() }, GrammarElement::RuleReference { name: "expression_stmt".to_string() }] },
            },
            GrammarRule {
                name: "var_declaration".to_string(),
                line_number: 31,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "KEYWORD".to_string() }, GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::TokenReference { name: "EQUALS".to_string() }, GrammarElement::RuleReference { name: "expression".to_string() }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }] },
            },
            GrammarRule {
                name: "assignment".to_string(),
                line_number: 32,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::TokenReference { name: "EQUALS".to_string() }, GrammarElement::RuleReference { name: "expression".to_string() }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }] },
            },
            GrammarRule {
                name: "expression_stmt".to_string(),
                line_number: 33,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "expression".to_string() }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }] },
            },
            GrammarRule {
                name: "expression".to_string(),
                line_number: 34,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "term".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "PLUS".to_string() }, GrammarElement::TokenReference { name: "MINUS".to_string() }] }) }, GrammarElement::RuleReference { name: "term".to_string() }] }) }] },
            },
            GrammarRule {
                name: "term".to_string(),
                line_number: 35,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "factor".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "STAR".to_string() }, GrammarElement::TokenReference { name: "SLASH".to_string() }] }) }, GrammarElement::RuleReference { name: "factor".to_string() }] }) }] },
            },
            GrammarRule {
                name: "factor".to_string(),
                line_number: 36,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "NUMBER".to_string() }, GrammarElement::TokenReference { name: "STRING".to_string() }, GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::TokenReference { name: "KEYWORD".to_string() }, GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "LPAREN".to_string() }, GrammarElement::RuleReference { name: "expression".to_string() }, GrammarElement::TokenReference { name: "RPAREN".to_string() }] }] },
            },
        ],
    }
}
