// AUTO-GENERATED FILE — DO NOT EDIT
// Source: css.grammar
// Regenerate with: grammar-tools compile-grammar css.grammar
//
// This file embeds a ParserGrammar as native Rust data structures.
// Call `parser_grammar()` instead of reading and parsing the .grammar file.

use grammar_tools::parser_grammar::{GrammarElement, GrammarRule, ParserGrammar};

pub fn parser_grammar() -> ParserGrammar {
    ParserGrammar {
        rules: vec![
        GrammarRule {
            name: r#"stylesheet"#.to_string(),
            body: GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"rule"#.to_string() }) },
            line_number: 33,
        },
        GrammarRule {
            name: r#"rule"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"at_rule"#.to_string() },
                GrammarElement::RuleReference { name: r#"qualified_rule"#.to_string() },
            ] },
            line_number: 35,
        },
        GrammarRule {
            name: r#"at_rule"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"AT_KEYWORD"#.to_string() },
                GrammarElement::RuleReference { name: r#"at_prelude"#.to_string() },
                GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                        GrammarElement::RuleReference { name: r#"block"#.to_string() },
                    ] }) },
            ] },
            line_number: 55,
        },
        GrammarRule {
            name: r#"at_prelude"#.to_string(),
            body: GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"at_prelude_token"#.to_string() }) },
            line_number: 61,
        },
        GrammarRule {
            name: r#"at_prelude_token"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                GrammarElement::TokenReference { name: r#"DIMENSION"#.to_string() },
                GrammarElement::TokenReference { name: r#"PERCENTAGE"#.to_string() },
                GrammarElement::TokenReference { name: r#"HASH"#.to_string() },
                GrammarElement::TokenReference { name: r#"CUSTOM_PROPERTY"#.to_string() },
                GrammarElement::TokenReference { name: r#"UNICODE_RANGE"#.to_string() },
                GrammarElement::RuleReference { name: r#"function_in_prelude"#.to_string() },
                GrammarElement::RuleReference { name: r#"paren_block"#.to_string() },
                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                GrammarElement::TokenReference { name: r#"SLASH"#.to_string() },
                GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                GrammarElement::TokenReference { name: r#"GREATER"#.to_string() },
                GrammarElement::TokenReference { name: r#"TILDE"#.to_string() },
                GrammarElement::TokenReference { name: r#"PIPE"#.to_string() },
                GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                GrammarElement::TokenReference { name: r#"AMPERSAND"#.to_string() },
                GrammarElement::TokenReference { name: r#"CDO"#.to_string() },
                GrammarElement::TokenReference { name: r#"CDC"#.to_string() },
            ] },
            line_number: 63,
        },
        GrammarRule {
            name: r#"function_in_prelude"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"FUNCTION"#.to_string() },
                GrammarElement::RuleReference { name: r#"at_prelude_tokens"#.to_string() },
                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
            ] },
            line_number: 71,
        },
        GrammarRule {
            name: r#"paren_block"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                GrammarElement::RuleReference { name: r#"at_prelude_tokens"#.to_string() },
                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
            ] },
            line_number: 72,
        },
        GrammarRule {
            name: r#"at_prelude_tokens"#.to_string(),
            body: GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"at_prelude_token"#.to_string() }) },
            line_number: 73,
        },
        GrammarRule {
            name: r#"qualified_rule"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"selector_list"#.to_string() },
                GrammarElement::RuleReference { name: r#"block"#.to_string() },
            ] },
            line_number: 85,
        },
        GrammarRule {
            name: r#"selector_list"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"complex_selector"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::RuleReference { name: r#"complex_selector"#.to_string() },
                    ] }) },
            ] },
            line_number: 96,
        },
        GrammarRule {
            name: r#"complex_selector"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"compound_selector"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"combinator"#.to_string() }) },
                        GrammarElement::RuleReference { name: r#"compound_selector"#.to_string() },
                    ] }) },
            ] },
            line_number: 105,
        },
        GrammarRule {
            name: r#"combinator"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"GREATER"#.to_string() },
                GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                GrammarElement::TokenReference { name: r#"TILDE"#.to_string() },
            ] },
            line_number: 112,
        },
        GrammarRule {
            name: r#"compound_selector"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"simple_selector"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"subclass_selector"#.to_string() }) },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"subclass_selector"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"subclass_selector"#.to_string() }) },
                ] },
            ] },
            line_number: 124,
        },
        GrammarRule {
            name: r#"simple_selector"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                GrammarElement::TokenReference { name: r#"AMPERSAND"#.to_string() },
            ] },
            line_number: 131,
        },
        GrammarRule {
            name: r#"subclass_selector"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"class_selector"#.to_string() },
                GrammarElement::RuleReference { name: r#"id_selector"#.to_string() },
                GrammarElement::RuleReference { name: r#"attribute_selector"#.to_string() },
                GrammarElement::RuleReference { name: r#"pseudo_class"#.to_string() },
                GrammarElement::RuleReference { name: r#"pseudo_element"#.to_string() },
            ] },
            line_number: 139,
        },
        GrammarRule {
            name: r#"class_selector"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
            ] },
            line_number: 145,
        },
        GrammarRule {
            name: r#"id_selector"#.to_string(),
            body: GrammarElement::TokenReference { name: r#"HASH"#.to_string() },
            line_number: 150,
        },
        GrammarRule {
            name: r#"attribute_selector"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"attr_matcher"#.to_string() },
                        GrammarElement::RuleReference { name: r#"attr_value"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"IDENT"#.to_string() }) },
                    ] }) },
                GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
            ] },
            line_number: 161,
        },
        GrammarRule {
            name: r#"attr_matcher"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                GrammarElement::TokenReference { name: r#"TILDE_EQUALS"#.to_string() },
                GrammarElement::TokenReference { name: r#"PIPE_EQUALS"#.to_string() },
                GrammarElement::TokenReference { name: r#"CARET_EQUALS"#.to_string() },
                GrammarElement::TokenReference { name: r#"DOLLAR_EQUALS"#.to_string() },
                GrammarElement::TokenReference { name: r#"STAR_EQUALS"#.to_string() },
            ] },
            line_number: 163,
        },
        GrammarRule {
            name: r#"attr_value"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
            ] },
            line_number: 166,
        },
        GrammarRule {
            name: r#"pseudo_class"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"FUNCTION"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pseudo_class_args"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                ] },
            ] },
            line_number: 173,
        },
        GrammarRule {
            name: r#"pseudo_class_args"#.to_string(),
            body: GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"pseudo_class_arg"#.to_string() }) },
            line_number: 181,
        },
        GrammarRule {
            name: r#"pseudo_class_arg"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                GrammarElement::TokenReference { name: r#"DIMENSION"#.to_string() },
                GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                GrammarElement::TokenReference { name: r#"HASH"#.to_string() },
                GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                GrammarElement::TokenReference { name: r#"AMPERSAND"#.to_string() },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"FUNCTION"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pseudo_class_args"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pseudo_class_args"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                ] },
            ] },
            line_number: 183,
        },
        GrammarRule {
            name: r#"pseudo_element"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"COLON_COLON"#.to_string() },
                GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
            ] },
            line_number: 190,
        },
        GrammarRule {
            name: r#"block"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                GrammarElement::RuleReference { name: r#"block_contents"#.to_string() },
                GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
            ] },
            line_number: 200,
        },
        GrammarRule {
            name: r#"block_contents"#.to_string(),
            body: GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"block_item"#.to_string() }) },
            line_number: 202,
        },
        GrammarRule {
            name: r#"block_item"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"at_rule"#.to_string() },
                GrammarElement::RuleReference { name: r#"declaration_or_nested"#.to_string() },
            ] },
            line_number: 211,
        },
        GrammarRule {
            name: r#"declaration_or_nested"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"declaration"#.to_string() },
                GrammarElement::RuleReference { name: r#"qualified_rule"#.to_string() },
            ] },
            line_number: 217,
        },
        GrammarRule {
            name: r#"declaration"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"property"#.to_string() },
                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                GrammarElement::RuleReference { name: r#"value_list"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"priority"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
            ] },
            line_number: 231,
        },
        GrammarRule {
            name: r#"property"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                GrammarElement::TokenReference { name: r#"CUSTOM_PROPERTY"#.to_string() },
            ] },
            line_number: 233,
        },
        GrammarRule {
            name: r#"priority"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"BANG"#.to_string() },
                GrammarElement::Literal { value: r#"important"#.to_string() },
            ] },
            line_number: 238,
        },
        GrammarRule {
            name: r#"value_list"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"value"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"value"#.to_string() }) },
            ] },
            line_number: 251,
        },
        GrammarRule {
            name: r#"value"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"DIMENSION"#.to_string() },
                GrammarElement::TokenReference { name: r#"PERCENTAGE"#.to_string() },
                GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                GrammarElement::TokenReference { name: r#"HASH"#.to_string() },
                GrammarElement::TokenReference { name: r#"CUSTOM_PROPERTY"#.to_string() },
                GrammarElement::TokenReference { name: r#"UNICODE_RANGE"#.to_string() },
                GrammarElement::RuleReference { name: r#"function_call"#.to_string() },
                GrammarElement::TokenReference { name: r#"SLASH"#.to_string() },
                GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
            ] },
            line_number: 253,
        },
        GrammarRule {
            name: r#"function_call"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"FUNCTION"#.to_string() },
                    GrammarElement::RuleReference { name: r#"function_args"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                GrammarElement::TokenReference { name: r#"URL_TOKEN"#.to_string() },
            ] },
            line_number: 267,
        },
        GrammarRule {
            name: r#"function_args"#.to_string(),
            body: GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"function_arg"#.to_string() }) },
            line_number: 272,
        },
        GrammarRule {
            name: r#"function_arg"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"DIMENSION"#.to_string() },
                GrammarElement::TokenReference { name: r#"PERCENTAGE"#.to_string() },
                GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                GrammarElement::TokenReference { name: r#"HASH"#.to_string() },
                GrammarElement::TokenReference { name: r#"CUSTOM_PROPERTY"#.to_string() },
                GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                GrammarElement::TokenReference { name: r#"SLASH"#.to_string() },
                GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"FUNCTION"#.to_string() },
                    GrammarElement::RuleReference { name: r#"function_args"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
            ] },
            line_number: 274,
        },
    ],
        version: 1,
    }
}
