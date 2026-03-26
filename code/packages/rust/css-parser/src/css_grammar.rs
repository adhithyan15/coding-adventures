// AUTO-GENERATED FILE - DO NOT EDIT
#![allow(clippy::all)]
use grammar_tools::parser_grammar::{ParserGrammar, GrammarRule, GrammarElement};

pub fn CssGrammar() -> ParserGrammar {
    ParserGrammar {
        version: 1,
        rules: vec![
            GrammarRule {
                name: "stylesheet".to_string(),
                line_number: 33,
                body: GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: "rule".to_string() }) },
            },
            GrammarRule {
                name: "rule".to_string(),
                line_number: 35,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::RuleReference { name: "at_rule".to_string() }, GrammarElement::RuleReference { name: "qualified_rule".to_string() }] },
            },
            GrammarRule {
                name: "at_rule".to_string(),
                line_number: 55,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "AT_KEYWORD".to_string() }, GrammarElement::RuleReference { name: "at_prelude".to_string() }, GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "SEMICOLON".to_string() }, GrammarElement::RuleReference { name: "block".to_string() }] }) }] },
            },
            GrammarRule {
                name: "at_prelude".to_string(),
                line_number: 61,
                body: GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: "at_prelude_token".to_string() }) },
            },
            GrammarRule {
                name: "at_prelude_token".to_string(),
                line_number: 63,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "IDENT".to_string() }, GrammarElement::TokenReference { name: "STRING".to_string() }, GrammarElement::TokenReference { name: "NUMBER".to_string() }, GrammarElement::TokenReference { name: "DIMENSION".to_string() }, GrammarElement::TokenReference { name: "PERCENTAGE".to_string() }, GrammarElement::TokenReference { name: "HASH".to_string() }, GrammarElement::TokenReference { name: "CUSTOM_PROPERTY".to_string() }, GrammarElement::TokenReference { name: "UNICODE_RANGE".to_string() }, GrammarElement::RuleReference { name: "function_in_prelude".to_string() }, GrammarElement::RuleReference { name: "paren_block".to_string() }, GrammarElement::TokenReference { name: "COLON".to_string() }, GrammarElement::TokenReference { name: "COMMA".to_string() }, GrammarElement::TokenReference { name: "SLASH".to_string() }, GrammarElement::TokenReference { name: "DOT".to_string() }, GrammarElement::TokenReference { name: "STAR".to_string() }, GrammarElement::TokenReference { name: "PLUS".to_string() }, GrammarElement::TokenReference { name: "MINUS".to_string() }, GrammarElement::TokenReference { name: "GREATER".to_string() }, GrammarElement::TokenReference { name: "TILDE".to_string() }, GrammarElement::TokenReference { name: "PIPE".to_string() }, GrammarElement::TokenReference { name: "EQUALS".to_string() }, GrammarElement::TokenReference { name: "AMPERSAND".to_string() }, GrammarElement::TokenReference { name: "CDO".to_string() }, GrammarElement::TokenReference { name: "CDC".to_string() }] },
            },
            GrammarRule {
                name: "function_in_prelude".to_string(),
                line_number: 71,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "FUNCTION".to_string() }, GrammarElement::RuleReference { name: "at_prelude_tokens".to_string() }, GrammarElement::TokenReference { name: "RPAREN".to_string() }] },
            },
            GrammarRule {
                name: "paren_block".to_string(),
                line_number: 72,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "LPAREN".to_string() }, GrammarElement::RuleReference { name: "at_prelude_tokens".to_string() }, GrammarElement::TokenReference { name: "RPAREN".to_string() }] },
            },
            GrammarRule {
                name: "at_prelude_tokens".to_string(),
                line_number: 73,
                body: GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: "at_prelude_token".to_string() }) },
            },
            GrammarRule {
                name: "qualified_rule".to_string(),
                line_number: 85,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "selector_list".to_string() }, GrammarElement::RuleReference { name: "block".to_string() }] },
            },
            GrammarRule {
                name: "selector_list".to_string(),
                line_number: 96,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "complex_selector".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "COMMA".to_string() }, GrammarElement::RuleReference { name: "complex_selector".to_string() }] }) }] },
            },
            GrammarRule {
                name: "complex_selector".to_string(),
                line_number: 105,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "compound_selector".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: "combinator".to_string() }) }, GrammarElement::RuleReference { name: "compound_selector".to_string() }] }) }] },
            },
            GrammarRule {
                name: "combinator".to_string(),
                line_number: 112,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "GREATER".to_string() }, GrammarElement::TokenReference { name: "PLUS".to_string() }, GrammarElement::TokenReference { name: "TILDE".to_string() }] },
            },
            GrammarRule {
                name: "compound_selector".to_string(),
                line_number: 124,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "simple_selector".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: "subclass_selector".to_string() }) }] }, GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "subclass_selector".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: "subclass_selector".to_string() }) }] }] },
            },
            GrammarRule {
                name: "simple_selector".to_string(),
                line_number: 131,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "IDENT".to_string() }, GrammarElement::TokenReference { name: "STAR".to_string() }, GrammarElement::TokenReference { name: "AMPERSAND".to_string() }] },
            },
            GrammarRule {
                name: "subclass_selector".to_string(),
                line_number: 139,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::RuleReference { name: "class_selector".to_string() }, GrammarElement::RuleReference { name: "id_selector".to_string() }, GrammarElement::RuleReference { name: "attribute_selector".to_string() }, GrammarElement::RuleReference { name: "pseudo_class".to_string() }, GrammarElement::RuleReference { name: "pseudo_element".to_string() }] },
            },
            GrammarRule {
                name: "class_selector".to_string(),
                line_number: 145,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "DOT".to_string() }, GrammarElement::TokenReference { name: "IDENT".to_string() }] },
            },
            GrammarRule {
                name: "id_selector".to_string(),
                line_number: 150,
                body: GrammarElement::TokenReference { name: "HASH".to_string() },
            },
            GrammarRule {
                name: "attribute_selector".to_string(),
                line_number: 161,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "LBRACKET".to_string() }, GrammarElement::TokenReference { name: "IDENT".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "attr_matcher".to_string() }, GrammarElement::RuleReference { name: "attr_value".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: "IDENT".to_string() }) }] }) }, GrammarElement::TokenReference { name: "RBRACKET".to_string() }] },
            },
            GrammarRule {
                name: "attr_matcher".to_string(),
                line_number: 163,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "EQUALS".to_string() }, GrammarElement::TokenReference { name: "TILDE_EQUALS".to_string() }, GrammarElement::TokenReference { name: "PIPE_EQUALS".to_string() }, GrammarElement::TokenReference { name: "CARET_EQUALS".to_string() }, GrammarElement::TokenReference { name: "DOLLAR_EQUALS".to_string() }, GrammarElement::TokenReference { name: "STAR_EQUALS".to_string() }] },
            },
            GrammarRule {
                name: "attr_value".to_string(),
                line_number: 166,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "IDENT".to_string() }, GrammarElement::TokenReference { name: "STRING".to_string() }] },
            },
            GrammarRule {
                name: "pseudo_class".to_string(),
                line_number: 173,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "COLON".to_string() }, GrammarElement::TokenReference { name: "FUNCTION".to_string() }, GrammarElement::RuleReference { name: "pseudo_class_args".to_string() }, GrammarElement::TokenReference { name: "RPAREN".to_string() }] }, GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "COLON".to_string() }, GrammarElement::TokenReference { name: "IDENT".to_string() }] }] },
            },
            GrammarRule {
                name: "pseudo_class_args".to_string(),
                line_number: 181,
                body: GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: "pseudo_class_arg".to_string() }) },
            },
            GrammarRule {
                name: "pseudo_class_arg".to_string(),
                line_number: 183,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "IDENT".to_string() }, GrammarElement::TokenReference { name: "NUMBER".to_string() }, GrammarElement::TokenReference { name: "DIMENSION".to_string() }, GrammarElement::TokenReference { name: "STRING".to_string() }, GrammarElement::TokenReference { name: "HASH".to_string() }, GrammarElement::TokenReference { name: "PLUS".to_string() }, GrammarElement::TokenReference { name: "COMMA".to_string() }, GrammarElement::TokenReference { name: "DOT".to_string() }, GrammarElement::TokenReference { name: "STAR".to_string() }, GrammarElement::TokenReference { name: "COLON".to_string() }, GrammarElement::TokenReference { name: "AMPERSAND".to_string() }, GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "FUNCTION".to_string() }, GrammarElement::RuleReference { name: "pseudo_class_args".to_string() }, GrammarElement::TokenReference { name: "RPAREN".to_string() }] }, GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "LBRACKET".to_string() }, GrammarElement::RuleReference { name: "pseudo_class_args".to_string() }, GrammarElement::TokenReference { name: "RBRACKET".to_string() }] }] },
            },
            GrammarRule {
                name: "pseudo_element".to_string(),
                line_number: 190,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "COLON_COLON".to_string() }, GrammarElement::TokenReference { name: "IDENT".to_string() }] },
            },
            GrammarRule {
                name: "block".to_string(),
                line_number: 200,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "LBRACE".to_string() }, GrammarElement::RuleReference { name: "block_contents".to_string() }, GrammarElement::TokenReference { name: "RBRACE".to_string() }] },
            },
            GrammarRule {
                name: "block_contents".to_string(),
                line_number: 202,
                body: GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: "block_item".to_string() }) },
            },
            GrammarRule {
                name: "block_item".to_string(),
                line_number: 211,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::RuleReference { name: "at_rule".to_string() }, GrammarElement::RuleReference { name: "declaration_or_nested".to_string() }] },
            },
            GrammarRule {
                name: "declaration_or_nested".to_string(),
                line_number: 217,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::RuleReference { name: "declaration".to_string() }, GrammarElement::RuleReference { name: "qualified_rule".to_string() }] },
            },
            GrammarRule {
                name: "declaration".to_string(),
                line_number: 231,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "property".to_string() }, GrammarElement::TokenReference { name: "COLON".to_string() }, GrammarElement::RuleReference { name: "value_list".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: "priority".to_string() }) }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }] },
            },
            GrammarRule {
                name: "property".to_string(),
                line_number: 233,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "IDENT".to_string() }, GrammarElement::TokenReference { name: "CUSTOM_PROPERTY".to_string() }] },
            },
            GrammarRule {
                name: "priority".to_string(),
                line_number: 238,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "BANG".to_string() }, GrammarElement::Literal { value: "important".to_string() }] },
            },
            GrammarRule {
                name: "value_list".to_string(),
                line_number: 251,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "value".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: "value".to_string() }) }] },
            },
            GrammarRule {
                name: "value".to_string(),
                line_number: 253,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "DIMENSION".to_string() }, GrammarElement::TokenReference { name: "PERCENTAGE".to_string() }, GrammarElement::TokenReference { name: "NUMBER".to_string() }, GrammarElement::TokenReference { name: "STRING".to_string() }, GrammarElement::TokenReference { name: "IDENT".to_string() }, GrammarElement::TokenReference { name: "HASH".to_string() }, GrammarElement::TokenReference { name: "CUSTOM_PROPERTY".to_string() }, GrammarElement::TokenReference { name: "UNICODE_RANGE".to_string() }, GrammarElement::RuleReference { name: "function_call".to_string() }, GrammarElement::TokenReference { name: "SLASH".to_string() }, GrammarElement::TokenReference { name: "COMMA".to_string() }, GrammarElement::TokenReference { name: "PLUS".to_string() }, GrammarElement::TokenReference { name: "MINUS".to_string() }] },
            },
            GrammarRule {
                name: "function_call".to_string(),
                line_number: 267,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "FUNCTION".to_string() }, GrammarElement::RuleReference { name: "function_args".to_string() }, GrammarElement::TokenReference { name: "RPAREN".to_string() }] }, GrammarElement::TokenReference { name: "URL_TOKEN".to_string() }] },
            },
            GrammarRule {
                name: "function_args".to_string(),
                line_number: 272,
                body: GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: "function_arg".to_string() }) },
            },
            GrammarRule {
                name: "function_arg".to_string(),
                line_number: 274,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "DIMENSION".to_string() }, GrammarElement::TokenReference { name: "PERCENTAGE".to_string() }, GrammarElement::TokenReference { name: "NUMBER".to_string() }, GrammarElement::TokenReference { name: "STRING".to_string() }, GrammarElement::TokenReference { name: "IDENT".to_string() }, GrammarElement::TokenReference { name: "HASH".to_string() }, GrammarElement::TokenReference { name: "CUSTOM_PROPERTY".to_string() }, GrammarElement::TokenReference { name: "COMMA".to_string() }, GrammarElement::TokenReference { name: "SLASH".to_string() }, GrammarElement::TokenReference { name: "PLUS".to_string() }, GrammarElement::TokenReference { name: "MINUS".to_string() }, GrammarElement::TokenReference { name: "STAR".to_string() }, GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "FUNCTION".to_string() }, GrammarElement::RuleReference { name: "function_args".to_string() }, GrammarElement::TokenReference { name: "RPAREN".to_string() }] }] },
            },
        ],
    }
}
