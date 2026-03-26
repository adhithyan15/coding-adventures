// AUTO-GENERATED FILE - DO NOT EDIT
#![allow(clippy::all)]
use grammar_tools::parser_grammar::{ParserGrammar, GrammarRule, GrammarElement};

pub fn LispGrammar() -> ParserGrammar {
    ParserGrammar {
        version: 1,
        rules: vec![
            GrammarRule {
                name: "program".to_string(),
                line_number: 2,
                body: GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: "sexpr".to_string() }) },
            },
            GrammarRule {
                name: "sexpr".to_string(),
                line_number: 3,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::RuleReference { name: "atom".to_string() }, GrammarElement::RuleReference { name: "list".to_string() }, GrammarElement::RuleReference { name: "quoted".to_string() }] },
            },
            GrammarRule {
                name: "atom".to_string(),
                line_number: 4,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "NUMBER".to_string() }, GrammarElement::TokenReference { name: "SYMBOL".to_string() }, GrammarElement::TokenReference { name: "STRING".to_string() }] },
            },
            GrammarRule {
                name: "list".to_string(),
                line_number: 5,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "LPAREN".to_string() }, GrammarElement::RuleReference { name: "list_body".to_string() }, GrammarElement::TokenReference { name: "RPAREN".to_string() }] },
            },
            GrammarRule {
                name: "list_body".to_string(),
                line_number: 6,
                body: GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "sexpr".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: "sexpr".to_string() }) }, GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "DOT".to_string() }, GrammarElement::RuleReference { name: "sexpr".to_string() }] }) }] }) },
            },
            GrammarRule {
                name: "quoted".to_string(),
                line_number: 7,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "QUOTE".to_string() }, GrammarElement::RuleReference { name: "sexpr".to_string() }] },
            },
        ],
    }
}
