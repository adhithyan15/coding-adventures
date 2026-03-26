// AUTO-GENERATED FILE - DO NOT EDIT
#![allow(clippy::all)]
use grammar_tools::parser_grammar::{ParserGrammar, GrammarRule, GrammarElement};

pub fn LatticeGrammar() -> ParserGrammar {
    ParserGrammar {
        version: 1,
        rules: vec![
            GrammarRule {
                name: "stylesheet".to_string(),
                line_number: 37,
                body: GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: "rule".to_string() }) },
            },
            GrammarRule {
                name: "rule".to_string(),
                line_number: 39,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::RuleReference { name: "lattice_rule".to_string() }, GrammarElement::RuleReference { name: "at_rule".to_string() }, GrammarElement::RuleReference { name: "qualified_rule".to_string() }] },
            },
            GrammarRule {
                name: "lattice_rule".to_string(),
                line_number: 51,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::RuleReference { name: "variable_declaration".to_string() }, GrammarElement::RuleReference { name: "mixin_definition".to_string() }, GrammarElement::RuleReference { name: "function_definition".to_string() }, GrammarElement::RuleReference { name: "use_directive".to_string() }, GrammarElement::RuleReference { name: "lattice_control".to_string() }] },
            },
            GrammarRule {
                name: "variable_declaration".to_string(),
                line_number: 69,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "VARIABLE".to_string() }, GrammarElement::TokenReference { name: "COLON".to_string() }, GrammarElement::RuleReference { name: "value_list".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "BANG_DEFAULT".to_string() }, GrammarElement::TokenReference { name: "BANG_GLOBAL".to_string() }] }) }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }] },
            },
            GrammarRule {
                name: "mixin_definition".to_string(),
                line_number: 102,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "@mixin".to_string() }, GrammarElement::TokenReference { name: "FUNCTION".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: "mixin_params".to_string() }) }, GrammarElement::TokenReference { name: "RPAREN".to_string() }, GrammarElement::RuleReference { name: "block".to_string() }] }, GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "@mixin".to_string() }, GrammarElement::TokenReference { name: "IDENT".to_string() }, GrammarElement::RuleReference { name: "block".to_string() }] }] },
            },
            GrammarRule {
                name: "mixin_params".to_string(),
                line_number: 105,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "mixin_param".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "COMMA".to_string() }, GrammarElement::RuleReference { name: "mixin_param".to_string() }] }) }] },
            },
            GrammarRule {
                name: "mixin_param".to_string(),
                line_number: 112,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "VARIABLE".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "COLON".to_string() }, GrammarElement::RuleReference { name: "mixin_value_list".to_string() }] }) }] },
            },
            GrammarRule {
                name: "mixin_value_list".to_string(),
                line_number: 117,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "mixin_value".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: "mixin_value".to_string() }) }] },
            },
            GrammarRule {
                name: "mixin_value".to_string(),
                line_number: 119,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "DIMENSION".to_string() }, GrammarElement::TokenReference { name: "PERCENTAGE".to_string() }, GrammarElement::TokenReference { name: "NUMBER".to_string() }, GrammarElement::TokenReference { name: "STRING".to_string() }, GrammarElement::TokenReference { name: "IDENT".to_string() }, GrammarElement::TokenReference { name: "HASH".to_string() }, GrammarElement::TokenReference { name: "CUSTOM_PROPERTY".to_string() }, GrammarElement::TokenReference { name: "UNICODE_RANGE".to_string() }, GrammarElement::RuleReference { name: "function_call".to_string() }, GrammarElement::TokenReference { name: "VARIABLE".to_string() }, GrammarElement::TokenReference { name: "SLASH".to_string() }, GrammarElement::TokenReference { name: "PLUS".to_string() }, GrammarElement::TokenReference { name: "MINUS".to_string() }] },
            },
            GrammarRule {
                name: "include_directive".to_string(),
                line_number: 130,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "@include".to_string() }, GrammarElement::TokenReference { name: "FUNCTION".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: "include_args".to_string() }) }, GrammarElement::TokenReference { name: "RPAREN".to_string() }, GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "SEMICOLON".to_string() }, GrammarElement::RuleReference { name: "block".to_string() }] }) }] }, GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "@include".to_string() }, GrammarElement::TokenReference { name: "IDENT".to_string() }, GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "SEMICOLON".to_string() }, GrammarElement::RuleReference { name: "block".to_string() }] }) }] }] },
            },
            GrammarRule {
                name: "include_args".to_string(),
                line_number: 133,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "include_arg".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "COMMA".to_string() }, GrammarElement::RuleReference { name: "include_arg".to_string() }] }) }] },
            },
            GrammarRule {
                name: "include_arg".to_string(),
                line_number: 137,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "VARIABLE".to_string() }, GrammarElement::TokenReference { name: "COLON".to_string() }, GrammarElement::RuleReference { name: "value_list".to_string() }] }, GrammarElement::RuleReference { name: "value_list".to_string() }] },
            },
            GrammarRule {
                name: "lattice_control".to_string(),
                line_number: 160,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::RuleReference { name: "if_directive".to_string() }, GrammarElement::RuleReference { name: "for_directive".to_string() }, GrammarElement::RuleReference { name: "each_directive".to_string() }, GrammarElement::RuleReference { name: "while_directive".to_string() }] },
            },
            GrammarRule {
                name: "if_directive".to_string(),
                line_number: 164,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "@if".to_string() }, GrammarElement::RuleReference { name: "lattice_expression".to_string() }, GrammarElement::RuleReference { name: "block".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "@else".to_string() }, GrammarElement::Literal { value: "if".to_string() }, GrammarElement::RuleReference { name: "lattice_expression".to_string() }, GrammarElement::RuleReference { name: "block".to_string() }] }) }, GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "@else".to_string() }, GrammarElement::RuleReference { name: "block".to_string() }] }) }] },
            },
            GrammarRule {
                name: "for_directive".to_string(),
                line_number: 171,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "@for".to_string() }, GrammarElement::TokenReference { name: "VARIABLE".to_string() }, GrammarElement::Literal { value: "from".to_string() }, GrammarElement::RuleReference { name: "lattice_expression".to_string() }, GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![GrammarElement::Literal { value: "through".to_string() }, GrammarElement::Literal { value: "to".to_string() }] }) }, GrammarElement::RuleReference { name: "lattice_expression".to_string() }, GrammarElement::RuleReference { name: "block".to_string() }] },
            },
            GrammarRule {
                name: "each_directive".to_string(),
                line_number: 176,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "@each".to_string() }, GrammarElement::TokenReference { name: "VARIABLE".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "COMMA".to_string() }, GrammarElement::TokenReference { name: "VARIABLE".to_string() }] }) }, GrammarElement::Literal { value: "in".to_string() }, GrammarElement::RuleReference { name: "each_list".to_string() }, GrammarElement::RuleReference { name: "block".to_string() }] },
            },
            GrammarRule {
                name: "each_list".to_string(),
                line_number: 179,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "value".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "COMMA".to_string() }, GrammarElement::RuleReference { name: "value".to_string() }] }) }] },
            },
            GrammarRule {
                name: "while_directive".to_string(),
                line_number: 184,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "@while".to_string() }, GrammarElement::RuleReference { name: "lattice_expression".to_string() }, GrammarElement::RuleReference { name: "block".to_string() }] },
            },
            GrammarRule {
                name: "lattice_expression".to_string(),
                line_number: 203,
                body: GrammarElement::RuleReference { name: "lattice_or_expr".to_string() },
            },
            GrammarRule {
                name: "lattice_or_expr".to_string(),
                line_number: 205,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "lattice_and_expr".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "or".to_string() }, GrammarElement::RuleReference { name: "lattice_and_expr".to_string() }] }) }] },
            },
            GrammarRule {
                name: "lattice_and_expr".to_string(),
                line_number: 207,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "lattice_comparison".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "and".to_string() }, GrammarElement::RuleReference { name: "lattice_comparison".to_string() }] }) }] },
            },
            GrammarRule {
                name: "lattice_comparison".to_string(),
                line_number: 209,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "lattice_additive".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "comparison_op".to_string() }, GrammarElement::RuleReference { name: "lattice_additive".to_string() }] }) }] },
            },
            GrammarRule {
                name: "comparison_op".to_string(),
                line_number: 211,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "EQUALS_EQUALS".to_string() }, GrammarElement::TokenReference { name: "NOT_EQUALS".to_string() }, GrammarElement::TokenReference { name: "GREATER".to_string() }, GrammarElement::TokenReference { name: "GREATER_EQUALS".to_string() }, GrammarElement::TokenReference { name: "LESS".to_string() }, GrammarElement::TokenReference { name: "LESS_EQUALS".to_string() }] },
            },
            GrammarRule {
                name: "lattice_additive".to_string(),
                line_number: 214,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "lattice_multiplicative".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "PLUS".to_string() }, GrammarElement::TokenReference { name: "MINUS".to_string() }] }) }, GrammarElement::RuleReference { name: "lattice_multiplicative".to_string() }] }) }] },
            },
            GrammarRule {
                name: "lattice_multiplicative".to_string(),
                line_number: 219,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "lattice_unary".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "STAR".to_string() }, GrammarElement::TokenReference { name: "SLASH".to_string() }] }) }, GrammarElement::RuleReference { name: "lattice_unary".to_string() }] }) }] },
            },
            GrammarRule {
                name: "lattice_unary".to_string(),
                line_number: 221,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "MINUS".to_string() }, GrammarElement::RuleReference { name: "lattice_unary".to_string() }] }, GrammarElement::RuleReference { name: "lattice_primary".to_string() }] },
            },
            GrammarRule {
                name: "lattice_primary".to_string(),
                line_number: 224,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "VARIABLE".to_string() }, GrammarElement::TokenReference { name: "NUMBER".to_string() }, GrammarElement::TokenReference { name: "DIMENSION".to_string() }, GrammarElement::TokenReference { name: "PERCENTAGE".to_string() }, GrammarElement::TokenReference { name: "STRING".to_string() }, GrammarElement::TokenReference { name: "IDENT".to_string() }, GrammarElement::TokenReference { name: "HASH".to_string() }, GrammarElement::Literal { value: "true".to_string() }, GrammarElement::Literal { value: "false".to_string() }, GrammarElement::Literal { value: "null".to_string() }, GrammarElement::RuleReference { name: "function_call".to_string() }, GrammarElement::RuleReference { name: "map_literal".to_string() }, GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "LPAREN".to_string() }, GrammarElement::RuleReference { name: "lattice_expression".to_string() }, GrammarElement::TokenReference { name: "RPAREN".to_string() }] }] },
            },
            GrammarRule {
                name: "map_literal".to_string(),
                line_number: 235,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "LPAREN".to_string() }, GrammarElement::RuleReference { name: "map_entry".to_string() }, GrammarElement::TokenReference { name: "COMMA".to_string() }, GrammarElement::RuleReference { name: "map_entry".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "COMMA".to_string() }, GrammarElement::RuleReference { name: "map_entry".to_string() }] }) }, GrammarElement::TokenReference { name: "RPAREN".to_string() }] },
            },
            GrammarRule {
                name: "map_entry".to_string(),
                line_number: 237,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "IDENT".to_string() }, GrammarElement::TokenReference { name: "STRING".to_string() }] }) }, GrammarElement::TokenReference { name: "COLON".to_string() }, GrammarElement::RuleReference { name: "lattice_expression".to_string() }] },
            },
            GrammarRule {
                name: "function_definition".to_string(),
                line_number: 261,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "@function".to_string() }, GrammarElement::TokenReference { name: "FUNCTION".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: "mixin_params".to_string() }) }, GrammarElement::TokenReference { name: "RPAREN".to_string() }, GrammarElement::RuleReference { name: "function_body".to_string() }] }, GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "@function".to_string() }, GrammarElement::TokenReference { name: "IDENT".to_string() }, GrammarElement::RuleReference { name: "function_body".to_string() }] }] },
            },
            GrammarRule {
                name: "function_body".to_string(),
                line_number: 264,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "LBRACE".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: "function_body_item".to_string() }) }, GrammarElement::TokenReference { name: "RBRACE".to_string() }] },
            },
            GrammarRule {
                name: "function_body_item".to_string(),
                line_number: 266,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::RuleReference { name: "variable_declaration".to_string() }, GrammarElement::RuleReference { name: "return_directive".to_string() }, GrammarElement::RuleReference { name: "lattice_control".to_string() }] },
            },
            GrammarRule {
                name: "return_directive".to_string(),
                line_number: 268,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "@return".to_string() }, GrammarElement::RuleReference { name: "lattice_expression".to_string() }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }] },
            },
            GrammarRule {
                name: "use_directive".to_string(),
                line_number: 281,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "@use".to_string() }, GrammarElement::TokenReference { name: "STRING".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "as".to_string() }, GrammarElement::TokenReference { name: "IDENT".to_string() }] }) }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }] },
            },
            GrammarRule {
                name: "at_rule".to_string(),
                line_number: 294,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "AT_KEYWORD".to_string() }, GrammarElement::RuleReference { name: "at_prelude".to_string() }, GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "SEMICOLON".to_string() }, GrammarElement::RuleReference { name: "block".to_string() }] }) }] },
            },
            GrammarRule {
                name: "at_prelude".to_string(),
                line_number: 296,
                body: GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: "at_prelude_token".to_string() }) },
            },
            GrammarRule {
                name: "at_prelude_token".to_string(),
                line_number: 298,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "IDENT".to_string() }, GrammarElement::TokenReference { name: "STRING".to_string() }, GrammarElement::TokenReference { name: "NUMBER".to_string() }, GrammarElement::TokenReference { name: "DIMENSION".to_string() }, GrammarElement::TokenReference { name: "PERCENTAGE".to_string() }, GrammarElement::TokenReference { name: "HASH".to_string() }, GrammarElement::TokenReference { name: "CUSTOM_PROPERTY".to_string() }, GrammarElement::TokenReference { name: "UNICODE_RANGE".to_string() }, GrammarElement::TokenReference { name: "VARIABLE".to_string() }, GrammarElement::RuleReference { name: "function_in_prelude".to_string() }, GrammarElement::RuleReference { name: "paren_block".to_string() }, GrammarElement::TokenReference { name: "COLON".to_string() }, GrammarElement::TokenReference { name: "COMMA".to_string() }, GrammarElement::TokenReference { name: "SLASH".to_string() }, GrammarElement::TokenReference { name: "DOT".to_string() }, GrammarElement::TokenReference { name: "STAR".to_string() }, GrammarElement::TokenReference { name: "PLUS".to_string() }, GrammarElement::TokenReference { name: "MINUS".to_string() }, GrammarElement::TokenReference { name: "GREATER".to_string() }, GrammarElement::TokenReference { name: "TILDE".to_string() }, GrammarElement::TokenReference { name: "PIPE".to_string() }, GrammarElement::TokenReference { name: "EQUALS".to_string() }, GrammarElement::TokenReference { name: "AMPERSAND".to_string() }, GrammarElement::TokenReference { name: "CDO".to_string() }, GrammarElement::TokenReference { name: "CDC".to_string() }] },
            },
            GrammarRule {
                name: "function_in_prelude".to_string(),
                line_number: 306,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "FUNCTION".to_string() }, GrammarElement::RuleReference { name: "at_prelude_tokens".to_string() }, GrammarElement::TokenReference { name: "RPAREN".to_string() }] },
            },
            GrammarRule {
                name: "paren_block".to_string(),
                line_number: 307,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "LPAREN".to_string() }, GrammarElement::RuleReference { name: "at_prelude_tokens".to_string() }, GrammarElement::TokenReference { name: "RPAREN".to_string() }] },
            },
            GrammarRule {
                name: "at_prelude_tokens".to_string(),
                line_number: 308,
                body: GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: "at_prelude_token".to_string() }) },
            },
            GrammarRule {
                name: "qualified_rule".to_string(),
                line_number: 314,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "selector_list".to_string() }, GrammarElement::RuleReference { name: "block".to_string() }] },
            },
            GrammarRule {
                name: "selector_list".to_string(),
                line_number: 320,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "complex_selector".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "COMMA".to_string() }, GrammarElement::RuleReference { name: "complex_selector".to_string() }] }) }] },
            },
            GrammarRule {
                name: "complex_selector".to_string(),
                line_number: 322,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "compound_selector".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: "combinator".to_string() }) }, GrammarElement::RuleReference { name: "compound_selector".to_string() }] }) }] },
            },
            GrammarRule {
                name: "combinator".to_string(),
                line_number: 324,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "GREATER".to_string() }, GrammarElement::TokenReference { name: "PLUS".to_string() }, GrammarElement::TokenReference { name: "TILDE".to_string() }] },
            },
            GrammarRule {
                name: "compound_selector".to_string(),
                line_number: 326,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "simple_selector".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: "subclass_selector".to_string() }) }] }, GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "subclass_selector".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: "subclass_selector".to_string() }) }] }] },
            },
            GrammarRule {
                name: "simple_selector".to_string(),
                line_number: 330,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "IDENT".to_string() }, GrammarElement::TokenReference { name: "STAR".to_string() }, GrammarElement::TokenReference { name: "AMPERSAND".to_string() }, GrammarElement::TokenReference { name: "VARIABLE".to_string() }] },
            },
            GrammarRule {
                name: "subclass_selector".to_string(),
                line_number: 333,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::RuleReference { name: "class_selector".to_string() }, GrammarElement::RuleReference { name: "id_selector".to_string() }, GrammarElement::RuleReference { name: "placeholder_selector".to_string() }, GrammarElement::RuleReference { name: "attribute_selector".to_string() }, GrammarElement::RuleReference { name: "pseudo_class".to_string() }, GrammarElement::RuleReference { name: "pseudo_element".to_string() }] },
            },
            GrammarRule {
                name: "placeholder_selector".to_string(),
                line_number: 337,
                body: GrammarElement::TokenReference { name: "PLACEHOLDER".to_string() },
            },
            GrammarRule {
                name: "class_selector".to_string(),
                line_number: 339,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "DOT".to_string() }, GrammarElement::TokenReference { name: "IDENT".to_string() }] },
            },
            GrammarRule {
                name: "id_selector".to_string(),
                line_number: 341,
                body: GrammarElement::TokenReference { name: "HASH".to_string() },
            },
            GrammarRule {
                name: "attribute_selector".to_string(),
                line_number: 343,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "LBRACKET".to_string() }, GrammarElement::TokenReference { name: "IDENT".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "attr_matcher".to_string() }, GrammarElement::RuleReference { name: "attr_value".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: "IDENT".to_string() }) }] }) }, GrammarElement::TokenReference { name: "RBRACKET".to_string() }] },
            },
            GrammarRule {
                name: "attr_matcher".to_string(),
                line_number: 345,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "EQUALS".to_string() }, GrammarElement::TokenReference { name: "TILDE_EQUALS".to_string() }, GrammarElement::TokenReference { name: "PIPE_EQUALS".to_string() }, GrammarElement::TokenReference { name: "CARET_EQUALS".to_string() }, GrammarElement::TokenReference { name: "DOLLAR_EQUALS".to_string() }, GrammarElement::TokenReference { name: "STAR_EQUALS".to_string() }] },
            },
            GrammarRule {
                name: "attr_value".to_string(),
                line_number: 348,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "IDENT".to_string() }, GrammarElement::TokenReference { name: "STRING".to_string() }] },
            },
            GrammarRule {
                name: "pseudo_class".to_string(),
                line_number: 350,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "COLON".to_string() }, GrammarElement::TokenReference { name: "FUNCTION".to_string() }, GrammarElement::RuleReference { name: "pseudo_class_args".to_string() }, GrammarElement::TokenReference { name: "RPAREN".to_string() }] }, GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "COLON".to_string() }, GrammarElement::TokenReference { name: "IDENT".to_string() }] }] },
            },
            GrammarRule {
                name: "pseudo_class_args".to_string(),
                line_number: 353,
                body: GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: "pseudo_class_arg".to_string() }) },
            },
            GrammarRule {
                name: "pseudo_class_arg".to_string(),
                line_number: 355,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "IDENT".to_string() }, GrammarElement::TokenReference { name: "NUMBER".to_string() }, GrammarElement::TokenReference { name: "DIMENSION".to_string() }, GrammarElement::TokenReference { name: "STRING".to_string() }, GrammarElement::TokenReference { name: "HASH".to_string() }, GrammarElement::TokenReference { name: "PLUS".to_string() }, GrammarElement::TokenReference { name: "COMMA".to_string() }, GrammarElement::TokenReference { name: "DOT".to_string() }, GrammarElement::TokenReference { name: "STAR".to_string() }, GrammarElement::TokenReference { name: "COLON".to_string() }, GrammarElement::TokenReference { name: "AMPERSAND".to_string() }, GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "FUNCTION".to_string() }, GrammarElement::RuleReference { name: "pseudo_class_args".to_string() }, GrammarElement::TokenReference { name: "RPAREN".to_string() }] }, GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "LBRACKET".to_string() }, GrammarElement::RuleReference { name: "pseudo_class_args".to_string() }, GrammarElement::TokenReference { name: "RBRACKET".to_string() }] }] },
            },
            GrammarRule {
                name: "pseudo_element".to_string(),
                line_number: 360,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "COLON_COLON".to_string() }, GrammarElement::TokenReference { name: "IDENT".to_string() }] },
            },
            GrammarRule {
                name: "block".to_string(),
                line_number: 370,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "LBRACE".to_string() }, GrammarElement::RuleReference { name: "block_contents".to_string() }, GrammarElement::TokenReference { name: "RBRACE".to_string() }] },
            },
            GrammarRule {
                name: "block_contents".to_string(),
                line_number: 372,
                body: GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: "block_item".to_string() }) },
            },
            GrammarRule {
                name: "block_item".to_string(),
                line_number: 374,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::RuleReference { name: "lattice_block_item".to_string() }, GrammarElement::RuleReference { name: "at_rule".to_string() }, GrammarElement::RuleReference { name: "declaration_or_nested".to_string() }] },
            },
            GrammarRule {
                name: "lattice_block_item".to_string(),
                line_number: 380,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::RuleReference { name: "variable_declaration".to_string() }, GrammarElement::RuleReference { name: "include_directive".to_string() }, GrammarElement::RuleReference { name: "lattice_control".to_string() }, GrammarElement::RuleReference { name: "content_directive".to_string() }, GrammarElement::RuleReference { name: "extend_directive".to_string() }, GrammarElement::RuleReference { name: "at_root_directive".to_string() }] },
            },
            GrammarRule {
                name: "content_directive".to_string(),
                line_number: 390,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "@content".to_string() }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }] },
            },
            GrammarRule {
                name: "extend_directive".to_string(),
                line_number: 398,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "@extend".to_string() }, GrammarElement::RuleReference { name: "selector_list".to_string() }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }] },
            },
            GrammarRule {
                name: "at_root_directive".to_string(),
                line_number: 403,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "@at-root".to_string() }, GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "selector_list".to_string() }, GrammarElement::RuleReference { name: "block".to_string() }] }, GrammarElement::RuleReference { name: "block".to_string() }] }) }] },
            },
            GrammarRule {
                name: "declaration_or_nested".to_string(),
                line_number: 405,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::RuleReference { name: "declaration".to_string() }, GrammarElement::RuleReference { name: "qualified_rule".to_string() }] },
            },
            GrammarRule {
                name: "declaration".to_string(),
                line_number: 414,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "property".to_string() }, GrammarElement::TokenReference { name: "COLON".to_string() }, GrammarElement::RuleReference { name: "value_list".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: "priority".to_string() }) }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }] }, GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "property".to_string() }, GrammarElement::TokenReference { name: "COLON".to_string() }, GrammarElement::RuleReference { name: "block".to_string() }] }] },
            },
            GrammarRule {
                name: "property".to_string(),
                line_number: 417,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "IDENT".to_string() }, GrammarElement::TokenReference { name: "CUSTOM_PROPERTY".to_string() }] },
            },
            GrammarRule {
                name: "priority".to_string(),
                line_number: 419,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "BANG".to_string() }, GrammarElement::Literal { value: "important".to_string() }] },
            },
            GrammarRule {
                name: "value_list".to_string(),
                line_number: 430,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "value".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: "value".to_string() }) }] },
            },
            GrammarRule {
                name: "value".to_string(),
                line_number: 432,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "DIMENSION".to_string() }, GrammarElement::TokenReference { name: "PERCENTAGE".to_string() }, GrammarElement::TokenReference { name: "NUMBER".to_string() }, GrammarElement::TokenReference { name: "STRING".to_string() }, GrammarElement::TokenReference { name: "IDENT".to_string() }, GrammarElement::TokenReference { name: "HASH".to_string() }, GrammarElement::TokenReference { name: "CUSTOM_PROPERTY".to_string() }, GrammarElement::TokenReference { name: "UNICODE_RANGE".to_string() }, GrammarElement::RuleReference { name: "function_call".to_string() }, GrammarElement::TokenReference { name: "VARIABLE".to_string() }, GrammarElement::TokenReference { name: "SLASH".to_string() }, GrammarElement::TokenReference { name: "COMMA".to_string() }, GrammarElement::TokenReference { name: "PLUS".to_string() }, GrammarElement::TokenReference { name: "MINUS".to_string() }, GrammarElement::RuleReference { name: "map_literal".to_string() }] },
            },
            GrammarRule {
                name: "function_call".to_string(),
                line_number: 438,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "FUNCTION".to_string() }, GrammarElement::RuleReference { name: "function_args".to_string() }, GrammarElement::TokenReference { name: "RPAREN".to_string() }] }, GrammarElement::TokenReference { name: "URL_TOKEN".to_string() }] },
            },
            GrammarRule {
                name: "function_args".to_string(),
                line_number: 441,
                body: GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: "function_arg".to_string() }) },
            },
            GrammarRule {
                name: "function_arg".to_string(),
                line_number: 443,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "DIMENSION".to_string() }, GrammarElement::TokenReference { name: "PERCENTAGE".to_string() }, GrammarElement::TokenReference { name: "NUMBER".to_string() }, GrammarElement::TokenReference { name: "STRING".to_string() }, GrammarElement::TokenReference { name: "IDENT".to_string() }, GrammarElement::TokenReference { name: "HASH".to_string() }, GrammarElement::TokenReference { name: "CUSTOM_PROPERTY".to_string() }, GrammarElement::TokenReference { name: "COMMA".to_string() }, GrammarElement::TokenReference { name: "SLASH".to_string() }, GrammarElement::TokenReference { name: "PLUS".to_string() }, GrammarElement::TokenReference { name: "MINUS".to_string() }, GrammarElement::TokenReference { name: "STAR".to_string() }, GrammarElement::TokenReference { name: "VARIABLE".to_string() }, GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "FUNCTION".to_string() }, GrammarElement::RuleReference { name: "function_args".to_string() }, GrammarElement::TokenReference { name: "RPAREN".to_string() }] }] },
            },
        ],
    }
}
