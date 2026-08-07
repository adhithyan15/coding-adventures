// AUTO-GENERATED FILE — DO NOT EDIT
// Source: c.grammar
// Regenerate with: grammar-tools compile-grammar c.grammar
//
// This file embeds a ParserGrammar as native Rust data structures.
// Call `parser_grammar()` instead of reading and parsing the .grammar file.

use grammar_tools::parser_grammar::{GrammarElement, GrammarRule, ParserGrammar};

pub fn parser_grammar() -> ParserGrammar {
    ParserGrammar {
        rules: vec![
        GrammarRule {
            name: r#"translation_unit"#.to_string(),
            body: GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"function_def"#.to_string() }) },
            line_number: 28,
        },
        GrammarRule {
            name: r#"function_def"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"type_spec"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"param_list"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                GrammarElement::RuleReference { name: r#"compound_stmt"#.to_string() },
            ] },
            line_number: 30,
        },
        GrammarRule {
            name: r#"param_list"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"param"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"param"#.to_string() },
                        ] }) },
                ] },
                GrammarElement::Literal { value: r#"void"#.to_string() },
            ] },
            line_number: 32,
        },
        GrammarRule {
            name: r#"param"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"type_spec"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
            ] },
            line_number: 33,
        },
        GrammarRule {
            name: r#"type_spec"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"type_kw"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"type_kw"#.to_string() }) },
            ] },
            line_number: 41,
        },
        GrammarRule {
            name: r#"type_kw"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Literal { value: r#"void"#.to_string() },
                GrammarElement::Literal { value: r#"char"#.to_string() },
                GrammarElement::Literal { value: r#"short"#.to_string() },
                GrammarElement::Literal { value: r#"int"#.to_string() },
                GrammarElement::Literal { value: r#"long"#.to_string() },
                GrammarElement::Literal { value: r#"signed"#.to_string() },
                GrammarElement::Literal { value: r#"unsigned"#.to_string() },
                GrammarElement::Literal { value: r#"float"#.to_string() },
                GrammarElement::Literal { value: r#"double"#.to_string() },
                GrammarElement::Literal { value: r#"int8_t"#.to_string() },
                GrammarElement::Literal { value: r#"int16_t"#.to_string() },
                GrammarElement::Literal { value: r#"int32_t"#.to_string() },
                GrammarElement::Literal { value: r#"int64_t"#.to_string() },
                GrammarElement::Literal { value: r#"uint8_t"#.to_string() },
                GrammarElement::Literal { value: r#"uint16_t"#.to_string() },
                GrammarElement::Literal { value: r#"uint32_t"#.to_string() },
                GrammarElement::Literal { value: r#"uint64_t"#.to_string() },
                GrammarElement::Literal { value: r#"size_t"#.to_string() },
            ] },
            line_number: 43,
        },
        GrammarRule {
            name: r#"compound_stmt"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"block_item"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
            ] },
            line_number: 52,
        },
        GrammarRule {
            name: r#"block_item"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"declaration"#.to_string() },
                GrammarElement::RuleReference { name: r#"statement"#.to_string() },
            ] },
            line_number: 54,
        },
        GrammarRule {
            name: r#"declaration"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"init_declarator"#.to_string() },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
            ] },
            line_number: 58,
        },
        GrammarRule {
            name: r#"init_declarator"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"type_spec"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"INT_LIT"#.to_string() }) },
                        GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                    ] }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"EQ"#.to_string() },
                        GrammarElement::RuleReference { name: r#"initializer"#.to_string() },
                    ] }) },
            ] },
            line_number: 62,
        },
        GrammarRule {
            name: r#"initializer"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"init_list"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                GrammarElement::RuleReference { name: r#"expr"#.to_string() },
            ] },
            line_number: 63,
        },
        GrammarRule {
            name: r#"init_list"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                    ] }) },
            ] },
            line_number: 64,
        },
        GrammarRule {
            name: r#"statement"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"compound_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"if_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"while_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"for_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"return_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"expr_stmt"#.to_string() },
            ] },
            line_number: 66,
        },
        GrammarRule {
            name: r#"if_stmt"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"if"#.to_string() },
                GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                GrammarElement::RuleReference { name: r#"statement"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"else"#.to_string() },
                        GrammarElement::RuleReference { name: r#"statement"#.to_string() },
                    ] }) },
            ] },
            line_number: 73,
        },
        GrammarRule {
            name: r#"while_stmt"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"while"#.to_string() },
                GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                GrammarElement::RuleReference { name: r#"statement"#.to_string() },
            ] },
            line_number: 74,
        },
        GrammarRule {
            name: r#"for_stmt"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"for"#.to_string() },
                GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"for_clause"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"expr"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"expr"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                GrammarElement::RuleReference { name: r#"statement"#.to_string() },
            ] },
            line_number: 75,
        },
        GrammarRule {
            name: r#"for_clause"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"init_declarator"#.to_string() },
                GrammarElement::RuleReference { name: r#"expr"#.to_string() },
            ] },
            line_number: 76,
        },
        GrammarRule {
            name: r#"return_stmt"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"return"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"expr"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
            ] },
            line_number: 77,
        },
        GrammarRule {
            name: r#"expr_stmt"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"expr"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
            ] },
            line_number: 78,
        },
        GrammarRule {
            name: r#"expr"#.to_string(),
            body: GrammarElement::RuleReference { name: r#"assignment"#.to_string() },
            line_number: 84,
        },
        GrammarRule {
            name: r#"assignment"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"unary"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQ"#.to_string() },
                    GrammarElement::RuleReference { name: r#"assignment"#.to_string() },
                ] },
                GrammarElement::RuleReference { name: r#"logical_or"#.to_string() },
            ] },
            line_number: 87,
        },
        GrammarRule {
            name: r#"logical_or"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"logical_and"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"OR_OR"#.to_string() },
                        GrammarElement::RuleReference { name: r#"logical_and"#.to_string() },
                    ] }) },
            ] },
            line_number: 89,
        },
        GrammarRule {
            name: r#"logical_and"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"bit_or"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"AND_AND"#.to_string() },
                        GrammarElement::RuleReference { name: r#"bit_or"#.to_string() },
                    ] }) },
            ] },
            line_number: 90,
        },
        GrammarRule {
            name: r#"bit_or"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"bit_xor"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"PIPE"#.to_string() },
                        GrammarElement::RuleReference { name: r#"bit_xor"#.to_string() },
                    ] }) },
            ] },
            line_number: 91,
        },
        GrammarRule {
            name: r#"bit_xor"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"bit_and"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"CARET"#.to_string() },
                        GrammarElement::RuleReference { name: r#"bit_and"#.to_string() },
                    ] }) },
            ] },
            line_number: 92,
        },
        GrammarRule {
            name: r#"bit_and"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"equality"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"AMP"#.to_string() },
                        GrammarElement::RuleReference { name: r#"equality"#.to_string() },
                    ] }) },
            ] },
            line_number: 93,
        },
        GrammarRule {
            name: r#"equality"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"relational"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::TokenReference { name: r#"EQ_EQ"#.to_string() },
                                GrammarElement::TokenReference { name: r#"NE"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"relational"#.to_string() },
                    ] }) },
            ] },
            line_number: 94,
        },
        GrammarRule {
            name: r#"relational"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"shift"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::TokenReference { name: r#"LE"#.to_string() },
                                GrammarElement::TokenReference { name: r#"GE"#.to_string() },
                                GrammarElement::TokenReference { name: r#"LT"#.to_string() },
                                GrammarElement::TokenReference { name: r#"GT"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"shift"#.to_string() },
                    ] }) },
            ] },
            line_number: 95,
        },
        GrammarRule {
            name: r#"shift"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"additive"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::TokenReference { name: r#"SHL"#.to_string() },
                                GrammarElement::TokenReference { name: r#"SHR"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"additive"#.to_string() },
                    ] }) },
            ] },
            line_number: 96,
        },
        GrammarRule {
            name: r#"additive"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"multiplicative"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                                GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"multiplicative"#.to_string() },
                    ] }) },
            ] },
            line_number: 97,
        },
        GrammarRule {
            name: r#"multiplicative"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"cast"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                                GrammarElement::TokenReference { name: r#"SLASH"#.to_string() },
                                GrammarElement::TokenReference { name: r#"PERCENT"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"cast"#.to_string() },
                    ] }) },
            ] },
            line_number: 98,
        },
        GrammarRule {
            name: r#"cast"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_spec"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"cast"#.to_string() },
                ] },
                GrammarElement::RuleReference { name: r#"unary"#.to_string() },
            ] },
            line_number: 103,
        },
        GrammarRule {
            name: r#"unary"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                            GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                            GrammarElement::TokenReference { name: r#"TILDE"#.to_string() },
                            GrammarElement::TokenReference { name: r#"BANG"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"unary"#.to_string() },
                ] },
                GrammarElement::RuleReference { name: r#"postfix"#.to_string() },
            ] },
            line_number: 105,
        },
        GrammarRule {
            name: r#"postfix"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"primary"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::RuleReference { name: r#"call_suffix"#.to_string() },
                        GrammarElement::RuleReference { name: r#"index_suffix"#.to_string() },
                    ] }) },
            ] },
            line_number: 107,
        },
        GrammarRule {
            name: r#"call_suffix"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"arg_list"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
            ] },
            line_number: 108,
        },
        GrammarRule {
            name: r#"index_suffix"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
            ] },
            line_number: 109,
        },
        GrammarRule {
            name: r#"arg_list"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                    ] }) },
            ] },
            line_number: 110,
        },
        GrammarRule {
            name: r#"primary"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"FLOAT_LIT"#.to_string() },
                GrammarElement::TokenReference { name: r#"INT_LIT"#.to_string() },
                GrammarElement::TokenReference { name: r#"CHAR_LIT"#.to_string() },
                GrammarElement::TokenReference { name: r#"STR_LIT"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
            ] },
            line_number: 112,
        },
    ],
        version: 1,
    }
}
