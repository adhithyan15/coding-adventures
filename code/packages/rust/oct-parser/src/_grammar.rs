// AUTO-GENERATED FILE — DO NOT EDIT
// Source: oct.grammar
// Regenerate with: grammar-tools compile-grammar oct.grammar
//
// This file embeds a ParserGrammar as native Rust data structures.
// Call `parser_grammar()` instead of reading and parsing the .grammar file.

use grammar_tools::parser_grammar::{GrammarElement, GrammarRule, ParserGrammar};

pub fn parser_grammar() -> ParserGrammar {
    ParserGrammar {
        rules: vec![
        GrammarRule {
            name: r#"program"#.to_string(),
            body: GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"top_decl"#.to_string() }) },
            line_number: 48,
        },
        GrammarRule {
            name: r#"top_decl"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"static_decl"#.to_string() },
                GrammarElement::RuleReference { name: r#"fn_decl"#.to_string() },
            ] },
            line_number: 52,
        },
        GrammarRule {
            name: r#"static_decl"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"static"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                GrammarElement::RuleReference { name: r#"type"#.to_string() },
                GrammarElement::TokenReference { name: r#"EQ"#.to_string() },
                GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
            ] },
            line_number: 69,
        },
        GrammarRule {
            name: r#"fn_decl"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"fn"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"param_list"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                        GrammarElement::RuleReference { name: r#"type"#.to_string() },
                    ] }) },
                GrammarElement::RuleReference { name: r#"block"#.to_string() },
            ] },
            line_number: 88,
        },
        GrammarRule {
            name: r#"param_list"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"param"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::RuleReference { name: r#"param"#.to_string() },
                    ] }) },
            ] },
            line_number: 92,
        },
        GrammarRule {
            name: r#"param"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                GrammarElement::RuleReference { name: r#"type"#.to_string() },
            ] },
            line_number: 96,
        },
        GrammarRule {
            name: r#"block"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"stmt"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
            ] },
            line_number: 106,
        },
        GrammarRule {
            name: r#"stmt"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"let_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"static_decl"#.to_string() },
                GrammarElement::RuleReference { name: r#"assign_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"return_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"if_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"while_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"loop_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"break_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"expr_stmt"#.to_string() },
            ] },
            line_number: 117,
        },
        GrammarRule {
            name: r#"let_stmt"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"let"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                GrammarElement::RuleReference { name: r#"type"#.to_string() },
                GrammarElement::TokenReference { name: r#"EQ"#.to_string() },
                GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
            ] },
            line_number: 139,
        },
        GrammarRule {
            name: r#"assign_stmt"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"EQ"#.to_string() },
                GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
            ] },
            line_number: 148,
        },
        GrammarRule {
            name: r#"return_stmt"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"return"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"expr"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
            ] },
            line_number: 157,
        },
        GrammarRule {
            name: r#"if_stmt"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"if"#.to_string() },
                GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                GrammarElement::RuleReference { name: r#"block"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"else"#.to_string() },
                        GrammarElement::RuleReference { name: r#"block"#.to_string() },
                    ] }) },
            ] },
            line_number: 166,
        },
        GrammarRule {
            name: r#"while_stmt"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"while"#.to_string() },
                GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                GrammarElement::RuleReference { name: r#"block"#.to_string() },
            ] },
            line_number: 174,
        },
        GrammarRule {
            name: r#"loop_stmt"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"loop"#.to_string() },
                GrammarElement::RuleReference { name: r#"block"#.to_string() },
            ] },
            line_number: 181,
        },
        GrammarRule {
            name: r#"break_stmt"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"break"#.to_string() },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
            ] },
            line_number: 185,
        },
        GrammarRule {
            name: r#"expr_stmt"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
            ] },
            line_number: 196,
        },
        GrammarRule {
            name: r#"type"#.to_string(),
            body: GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
            line_number: 230,
        },
        GrammarRule {
            name: r#"expr"#.to_string(),
            body: GrammarElement::RuleReference { name: r#"or_expr"#.to_string() },
            line_number: 269,
        },
        GrammarRule {
            name: r#"or_expr"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"and_expr"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LOR"#.to_string() },
                        GrammarElement::RuleReference { name: r#"and_expr"#.to_string() },
                    ] }) },
            ] },
            line_number: 274,
        },
        GrammarRule {
            name: r#"and_expr"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"eq_expr"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LAND"#.to_string() },
                        GrammarElement::RuleReference { name: r#"eq_expr"#.to_string() },
                    ] }) },
            ] },
            line_number: 278,
        },
        GrammarRule {
            name: r#"eq_expr"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"cmp_expr"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::TokenReference { name: r#"EQ_EQ"#.to_string() },
                                GrammarElement::TokenReference { name: r#"NEQ"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"cmp_expr"#.to_string() },
                    ] }) },
            ] },
            line_number: 285,
        },
        GrammarRule {
            name: r#"cmp_expr"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"add_expr"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::TokenReference { name: r#"LT"#.to_string() },
                                GrammarElement::TokenReference { name: r#"GT"#.to_string() },
                                GrammarElement::TokenReference { name: r#"LEQ"#.to_string() },
                                GrammarElement::TokenReference { name: r#"GEQ"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"add_expr"#.to_string() },
                    ] }) },
            ] },
            line_number: 291,
        },
        GrammarRule {
            name: r#"add_expr"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"bitwise_expr"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                                GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"bitwise_expr"#.to_string() },
                    ] }) },
            ] },
            line_number: 297,
        },
        GrammarRule {
            name: r#"bitwise_expr"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"unary_expr"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::TokenReference { name: r#"AMP"#.to_string() },
                                GrammarElement::TokenReference { name: r#"PIPE"#.to_string() },
                                GrammarElement::TokenReference { name: r#"CARET"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"unary_expr"#.to_string() },
                    ] }) },
            ] },
            line_number: 302,
        },
        GrammarRule {
            name: r#"unary_expr"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"BANG"#.to_string() },
                            GrammarElement::TokenReference { name: r#"TILDE"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"unary_expr"#.to_string() },
                ] },
                GrammarElement::RuleReference { name: r#"primary"#.to_string() },
            ] },
            line_number: 310,
        },
        GrammarRule {
            name: r#"primary"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"intrinsic_call"#.to_string() },
                GrammarElement::RuleReference { name: r#"call_expr"#.to_string() },
                GrammarElement::TokenReference { name: r#"INT_LIT"#.to_string() },
                GrammarElement::TokenReference { name: r#"HEX_LIT"#.to_string() },
                GrammarElement::TokenReference { name: r#"BIN_LIT"#.to_string() },
                GrammarElement::Literal { value: r#"true"#.to_string() },
                GrammarElement::Literal { value: r#"false"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
            ] },
            line_number: 323,
        },
        GrammarRule {
            name: r#"call_expr"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"arg_list"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
            ] },
            line_number: 341,
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
            line_number: 344,
        },
        GrammarRule {
            name: r#"intrinsic_call"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"in"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"out"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"adc"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"sbb"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"rlc"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"rrc"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"ral"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"rar"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"carry"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"parity"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
            ] },
            line_number: 370,
        },
    ],
        version: 1,
    }
}
