// AUTO-GENERATED FILE — DO NOT EDIT
// Source: dartmouth_basic.grammar
// Regenerate with: grammar-tools compile-grammar dartmouth_basic.grammar
//
// This file embeds a ParserGrammar as native Rust data structures.
// Call `parser_grammar()` instead of reading and parsing the .grammar file.

use grammar_tools::parser_grammar::{GrammarElement, GrammarRule, ParserGrammar};

pub fn parser_grammar() -> ParserGrammar {
    ParserGrammar {
        rules: vec![
        GrammarRule {
            name: r#"program"#.to_string(),
            body: GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"line"#.to_string() }) },
            line_number: 70,
        },
        GrammarRule {
            name: r#"line"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"LINE_NUM"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"statement"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
            ] },
            line_number: 81,
        },
        GrammarRule {
            name: r#"statement"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"let_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"print_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"input_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"if_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"goto_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"gosub_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"return_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"for_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"next_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"end_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"stop_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"rem_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"read_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"data_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"restore_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"dim_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"def_stmt"#.to_string() },
            ] },
            line_number: 91,
        },
        GrammarRule {
            name: r#"let_stmt"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"LET"#.to_string() },
                GrammarElement::RuleReference { name: r#"variable"#.to_string() },
                GrammarElement::TokenReference { name: r#"EQ"#.to_string() },
                GrammarElement::RuleReference { name: r#"expr"#.to_string() },
            ] },
            line_number: 121,
        },
        GrammarRule {
            name: r#"print_stmt"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"PRINT"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"print_list"#.to_string() }) },
            ] },
            line_number: 137,
        },
        GrammarRule {
            name: r#"print_list"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"print_item"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"print_sep"#.to_string() },
                        GrammarElement::RuleReference { name: r#"print_item"#.to_string() },
                    ] }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"print_sep"#.to_string() }) },
            ] },
            line_number: 139,
        },
        GrammarRule {
            name: r#"print_item"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                GrammarElement::RuleReference { name: r#"expr"#.to_string() },
            ] },
            line_number: 141,
        },
        GrammarRule {
            name: r#"print_sep"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
            ] },
            line_number: 143,
        },
        GrammarRule {
            name: r#"input_stmt"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"INPUT"#.to_string() },
                GrammarElement::RuleReference { name: r#"variable"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::RuleReference { name: r#"variable"#.to_string() },
                    ] }) },
            ] },
            line_number: 155,
        },
        GrammarRule {
            name: r#"if_stmt"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"IF"#.to_string() },
                GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                GrammarElement::RuleReference { name: r#"relop"#.to_string() },
                GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                GrammarElement::Literal { value: r#"THEN"#.to_string() },
                GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
            ] },
            line_number: 170,
        },
        GrammarRule {
            name: r#"relop"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"EQ"#.to_string() },
                GrammarElement::TokenReference { name: r#"LT"#.to_string() },
                GrammarElement::TokenReference { name: r#"GT"#.to_string() },
                GrammarElement::TokenReference { name: r#"LE"#.to_string() },
                GrammarElement::TokenReference { name: r#"GE"#.to_string() },
                GrammarElement::TokenReference { name: r#"NE"#.to_string() },
            ] },
            line_number: 172,
        },
        GrammarRule {
            name: r#"goto_stmt"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"GOTO"#.to_string() },
                GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
            ] },
            line_number: 183,
        },
        GrammarRule {
            name: r#"gosub_stmt"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"GOSUB"#.to_string() },
                GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
            ] },
            line_number: 198,
        },
        GrammarRule {
            name: r#"return_stmt"#.to_string(),
            body: GrammarElement::Literal { value: r#"RETURN"#.to_string() },
            line_number: 200,
        },
        GrammarRule {
            name: r#"for_stmt"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"FOR"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"EQ"#.to_string() },
                GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                GrammarElement::Literal { value: r#"TO"#.to_string() },
                GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"STEP"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                    ] }) },
            ] },
            line_number: 222,
        },
        GrammarRule {
            name: r#"next_stmt"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"NEXT"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
            ] },
            line_number: 224,
        },
        GrammarRule {
            name: r#"end_stmt"#.to_string(),
            body: GrammarElement::Literal { value: r#"END"#.to_string() },
            line_number: 233,
        },
        GrammarRule {
            name: r#"stop_stmt"#.to_string(),
            body: GrammarElement::Literal { value: r#"STOP"#.to_string() },
            line_number: 234,
        },
        GrammarRule {
            name: r#"rem_stmt"#.to_string(),
            body: GrammarElement::Literal { value: r#"REM"#.to_string() },
            line_number: 247,
        },
        GrammarRule {
            name: r#"read_stmt"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"READ"#.to_string() },
                GrammarElement::RuleReference { name: r#"variable"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::RuleReference { name: r#"variable"#.to_string() },
                    ] }) },
            ] },
            line_number: 263,
        },
        GrammarRule {
            name: r#"data_stmt"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"DATA"#.to_string() },
                GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                    ] }) },
            ] },
            line_number: 265,
        },
        GrammarRule {
            name: r#"restore_stmt"#.to_string(),
            body: GrammarElement::Literal { value: r#"RESTORE"#.to_string() },
            line_number: 267,
        },
        GrammarRule {
            name: r#"dim_stmt"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"DIM"#.to_string() },
                GrammarElement::RuleReference { name: r#"dim_decl"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::RuleReference { name: r#"dim_decl"#.to_string() },
                    ] }) },
            ] },
            line_number: 280,
        },
        GrammarRule {
            name: r#"dim_decl"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                    ] }) },
                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
            ] },
            line_number: 282,
        },
        GrammarRule {
            name: r#"def_stmt"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"DEF"#.to_string() },
                GrammarElement::TokenReference { name: r#"USER_FN"#.to_string() },
                GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                GrammarElement::TokenReference { name: r#"EQ"#.to_string() },
                GrammarElement::RuleReference { name: r#"expr"#.to_string() },
            ] },
            line_number: 295,
        },
        GrammarRule {
            name: r#"variable"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
            ] },
            line_number: 312,
        },
        GrammarRule {
            name: r#"expr"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"term"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                                GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"term"#.to_string() },
                    ] }) },
            ] },
            line_number: 335,
        },
        GrammarRule {
            name: r#"term"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"power"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                                GrammarElement::TokenReference { name: r#"SLASH"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"power"#.to_string() },
                    ] }) },
            ] },
            line_number: 337,
        },
        GrammarRule {
            name: r#"power"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"unary"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"CARET"#.to_string() },
                        GrammarElement::RuleReference { name: r#"power"#.to_string() },
                    ] }) },
            ] },
            line_number: 343,
        },
        GrammarRule {
            name: r#"unary"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"primary"#.to_string() },
                ] },
                GrammarElement::RuleReference { name: r#"primary"#.to_string() },
            ] },
            line_number: 348,
        },
        GrammarRule {
            name: r#"primary"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"BUILTIN_FN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"USER_FN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                GrammarElement::RuleReference { name: r#"variable"#.to_string() },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
            ] },
            line_number: 366,
        },
    ],
        version: 1,
    }
}
