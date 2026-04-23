// AUTO-GENERATED FILE — DO NOT EDIT
// Source: nib.grammar
// Regenerate with: grammar-tools compile-grammar nib.grammar
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
            line_number: 42,
        },
        GrammarRule {
            name: r#"top_decl"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"const_decl"#.to_string() },
                GrammarElement::RuleReference { name: r#"static_decl"#.to_string() },
                GrammarElement::RuleReference { name: r#"fn_decl"#.to_string() },
            ] },
            line_number: 47,
        },
        GrammarRule {
            name: r#"const_decl"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"const"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                GrammarElement::RuleReference { name: r#"type"#.to_string() },
                GrammarElement::TokenReference { name: r#"EQ"#.to_string() },
                GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
            ] },
            line_number: 60,
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
            line_number: 66,
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
            line_number: 77,
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
            line_number: 80,
        },
        GrammarRule {
            name: r#"param"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                GrammarElement::RuleReference { name: r#"type"#.to_string() },
            ] },
            line_number: 87,
        },
        GrammarRule {
            name: r#"block"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"stmt"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
            ] },
            line_number: 98,
        },
        GrammarRule {
            name: r#"stmt"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"let_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"assign_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"return_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"for_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"if_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"expr_stmt"#.to_string() },
            ] },
            line_number: 109,
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
            line_number: 121,
        },
        GrammarRule {
            name: r#"assign_stmt"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"EQ"#.to_string() },
                GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
            ] },
            line_number: 126,
        },
        GrammarRule {
            name: r#"return_stmt"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"return"#.to_string() },
                GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
            ] },
            line_number: 131,
        },
        GrammarRule {
            name: r#"for_stmt"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"for"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                GrammarElement::RuleReference { name: r#"type"#.to_string() },
                GrammarElement::Literal { value: r#"in"#.to_string() },
                GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                GrammarElement::TokenReference { name: r#"RANGE"#.to_string() },
                GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                GrammarElement::RuleReference { name: r#"block"#.to_string() },
            ] },
            line_number: 154,
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
            line_number: 160,
        },
        GrammarRule {
            name: r#"expr_stmt"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
            ] },
            line_number: 167,
        },
        GrammarRule {
            name: r#"type"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Literal { value: r#"u4"#.to_string() },
                GrammarElement::Literal { value: r#"u8"#.to_string() },
                GrammarElement::Literal { value: r#"bcd"#.to_string() },
                GrammarElement::Literal { value: r#"bool"#.to_string() },
            ] },
            line_number: 202,
        },
        GrammarRule {
            name: r#"expr"#.to_string(),
            body: GrammarElement::RuleReference { name: r#"or_expr"#.to_string() },
            line_number: 242,
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
            line_number: 248,
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
            line_number: 252,
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
            line_number: 257,
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
            line_number: 263,
        },
        GrammarRule {
            name: r#"add_expr"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"bitwise_expr"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                                GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                                GrammarElement::TokenReference { name: r#"WRAP_ADD"#.to_string() },
                                GrammarElement::TokenReference { name: r#"SAT_ADD"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"bitwise_expr"#.to_string() },
                    ] }) },
            ] },
            line_number: 276,
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
            line_number: 282,
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
            line_number: 290,
        },
        GrammarRule {
            name: r#"primary"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"INT_LIT"#.to_string() },
                GrammarElement::TokenReference { name: r#"HEX_LIT"#.to_string() },
                GrammarElement::Literal { value: r#"true"#.to_string() },
                GrammarElement::Literal { value: r#"false"#.to_string() },
                GrammarElement::RuleReference { name: r#"call_expr"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
            ] },
            line_number: 298,
        },
        GrammarRule {
            name: r#"call_expr"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"arg_list"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
            ] },
            line_number: 321,
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
            line_number: 324,
        },
    ],
        version: 1,
    }
}
