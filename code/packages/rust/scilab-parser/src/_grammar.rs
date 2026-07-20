// AUTO-GENERATED FILE — DO NOT EDIT
// Source: scilab.grammar
// Regenerate with: grammar-tools compile-grammar scilab.grammar
//
// This file embeds a ParserGrammar as native Rust data structures.
// Call `parser_grammar()` instead of reading and parsing the .grammar file.

use grammar_tools::parser_grammar::{GrammarElement, GrammarRule, ParserGrammar};

pub fn parser_grammar() -> ParserGrammar {
    ParserGrammar {
        rules: vec![
        GrammarRule {
            name: r#"program"#.to_string(),
            body: GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"statement_line"#.to_string() }) },
            line_number: 95,
        },
        GrammarRule {
            name: r#"statement_line"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"statement"#.to_string() },
                    GrammarElement::RuleReference { name: r#"stmt_term"#.to_string() },
                ] },
                GrammarElement::RuleReference { name: r#"statement"#.to_string() },
                GrammarElement::RuleReference { name: r#"stmt_term"#.to_string() },
            ] },
            line_number: 100,
        },
        GrammarRule {
            name: r#"stmt_term"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
            ] },
            line_number: 104,
        },
        GrammarRule {
            name: r#"statement"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"func_def"#.to_string() },
                GrammarElement::RuleReference { name: r#"if_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"select_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"for_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"while_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"break_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"continue_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"expr"#.to_string() },
            ] },
            line_number: 106,
        },
        GrammarRule {
            name: r#"block_body"#.to_string(),
            body: GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"statement_line"#.to_string() }) },
            line_number: 118,
        },
        GrammarRule {
            name: r#"stmt_sep"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Literal { value: r#"then"#.to_string() },
                GrammarElement::Literal { value: r#"do"#.to_string() },
                GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
            ] },
            line_number: 149,
        },
        GrammarRule {
            name: r#"if_stmt"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"if"#.to_string() },
                GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                GrammarElement::RuleReference { name: r#"stmt_sep"#.to_string() },
                GrammarElement::RuleReference { name: r#"block_body"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"elseif_clause"#.to_string() }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"else_clause"#.to_string() }) },
                GrammarElement::Literal { value: r#"end"#.to_string() },
            ] },
            line_number: 168,
        },
        GrammarRule {
            name: r#"elseif_clause"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"elseif"#.to_string() },
                GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                GrammarElement::RuleReference { name: r#"stmt_sep"#.to_string() },
                GrammarElement::RuleReference { name: r#"block_body"#.to_string() },
            ] },
            line_number: 169,
        },
        GrammarRule {
            name: r#"else_clause"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"else"#.to_string() },
                GrammarElement::RuleReference { name: r#"block_body"#.to_string() },
            ] },
            line_number: 174,
        },
        GrammarRule {
            name: r#"select_stmt"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"select"#.to_string() },
                GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                GrammarElement::RuleReference { name: r#"stmt_sep"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"case_clause"#.to_string() }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"else_clause"#.to_string() }) },
                GrammarElement::Literal { value: r#"end"#.to_string() },
            ] },
            line_number: 184,
        },
        GrammarRule {
            name: r#"case_clause"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"case"#.to_string() },
                GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                GrammarElement::RuleReference { name: r#"stmt_sep"#.to_string() },
                GrammarElement::RuleReference { name: r#"block_body"#.to_string() },
            ] },
            line_number: 185,
        },
        GrammarRule {
            name: r#"while_stmt"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"while"#.to_string() },
                GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                GrammarElement::RuleReference { name: r#"stmt_sep"#.to_string() },
                GrammarElement::RuleReference { name: r#"block_body"#.to_string() },
                GrammarElement::Literal { value: r#"end"#.to_string() },
            ] },
            line_number: 187,
        },
        GrammarRule {
            name: r#"for_stmt"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"for"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"EQ"#.to_string() },
                GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                GrammarElement::RuleReference { name: r#"stmt_sep"#.to_string() },
                GrammarElement::RuleReference { name: r#"block_body"#.to_string() },
                GrammarElement::Literal { value: r#"end"#.to_string() },
            ] },
            line_number: 189,
        },
        GrammarRule {
            name: r#"break_stmt"#.to_string(),
            body: GrammarElement::Literal { value: r#"break"#.to_string() },
            line_number: 191,
        },
        GrammarRule {
            name: r#"continue_stmt"#.to_string(),
            body: GrammarElement::Literal { value: r#"continue"#.to_string() },
            line_number: 192,
        },
        GrammarRule {
            name: r#"func_def"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"function"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"func_returns"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"name_list"#.to_string() }) },
                        GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                    ] }) },
                GrammarElement::RuleReference { name: r#"block_body"#.to_string() },
                GrammarElement::Literal { value: r#"endfunction"#.to_string() },
            ] },
            line_number: 213,
        },
        GrammarRule {
            name: r#"func_returns"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQ"#.to_string() },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"name_list"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQ"#.to_string() },
                ] },
            ] },
            line_number: 214,
        },
        GrammarRule {
            name: r#"name_list"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    ] }) },
            ] },
            line_number: 216,
        },
        GrammarRule {
            name: r#"expr"#.to_string(),
            body: GrammarElement::RuleReference { name: r#"assignment"#.to_string() },
            line_number: 222,
        },
        GrammarRule {
            name: r#"assignment"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"logical_or"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"EQ"#.to_string() },
                        GrammarElement::RuleReference { name: r#"assignment"#.to_string() },
                    ] }) },
            ] },
            line_number: 226,
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
            line_number: 228,
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
            line_number: 229,
        },
        GrammarRule {
            name: r#"bit_or"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"bit_and"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"PIPE"#.to_string() },
                        GrammarElement::RuleReference { name: r#"bit_and"#.to_string() },
                    ] }) },
            ] },
            line_number: 230,
        },
        GrammarRule {
            name: r#"bit_and"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"comparison"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"AMP"#.to_string() },
                        GrammarElement::RuleReference { name: r#"comparison"#.to_string() },
                    ] }) },
            ] },
            line_number: 231,
        },
        GrammarRule {
            name: r#"comparison"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"colon_expr"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::TokenReference { name: r#"EQ_EQ"#.to_string() },
                                GrammarElement::TokenReference { name: r#"NE"#.to_string() },
                                GrammarElement::TokenReference { name: r#"NE_ALT"#.to_string() },
                                GrammarElement::TokenReference { name: r#"LE"#.to_string() },
                                GrammarElement::TokenReference { name: r#"GE"#.to_string() },
                                GrammarElement::TokenReference { name: r#"LT"#.to_string() },
                                GrammarElement::TokenReference { name: r#"GT"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"colon_expr"#.to_string() },
                    ] }) },
            ] },
            line_number: 241,
        },
        GrammarRule {
            name: r#"colon_expr"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"additive"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                        GrammarElement::RuleReference { name: r#"additive"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                                GrammarElement::RuleReference { name: r#"additive"#.to_string() },
                            ] }) },
                    ] }) },
            ] },
            line_number: 246,
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
            line_number: 248,
        },
        GrammarRule {
            name: r#"multiplicative"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"unary"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                                GrammarElement::TokenReference { name: r#"SLASH"#.to_string() },
                                GrammarElement::TokenReference { name: r#"BACKSLASH"#.to_string() },
                                GrammarElement::TokenReference { name: r#"ELEM_MUL"#.to_string() },
                                GrammarElement::TokenReference { name: r#"ELEM_RDIV"#.to_string() },
                                GrammarElement::TokenReference { name: r#"ELEM_LDIV"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"unary"#.to_string() },
                    ] }) },
            ] },
            line_number: 254,
        },
        GrammarRule {
            name: r#"unary"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                            GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                            GrammarElement::TokenReference { name: r#"TILDE"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"unary"#.to_string() },
                ] },
                GrammarElement::RuleReference { name: r#"power"#.to_string() },
            ] },
            line_number: 259,
        },
        GrammarRule {
            name: r#"power"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"postfix"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::TokenReference { name: r#"CARET"#.to_string() },
                                GrammarElement::TokenReference { name: r#"ELEM_POW"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"unary"#.to_string() },
                    ] }) },
            ] },
            line_number: 267,
        },
        GrammarRule {
            name: r#"postfix"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"primary"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::RuleReference { name: r#"transpose_suffix"#.to_string() },
                        GrammarElement::RuleReference { name: r#"call_suffix"#.to_string() },
                        GrammarElement::RuleReference { name: r#"cell_suffix"#.to_string() },
                        GrammarElement::RuleReference { name: r#"field_suffix"#.to_string() },
                    ] }) },
            ] },
            line_number: 273,
        },
        GrammarRule {
            name: r#"transpose_suffix"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"TRANSPOSE"#.to_string() },
                GrammarElement::TokenReference { name: r#"ELEM_TRANSPOSE"#.to_string() },
            ] },
            line_number: 275,
        },
        GrammarRule {
            name: r#"call_suffix"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"arg_list"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
            ] },
            line_number: 276,
        },
        GrammarRule {
            name: r#"cell_suffix"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"arg_list"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
            ] },
            line_number: 277,
        },
        GrammarRule {
            name: r#"field_suffix"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
            ] },
            line_number: 278,
        },
        GrammarRule {
            name: r#"arg_list"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"arg"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::RuleReference { name: r#"arg"#.to_string() },
                    ] }) },
            ] },
            line_number: 285,
        },
        GrammarRule {
            name: r#"arg"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                GrammarElement::RuleReference { name: r#"expr"#.to_string() },
            ] },
            line_number: 286,
        },
        GrammarRule {
            name: r#"primary"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                GrammarElement::TokenReference { name: r#"PERCENT_CONST"#.to_string() },
                GrammarElement::TokenReference { name: r#"DOLLAR"#.to_string() },
                GrammarElement::RuleReference { name: r#"matrix_literal"#.to_string() },
                GrammarElement::RuleReference { name: r#"cell_literal"#.to_string() },
                GrammarElement::RuleReference { name: r#"group"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
            ] },
            line_number: 317,
        },
        GrammarRule {
            name: r#"group"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
            ] },
            line_number: 326,
        },
        GrammarRule {
            name: r#"matrix_literal"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"matrix_rows"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
            ] },
            line_number: 349,
        },
        GrammarRule {
            name: r#"cell_literal"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"matrix_rows"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
            ] },
            line_number: 350,
        },
        GrammarRule {
            name: r#"matrix_rows"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"matrix_row"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"row_sep"#.to_string() },
                        GrammarElement::RuleReference { name: r#"matrix_row"#.to_string() },
                    ] }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"row_sep"#.to_string() }) },
            ] },
            line_number: 352,
        },
        GrammarRule {
            name: r#"row_sep"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
            ] },
            line_number: 353,
        },
        GrammarRule {
            name: r#"matrix_row"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                        GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                    ] }) },
            ] },
            line_number: 354,
        },
    ],
        version: 0,
    }
}
