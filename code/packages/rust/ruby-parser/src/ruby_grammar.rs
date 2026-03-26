// AUTO-GENERATED FILE - DO NOT EDIT
#![allow(clippy::all)]
use grammar_tools::parser_grammar::{ParserGrammar, GrammarRule, GrammarElement};

pub fn RubyGrammar() -> ParserGrammar {
    ParserGrammar {
        version: 1,
        rules: vec![
            GrammarRule {
                name: "program".to_string(),
                line_number: 22,
                body: GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: "statement".to_string() }) },
            },
            GrammarRule {
                name: "statement".to_string(),
                line_number: 23,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::RuleReference { name: "assignment".to_string() }, GrammarElement::RuleReference { name: "method_call".to_string() }, GrammarElement::RuleReference { name: "expression_stmt".to_string() }] },
            },
            GrammarRule {
                name: "assignment".to_string(),
                line_number: 24,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::TokenReference { name: "EQUALS".to_string() }, GrammarElement::RuleReference { name: "expression".to_string() }] },
            },
            GrammarRule {
                name: "method_call".to_string(),
                line_number: 25,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::TokenReference { name: "KEYWORD".to_string() }] }) }, GrammarElement::TokenReference { name: "LPAREN".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "expression".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "COMMA".to_string() }, GrammarElement::RuleReference { name: "expression".to_string() }] }) }] }) }, GrammarElement::TokenReference { name: "RPAREN".to_string() }] },
            },
            GrammarRule {
                name: "expression_stmt".to_string(),
                line_number: 26,
                body: GrammarElement::RuleReference { name: "expression".to_string() },
            },
            GrammarRule {
                name: "expression".to_string(),
                line_number: 27,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "term".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "PLUS".to_string() }, GrammarElement::TokenReference { name: "MINUS".to_string() }] }) }, GrammarElement::RuleReference { name: "term".to_string() }] }) }] },
            },
            GrammarRule {
                name: "term".to_string(),
                line_number: 28,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "factor".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "STAR".to_string() }, GrammarElement::TokenReference { name: "SLASH".to_string() }] }) }, GrammarElement::RuleReference { name: "factor".to_string() }] }) }] },
            },
            GrammarRule {
                name: "factor".to_string(),
                line_number: 29,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "NUMBER".to_string() }, GrammarElement::TokenReference { name: "STRING".to_string() }, GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::TokenReference { name: "KEYWORD".to_string() }, GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "LPAREN".to_string() }, GrammarElement::RuleReference { name: "expression".to_string() }, GrammarElement::TokenReference { name: "RPAREN".to_string() }] }] },
            },
        ],
    }
}
