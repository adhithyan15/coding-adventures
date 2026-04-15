// AUTO-GENERATED FILE - DO NOT EDIT
// Source family: python
// Regenerate with: grammar-tools generate-rust-compiled-grammars python
//
// This file embeds versioned ParserGrammar values as native Rust data structures.
// Call `parser_grammar` instead of reading and parsing grammar files at runtime.

use grammar_tools::parser_grammar::ParserGrammar;

pub const SUPPORTED_VERSIONS: &[&str] = &[
    "2.7",
    "3.0",
    "3.6",
    "3.8",
    "3.10",
    "3.12",
];

pub fn parser_grammar(version: &str) -> Option<ParserGrammar> {
    match version {
        "2.7" => Some(v_2_7::parser_grammar()),
        "3.0" => Some(v_3_0::parser_grammar()),
        "3.6" => Some(v_3_6::parser_grammar()),
        "3.8" => Some(v_3_8::parser_grammar()),
        "3.10" => Some(v_3_10::parser_grammar()),
        "3.12" => Some(v_3_12::parser_grammar()),
        _ => None,
    }
}

mod v_2_7 {
    // AUTO-GENERATED FILE — DO NOT EDIT
    // Source: python2.7.grammar
    // Regenerate with: grammar-tools compile-grammar python2.7.grammar
    //
    // This file embeds a ParserGrammar as native Rust data structures.
    // Call `parser_grammar()` instead of reading and parsing the .grammar file.

    use grammar_tools::parser_grammar::{GrammarElement, GrammarRule, ParserGrammar};

    pub fn parser_grammar() -> ParserGrammar {
        ParserGrammar {
            rules: vec![
            GrammarRule {
                name: r#"file"#.to_string(),
                body: GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                        GrammarElement::RuleReference { name: r#"statement"#.to_string() },
                    ] }) },
                line_number: 47,
            },
            GrammarRule {
                name: r#"statement"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"compound_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"simple_stmt"#.to_string() },
                ] },
                line_number: 64,
            },
            GrammarRule {
                name: r#"simple_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"small_stmt"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"small_stmt"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                ] },
                line_number: 74,
            },
            GrammarRule {
                name: r#"small_stmt"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"print_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"del_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pass_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"break_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"continue_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"return_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"yield_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"raise_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"import_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"global_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"exec_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"assert_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"assign_stmt"#.to_string() },
                ] },
                line_number: 76,
            },
            GrammarRule {
                name: r#"print_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"print"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::RuleReference { name: r#"print_chevron"#.to_string() },
                            GrammarElement::RuleReference { name: r#"print_args"#.to_string() },
                        ] }) },
                ] },
                line_number: 116,
            },
            GrammarRule {
                name: r#"print_chevron"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"RIGHT_SHIFT"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"print_args"#.to_string() },
                        ] }) },
                ] },
                line_number: 118,
            },
            GrammarRule {
                name: r#"print_args"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                ] },
                line_number: 120,
            },
            GrammarRule {
                name: r#"del_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"del"#.to_string() },
                    GrammarElement::RuleReference { name: r#"target_list"#.to_string() },
                ] },
                line_number: 133,
            },
            GrammarRule {
                name: r#"pass_stmt"#.to_string(),
                body: GrammarElement::Literal { value: r#"pass"#.to_string() },
                line_number: 145,
            },
            GrammarRule {
                name: r#"break_stmt"#.to_string(),
                body: GrammarElement::Literal { value: r#"break"#.to_string() },
                line_number: 156,
            },
            GrammarRule {
                name: r#"continue_stmt"#.to_string(),
                body: GrammarElement::Literal { value: r#"continue"#.to_string() },
                line_number: 158,
            },
            GrammarRule {
                name: r#"return_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"return"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"expression_list"#.to_string() }) },
                ] },
                line_number: 170,
            },
            GrammarRule {
                name: r#"yield_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"yield"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"expression_list"#.to_string() }) },
                ] },
                line_number: 183,
            },
            GrammarRule {
                name: r#"raise_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"raise"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                                        ] }) },
                                ] }) },
                        ] }) },
                ] },
                line_number: 205,
            },
            GrammarRule {
                name: r#"import_stmt"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"import_name"#.to_string() },
                    GrammarElement::RuleReference { name: r#"import_from"#.to_string() },
                ] },
                line_number: 230,
            },
            GrammarRule {
                name: r#"import_name"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"import"#.to_string() },
                    GrammarElement::RuleReference { name: r#"dotted_as_names"#.to_string() },
                ] },
                line_number: 232,
            },
            GrammarRule {
                name: r#"import_from"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"from"#.to_string() },
                    GrammarElement::RuleReference { name: r#"import_from_module"#.to_string() },
                    GrammarElement::Literal { value: r#"import"#.to_string() },
                    GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                            GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                                GrammarElement::RuleReference { name: r#"import_as_names"#.to_string() },
                                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                            ] },
                            GrammarElement::RuleReference { name: r#"import_as_names"#.to_string() },
                        ] }) },
                ] },
                line_number: 234,
            },
            GrammarRule {
                name: r#"import_from_module"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"DOT"#.to_string() }) },
                        GrammarElement::RuleReference { name: r#"dotted_name"#.to_string() },
                    ] },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"DOT"#.to_string() }) },
                ] },
                line_number: 241,
            },
            GrammarRule {
                name: r#"dotted_name"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 244,
            },
            GrammarRule {
                name: r#"dotted_as_names"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"dotted_as_name"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"dotted_as_name"#.to_string() },
                        ] }) },
                ] },
                line_number: 247,
            },
            GrammarRule {
                name: r#"dotted_as_name"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"dotted_name"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"as"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 248,
            },
            GrammarRule {
                name: r#"import_as_names"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"import_as_name"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"import_as_name"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                ] },
                line_number: 250,
            },
            GrammarRule {
                name: r#"import_as_name"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"as"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 251,
            },
            GrammarRule {
                name: r#"global_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"global"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 263,
            },
            GrammarRule {
                name: r#"exec_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"exec"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"in"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                                ] }) },
                        ] }) },
                ] },
                line_number: 285,
            },
            GrammarRule {
                name: r#"assert_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"assert"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                ] },
                line_number: 298,
            },
            GrammarRule {
                name: r#"assign_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"expression_list"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"assign_tail"#.to_string() }) },
                ] },
                line_number: 326,
            },
            GrammarRule {
                name: r#"assign_tail"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"augmented_assign_op"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expression_list"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                                GrammarElement::RuleReference { name: r#"expression_list"#.to_string() },
                            ] }) },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                                GrammarElement::RuleReference { name: r#"expression_list"#.to_string() },
                            ] }) },
                    ] },
                ] },
                line_number: 328,
            },
            GrammarRule {
                name: r#"augmented_assign_op"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"PLUS_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"MINUS_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STAR_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SLASH_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"FLOOR_DIV_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PERCENT_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"DOUBLE_STAR_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"AMP_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PIPE_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CARET_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LEFT_SHIFT_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RIGHT_SHIFT_EQUALS"#.to_string() },
                ] },
                line_number: 331,
            },
            GrammarRule {
                name: r#"target_list"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"target"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"target"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                ] },
                line_number: 347,
            },
            GrammarRule {
                name: r#"target"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"primary"#.to_string() },
                        GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"primary"#.to_string() },
                        GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                        GrammarElement::RuleReference { name: r#"subscript"#.to_string() },
                        GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                        GrammarElement::RuleReference { name: r#"target_list"#.to_string() },
                        GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                        GrammarElement::RuleReference { name: r#"target_list"#.to_string() },
                        GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                    ] },
                ] },
                line_number: 349,
            },
            GrammarRule {
                name: r#"compound_stmt"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"if_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"while_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"for_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"try_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"with_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"def_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"class_stmt"#.to_string() },
                ] },
                line_number: 359,
            },
            GrammarRule {
                name: r#"if_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"if"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"elif"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"else"#.to_string() },
                            GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                        ] }) },
                ] },
                line_number: 382,
            },
            GrammarRule {
                name: r#"while_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"while"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"else"#.to_string() },
                            GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                        ] }) },
                ] },
                line_number: 399,
            },
            GrammarRule {
                name: r#"for_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"for"#.to_string() },
                    GrammarElement::RuleReference { name: r#"target_list"#.to_string() },
                    GrammarElement::Literal { value: r#"in"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression_list"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"else"#.to_string() },
                            GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                        ] }) },
                ] },
                line_number: 421,
            },
            GrammarRule {
                name: r#"try_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"try"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                    GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::Sequence { elements: vec![
                                GrammarElement::RuleReference { name: r#"except_clauses"#.to_string() },
                                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                        GrammarElement::Literal { value: r#"else"#.to_string() },
                                        GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                                        GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                                    ] }) },
                                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                        GrammarElement::Literal { value: r#"finally"#.to_string() },
                                        GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                                        GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                                    ] }) },
                            ] },
                            GrammarElement::Sequence { elements: vec![
                                GrammarElement::Literal { value: r#"finally"#.to_string() },
                                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                                GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                            ] },
                        ] }) },
                ] },
                line_number: 455,
            },
            GrammarRule {
                name: r#"except_clauses"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"except_clause"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"except_clause"#.to_string() }) },
                ] },
                line_number: 459,
            },
            GrammarRule {
                name: r#"except_clause"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"except"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                            GrammarElement::Literal { value: r#"as"#.to_string() },
                                        ] }) },
                                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                ] },
                line_number: 466,
            },
            GrammarRule {
                name: r#"with_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"with"#.to_string() },
                    GrammarElement::RuleReference { name: r#"with_item"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"with_item"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                ] },
                line_number: 485,
            },
            GrammarRule {
                name: r#"with_item"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"as"#.to_string() },
                            GrammarElement::RuleReference { name: r#"target"#.to_string() },
                        ] }) },
                ] },
                line_number: 487,
            },
            GrammarRule {
                name: r#"def_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"decorator"#.to_string() }) },
                    GrammarElement::Literal { value: r#"def"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"parameters"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                ] },
                line_number: 512,
            },
            GrammarRule {
                name: r#"class_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"decorator"#.to_string() }) },
                    GrammarElement::Literal { value: r#"class"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"expression_list"#.to_string() }) },
                            GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                ] },
                line_number: 540,
            },
            GrammarRule {
                name: r#"decorator"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"AT"#.to_string() },
                    GrammarElement::RuleReference { name: r#"dotted_name"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"arguments"#.to_string() }) },
                            GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                ] },
                line_number: 555,
            },
            GrammarRule {
                name: r#"suite"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"simple_stmt"#.to_string() },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                        GrammarElement::TokenReference { name: r#"INDENT"#.to_string() },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"statement"#.to_string() }) },
                        GrammarElement::TokenReference { name: r#"DEDENT"#.to_string() },
                    ] },
                ] },
                line_number: 571,
            },
            GrammarRule {
                name: r#"parameters"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"parameter"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"parameter"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                ] },
                line_number: 597,
            },
            GrammarRule {
                name: r#"parameter"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"DOUBLE_STAR"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"fpdef"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            ] }) },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            ] }) },
                    ] },
                ] },
                line_number: 599,
            },
            GrammarRule {
                name: r#"fpdef"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"fpdef_list"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 609,
            },
            GrammarRule {
                name: r#"fpdef_list"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"fpdef_item"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"fpdef_item"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                ] },
                line_number: 611,
            },
            GrammarRule {
                name: r#"fpdef_item"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::RuleReference { name: r#"fpdef"#.to_string() },
                ] },
                line_number: 613,
            },
            GrammarRule {
                name: r#"expression_list"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                ] },
                line_number: 646,
            },
            GrammarRule {
                name: r#"expression"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"lambda_expr"#.to_string() },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"or_expr"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::Literal { value: r#"if"#.to_string() },
                                GrammarElement::RuleReference { name: r#"or_expr"#.to_string() },
                                GrammarElement::Literal { value: r#"else"#.to_string() },
                                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            ] }) },
                    ] },
                ] },
                line_number: 658,
            },
            GrammarRule {
                name: r#"lambda_expr"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"lambda"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"lambda_params"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 671,
            },
            GrammarRule {
                name: r#"lambda_params"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"lambda_param"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"lambda_param"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                ] },
                line_number: 673,
            },
            GrammarRule {
                name: r#"lambda_param"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            ] }) },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"DOUBLE_STAR"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    ] },
                ] },
                line_number: 675,
            },
            GrammarRule {
                name: r#"or_expr"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"and_expr"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"or"#.to_string() },
                            GrammarElement::RuleReference { name: r#"and_expr"#.to_string() },
                        ] }) },
                ] },
                line_number: 688,
            },
            GrammarRule {
                name: r#"and_expr"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"not_expr"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"and"#.to_string() },
                            GrammarElement::RuleReference { name: r#"not_expr"#.to_string() },
                        ] }) },
                ] },
                line_number: 698,
            },
            GrammarRule {
                name: r#"not_expr"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"not"#.to_string() },
                        GrammarElement::RuleReference { name: r#"not_expr"#.to_string() },
                    ] },
                    GrammarElement::RuleReference { name: r#"comparison"#.to_string() },
                ] },
                line_number: 708,
            },
            GrammarRule {
                name: r#"comparison"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"bitwise_or"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"comp_op"#.to_string() },
                            GrammarElement::RuleReference { name: r#"bitwise_or"#.to_string() },
                        ] }) },
                ] },
                line_number: 734,
            },
            GrammarRule {
                name: r#"comp_op"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"EQUALS_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NOT_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"DIAMOND"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"GREATER_EQUALS"#.to_string() },
                    GrammarElement::Literal { value: r#"in"#.to_string() },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"not"#.to_string() },
                        GrammarElement::Literal { value: r#"in"#.to_string() },
                    ] },
                    GrammarElement::Literal { value: r#"is"#.to_string() },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"is"#.to_string() },
                        GrammarElement::Literal { value: r#"not"#.to_string() },
                    ] },
                ] },
                line_number: 736,
            },
            GrammarRule {
                name: r#"bitwise_or"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"bitwise_xor"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"PIPE"#.to_string() },
                            GrammarElement::RuleReference { name: r#"bitwise_xor"#.to_string() },
                        ] }) },
                ] },
                line_number: 752,
            },
            GrammarRule {
                name: r#"bitwise_xor"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"bitwise_and"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"CARET"#.to_string() },
                            GrammarElement::RuleReference { name: r#"bitwise_and"#.to_string() },
                        ] }) },
                ] },
                line_number: 753,
            },
            GrammarRule {
                name: r#"bitwise_and"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"shift"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"AMP"#.to_string() },
                            GrammarElement::RuleReference { name: r#"shift"#.to_string() },
                        ] }) },
                ] },
                line_number: 754,
            },
            GrammarRule {
                name: r#"shift"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"arith"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                    GrammarElement::TokenReference { name: r#"LEFT_SHIFT"#.to_string() },
                                    GrammarElement::TokenReference { name: r#"RIGHT_SHIFT"#.to_string() },
                                ] }) },
                            GrammarElement::RuleReference { name: r#"arith"#.to_string() },
                        ] }) },
                ] },
                line_number: 765,
            },
            GrammarRule {
                name: r#"arith"#.to_string(),
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
                line_number: 774,
            },
            GrammarRule {
                name: r#"term"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"factor"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                    GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                                    GrammarElement::TokenReference { name: r#"SLASH"#.to_string() },
                                    GrammarElement::TokenReference { name: r#"FLOOR_DIV"#.to_string() },
                                    GrammarElement::TokenReference { name: r#"PERCENT"#.to_string() },
                                ] }) },
                            GrammarElement::RuleReference { name: r#"factor"#.to_string() },
                        ] }) },
                ] },
                line_number: 791,
            },
            GrammarRule {
                name: r#"factor"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                                GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                                GrammarElement::TokenReference { name: r#"TILDE"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"factor"#.to_string() },
                    ] },
                    GrammarElement::RuleReference { name: r#"power"#.to_string() },
                ] },
                line_number: 803,
            },
            GrammarRule {
                name: r#"power"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"primary"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"DOUBLE_STAR"#.to_string() },
                            GrammarElement::RuleReference { name: r#"factor"#.to_string() },
                        ] }) },
                ] },
                line_number: 817,
            },
            GrammarRule {
                name: r#"primary"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"atom"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"suffix"#.to_string() }) },
                ] },
                line_number: 835,
            },
            GrammarRule {
                name: r#"suffix"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                        GrammarElement::RuleReference { name: r#"subscript"#.to_string() },
                        GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"arguments"#.to_string() }) },
                        GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                    ] },
                ] },
                line_number: 837,
            },
            GrammarRule {
                name: r#"subscript"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"expression"#.to_string() }) },
                        GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"expression"#.to_string() }) },
                        GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"expression"#.to_string() }) },
                            ] }) },
                    ] },
                ] },
                line_number: 854,
            },
            GrammarRule {
                name: r#"atom"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"INT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"FLOAT"#.to_string() },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"STRING"#.to_string() }) },
                    ] },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::RuleReference { name: r#"list_expr"#.to_string() },
                    GrammarElement::RuleReference { name: r#"dict_or_set_expr"#.to_string() },
                    GrammarElement::RuleReference { name: r#"paren_expr"#.to_string() },
                    GrammarElement::RuleReference { name: r#"backtick_expr"#.to_string() },
                ] },
                line_number: 869,
            },
            GrammarRule {
                name: r#"backtick_expr"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"BACKTICK"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"BACKTICK"#.to_string() },
                ] },
                line_number: 898,
            },
            GrammarRule {
                name: r#"list_expr"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"list_body"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                ] },
                line_number: 913,
            },
            GrammarRule {
                name: r#"list_body"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        GrammarElement::RuleReference { name: r#"comp_clause"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            ] }) },
                        GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                    ] },
                ] },
                line_number: 915,
            },
            GrammarRule {
                name: r#"dict_or_set_expr"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"dict_or_set_body"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 941,
            },
            GrammarRule {
                name: r#"dict_or_set_body"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"dict_entry"#.to_string() },
                        GrammarElement::RuleReference { name: r#"comp_clause"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"dict_entry"#.to_string() },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                GrammarElement::RuleReference { name: r#"dict_entry"#.to_string() },
                            ] }) },
                        GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        GrammarElement::RuleReference { name: r#"comp_clause"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            ] }) },
                        GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                    ] },
                ] },
                line_number: 943,
            },
            GrammarRule {
                name: r#"dict_entry"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 948,
            },
            GrammarRule {
                name: r#"paren_expr"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"paren_body"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 972,
            },
            GrammarRule {
                name: r#"paren_body"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        GrammarElement::RuleReference { name: r#"comp_clause"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                                    ] }) },
                                GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                            ] }) },
                    ] },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 974,
            },
            GrammarRule {
                name: r#"comp_clause"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"comp_for"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::RuleReference { name: r#"comp_for"#.to_string() },
                            GrammarElement::RuleReference { name: r#"comp_if"#.to_string() },
                        ] }) },
                ] },
                line_number: 1003,
            },
            GrammarRule {
                name: r#"comp_for"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"for"#.to_string() },
                    GrammarElement::RuleReference { name: r#"target_list"#.to_string() },
                    GrammarElement::Literal { value: r#"in"#.to_string() },
                    GrammarElement::RuleReference { name: r#"or_expr"#.to_string() },
                ] },
                line_number: 1005,
            },
            GrammarRule {
                name: r#"comp_if"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"if"#.to_string() },
                    GrammarElement::RuleReference { name: r#"or_expr"#.to_string() },
                ] },
                line_number: 1007,
            },
            GrammarRule {
                name: r#"arguments"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"argument"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"argument"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                ] },
                line_number: 1031,
            },
            GrammarRule {
                name: r#"argument"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"DOUBLE_STAR"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    ] },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 1033,
            },
        ],
            version: 1,
        }
    }
}

mod v_3_0 {
    // AUTO-GENERATED FILE — DO NOT EDIT
    // Source: python3.0.grammar
    // Regenerate with: grammar-tools compile-grammar python3.0.grammar
    //
    // This file embeds a ParserGrammar as native Rust data structures.
    // Call `parser_grammar()` instead of reading and parsing the .grammar file.

    use grammar_tools::parser_grammar::{GrammarElement, GrammarRule, ParserGrammar};

    pub fn parser_grammar() -> ParserGrammar {
        ParserGrammar {
            rules: vec![
            GrammarRule {
                name: r#"file"#.to_string(),
                body: GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                        GrammarElement::RuleReference { name: r#"statement"#.to_string() },
                    ] }) },
                line_number: 103,
            },
            GrammarRule {
                name: r#"statement"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"compound_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"simple_stmt"#.to_string() },
                ] },
                line_number: 117,
            },
            GrammarRule {
                name: r#"simple_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"small_stmt"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"small_stmt"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                ] },
                line_number: 121,
            },
            GrammarRule {
                name: r#"small_stmt"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"return_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"yield_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"raise_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"break_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"continue_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pass_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"import_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"from_import_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"global_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"nonlocal_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"del_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"assert_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"assign_stmt"#.to_string() },
                ] },
                line_number: 123,
            },
            GrammarRule {
                name: r#"return_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"return"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"expression_list"#.to_string() }) },
                ] },
                line_number: 173,
            },
            GrammarRule {
                name: r#"yield_stmt"#.to_string(),
                body: GrammarElement::RuleReference { name: r#"yield_expr"#.to_string() },
                line_number: 182,
            },
            GrammarRule {
                name: r#"raise_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"raise"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::Literal { value: r#"from"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                                ] }) },
                        ] }) },
                ] },
                line_number: 211,
            },
            GrammarRule {
                name: r#"break_stmt"#.to_string(),
                body: GrammarElement::Literal { value: r#"break"#.to_string() },
                line_number: 214,
            },
            GrammarRule {
                name: r#"continue_stmt"#.to_string(),
                body: GrammarElement::Literal { value: r#"continue"#.to_string() },
                line_number: 217,
            },
            GrammarRule {
                name: r#"pass_stmt"#.to_string(),
                body: GrammarElement::Literal { value: r#"pass"#.to_string() },
                line_number: 227,
            },
            GrammarRule {
                name: r#"import_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"import"#.to_string() },
                    GrammarElement::RuleReference { name: r#"dotted_name"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"as"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"dotted_name"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::Literal { value: r#"as"#.to_string() },
                                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                                ] }) },
                        ] }) },
                ] },
                line_number: 234,
            },
            GrammarRule {
                name: r#"from_import_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"from"#.to_string() },
                    GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::Sequence { elements: vec![
                                GrammarElement::RuleReference { name: r#"dots"#.to_string() },
                                GrammarElement::RuleReference { name: r#"dotted_name"#.to_string() },
                            ] },
                            GrammarElement::RuleReference { name: r#"dots"#.to_string() },
                            GrammarElement::RuleReference { name: r#"dotted_name"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"import"#.to_string() },
                    GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                            GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                                GrammarElement::RuleReference { name: r#"import_names"#.to_string() },
                                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                            ] },
                            GrammarElement::RuleReference { name: r#"import_names"#.to_string() },
                        ] }) },
                ] },
                line_number: 248,
            },
            GrammarRule {
                name: r#"dots"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"DOT"#.to_string() }) },
                ] },
                line_number: 253,
            },
            GrammarRule {
                name: r#"import_names"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"as"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::Literal { value: r#"as"#.to_string() },
                                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                ] },
                line_number: 255,
            },
            GrammarRule {
                name: r#"dotted_name"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 257,
            },
            GrammarRule {
                name: r#"global_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"global"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 263,
            },
            GrammarRule {
                name: r#"nonlocal_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"nonlocal"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 295,
            },
            GrammarRule {
                name: r#"del_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"del"#.to_string() },
                    GrammarElement::RuleReference { name: r#"target_list"#.to_string() },
                ] },
                line_number: 302,
            },
            GrammarRule {
                name: r#"assert_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"assert"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                ] },
                line_number: 307,
            },
            GrammarRule {
                name: r#"assign_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"expression_list"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"expression_list"#.to_string() },
                                ] }) },
                            GrammarElement::Sequence { elements: vec![
                                GrammarElement::RuleReference { name: r#"augmented_assign_op"#.to_string() },
                                GrammarElement::RuleReference { name: r#"expression_list"#.to_string() },
                            ] },
                        ] }) },
                ] },
                line_number: 337,
            },
            GrammarRule {
                name: r#"augmented_assign_op"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"PLUS_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"MINUS_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STAR_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SLASH_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"FLOOR_DIV_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PERCENT_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"AMP_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PIPE_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CARET_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LEFT_SHIFT_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RIGHT_SHIFT_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"DOUBLE_STAR_EQUALS"#.to_string() },
                ] },
                line_number: 341,
            },
            GrammarRule {
                name: r#"compound_stmt"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"if_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"while_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"for_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"try_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"with_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"def_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"class_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"decorated"#.to_string() },
                ] },
                line_number: 350,
            },
            GrammarRule {
                name: r#"if_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"if"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"elif"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"else"#.to_string() },
                            GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                        ] }) },
                ] },
                line_number: 369,
            },
            GrammarRule {
                name: r#"while_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"while"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"else"#.to_string() },
                            GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                        ] }) },
                ] },
                line_number: 381,
            },
            GrammarRule {
                name: r#"for_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"for"#.to_string() },
                    GrammarElement::RuleReference { name: r#"target_list"#.to_string() },
                    GrammarElement::Literal { value: r#"in"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression_list"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"else"#.to_string() },
                            GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                        ] }) },
                ] },
                line_number: 398,
            },
            GrammarRule {
                name: r#"try_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"try"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                    GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::Sequence { elements: vec![
                                GrammarElement::RuleReference { name: r#"except_clause"#.to_string() },
                                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"except_clause"#.to_string() }) },
                                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                        GrammarElement::Literal { value: r#"else"#.to_string() },
                                        GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                                        GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                                    ] }) },
                                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                        GrammarElement::Literal { value: r#"finally"#.to_string() },
                                        GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                                        GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                                    ] }) },
                            ] },
                            GrammarElement::Sequence { elements: vec![
                                GrammarElement::Literal { value: r#"finally"#.to_string() },
                                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                                GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                            ] },
                        ] }) },
                ] },
                line_number: 432,
            },
            GrammarRule {
                name: r#"except_clause"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"except"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::Literal { value: r#"as"#.to_string() },
                                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                ] },
                line_number: 438,
            },
            GrammarRule {
                name: r#"with_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"with"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"as"#.to_string() },
                            GrammarElement::RuleReference { name: r#"target"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                ] },
                line_number: 456,
            },
            GrammarRule {
                name: r#"def_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"def"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"parameters"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                ] },
                line_number: 494,
            },
            GrammarRule {
                name: r#"parameters"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"parameter"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"parameter"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                ] },
                line_number: 529,
            },
            GrammarRule {
                name: r#"parameter"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"DOUBLE_STAR"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            ] }) },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            ] }) },
                    ] },
                    GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            ] }) },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            ] }) },
                    ] },
                ] },
                line_number: 531,
            },
            GrammarRule {
                name: r#"class_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"class"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"class_args"#.to_string() }) },
                            GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                ] },
                line_number: 566,
            },
            GrammarRule {
                name: r#"class_args"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"class_arg"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"class_arg"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                ] },
                line_number: 568,
            },
            GrammarRule {
                name: r#"class_arg"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"DOUBLE_STAR"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    ] },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 570,
            },
            GrammarRule {
                name: r#"decorated"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"decorator"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"decorator"#.to_string() }) },
                    GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::RuleReference { name: r#"def_stmt"#.to_string() },
                            GrammarElement::RuleReference { name: r#"class_stmt"#.to_string() },
                        ] }) },
                ] },
                line_number: 596,
            },
            GrammarRule {
                name: r#"decorator"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"AT"#.to_string() },
                    GrammarElement::RuleReference { name: r#"dotted_name"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"arguments"#.to_string() }) },
                            GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                ] },
                line_number: 598,
            },
            GrammarRule {
                name: r#"suite"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"simple_stmt"#.to_string() },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                        GrammarElement::TokenReference { name: r#"INDENT"#.to_string() },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"statement"#.to_string() }) },
                        GrammarElement::TokenReference { name: r#"DEDENT"#.to_string() },
                    ] },
                ] },
                line_number: 614,
            },
            GrammarRule {
                name: r#"expression_list"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                ] },
                line_number: 643,
            },
            GrammarRule {
                name: r#"expression"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"lambda_expr"#.to_string() },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"or_expr"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::Literal { value: r#"if"#.to_string() },
                                GrammarElement::RuleReference { name: r#"or_expr"#.to_string() },
                                GrammarElement::Literal { value: r#"else"#.to_string() },
                                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            ] }) },
                    ] },
                ] },
                line_number: 648,
            },
            GrammarRule {
                name: r#"lambda_expr"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"lambda"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"lambda_params"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 656,
            },
            GrammarRule {
                name: r#"lambda_params"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"lambda_param"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"lambda_param"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                ] },
                line_number: 657,
            },
            GrammarRule {
                name: r#"lambda_param"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            ] }) },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"DOUBLE_STAR"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    ] },
                    GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                ] },
                line_number: 658,
            },
            GrammarRule {
                name: r#"or_expr"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"and_expr"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"or"#.to_string() },
                            GrammarElement::RuleReference { name: r#"and_expr"#.to_string() },
                        ] }) },
                ] },
                line_number: 662,
            },
            GrammarRule {
                name: r#"and_expr"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"not_expr"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"and"#.to_string() },
                            GrammarElement::RuleReference { name: r#"not_expr"#.to_string() },
                        ] }) },
                ] },
                line_number: 666,
            },
            GrammarRule {
                name: r#"not_expr"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"not"#.to_string() },
                        GrammarElement::RuleReference { name: r#"not_expr"#.to_string() },
                    ] },
                    GrammarElement::RuleReference { name: r#"comparison"#.to_string() },
                ] },
                line_number: 670,
            },
            GrammarRule {
                name: r#"comparison"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"bitwise_or"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"comp_op"#.to_string() },
                            GrammarElement::RuleReference { name: r#"bitwise_or"#.to_string() },
                        ] }) },
                ] },
                line_number: 687,
            },
            GrammarRule {
                name: r#"comp_op"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"EQUALS_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NOT_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"GREATER_EQUALS"#.to_string() },
                    GrammarElement::Literal { value: r#"in"#.to_string() },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"not"#.to_string() },
                        GrammarElement::Literal { value: r#"in"#.to_string() },
                    ] },
                    GrammarElement::Literal { value: r#"is"#.to_string() },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"is"#.to_string() },
                        GrammarElement::Literal { value: r#"not"#.to_string() },
                    ] },
                ] },
                line_number: 689,
            },
            GrammarRule {
                name: r#"bitwise_or"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"bitwise_xor"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"PIPE"#.to_string() },
                            GrammarElement::RuleReference { name: r#"bitwise_xor"#.to_string() },
                        ] }) },
                ] },
                line_number: 700,
            },
            GrammarRule {
                name: r#"bitwise_xor"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"bitwise_and"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"CARET"#.to_string() },
                            GrammarElement::RuleReference { name: r#"bitwise_and"#.to_string() },
                        ] }) },
                ] },
                line_number: 701,
            },
            GrammarRule {
                name: r#"bitwise_and"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"shift"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"AMP"#.to_string() },
                            GrammarElement::RuleReference { name: r#"shift"#.to_string() },
                        ] }) },
                ] },
                line_number: 702,
            },
            GrammarRule {
                name: r#"shift"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"arith"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                    GrammarElement::TokenReference { name: r#"LEFT_SHIFT"#.to_string() },
                                    GrammarElement::TokenReference { name: r#"RIGHT_SHIFT"#.to_string() },
                                ] }) },
                            GrammarElement::RuleReference { name: r#"arith"#.to_string() },
                        ] }) },
                ] },
                line_number: 705,
            },
            GrammarRule {
                name: r#"arith"#.to_string(),
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
                line_number: 709,
            },
            GrammarRule {
                name: r#"term"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"factor"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                    GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                                    GrammarElement::TokenReference { name: r#"SLASH"#.to_string() },
                                    GrammarElement::TokenReference { name: r#"FLOOR_DIV"#.to_string() },
                                    GrammarElement::TokenReference { name: r#"PERCENT"#.to_string() },
                                ] }) },
                            GrammarElement::RuleReference { name: r#"factor"#.to_string() },
                        ] }) },
                ] },
                line_number: 726,
            },
            GrammarRule {
                name: r#"factor"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                                GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                                GrammarElement::TokenReference { name: r#"TILDE"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"factor"#.to_string() },
                    ] },
                    GrammarElement::RuleReference { name: r#"power"#.to_string() },
                ] },
                line_number: 732,
            },
            GrammarRule {
                name: r#"power"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"primary"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"DOUBLE_STAR"#.to_string() },
                            GrammarElement::RuleReference { name: r#"factor"#.to_string() },
                        ] }) },
                ] },
                line_number: 738,
            },
            GrammarRule {
                name: r#"primary"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"atom"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"suffix"#.to_string() }) },
                ] },
                line_number: 753,
            },
            GrammarRule {
                name: r#"suffix"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                        GrammarElement::RuleReference { name: r#"subscript"#.to_string() },
                        GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"arguments"#.to_string() }) },
                        GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                    ] },
                ] },
                line_number: 755,
            },
            GrammarRule {
                name: r#"subscript"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"expression"#.to_string() }) },
                        GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"expression"#.to_string() }) },
                        GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"expression"#.to_string() }) },
                            ] }) },
                    ] },
                ] },
                line_number: 768,
            },
            GrammarRule {
                name: r#"atom"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"INT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"FLOAT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"IMAG"#.to_string() },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"STRING"#.to_string() }) },
                    ] },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Literal { value: r#"True"#.to_string() },
                    GrammarElement::Literal { value: r#"False"#.to_string() },
                    GrammarElement::Literal { value: r#"None"#.to_string() },
                    GrammarElement::TokenReference { name: r#"ELLIPSIS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"list_expr"#.to_string() },
                    GrammarElement::RuleReference { name: r#"dict_or_set_expr"#.to_string() },
                    GrammarElement::RuleReference { name: r#"paren_expr"#.to_string() },
                    GrammarElement::RuleReference { name: r#"generator_expr"#.to_string() },
                ] },
                line_number: 777,
            },
            GrammarRule {
                name: r#"yield_expr"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"yield"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"expression_list"#.to_string() }) },
                ] },
                line_number: 821,
            },
            GrammarRule {
                name: r#"list_expr"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"list_body"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                ] },
                line_number: 830,
            },
            GrammarRule {
                name: r#"list_body"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        GrammarElement::RuleReference { name: r#"comp_clause"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            ] }) },
                        GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                    ] },
                ] },
                line_number: 832,
            },
            GrammarRule {
                name: r#"dict_or_set_expr"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"dict_or_set_body"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 849,
            },
            GrammarRule {
                name: r#"dict_or_set_body"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        GrammarElement::RuleReference { name: r#"comp_clause"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            ] }) },
                        GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        GrammarElement::RuleReference { name: r#"comp_clause"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            ] }) },
                        GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                    ] },
                ] },
                line_number: 851,
            },
            GrammarRule {
                name: r#"paren_expr"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"paren_body"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 861,
            },
            GrammarRule {
                name: r#"paren_body"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                                    ] }) },
                                GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                            ] }) },
                    ] },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 863,
            },
            GrammarRule {
                name: r#"generator_expr"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"comp_clause"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 873,
            },
            GrammarRule {
                name: r#"comp_clause"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"comp_for"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::RuleReference { name: r#"comp_for"#.to_string() },
                            GrammarElement::RuleReference { name: r#"comp_if"#.to_string() },
                        ] }) },
                ] },
                line_number: 887,
            },
            GrammarRule {
                name: r#"comp_for"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"for"#.to_string() },
                    GrammarElement::RuleReference { name: r#"target_list"#.to_string() },
                    GrammarElement::Literal { value: r#"in"#.to_string() },
                    GrammarElement::RuleReference { name: r#"or_expr"#.to_string() },
                ] },
                line_number: 889,
            },
            GrammarRule {
                name: r#"comp_if"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"if"#.to_string() },
                    GrammarElement::RuleReference { name: r#"or_expr"#.to_string() },
                ] },
                line_number: 891,
            },
            GrammarRule {
                name: r#"target_list"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"target"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"target"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                ] },
                line_number: 919,
            },
            GrammarRule {
                name: r#"target"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    ] },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::RuleReference { name: r#"primary"#.to_string() },
                ] },
                line_number: 921,
            },
            GrammarRule {
                name: r#"arguments"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"argument"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"argument"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                ] },
                line_number: 945,
            },
            GrammarRule {
                name: r#"argument"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"DOUBLE_STAR"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    ] },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 947,
            },
        ],
            version: 1,
        }
    }
}

mod v_3_6 {
    // AUTO-GENERATED FILE — DO NOT EDIT
    // Source: python3.6.grammar
    // Regenerate with: grammar-tools compile-grammar python3.6.grammar
    //
    // This file embeds a ParserGrammar as native Rust data structures.
    // Call `parser_grammar()` instead of reading and parsing the .grammar file.

    use grammar_tools::parser_grammar::{GrammarElement, GrammarRule, ParserGrammar};

    pub fn parser_grammar() -> ParserGrammar {
        ParserGrammar {
            rules: vec![
            GrammarRule {
                name: r#"file"#.to_string(),
                body: GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                        GrammarElement::RuleReference { name: r#"statement"#.to_string() },
                    ] }) },
                line_number: 39,
            },
            GrammarRule {
                name: r#"statement"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"compound_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"simple_stmt"#.to_string() },
                ] },
                line_number: 53,
            },
            GrammarRule {
                name: r#"simple_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"small_stmt"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"small_stmt"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                ] },
                line_number: 58,
            },
            GrammarRule {
                name: r#"small_stmt"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"return_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"break_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"continue_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pass_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"import_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"from_import_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"raise_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"yield_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"del_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"assert_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"global_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"nonlocal_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"annotated_assign_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"assign_stmt"#.to_string() },
                ] },
                line_number: 60,
            },
            GrammarRule {
                name: r#"return_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"return"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"expression_list"#.to_string() }) },
                ] },
                line_number: 83,
            },
            GrammarRule {
                name: r#"break_stmt"#.to_string(),
                body: GrammarElement::Literal { value: r#"break"#.to_string() },
                line_number: 86,
            },
            GrammarRule {
                name: r#"continue_stmt"#.to_string(),
                body: GrammarElement::Literal { value: r#"continue"#.to_string() },
                line_number: 89,
            },
            GrammarRule {
                name: r#"pass_stmt"#.to_string(),
                body: GrammarElement::Literal { value: r#"pass"#.to_string() },
                line_number: 97,
            },
            GrammarRule {
                name: r#"import_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"import"#.to_string() },
                    GrammarElement::RuleReference { name: r#"dotted_name"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"as"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"dotted_name"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::Literal { value: r#"as"#.to_string() },
                                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                                ] }) },
                        ] }) },
                ] },
                line_number: 104,
            },
            GrammarRule {
                name: r#"from_import_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"from"#.to_string() },
                    GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::RuleReference { name: r#"dotted_name"#.to_string() },
                            GrammarElement::RuleReference { name: r#"relative_import"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"import"#.to_string() },
                    GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                            GrammarElement::RuleReference { name: r#"import_names"#.to_string() },
                        ] }) },
                ] },
                line_number: 113,
            },
            GrammarRule {
                name: r#"relative_import"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"DOT"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"dotted_name"#.to_string() }) },
                ] },
                line_number: 116,
            },
            GrammarRule {
                name: r#"import_names"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"import_name"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"import_name"#.to_string() },
                        ] }) },
                ] },
                line_number: 118,
            },
            GrammarRule {
                name: r#"import_name"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"as"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 119,
            },
            GrammarRule {
                name: r#"dotted_name"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 121,
            },
            GrammarRule {
                name: r#"raise_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"raise"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::Literal { value: r#"from"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                                ] }) },
                        ] }) },
                ] },
                line_number: 127,
            },
            GrammarRule {
                name: r#"yield_stmt"#.to_string(),
                body: GrammarElement::RuleReference { name: r#"yield_expr"#.to_string() },
                line_number: 133,
            },
            GrammarRule {
                name: r#"yield_expr"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"yield"#.to_string() },
                        GrammarElement::Literal { value: r#"from"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"yield"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"expression_list"#.to_string() }) },
                    ] },
                ] },
                line_number: 135,
            },
            GrammarRule {
                name: r#"del_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"del"#.to_string() },
                    GrammarElement::RuleReference { name: r#"target_list"#.to_string() },
                ] },
                line_number: 143,
            },
            GrammarRule {
                name: r#"assert_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"assert"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                ] },
                line_number: 148,
            },
            GrammarRule {
                name: r#"global_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"global"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 153,
            },
            GrammarRule {
                name: r#"nonlocal_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"nonlocal"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 158,
            },
            GrammarRule {
                name: r#"annotated_assign_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"target"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                ] },
                line_number: 198,
            },
            GrammarRule {
                name: r#"assign_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"expression_list"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                                GrammarElement::RuleReference { name: r#"expression_list"#.to_string() },
                                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                        GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                                        GrammarElement::RuleReference { name: r#"expression_list"#.to_string() },
                                    ] }) },
                            ] },
                            GrammarElement::Sequence { elements: vec![
                                GrammarElement::RuleReference { name: r#"augmented_assign_op"#.to_string() },
                                GrammarElement::RuleReference { name: r#"expression_list"#.to_string() },
                            ] },
                        ] }) },
                ] },
                line_number: 212,
            },
            GrammarRule {
                name: r#"augmented_assign_op"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"PLUS_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"MINUS_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STAR_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SLASH_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"FLOOR_DIV_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PERCENT_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"AMP_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PIPE_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CARET_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LEFT_SHIFT_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RIGHT_SHIFT_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"DOUBLE_STAR_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"AT_EQUALS"#.to_string() },
                ] },
                line_number: 216,
            },
            GrammarRule {
                name: r#"compound_stmt"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"if_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"for_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"while_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"try_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"with_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"def_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"class_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"decorated_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"async_stmt"#.to_string() },
                ] },
                line_number: 228,
            },
            GrammarRule {
                name: r#"if_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"if"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"elif"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"else"#.to_string() },
                            GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                        ] }) },
                ] },
                line_number: 252,
            },
            GrammarRule {
                name: r#"for_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"for"#.to_string() },
                    GrammarElement::RuleReference { name: r#"target_list"#.to_string() },
                    GrammarElement::Literal { value: r#"in"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression_list"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"else"#.to_string() },
                            GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                        ] }) },
                ] },
                line_number: 276,
            },
            GrammarRule {
                name: r#"while_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"while"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"else"#.to_string() },
                            GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                        ] }) },
                ] },
                line_number: 289,
            },
            GrammarRule {
                name: r#"try_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"try"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                    GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::Sequence { elements: vec![
                                GrammarElement::RuleReference { name: r#"except_clauses"#.to_string() },
                                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                        GrammarElement::Literal { value: r#"else"#.to_string() },
                                        GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                                        GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                                    ] }) },
                                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                        GrammarElement::Literal { value: r#"finally"#.to_string() },
                                        GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                                        GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                                    ] }) },
                            ] },
                            GrammarElement::Sequence { elements: vec![
                                GrammarElement::Literal { value: r#"finally"#.to_string() },
                                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                                GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                            ] },
                        ] }) },
                ] },
                line_number: 327,
            },
            GrammarRule {
                name: r#"except_clauses"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"except_clause"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"except_clause"#.to_string() }) },
                ] },
                line_number: 331,
            },
            GrammarRule {
                name: r#"except_clause"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"except"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::Literal { value: r#"as"#.to_string() },
                                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                ] },
                line_number: 333,
            },
            GrammarRule {
                name: r#"with_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"with"#.to_string() },
                    GrammarElement::RuleReference { name: r#"with_item"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"with_item"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                ] },
                line_number: 349,
            },
            GrammarRule {
                name: r#"with_item"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"as"#.to_string() },
                            GrammarElement::RuleReference { name: r#"target"#.to_string() },
                        ] }) },
                ] },
                line_number: 351,
            },
            GrammarRule {
                name: r#"def_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"def"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"parameters"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                ] },
                line_number: 370,
            },
            GrammarRule {
                name: r#"class_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"class"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"arguments"#.to_string() }) },
                            GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                ] },
                line_number: 391,
            },
            GrammarRule {
                name: r#"decorated_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"decorator"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"decorator"#.to_string() }) },
                    GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::RuleReference { name: r#"def_stmt"#.to_string() },
                            GrammarElement::RuleReference { name: r#"class_stmt"#.to_string() },
                            GrammarElement::RuleReference { name: r#"async_stmt"#.to_string() },
                        ] }) },
                ] },
                line_number: 408,
            },
            GrammarRule {
                name: r#"decorator"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"AT"#.to_string() },
                    GrammarElement::RuleReference { name: r#"dotted_name"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"arguments"#.to_string() }) },
                            GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                ] },
                line_number: 410,
            },
            GrammarRule {
                name: r#"async_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"async"#.to_string() },
                    GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::RuleReference { name: r#"async_def_stmt"#.to_string() },
                            GrammarElement::RuleReference { name: r#"async_for_stmt"#.to_string() },
                            GrammarElement::RuleReference { name: r#"async_with_stmt"#.to_string() },
                        ] }) },
                ] },
                line_number: 461,
            },
            GrammarRule {
                name: r#"async_def_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"def"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"parameters"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                ] },
                line_number: 470,
            },
            GrammarRule {
                name: r#"async_for_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"for"#.to_string() },
                    GrammarElement::RuleReference { name: r#"target_list"#.to_string() },
                    GrammarElement::Literal { value: r#"in"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression_list"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"else"#.to_string() },
                            GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                        ] }) },
                ] },
                line_number: 479,
            },
            GrammarRule {
                name: r#"async_with_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"with"#.to_string() },
                    GrammarElement::RuleReference { name: r#"with_item"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"with_item"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                ] },
                line_number: 488,
            },
            GrammarRule {
                name: r#"suite"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"simple_stmt"#.to_string() },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                        GrammarElement::TokenReference { name: r#"INDENT"#.to_string() },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"statement"#.to_string() }) },
                        GrammarElement::TokenReference { name: r#"DEDENT"#.to_string() },
                    ] },
                ] },
                line_number: 505,
            },
            GrammarRule {
                name: r#"parameters"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"parameter"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"parameter"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                ] },
                line_number: 534,
            },
            GrammarRule {
                name: r#"parameter"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"DOUBLE_STAR"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            ] }) },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            ] }) },
                    ] },
                    GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            ] }) },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            ] }) },
                    ] },
                ] },
                line_number: 536,
            },
            GrammarRule {
                name: r#"target"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"primary"#.to_string() },
                        GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"primary"#.to_string() },
                        GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                        GrammarElement::RuleReference { name: r#"subscript"#.to_string() },
                        GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                    ] },
                ] },
                line_number: 557,
            },
            GrammarRule {
                name: r#"target_list"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"target_item"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"target_item"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                ] },
                line_number: 559,
            },
            GrammarRule {
                name: r#"target_item"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                        GrammarElement::RuleReference { name: r#"target"#.to_string() },
                    ] },
                    GrammarElement::RuleReference { name: r#"target"#.to_string() },
                ] },
                line_number: 561,
            },
            GrammarRule {
                name: r#"expression_list"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                ] },
                line_number: 595,
            },
            GrammarRule {
                name: r#"expression"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"lambda_expr"#.to_string() },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"or_expr"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::Literal { value: r#"if"#.to_string() },
                                GrammarElement::RuleReference { name: r#"or_expr"#.to_string() },
                                GrammarElement::Literal { value: r#"else"#.to_string() },
                                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            ] }) },
                    ] },
                ] },
                line_number: 598,
            },
            GrammarRule {
                name: r#"lambda_expr"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"lambda"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"lambda_params"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 604,
            },
            GrammarRule {
                name: r#"lambda_params"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"lambda_param"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"lambda_param"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                ] },
                line_number: 605,
            },
            GrammarRule {
                name: r#"lambda_param"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            ] }) },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    ] },
                    GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"DOUBLE_STAR"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    ] },
                ] },
                line_number: 606,
            },
            GrammarRule {
                name: r#"or_expr"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"and_expr"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"or"#.to_string() },
                            GrammarElement::RuleReference { name: r#"and_expr"#.to_string() },
                        ] }) },
                ] },
                line_number: 613,
            },
            GrammarRule {
                name: r#"and_expr"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"not_expr"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"and"#.to_string() },
                            GrammarElement::RuleReference { name: r#"not_expr"#.to_string() },
                        ] }) },
                ] },
                line_number: 617,
            },
            GrammarRule {
                name: r#"not_expr"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"not"#.to_string() },
                        GrammarElement::RuleReference { name: r#"not_expr"#.to_string() },
                    ] },
                    GrammarElement::RuleReference { name: r#"comparison"#.to_string() },
                ] },
                line_number: 621,
            },
            GrammarRule {
                name: r#"comparison"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"bitwise_or"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"comp_op"#.to_string() },
                            GrammarElement::RuleReference { name: r#"bitwise_or"#.to_string() },
                        ] }) },
                ] },
                line_number: 633,
            },
            GrammarRule {
                name: r#"comp_op"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"EQUALS_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NOT_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"GREATER_EQUALS"#.to_string() },
                    GrammarElement::Literal { value: r#"in"#.to_string() },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"not"#.to_string() },
                        GrammarElement::Literal { value: r#"in"#.to_string() },
                    ] },
                    GrammarElement::Literal { value: r#"is"#.to_string() },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"is"#.to_string() },
                        GrammarElement::Literal { value: r#"not"#.to_string() },
                    ] },
                ] },
                line_number: 635,
            },
            GrammarRule {
                name: r#"bitwise_or"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"bitwise_xor"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"PIPE"#.to_string() },
                            GrammarElement::RuleReference { name: r#"bitwise_xor"#.to_string() },
                        ] }) },
                ] },
                line_number: 645,
            },
            GrammarRule {
                name: r#"bitwise_xor"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"bitwise_and"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"CARET"#.to_string() },
                            GrammarElement::RuleReference { name: r#"bitwise_and"#.to_string() },
                        ] }) },
                ] },
                line_number: 646,
            },
            GrammarRule {
                name: r#"bitwise_and"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"shift"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"AMP"#.to_string() },
                            GrammarElement::RuleReference { name: r#"shift"#.to_string() },
                        ] }) },
                ] },
                line_number: 647,
            },
            GrammarRule {
                name: r#"shift"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"arith"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                    GrammarElement::TokenReference { name: r#"LEFT_SHIFT"#.to_string() },
                                    GrammarElement::TokenReference { name: r#"RIGHT_SHIFT"#.to_string() },
                                ] }) },
                            GrammarElement::RuleReference { name: r#"arith"#.to_string() },
                        ] }) },
                ] },
                line_number: 652,
            },
            GrammarRule {
                name: r#"arith"#.to_string(),
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
                line_number: 657,
            },
            GrammarRule {
                name: r#"term"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"factor"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                    GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                                    GrammarElement::TokenReference { name: r#"SLASH"#.to_string() },
                                    GrammarElement::TokenReference { name: r#"FLOOR_DIV"#.to_string() },
                                    GrammarElement::TokenReference { name: r#"PERCENT"#.to_string() },
                                    GrammarElement::TokenReference { name: r#"AT"#.to_string() },
                                ] }) },
                            GrammarElement::RuleReference { name: r#"factor"#.to_string() },
                        ] }) },
                ] },
                line_number: 665,
            },
            GrammarRule {
                name: r#"factor"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                                GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                                GrammarElement::TokenReference { name: r#"TILDE"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"factor"#.to_string() },
                    ] },
                    GrammarElement::RuleReference { name: r#"power"#.to_string() },
                ] },
                line_number: 671,
            },
            GrammarRule {
                name: r#"power"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"await_expr"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"DOUBLE_STAR"#.to_string() },
                            GrammarElement::RuleReference { name: r#"factor"#.to_string() },
                        ] }) },
                ] },
                line_number: 679,
            },
            GrammarRule {
                name: r#"await_expr"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"await"#.to_string() },
                        GrammarElement::RuleReference { name: r#"primary"#.to_string() },
                    ] },
                    GrammarElement::RuleReference { name: r#"primary"#.to_string() },
                ] },
                line_number: 710,
            },
            GrammarRule {
                name: r#"primary"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"atom"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"suffix"#.to_string() }) },
                ] },
                line_number: 728,
            },
            GrammarRule {
                name: r#"suffix"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                        GrammarElement::RuleReference { name: r#"subscript"#.to_string() },
                        GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"arguments"#.to_string() }) },
                        GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                    ] },
                ] },
                line_number: 730,
            },
            GrammarRule {
                name: r#"subscript"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"expression"#.to_string() }) },
                        GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"expression"#.to_string() }) },
                        GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"expression"#.to_string() }) },
                            ] }) },
                    ] },
                ] },
                line_number: 742,
            },
            GrammarRule {
                name: r#"atom"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"INT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"FLOAT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"IMAGINARY"#.to_string() },
                    GrammarElement::RuleReference { name: r#"string_atom"#.to_string() },
                    GrammarElement::RuleReference { name: r#"fstring"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Literal { value: r#"True"#.to_string() },
                    GrammarElement::Literal { value: r#"False"#.to_string() },
                    GrammarElement::Literal { value: r#"None"#.to_string() },
                    GrammarElement::TokenReference { name: r#"ELLIPSIS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"list_expr"#.to_string() },
                    GrammarElement::RuleReference { name: r#"dict_set_expr"#.to_string() },
                    GrammarElement::RuleReference { name: r#"paren_expr"#.to_string() },
                    GrammarElement::RuleReference { name: r#"yield_expr"#.to_string() },
                ] },
                line_number: 751,
            },
            GrammarRule {
                name: r#"string_atom"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                            GrammarElement::RuleReference { name: r#"fstring"#.to_string() },
                        ] }) },
                ] },
                line_number: 769,
            },
            GrammarRule {
                name: r#"fstring"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"FSTRING_START"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"fstring_part"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"FSTRING_END"#.to_string() },
                ] },
                line_number: 833,
            },
            GrammarRule {
                name: r#"fstring_part"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"FSTRING_MIDDLE"#.to_string() },
                    GrammarElement::RuleReference { name: r#"fstring_expr"#.to_string() },
                ] },
                line_number: 835,
            },
            GrammarRule {
                name: r#"fstring_expr"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACE_EXPR"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"fstring_conversion"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"fstring_format_spec"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"RBRACE_EXPR"#.to_string() },
                ] },
                line_number: 849,
            },
            GrammarRule {
                name: r#"fstring_conversion"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"EXCL"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                ] },
                line_number: 854,
            },
            GrammarRule {
                name: r#"fstring_format_spec"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"FSTRING_MIDDLE"#.to_string() },
                            GrammarElement::RuleReference { name: r#"fstring_expr"#.to_string() },
                        ] }) },
                ] },
                line_number: 862,
            },
            GrammarRule {
                name: r#"list_expr"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"list_body"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                ] },
                line_number: 878,
            },
            GrammarRule {
                name: r#"list_body"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        GrammarElement::RuleReference { name: r#"comp_clause"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            ] }) },
                        GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                    ] },
                ] },
                line_number: 880,
            },
            GrammarRule {
                name: r#"dict_set_expr"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"dict_set_body"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 902,
            },
            GrammarRule {
                name: r#"dict_set_body"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"dict_body"#.to_string() },
                    GrammarElement::RuleReference { name: r#"set_body"#.to_string() },
                ] },
                line_number: 904,
            },
            GrammarRule {
                name: r#"dict_body"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"dict_entry"#.to_string() },
                        GrammarElement::RuleReference { name: r#"comp_clause"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"dict_entry"#.to_string() },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                GrammarElement::RuleReference { name: r#"dict_entry"#.to_string() },
                            ] }) },
                        GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                    ] },
                ] },
                line_number: 906,
            },
            GrammarRule {
                name: r#"dict_entry"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"DOUBLE_STAR"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    ] },
                ] },
                line_number: 909,
            },
            GrammarRule {
                name: r#"set_body"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        GrammarElement::RuleReference { name: r#"comp_clause"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            ] }) },
                        GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                    ] },
                ] },
                line_number: 912,
            },
            GrammarRule {
                name: r#"paren_expr"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"paren_body"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 925,
            },
            GrammarRule {
                name: r#"paren_body"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        GrammarElement::RuleReference { name: r#"comp_clause"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                                    ] }) },
                                GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                            ] }) },
                    ] },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 927,
            },
            GrammarRule {
                name: r#"comp_clause"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"comp_for"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::RuleReference { name: r#"comp_for"#.to_string() },
                            GrammarElement::RuleReference { name: r#"comp_if"#.to_string() },
                        ] }) },
                ] },
                line_number: 955,
            },
            GrammarRule {
                name: r#"comp_for"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"for"#.to_string() },
                        GrammarElement::RuleReference { name: r#"target_list"#.to_string() },
                        GrammarElement::Literal { value: r#"in"#.to_string() },
                        GrammarElement::RuleReference { name: r#"or_expr"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"async"#.to_string() },
                        GrammarElement::Literal { value: r#"for"#.to_string() },
                        GrammarElement::RuleReference { name: r#"target_list"#.to_string() },
                        GrammarElement::Literal { value: r#"in"#.to_string() },
                        GrammarElement::RuleReference { name: r#"or_expr"#.to_string() },
                    ] },
                ] },
                line_number: 959,
            },
            GrammarRule {
                name: r#"comp_if"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"if"#.to_string() },
                    GrammarElement::RuleReference { name: r#"or_expr"#.to_string() },
                ] },
                line_number: 964,
            },
            GrammarRule {
                name: r#"arguments"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"argument"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"argument"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                ] },
                line_number: 987,
            },
            GrammarRule {
                name: r#"argument"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"DOUBLE_STAR"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    ] },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 989,
            },
        ],
            version: 1,
        }
    }
}

mod v_3_8 {
    // AUTO-GENERATED FILE — DO NOT EDIT
    // Source: python3.8.grammar
    // Regenerate with: grammar-tools compile-grammar python3.8.grammar
    //
    // This file embeds a ParserGrammar as native Rust data structures.
    // Call `parser_grammar()` instead of reading and parsing the .grammar file.

    use grammar_tools::parser_grammar::{GrammarElement, GrammarRule, ParserGrammar};

    pub fn parser_grammar() -> ParserGrammar {
        ParserGrammar {
            rules: vec![
            GrammarRule {
                name: r#"file_input"#.to_string(),
                body: GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                        GrammarElement::RuleReference { name: r#"statement"#.to_string() },
                    ] }) },
                line_number: 71,
            },
            GrammarRule {
                name: r#"single_input"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                    GrammarElement::RuleReference { name: r#"simple_stmt"#.to_string() },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"compound_stmt"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                    ] },
                ] },
                line_number: 72,
            },
            GrammarRule {
                name: r#"eval_input"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"expression_list"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                ] },
                line_number: 73,
            },
            GrammarRule {
                name: r#"statement"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"compound_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"simple_stmt"#.to_string() },
                ] },
                line_number: 94,
            },
            GrammarRule {
                name: r#"simple_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"small_stmt"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"small_stmt"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                ] },
                line_number: 102,
            },
            GrammarRule {
                name: r#"small_stmt"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"return_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"raise_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"import_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"from_import_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"global_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"nonlocal_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"assert_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"del_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"break_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"continue_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pass_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"yield_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"assign_stmt"#.to_string() },
                ] },
                line_number: 104,
            },
            GrammarRule {
                name: r#"return_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"return"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"expression_list"#.to_string() }) },
                ] },
                line_number: 130,
            },
            GrammarRule {
                name: r#"raise_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"raise"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::Literal { value: r#"from"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                                ] }) },
                        ] }) },
                ] },
                line_number: 140,
            },
            GrammarRule {
                name: r#"import_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"import"#.to_string() },
                    GrammarElement::RuleReference { name: r#"dotted_as_names"#.to_string() },
                ] },
                line_number: 149,
            },
            GrammarRule {
                name: r#"dotted_as_names"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"dotted_as_name"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"dotted_as_name"#.to_string() },
                        ] }) },
                ] },
                line_number: 151,
            },
            GrammarRule {
                name: r#"dotted_as_name"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"dotted_name"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"as"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 152,
            },
            GrammarRule {
                name: r#"dotted_name"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 153,
            },
            GrammarRule {
                name: r#"from_import_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"from"#.to_string() },
                    GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::Sequence { elements: vec![
                                GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"DOT"#.to_string() }) },
                                GrammarElement::RuleReference { name: r#"dotted_name"#.to_string() },
                            ] },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"DOT"#.to_string() }) },
                        ] }) },
                    GrammarElement::Literal { value: r#"import"#.to_string() },
                    GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                            GrammarElement::RuleReference { name: r#"import_as_names"#.to_string() },
                            GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                                GrammarElement::RuleReference { name: r#"import_as_names"#.to_string() },
                                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                            ] },
                        ] }) },
                ] },
                line_number: 168,
            },
            GrammarRule {
                name: r#"import_as_names"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"import_as_name"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"import_as_name"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                ] },
                line_number: 172,
            },
            GrammarRule {
                name: r#"import_as_name"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"as"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 173,
            },
            GrammarRule {
                name: r#"global_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"global"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 185,
            },
            GrammarRule {
                name: r#"nonlocal_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"nonlocal"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 200,
            },
            GrammarRule {
                name: r#"assert_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"assert"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                ] },
                line_number: 210,
            },
            GrammarRule {
                name: r#"del_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"del"#.to_string() },
                    GrammarElement::RuleReference { name: r#"target_list"#.to_string() },
                ] },
                line_number: 219,
            },
            GrammarRule {
                name: r#"break_stmt"#.to_string(),
                body: GrammarElement::Literal { value: r#"break"#.to_string() },
                line_number: 229,
            },
            GrammarRule {
                name: r#"continue_stmt"#.to_string(),
                body: GrammarElement::Literal { value: r#"continue"#.to_string() },
                line_number: 230,
            },
            GrammarRule {
                name: r#"pass_stmt"#.to_string(),
                body: GrammarElement::Literal { value: r#"pass"#.to_string() },
                line_number: 239,
            },
            GrammarRule {
                name: r#"yield_stmt"#.to_string(),
                body: GrammarElement::RuleReference { name: r#"yield_expr"#.to_string() },
                line_number: 250,
            },
            GrammarRule {
                name: r#"assign_stmt"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"expression_list"#.to_string() },
                        GrammarElement::RuleReference { name: r#"assign_tail"#.to_string() },
                    ] },
                    GrammarElement::RuleReference { name: r#"expression_list"#.to_string() },
                ] },
                line_number: 278,
            },
            GrammarRule {
                name: r#"assign_tail"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                                GrammarElement::RuleReference { name: r#"expression_list"#.to_string() },
                            ] }) },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                                GrammarElement::RuleReference { name: r#"expression_list"#.to_string() },
                            ] }) },
                        GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::RuleReference { name: r#"yield_expr"#.to_string() },
                                GrammarElement::RuleReference { name: r#"expression_list"#.to_string() },
                            ] }) },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"augmented_assign_op"#.to_string() },
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::RuleReference { name: r#"yield_expr"#.to_string() },
                                GrammarElement::RuleReference { name: r#"expression_list"#.to_string() },
                            ] }) },
                    ] },
                ] },
                line_number: 281,
            },
            GrammarRule {
                name: r#"augmented_assign_op"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"PLUS_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"MINUS_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STAR_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SLASH_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"FLOOR_DIV_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PERCENT_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"AMP_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PIPE_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CARET_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LEFT_SHIFT_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RIGHT_SHIFT_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"DOUBLE_STAR_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"AT_EQUALS"#.to_string() },
                ] },
                line_number: 292,
            },
            GrammarRule {
                name: r#"target_list"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"target"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"target"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                ] },
                line_number: 314,
            },
            GrammarRule {
                name: r#"target"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    ] },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 316,
            },
            GrammarRule {
                name: r#"compound_stmt"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"if_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"for_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"while_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"try_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"with_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"def_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"class_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"async_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"decorated"#.to_string() },
                ] },
                line_number: 326,
            },
            GrammarRule {
                name: r#"if_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"if"#.to_string() },
                    GrammarElement::RuleReference { name: r#"named_expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"elif"#.to_string() },
                            GrammarElement::RuleReference { name: r#"named_expression"#.to_string() },
                            GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"else"#.to_string() },
                            GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                        ] }) },
                ] },
                line_number: 354,
            },
            GrammarRule {
                name: r#"for_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"for"#.to_string() },
                    GrammarElement::RuleReference { name: r#"target_list"#.to_string() },
                    GrammarElement::Literal { value: r#"in"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression_list"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"else"#.to_string() },
                            GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                        ] }) },
                ] },
                line_number: 386,
            },
            GrammarRule {
                name: r#"while_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"while"#.to_string() },
                    GrammarElement::RuleReference { name: r#"named_expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"else"#.to_string() },
                            GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                        ] }) },
                ] },
                line_number: 406,
            },
            GrammarRule {
                name: r#"try_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"try"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                    GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::Sequence { elements: vec![
                                GrammarElement::RuleReference { name: r#"except_clauses"#.to_string() },
                                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                        GrammarElement::Literal { value: r#"else"#.to_string() },
                                        GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                                        GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                                    ] }) },
                                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                        GrammarElement::Literal { value: r#"finally"#.to_string() },
                                        GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                                        GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                                    ] }) },
                            ] },
                            GrammarElement::Sequence { elements: vec![
                                GrammarElement::Literal { value: r#"finally"#.to_string() },
                                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                                GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                            ] },
                        ] }) },
                ] },
                line_number: 448,
            },
            GrammarRule {
                name: r#"except_clauses"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"except_clause"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"except_clause"#.to_string() }) },
                ] },
                line_number: 452,
            },
            GrammarRule {
                name: r#"except_clause"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"except"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::Literal { value: r#"as"#.to_string() },
                                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                ] },
                line_number: 459,
            },
            GrammarRule {
                name: r#"with_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"with"#.to_string() },
                    GrammarElement::RuleReference { name: r#"with_item"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"with_item"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                ] },
                line_number: 480,
            },
            GrammarRule {
                name: r#"with_item"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"as"#.to_string() },
                            GrammarElement::RuleReference { name: r#"target"#.to_string() },
                        ] }) },
                ] },
                line_number: 482,
            },
            GrammarRule {
                name: r#"def_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"def"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"parameters"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                ] },
                line_number: 512,
            },
            GrammarRule {
                name: r#"parameters"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"parameter_list"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                ] },
                line_number: 552,
            },
            GrammarRule {
                name: r#"parameter_list"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"param_or_sep"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"param_or_sep"#.to_string() },
                        ] }) },
                ] },
                line_number: 563,
            },
            GrammarRule {
                name: r#"param_or_sep"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"DOUBLE_STAR"#.to_string() },
                        GrammarElement::RuleReference { name: r#"typed_param"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                        GrammarElement::RuleReference { name: r#"typed_param"#.to_string() },
                    ] },
                    GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SLASH"#.to_string() },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"typed_param"#.to_string() },
                        GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    ] },
                    GrammarElement::RuleReference { name: r#"typed_param"#.to_string() },
                ] },
                line_number: 565,
            },
            GrammarRule {
                name: r#"typed_param"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                ] },
                line_number: 575,
            },
            GrammarRule {
                name: r#"class_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"class"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"arguments"#.to_string() }) },
                            GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                ] },
                line_number: 602,
            },
            GrammarRule {
                name: r#"async_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"async"#.to_string() },
                    GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::RuleReference { name: r#"def_stmt"#.to_string() },
                            GrammarElement::RuleReference { name: r#"for_stmt"#.to_string() },
                            GrammarElement::RuleReference { name: r#"with_stmt"#.to_string() },
                        ] }) },
                ] },
                line_number: 621,
            },
            GrammarRule {
                name: r#"decorated"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"decorator"#.to_string() }) },
                    GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::RuleReference { name: r#"def_stmt"#.to_string() },
                            GrammarElement::RuleReference { name: r#"class_stmt"#.to_string() },
                            GrammarElement::RuleReference { name: r#"async_stmt"#.to_string() },
                        ] }) },
                ] },
                line_number: 647,
            },
            GrammarRule {
                name: r#"decorator"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"AT"#.to_string() },
                    GrammarElement::RuleReference { name: r#"dotted_name"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"arguments"#.to_string() }) },
                            GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                ] },
                line_number: 649,
            },
            GrammarRule {
                name: r#"suite"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"simple_stmt"#.to_string() },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                        GrammarElement::TokenReference { name: r#"INDENT"#.to_string() },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"statement"#.to_string() }) },
                        GrammarElement::TokenReference { name: r#"DEDENT"#.to_string() },
                    ] },
                ] },
                line_number: 670,
            },
            GrammarRule {
                name: r#"expression_list"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                ] },
                line_number: 709,
            },
            GrammarRule {
                name: r#"named_expression"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        GrammarElement::TokenReference { name: r#"COLONEQUALS"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    ] },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 756,
            },
            GrammarRule {
                name: r#"expression"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"lambda_expr"#.to_string() },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"or_expr"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::Literal { value: r#"if"#.to_string() },
                                GrammarElement::RuleReference { name: r#"or_expr"#.to_string() },
                                GrammarElement::Literal { value: r#"else"#.to_string() },
                                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            ] }) },
                    ] },
                ] },
                line_number: 763,
            },
            GrammarRule {
                name: r#"lambda_expr"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"lambda"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"lambda_params"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 784,
            },
            GrammarRule {
                name: r#"lambda_params"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"lambda_param"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"lambda_param"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                ] },
                line_number: 786,
            },
            GrammarRule {
                name: r#"lambda_param"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"DOUBLE_STAR"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    ] },
                    GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SLASH"#.to_string() },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    ] },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                ] },
                line_number: 788,
            },
            GrammarRule {
                name: r#"or_expr"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"and_expr"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"or"#.to_string() },
                            GrammarElement::RuleReference { name: r#"and_expr"#.to_string() },
                        ] }) },
                ] },
                line_number: 804,
            },
            GrammarRule {
                name: r#"and_expr"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"not_expr"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"and"#.to_string() },
                            GrammarElement::RuleReference { name: r#"not_expr"#.to_string() },
                        ] }) },
                ] },
                line_number: 811,
            },
            GrammarRule {
                name: r#"not_expr"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"not"#.to_string() },
                        GrammarElement::RuleReference { name: r#"not_expr"#.to_string() },
                    ] },
                    GrammarElement::RuleReference { name: r#"comparison"#.to_string() },
                ] },
                line_number: 817,
            },
            GrammarRule {
                name: r#"comparison"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"bitwise_or"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"comp_op"#.to_string() },
                            GrammarElement::RuleReference { name: r#"bitwise_or"#.to_string() },
                        ] }) },
                ] },
                line_number: 838,
            },
            GrammarRule {
                name: r#"comp_op"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"EQUALS_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NOT_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"GREATER_EQUALS"#.to_string() },
                    GrammarElement::Literal { value: r#"in"#.to_string() },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"not"#.to_string() },
                        GrammarElement::Literal { value: r#"in"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"is"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"not"#.to_string() }) },
                    ] },
                ] },
                line_number: 840,
            },
            GrammarRule {
                name: r#"bitwise_or"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"bitwise_xor"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"PIPE"#.to_string() },
                            GrammarElement::RuleReference { name: r#"bitwise_xor"#.to_string() },
                        ] }) },
                ] },
                line_number: 860,
            },
            GrammarRule {
                name: r#"bitwise_xor"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"bitwise_and"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"CARET"#.to_string() },
                            GrammarElement::RuleReference { name: r#"bitwise_and"#.to_string() },
                        ] }) },
                ] },
                line_number: 861,
            },
            GrammarRule {
                name: r#"bitwise_and"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"shift"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"AMP"#.to_string() },
                            GrammarElement::RuleReference { name: r#"shift"#.to_string() },
                        ] }) },
                ] },
                line_number: 862,
            },
            GrammarRule {
                name: r#"shift"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"arith"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                    GrammarElement::TokenReference { name: r#"LEFT_SHIFT"#.to_string() },
                                    GrammarElement::TokenReference { name: r#"RIGHT_SHIFT"#.to_string() },
                                ] }) },
                            GrammarElement::RuleReference { name: r#"arith"#.to_string() },
                        ] }) },
                ] },
                line_number: 867,
            },
            GrammarRule {
                name: r#"arith"#.to_string(),
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
                line_number: 875,
            },
            GrammarRule {
                name: r#"term"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"factor"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                    GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                                    GrammarElement::TokenReference { name: r#"SLASH"#.to_string() },
                                    GrammarElement::TokenReference { name: r#"FLOOR_DIV"#.to_string() },
                                    GrammarElement::TokenReference { name: r#"PERCENT"#.to_string() },
                                    GrammarElement::TokenReference { name: r#"AT"#.to_string() },
                                ] }) },
                            GrammarElement::RuleReference { name: r#"factor"#.to_string() },
                        ] }) },
                ] },
                line_number: 884,
            },
            GrammarRule {
                name: r#"factor"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                                GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                                GrammarElement::TokenReference { name: r#"TILDE"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"factor"#.to_string() },
                    ] },
                    GrammarElement::RuleReference { name: r#"power"#.to_string() },
                ] },
                line_number: 890,
            },
            GrammarRule {
                name: r#"power"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"await_expr"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"DOUBLE_STAR"#.to_string() },
                            GrammarElement::RuleReference { name: r#"factor"#.to_string() },
                        ] }) },
                ] },
                line_number: 899,
            },
            GrammarRule {
                name: r#"await_expr"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"await"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"primary"#.to_string() },
                ] },
                line_number: 912,
            },
            GrammarRule {
                name: r#"primary"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"atom"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"suffix"#.to_string() }) },
                ] },
                line_number: 930,
            },
            GrammarRule {
                name: r#"suffix"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                        GrammarElement::RuleReference { name: r#"subscript_list"#.to_string() },
                        GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"arguments"#.to_string() }) },
                        GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                    ] },
                ] },
                line_number: 932,
            },
            GrammarRule {
                name: r#"subscript_list"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"subscript"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"subscript"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                ] },
                line_number: 939,
            },
            GrammarRule {
                name: r#"subscript"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"expression"#.to_string() }) },
                        GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"expression"#.to_string() }) },
                        GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"expression"#.to_string() }) },
                            ] }) },
                    ] },
                ] },
                line_number: 946,
            },
            GrammarRule {
                name: r#"atom"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"INT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"FLOAT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"IMAG"#.to_string() },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"STRING"#.to_string() }) },
                    ] },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Literal { value: r#"True"#.to_string() },
                    GrammarElement::Literal { value: r#"False"#.to_string() },
                    GrammarElement::Literal { value: r#"None"#.to_string() },
                    GrammarElement::TokenReference { name: r#"ELLIPSIS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"list_expr"#.to_string() },
                    GrammarElement::RuleReference { name: r#"dict_expr"#.to_string() },
                    GrammarElement::RuleReference { name: r#"set_expr"#.to_string() },
                    GrammarElement::RuleReference { name: r#"paren_expr"#.to_string() },
                ] },
                line_number: 956,
            },
            GrammarRule {
                name: r#"list_expr"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"list_body"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                ] },
                line_number: 995,
            },
            GrammarRule {
                name: r#"list_body"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        GrammarElement::RuleReference { name: r#"comp_clause"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            ] }) },
                        GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                    ] },
                ] },
                line_number: 997,
            },
            GrammarRule {
                name: r#"dict_expr"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"dict_body"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 1014,
            },
            GrammarRule {
                name: r#"dict_body"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"dict_entry"#.to_string() },
                        GrammarElement::RuleReference { name: r#"comp_clause"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"dict_entry"#.to_string() },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                GrammarElement::RuleReference { name: r#"dict_entry"#.to_string() },
                            ] }) },
                        GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                    ] },
                ] },
                line_number: 1016,
            },
            GrammarRule {
                name: r#"dict_entry"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"DOUBLE_STAR"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    ] },
                ] },
                line_number: 1019,
            },
            GrammarRule {
                name: r#"set_expr"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::RuleReference { name: r#"set_body"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 1036,
            },
            GrammarRule {
                name: r#"set_body"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        GrammarElement::RuleReference { name: r#"comp_clause"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                                    ] }) },
                                GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                            ] }) },
                    ] },
                ] },
                line_number: 1038,
            },
            GrammarRule {
                name: r#"paren_expr"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"paren_body"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 1055,
            },
            GrammarRule {
                name: r#"paren_body"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        GrammarElement::RuleReference { name: r#"comp_clause"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                                    ] }) },
                                GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                            ] }) },
                    ] },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 1057,
            },
            GrammarRule {
                name: r#"comp_clause"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"comp_for"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::RuleReference { name: r#"comp_for"#.to_string() },
                            GrammarElement::RuleReference { name: r#"comp_if"#.to_string() },
                        ] }) },
                ] },
                line_number: 1087,
            },
            GrammarRule {
                name: r#"comp_for"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"for"#.to_string() },
                    GrammarElement::RuleReference { name: r#"target_list"#.to_string() },
                    GrammarElement::Literal { value: r#"in"#.to_string() },
                    GrammarElement::RuleReference { name: r#"or_expr"#.to_string() },
                ] },
                line_number: 1089,
            },
            GrammarRule {
                name: r#"comp_if"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"if"#.to_string() },
                    GrammarElement::RuleReference { name: r#"or_expr"#.to_string() },
                ] },
                line_number: 1091,
            },
            GrammarRule {
                name: r#"arguments"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"argument"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"argument"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                ] },
                line_number: 1116,
            },
            GrammarRule {
                name: r#"argument"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"DOUBLE_STAR"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        GrammarElement::RuleReference { name: r#"comp_clause"#.to_string() },
                    ] },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 1118,
            },
            GrammarRule {
                name: r#"yield_expr"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"yield"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::Sequence { elements: vec![
                                GrammarElement::Literal { value: r#"from"#.to_string() },
                                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            ] },
                            GrammarElement::RuleReference { name: r#"expression_list"#.to_string() },
                        ] }) },
                ] },
                line_number: 1147,
            },
        ],
            version: 1,
        }
    }
}

mod v_3_10 {
    // AUTO-GENERATED FILE — DO NOT EDIT
    // Source: python3.10.grammar
    // Regenerate with: grammar-tools compile-grammar python3.10.grammar
    //
    // This file embeds a ParserGrammar as native Rust data structures.
    // Call `parser_grammar()` instead of reading and parsing the .grammar file.

    use grammar_tools::parser_grammar::{GrammarElement, GrammarRule, ParserGrammar};

    pub fn parser_grammar() -> ParserGrammar {
        ParserGrammar {
            rules: vec![
            GrammarRule {
                name: r#"file"#.to_string(),
                body: GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                        GrammarElement::RuleReference { name: r#"statement"#.to_string() },
                    ] }) },
                line_number: 56,
            },
            GrammarRule {
                name: r#"statement"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"compound_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"simple_stmt"#.to_string() },
                ] },
                line_number: 70,
            },
            GrammarRule {
                name: r#"simple_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"small_stmt"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"small_stmt"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                ] },
                line_number: 74,
            },
            GrammarRule {
                name: r#"small_stmt"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"return_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"raise_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"assert_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"del_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pass_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"break_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"continue_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"import_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"global_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"nonlocal_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"assign_stmt"#.to_string() },
                ] },
                line_number: 76,
            },
            GrammarRule {
                name: r#"return_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"return"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"expression_list"#.to_string() }) },
                ] },
                line_number: 96,
            },
            GrammarRule {
                name: r#"raise_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"raise"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::Literal { value: r#"from"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                                ] }) },
                        ] }) },
                ] },
                line_number: 102,
            },
            GrammarRule {
                name: r#"assert_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"assert"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                ] },
                line_number: 107,
            },
            GrammarRule {
                name: r#"del_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"del"#.to_string() },
                    GrammarElement::RuleReference { name: r#"target_list"#.to_string() },
                ] },
                line_number: 114,
            },
            GrammarRule {
                name: r#"pass_stmt"#.to_string(),
                body: GrammarElement::Literal { value: r#"pass"#.to_string() },
                line_number: 119,
            },
            GrammarRule {
                name: r#"break_stmt"#.to_string(),
                body: GrammarElement::Literal { value: r#"break"#.to_string() },
                line_number: 122,
            },
            GrammarRule {
                name: r#"continue_stmt"#.to_string(),
                body: GrammarElement::Literal { value: r#"continue"#.to_string() },
                line_number: 125,
            },
            GrammarRule {
                name: r#"import_stmt"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"import"#.to_string() },
                        GrammarElement::RuleReference { name: r#"dotted_name_list"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"from"#.to_string() },
                        GrammarElement::RuleReference { name: r#"import_from"#.to_string() },
                        GrammarElement::Literal { value: r#"import"#.to_string() },
                        GrammarElement::RuleReference { name: r#"import_targets"#.to_string() },
                    ] },
                ] },
                line_number: 134,
            },
            GrammarRule {
                name: r#"dotted_name_list"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"dotted_name"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"dotted_name"#.to_string() },
                        ] }) },
                ] },
                line_number: 137,
            },
            GrammarRule {
                name: r#"dotted_name"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 138,
            },
            GrammarRule {
                name: r#"import_from"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"DOT"#.to_string() }) },
                        GrammarElement::RuleReference { name: r#"dotted_name"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"DOT"#.to_string() }) },
                        GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                    ] },
                ] },
                line_number: 140,
            },
            GrammarRule {
                name: r#"import_targets"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                        GrammarElement::RuleReference { name: r#"import_name_list"#.to_string() },
                        GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                    ] },
                    GrammarElement::RuleReference { name: r#"import_name_list"#.to_string() },
                ] },
                line_number: 141,
            },
            GrammarRule {
                name: r#"import_name_list"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"import_name"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"import_name"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                ] },
                line_number: 144,
            },
            GrammarRule {
                name: r#"import_name"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"as"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 145,
            },
            GrammarRule {
                name: r#"global_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"global"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 150,
            },
            GrammarRule {
                name: r#"nonlocal_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"nonlocal"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 159,
            },
            GrammarRule {
                name: r#"assign_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"expression_list"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::RuleReference { name: r#"assign_suffix"#.to_string() },
                            GrammarElement::RuleReference { name: r#"augmented_assign"#.to_string() },
                            GrammarElement::RuleReference { name: r#"annotation"#.to_string() },
                        ] }) },
                ] },
                line_number: 175,
            },
            GrammarRule {
                name: r#"assign_suffix"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression_list"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression_list"#.to_string() },
                        ] }) },
                ] },
                line_number: 178,
            },
            GrammarRule {
                name: r#"augmented_assign"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"augmented_assign_op"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression_list"#.to_string() },
                ] },
                line_number: 180,
            },
            GrammarRule {
                name: r#"augmented_assign_op"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"PLUS_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"MINUS_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STAR_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SLASH_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"FLOOR_DIV_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PERCENT_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"AMP_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PIPE_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CARET_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LEFT_SHIFT_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RIGHT_SHIFT_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"DOUBLE_STAR_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"AT_EQUALS"#.to_string() },
                ] },
                line_number: 182,
            },
            GrammarRule {
                name: r#"annotation"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression_list"#.to_string() },
                        ] }) },
                ] },
                line_number: 191,
            },
            GrammarRule {
                name: r#"compound_stmt"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"if_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"for_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"while_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"with_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"try_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"def_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"class_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"async_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"match_stmt"#.to_string() },
                ] },
                line_number: 199,
            },
            GrammarRule {
                name: r#"if_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"if"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"elif"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"else"#.to_string() },
                            GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                        ] }) },
                ] },
                line_number: 217,
            },
            GrammarRule {
                name: r#"for_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"for"#.to_string() },
                    GrammarElement::RuleReference { name: r#"target_list"#.to_string() },
                    GrammarElement::Literal { value: r#"in"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression_list"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"else"#.to_string() },
                            GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                        ] }) },
                ] },
                line_number: 230,
            },
            GrammarRule {
                name: r#"while_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"while"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"else"#.to_string() },
                            GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                        ] }) },
                ] },
                line_number: 239,
            },
            GrammarRule {
                name: r#"with_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"with"#.to_string() },
                    GrammarElement::RuleReference { name: r#"with_items"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                ] },
                line_number: 265,
            },
            GrammarRule {
                name: r#"with_items"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                        GrammarElement::RuleReference { name: r#"with_item"#.to_string() },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                GrammarElement::RuleReference { name: r#"with_item"#.to_string() },
                            ] }) },
                        GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                        GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"with_item"#.to_string() },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                GrammarElement::RuleReference { name: r#"with_item"#.to_string() },
                            ] }) },
                    ] },
                ] },
                line_number: 267,
            },
            GrammarRule {
                name: r#"with_item"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"as"#.to_string() },
                            GrammarElement::RuleReference { name: r#"target"#.to_string() },
                        ] }) },
                ] },
                line_number: 270,
            },
            GrammarRule {
                name: r#"try_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"try"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                    GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::Sequence { elements: vec![
                                GrammarElement::RuleReference { name: r#"except_clauses"#.to_string() },
                                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                        GrammarElement::Literal { value: r#"else"#.to_string() },
                                        GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                                        GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                                    ] }) },
                                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                        GrammarElement::Literal { value: r#"finally"#.to_string() },
                                        GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                                        GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                                    ] }) },
                            ] },
                            GrammarElement::Sequence { elements: vec![
                                GrammarElement::Literal { value: r#"finally"#.to_string() },
                                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                                GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                            ] },
                        ] }) },
                ] },
                line_number: 286,
            },
            GrammarRule {
                name: r#"except_clauses"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"except_clause"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"except_clause"#.to_string() }) },
                ] },
                line_number: 290,
            },
            GrammarRule {
                name: r#"except_clause"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"except"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::Literal { value: r#"as"#.to_string() },
                                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                ] },
                line_number: 291,
            },
            GrammarRule {
                name: r#"def_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"decorator"#.to_string() }) },
                    GrammarElement::Literal { value: r#"def"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"parameters"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                ] },
                line_number: 302,
            },
            GrammarRule {
                name: r#"class_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"decorator"#.to_string() }) },
                    GrammarElement::Literal { value: r#"class"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"arguments"#.to_string() }) },
                            GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                ] },
                line_number: 315,
            },
            GrammarRule {
                name: r#"decorator"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"AT"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                ] },
                line_number: 323,
            },
            GrammarRule {
                name: r#"async_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"async"#.to_string() },
                    GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::RuleReference { name: r#"def_stmt"#.to_string() },
                            GrammarElement::RuleReference { name: r#"for_stmt"#.to_string() },
                            GrammarElement::RuleReference { name: r#"with_stmt"#.to_string() },
                        ] }) },
                ] },
                line_number: 331,
            },
            GrammarRule {
                name: r#"suite"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"simple_stmt"#.to_string() },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                        GrammarElement::TokenReference { name: r#"INDENT"#.to_string() },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"statement"#.to_string() }) },
                        GrammarElement::TokenReference { name: r#"DEDENT"#.to_string() },
                    ] },
                ] },
                line_number: 340,
            },
            GrammarRule {
                name: r#"match_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"match"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"INDENT"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"case_clause"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"DEDENT"#.to_string() },
                ] },
                line_number: 401,
            },
            GrammarRule {
                name: r#"case_clause"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"case"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"guard"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                ] },
                line_number: 418,
            },
            GrammarRule {
                name: r#"guard"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"if"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 427,
            },
            GrammarRule {
                name: r#"pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"or_pattern"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"as"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 462,
            },
            GrammarRule {
                name: r#"or_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"closed_pattern"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"PIPE"#.to_string() },
                            GrammarElement::RuleReference { name: r#"closed_pattern"#.to_string() },
                        ] }) },
                ] },
                line_number: 477,
            },
            GrammarRule {
                name: r#"closed_pattern"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"literal_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"capture_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"wildcard_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"value_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"group_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"sequence_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"mapping_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"class_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"star_pattern"#.to_string() },
                ] },
                line_number: 486,
            },
            GrammarRule {
                name: r#"literal_pattern"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"signed_number"#.to_string() },
                    GrammarElement::RuleReference { name: r#"complex_number"#.to_string() },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"STRING"#.to_string() }) },
                    ] },
                    GrammarElement::Literal { value: r#"True"#.to_string() },
                    GrammarElement::Literal { value: r#"False"#.to_string() },
                    GrammarElement::Literal { value: r#"None"#.to_string() },
                ] },
                line_number: 523,
            },
            GrammarRule {
                name: r#"signed_number"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"MINUS"#.to_string() }) },
                    GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"INT"#.to_string() },
                            GrammarElement::TokenReference { name: r#"FLOAT"#.to_string() },
                        ] }) },
                ] },
                line_number: 533,
            },
            GrammarRule {
                name: r#"complex_number"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"signed_number"#.to_string() },
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                                GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                            ] }) },
                        GrammarElement::TokenReference { name: r#"IMAG"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"MINUS"#.to_string() }) },
                        GrammarElement::TokenReference { name: r#"IMAG"#.to_string() },
                    ] },
                ] },
                line_number: 542,
            },
            GrammarRule {
                name: r#"capture_pattern"#.to_string(),
                body: GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                line_number: 572,
            },
            GrammarRule {
                name: r#"wildcard_pattern"#.to_string(),
                body: GrammarElement::Literal { value: r#"_"#.to_string() },
                line_number: 592,
            },
            GrammarRule {
                name: r#"value_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 619,
            },
            GrammarRule {
                name: r#"group_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 635,
            },
            GrammarRule {
                name: r#"sequence_pattern"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"sequence_pattern_content"#.to_string() }) },
                        GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                        GrammarElement::RuleReference { name: r#"sequence_pattern_tuple_content"#.to_string() },
                        GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                    ] },
                ] },
                line_number: 663,
            },
            GrammarRule {
                name: r#"sequence_pattern_content"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"maybe_star_pattern"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"maybe_star_pattern"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                ] },
                line_number: 667,
            },
            GrammarRule {
                name: r#"sequence_pattern_tuple_content"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"maybe_star_pattern"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"maybe_star_pattern"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"maybe_star_pattern"#.to_string() },
                                ] }) },
                            GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                        ] }) },
                ] },
                line_number: 671,
            },
            GrammarRule {
                name: r#"maybe_star_pattern"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"star_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                ] },
                line_number: 674,
            },
            GrammarRule {
                name: r#"star_pattern"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                        GrammarElement::Literal { value: r#"_"#.to_string() },
                    ] },
                ] },
                line_number: 689,
            },
            GrammarRule {
                name: r#"mapping_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"mapping_pattern_content"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 724,
            },
            GrammarRule {
                name: r#"mapping_pattern_content"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"mapping_entry"#.to_string() },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                GrammarElement::RuleReference { name: r#"mapping_entry"#.to_string() },
                            ] }) },
                        GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"double_star_capture"#.to_string() }) },
                            ] }) },
                    ] },
                    GrammarElement::RuleReference { name: r#"double_star_capture"#.to_string() },
                ] },
                line_number: 726,
            },
            GrammarRule {
                name: r#"mapping_entry"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::RuleReference { name: r#"literal_pattern"#.to_string() },
                            GrammarElement::RuleReference { name: r#"value_pattern"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                ] },
                line_number: 729,
            },
            GrammarRule {
                name: r#"double_star_capture"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"DOUBLE_STAR"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"DOUBLE_STAR"#.to_string() },
                        GrammarElement::Literal { value: r#"_"#.to_string() },
                    ] },
                ] },
                line_number: 732,
            },
            GrammarRule {
                name: r#"class_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"name_or_dotted"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"class_pattern_content"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 785,
            },
            GrammarRule {
                name: r#"name_or_dotted"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 787,
            },
            GrammarRule {
                name: r#"class_pattern_content"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"class_positional_patterns"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                GrammarElement::RuleReference { name: r#"class_keyword_patterns"#.to_string() },
                            ] }) },
                        GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"class_keyword_patterns"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                    ] },
                ] },
                line_number: 789,
            },
            GrammarRule {
                name: r#"class_positional_patterns"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                        ] }) },
                ] },
                line_number: 792,
            },
            GrammarRule {
                name: r#"class_keyword_patterns"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"keyword_pattern"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"keyword_pattern"#.to_string() },
                        ] }) },
                ] },
                line_number: 794,
            },
            GrammarRule {
                name: r#"keyword_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                ] },
                line_number: 796,
            },
            GrammarRule {
                name: r#"expression_list"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                ] },
                line_number: 828,
            },
            GrammarRule {
                name: r#"expression"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"lambda_expr"#.to_string() },
                    GrammarElement::RuleReference { name: r#"walrus_expr"#.to_string() },
                ] },
                line_number: 834,
            },
            GrammarRule {
                name: r#"walrus_expr"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"or_expr"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::Literal { value: r#"if"#.to_string() },
                                GrammarElement::RuleReference { name: r#"or_expr"#.to_string() },
                                GrammarElement::Literal { value: r#"else"#.to_string() },
                                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            ] }) },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"or_expr"#.to_string() },
                        GrammarElement::TokenReference { name: r#"WALRUS"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    ] },
                ] },
                line_number: 837,
            },
            GrammarRule {
                name: r#"lambda_expr"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"lambda"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"lambda_params"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 844,
            },
            GrammarRule {
                name: r#"lambda_params"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"lambda_param"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"lambda_param"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                ] },
                line_number: 845,
            },
            GrammarRule {
                name: r#"lambda_param"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            ] }) },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"DOUBLE_STAR"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    ] },
                    GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                ] },
                line_number: 846,
            },
            GrammarRule {
                name: r#"or_expr"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"and_expr"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"or"#.to_string() },
                            GrammarElement::RuleReference { name: r#"and_expr"#.to_string() },
                        ] }) },
                ] },
                line_number: 849,
            },
            GrammarRule {
                name: r#"and_expr"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"not_expr"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"and"#.to_string() },
                            GrammarElement::RuleReference { name: r#"not_expr"#.to_string() },
                        ] }) },
                ] },
                line_number: 852,
            },
            GrammarRule {
                name: r#"not_expr"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"not"#.to_string() },
                        GrammarElement::RuleReference { name: r#"not_expr"#.to_string() },
                    ] },
                    GrammarElement::RuleReference { name: r#"comparison"#.to_string() },
                ] },
                line_number: 855,
            },
            GrammarRule {
                name: r#"comparison"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"bitwise_or"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"comp_op"#.to_string() },
                            GrammarElement::RuleReference { name: r#"bitwise_or"#.to_string() },
                        ] }) },
                ] },
                line_number: 862,
            },
            GrammarRule {
                name: r#"comp_op"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"EQUALS_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NOT_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"GREATER_EQUALS"#.to_string() },
                    GrammarElement::Literal { value: r#"in"#.to_string() },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"not"#.to_string() },
                        GrammarElement::Literal { value: r#"in"#.to_string() },
                    ] },
                    GrammarElement::Literal { value: r#"is"#.to_string() },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"is"#.to_string() },
                        GrammarElement::Literal { value: r#"not"#.to_string() },
                    ] },
                ] },
                line_number: 864,
            },
            GrammarRule {
                name: r#"bitwise_or"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"bitwise_xor"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"PIPE"#.to_string() },
                            GrammarElement::RuleReference { name: r#"bitwise_xor"#.to_string() },
                        ] }) },
                ] },
                line_number: 871,
            },
            GrammarRule {
                name: r#"bitwise_xor"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"bitwise_and"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"CARET"#.to_string() },
                            GrammarElement::RuleReference { name: r#"bitwise_and"#.to_string() },
                        ] }) },
                ] },
                line_number: 872,
            },
            GrammarRule {
                name: r#"bitwise_and"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"shift"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"AMP"#.to_string() },
                            GrammarElement::RuleReference { name: r#"shift"#.to_string() },
                        ] }) },
                ] },
                line_number: 873,
            },
            GrammarRule {
                name: r#"shift"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"arith"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                    GrammarElement::TokenReference { name: r#"LEFT_SHIFT"#.to_string() },
                                    GrammarElement::TokenReference { name: r#"RIGHT_SHIFT"#.to_string() },
                                ] }) },
                            GrammarElement::RuleReference { name: r#"arith"#.to_string() },
                        ] }) },
                ] },
                line_number: 876,
            },
            GrammarRule {
                name: r#"arith"#.to_string(),
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
                line_number: 879,
            },
            GrammarRule {
                name: r#"term"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"factor"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                    GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                                    GrammarElement::TokenReference { name: r#"SLASH"#.to_string() },
                                    GrammarElement::TokenReference { name: r#"FLOOR_DIV"#.to_string() },
                                    GrammarElement::TokenReference { name: r#"PERCENT"#.to_string() },
                                    GrammarElement::TokenReference { name: r#"AT"#.to_string() },
                                ] }) },
                            GrammarElement::RuleReference { name: r#"factor"#.to_string() },
                        ] }) },
                ] },
                line_number: 884,
            },
            GrammarRule {
                name: r#"factor"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                                GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                                GrammarElement::TokenReference { name: r#"TILDE"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"factor"#.to_string() },
                    ] },
                    GrammarElement::RuleReference { name: r#"power"#.to_string() },
                ] },
                line_number: 887,
            },
            GrammarRule {
                name: r#"power"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"await_expr"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"DOUBLE_STAR"#.to_string() },
                            GrammarElement::RuleReference { name: r#"factor"#.to_string() },
                        ] }) },
                ] },
                line_number: 892,
            },
            GrammarRule {
                name: r#"await_expr"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"await"#.to_string() },
                        GrammarElement::RuleReference { name: r#"primary"#.to_string() },
                    ] },
                    GrammarElement::RuleReference { name: r#"primary"#.to_string() },
                ] },
                line_number: 897,
            },
            GrammarRule {
                name: r#"primary"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"atom"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"suffix"#.to_string() }) },
                ] },
                line_number: 906,
            },
            GrammarRule {
                name: r#"suffix"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                        GrammarElement::RuleReference { name: r#"subscript"#.to_string() },
                        GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"arguments"#.to_string() }) },
                        GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                    ] },
                ] },
                line_number: 908,
            },
            GrammarRule {
                name: r#"subscript"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"subscript_item"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"subscript_item"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                ] },
                line_number: 916,
            },
            GrammarRule {
                name: r#"subscript_item"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"expression"#.to_string() }) },
                        GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"expression"#.to_string() }) },
                        GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"expression"#.to_string() }) },
                            ] }) },
                    ] },
                ] },
                line_number: 918,
            },
            GrammarRule {
                name: r#"atom"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"INT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"FLOAT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"IMAG"#.to_string() },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"STRING"#.to_string() }) },
                    ] },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"ELLIPSIS"#.to_string() },
                    GrammarElement::Literal { value: r#"True"#.to_string() },
                    GrammarElement::Literal { value: r#"False"#.to_string() },
                    GrammarElement::Literal { value: r#"None"#.to_string() },
                    GrammarElement::RuleReference { name: r#"list_expr"#.to_string() },
                    GrammarElement::RuleReference { name: r#"dict_or_set_expr"#.to_string() },
                    GrammarElement::RuleReference { name: r#"paren_expr"#.to_string() },
                ] },
                line_number: 927,
            },
            GrammarRule {
                name: r#"list_expr"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"list_body"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                ] },
                line_number: 944,
            },
            GrammarRule {
                name: r#"list_body"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        GrammarElement::RuleReference { name: r#"comp_clause"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            ] }) },
                        GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                    ] },
                ] },
                line_number: 946,
            },
            GrammarRule {
                name: r#"dict_or_set_expr"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"dict_or_set_body"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 958,
            },
            GrammarRule {
                name: r#"dict_or_set_body"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"dict_body"#.to_string() },
                    GrammarElement::RuleReference { name: r#"set_body"#.to_string() },
                ] },
                line_number: 960,
            },
            GrammarRule {
                name: r#"dict_body"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"dict_entry"#.to_string() },
                        GrammarElement::RuleReference { name: r#"comp_clause"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"dict_entry"#.to_string() },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                GrammarElement::RuleReference { name: r#"dict_entry"#.to_string() },
                            ] }) },
                        GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"DOUBLE_STAR"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                        GrammarElement::RuleReference { name: r#"dict_entry"#.to_string() },
                                        GrammarElement::Sequence { elements: vec![
                                            GrammarElement::TokenReference { name: r#"DOUBLE_STAR"#.to_string() },
                                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                                        ] },
                                    ] }) },
                            ] }) },
                        GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                    ] },
                ] },
                line_number: 962,
            },
            GrammarRule {
                name: r#"dict_entry"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 966,
            },
            GrammarRule {
                name: r#"set_body"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        GrammarElement::RuleReference { name: r#"comp_clause"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            ] }) },
                        GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                    ] },
                ] },
                line_number: 968,
            },
            GrammarRule {
                name: r#"paren_expr"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"paren_body"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 977,
            },
            GrammarRule {
                name: r#"paren_body"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        GrammarElement::RuleReference { name: r#"comp_clause"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                                    ] }) },
                                GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                            ] }) },
                    ] },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 979,
            },
            GrammarRule {
                name: r#"comp_clause"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"comp_for"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::RuleReference { name: r#"comp_for"#.to_string() },
                            GrammarElement::RuleReference { name: r#"comp_if"#.to_string() },
                        ] }) },
                ] },
                line_number: 999,
            },
            GrammarRule {
                name: r#"comp_for"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"async"#.to_string() }) },
                    GrammarElement::Literal { value: r#"for"#.to_string() },
                    GrammarElement::RuleReference { name: r#"target_list"#.to_string() },
                    GrammarElement::Literal { value: r#"in"#.to_string() },
                    GrammarElement::RuleReference { name: r#"or_expr"#.to_string() },
                ] },
                line_number: 1001,
            },
            GrammarRule {
                name: r#"comp_if"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"if"#.to_string() },
                    GrammarElement::RuleReference { name: r#"or_expr"#.to_string() },
                ] },
                line_number: 1003,
            },
            GrammarRule {
                name: r#"target"#.to_string(),
                body: GrammarElement::RuleReference { name: r#"primary"#.to_string() },
                line_number: 1020,
            },
            GrammarRule {
                name: r#"target_list"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"target"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"target"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                ] },
                line_number: 1021,
            },
            GrammarRule {
                name: r#"arguments"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"argument"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"argument"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                ] },
                line_number: 1039,
            },
            GrammarRule {
                name: r#"argument"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        GrammarElement::RuleReference { name: r#"comp_clause"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"DOUBLE_STAR"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    ] },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 1041,
            },
            GrammarRule {
                name: r#"parameters"#.to_string(),
                body: GrammarElement::RuleReference { name: r#"parameter_list"#.to_string() },
                line_number: 1068,
            },
            GrammarRule {
                name: r#"parameter_list"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"param_with_default"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"param_with_default"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::Alternation { choices: vec![
                                    GrammarElement::RuleReference { name: r#"slash_params"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"star_params"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"double_star_param"#.to_string() },
                                ] }) },
                        ] }) },
                ] },
                line_number: 1070,
            },
            GrammarRule {
                name: r#"slash_params"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"SLASH"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"param_with_default"#.to_string() },
                                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                            GrammarElement::RuleReference { name: r#"param_with_default"#.to_string() },
                                        ] }) },
                                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                            GrammarElement::Optional { element: Box::new(GrammarElement::Alternation { choices: vec![
                                                    GrammarElement::RuleReference { name: r#"star_params"#.to_string() },
                                                    GrammarElement::RuleReference { name: r#"double_star_param"#.to_string() },
                                                ] }) },
                                        ] }) },
                                ] }) },
                        ] }) },
                ] },
                line_number: 1073,
            },
            GrammarRule {
                name: r#"star_params"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"NAME"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"param_with_default"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"param_with_default"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"double_star_param"#.to_string() }) },
                        ] }) },
                ] },
                line_number: 1076,
            },
            GrammarRule {
                name: r#"double_star_param"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"DOUBLE_STAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                ] },
                line_number: 1079,
            },
            GrammarRule {
                name: r#"param_with_default"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                ] },
                line_number: 1081,
            },
        ],
            version: 1,
        }
    }
}

mod v_3_12 {
    // AUTO-GENERATED FILE — DO NOT EDIT
    // Source: python3.12.grammar
    // Regenerate with: grammar-tools compile-grammar python3.12.grammar
    //
    // This file embeds a ParserGrammar as native Rust data structures.
    // Call `parser_grammar()` instead of reading and parsing the .grammar file.

    use grammar_tools::parser_grammar::{GrammarElement, GrammarRule, ParserGrammar};

    pub fn parser_grammar() -> ParserGrammar {
        ParserGrammar {
            rules: vec![
            GrammarRule {
                name: r#"file"#.to_string(),
                body: GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                        GrammarElement::RuleReference { name: r#"statement"#.to_string() },
                    ] }) },
                line_number: 34,
            },
            GrammarRule {
                name: r#"statement"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"compound_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"simple_stmt"#.to_string() },
                ] },
                line_number: 43,
            },
            GrammarRule {
                name: r#"simple_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"small_stmt"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"small_stmt"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                ] },
                line_number: 45,
            },
            GrammarRule {
                name: r#"small_stmt"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"return_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"import_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"from_import_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"raise_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pass_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"del_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"yield_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"assert_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"break_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"continue_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"global_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"nonlocal_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_alias_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"assign_stmt"#.to_string() },
                ] },
                line_number: 47,
            },
            GrammarRule {
                name: r#"return_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"return"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"expression_list"#.to_string() }) },
                ] },
                line_number: 66,
            },
            GrammarRule {
                name: r#"pass_stmt"#.to_string(),
                body: GrammarElement::Literal { value: r#"pass"#.to_string() },
                line_number: 67,
            },
            GrammarRule {
                name: r#"break_stmt"#.to_string(),
                body: GrammarElement::Literal { value: r#"break"#.to_string() },
                line_number: 68,
            },
            GrammarRule {
                name: r#"continue_stmt"#.to_string(),
                body: GrammarElement::Literal { value: r#"continue"#.to_string() },
                line_number: 69,
            },
            GrammarRule {
                name: r#"del_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"del"#.to_string() },
                    GrammarElement::RuleReference { name: r#"target_list"#.to_string() },
                ] },
                line_number: 70,
            },
            GrammarRule {
                name: r#"yield_stmt"#.to_string(),
                body: GrammarElement::RuleReference { name: r#"yield_expr"#.to_string() },
                line_number: 71,
            },
            GrammarRule {
                name: r#"assert_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"assert"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                ] },
                line_number: 75,
            },
            GrammarRule {
                name: r#"global_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"global"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 80,
            },
            GrammarRule {
                name: r#"nonlocal_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"nonlocal"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 81,
            },
            GrammarRule {
                name: r#"import_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"import"#.to_string() },
                    GrammarElement::RuleReference { name: r#"dotted_name"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"as"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"dotted_name"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::Literal { value: r#"as"#.to_string() },
                                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                                ] }) },
                        ] }) },
                ] },
                line_number: 86,
            },
            GrammarRule {
                name: r#"from_import_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"from"#.to_string() },
                    GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::Sequence { elements: vec![
                                GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"DOT"#.to_string() }) },
                                GrammarElement::RuleReference { name: r#"dotted_name"#.to_string() },
                            ] },
                            GrammarElement::Sequence { elements: vec![
                                GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"DOT"#.to_string() }) },
                                GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                            ] },
                        ] }) },
                    GrammarElement::Literal { value: r#"import"#.to_string() },
                    GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                            GrammarElement::RuleReference { name: r#"import_names"#.to_string() },
                        ] }) },
                ] },
                line_number: 93,
            },
            GrammarRule {
                name: r#"import_names"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"as"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::Literal { value: r#"as"#.to_string() },
                                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                                ] }) },
                        ] }) },
                ] },
                line_number: 96,
            },
            GrammarRule {
                name: r#"dotted_name"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 97,
            },
            GrammarRule {
                name: r#"raise_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"raise"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::Literal { value: r#"from"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                                ] }) },
                        ] }) },
                ] },
                line_number: 103,
            },
            GrammarRule {
                name: r#"type_alias_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"type"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"type_params"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 119,
            },
            GrammarRule {
                name: r#"type_params"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_param"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_param"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                ] },
                line_number: 136,
            },
            GrammarRule {
                name: r#"type_param"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            ] }) },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"DOUBLE_STAR"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    ] },
                ] },
                line_number: 138,
            },
            GrammarRule {
                name: r#"assign_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"expression_list"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                        GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                                        GrammarElement::RuleReference { name: r#"expression_list"#.to_string() },
                                    ] }) },
                            ] },
                            GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                                GrammarElement::RuleReference { name: r#"expression_list"#.to_string() },
                                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                        GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                                        GrammarElement::RuleReference { name: r#"expression_list"#.to_string() },
                                    ] }) },
                            ] },
                            GrammarElement::Sequence { elements: vec![
                                GrammarElement::RuleReference { name: r#"augmented_assign_op"#.to_string() },
                                GrammarElement::RuleReference { name: r#"expression_list"#.to_string() },
                            ] },
                        ] }) },
                ] },
                line_number: 154,
            },
            GrammarRule {
                name: r#"augmented_assign_op"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"PLUS_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"MINUS_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STAR_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SLASH_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"FLOOR_DIV_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PERCENT_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"AMP_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PIPE_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CARET_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LEFT_SHIFT_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RIGHT_SHIFT_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"DOUBLE_STAR_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"AT_EQUALS"#.to_string() },
                ] },
                line_number: 159,
            },
            GrammarRule {
                name: r#"compound_stmt"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"if_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"for_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"while_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"try_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"with_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"def_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"class_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"decorated"#.to_string() },
                    GrammarElement::RuleReference { name: r#"async_stmt"#.to_string() },
                    GrammarElement::RuleReference { name: r#"match_stmt"#.to_string() },
                ] },
                line_number: 168,
            },
            GrammarRule {
                name: r#"if_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"if"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"elif"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"else"#.to_string() },
                            GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                        ] }) },
                ] },
                line_number: 180,
            },
            GrammarRule {
                name: r#"for_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"for"#.to_string() },
                    GrammarElement::RuleReference { name: r#"target_list"#.to_string() },
                    GrammarElement::Literal { value: r#"in"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression_list"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"else"#.to_string() },
                            GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                        ] }) },
                ] },
                line_number: 186,
            },
            GrammarRule {
                name: r#"while_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"while"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"else"#.to_string() },
                            GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                        ] }) },
                ] },
                line_number: 190,
            },
            GrammarRule {
                name: r#"try_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"try"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                    GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::Sequence { elements: vec![
                                GrammarElement::RuleReference { name: r#"except_clauses"#.to_string() },
                                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                        GrammarElement::Literal { value: r#"else"#.to_string() },
                                        GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                                        GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                                    ] }) },
                                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                        GrammarElement::Literal { value: r#"finally"#.to_string() },
                                        GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                                        GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                                    ] }) },
                            ] },
                            GrammarElement::Sequence { elements: vec![
                                GrammarElement::RuleReference { name: r#"except_star_clauses"#.to_string() },
                                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                        GrammarElement::Literal { value: r#"else"#.to_string() },
                                        GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                                        GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                                    ] }) },
                                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                        GrammarElement::Literal { value: r#"finally"#.to_string() },
                                        GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                                        GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                                    ] }) },
                            ] },
                            GrammarElement::Sequence { elements: vec![
                                GrammarElement::Literal { value: r#"finally"#.to_string() },
                                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                                GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                            ] },
                        ] }) },
                ] },
                line_number: 202,
            },
            GrammarRule {
                name: r#"except_clauses"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"except_clause"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"except_clause"#.to_string() }) },
                ] },
                line_number: 207,
            },
            GrammarRule {
                name: r#"except_clause"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"except"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::Literal { value: r#"as"#.to_string() },
                                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                ] },
                line_number: 214,
            },
            GrammarRule {
                name: r#"except_star_clauses"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"except_star_clause"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"except_star_clause"#.to_string() }) },
                ] },
                line_number: 234,
            },
            GrammarRule {
                name: r#"except_star_clause"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"except"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"as"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                ] },
                line_number: 236,
            },
            GrammarRule {
                name: r#"with_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"with"#.to_string() },
                    GrammarElement::RuleReference { name: r#"with_items"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                ] },
                line_number: 241,
            },
            GrammarRule {
                name: r#"with_items"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"with_item"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"with_item"#.to_string() },
                        ] }) },
                ] },
                line_number: 242,
            },
            GrammarRule {
                name: r#"with_item"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"as"#.to_string() },
                            GrammarElement::RuleReference { name: r#"target"#.to_string() },
                        ] }) },
                ] },
                line_number: 243,
            },
            GrammarRule {
                name: r#"def_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"def"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"type_params"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"parameters"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                ] },
                line_number: 258,
            },
            GrammarRule {
                name: r#"parameters"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"parameter"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"parameter"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                ] },
                line_number: 280,
            },
            GrammarRule {
                name: r#"parameter"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"DOUBLE_STAR"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            ] }) },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            ] }) },
                    ] },
                    GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SLASH"#.to_string() },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            ] }) },
                        GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            ] }) },
                    ] },
                ] },
                line_number: 282,
            },
            GrammarRule {
                name: r#"class_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"class"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"type_params"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"arguments"#.to_string() }) },
                            GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                ] },
                line_number: 300,
            },
            GrammarRule {
                name: r#"decorated"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"decorator"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"decorator"#.to_string() }) },
                    GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::RuleReference { name: r#"def_stmt"#.to_string() },
                            GrammarElement::RuleReference { name: r#"class_stmt"#.to_string() },
                            GrammarElement::RuleReference { name: r#"async_stmt"#.to_string() },
                        ] }) },
                ] },
                line_number: 307,
            },
            GrammarRule {
                name: r#"decorator"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"AT"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                ] },
                line_number: 309,
            },
            GrammarRule {
                name: r#"async_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"async"#.to_string() },
                    GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::RuleReference { name: r#"def_stmt"#.to_string() },
                            GrammarElement::RuleReference { name: r#"for_stmt"#.to_string() },
                            GrammarElement::RuleReference { name: r#"with_stmt"#.to_string() },
                        ] }) },
                ] },
                line_number: 313,
            },
            GrammarRule {
                name: r#"match_stmt"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"match"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"INDENT"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"case_clause"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"DEDENT"#.to_string() },
                ] },
                line_number: 329,
            },
            GrammarRule {
                name: r#"case_clause"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"case"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"if"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"suite"#.to_string() },
                ] },
                line_number: 333,
            },
            GrammarRule {
                name: r#"pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"or_pattern"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"as"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 344,
            },
            GrammarRule {
                name: r#"or_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"closed_pattern"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"PIPE"#.to_string() },
                            GrammarElement::RuleReference { name: r#"closed_pattern"#.to_string() },
                        ] }) },
                ] },
                line_number: 346,
            },
            GrammarRule {
                name: r#"closed_pattern"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"literal_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"capture_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"wildcard_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"sequence_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"mapping_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"class_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"group_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"value_pattern"#.to_string() },
                ] },
                line_number: 348,
            },
            GrammarRule {
                name: r#"literal_pattern"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"INT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"FLOAT"#.to_string() },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"STRING"#.to_string() }) },
                    ] },
                    GrammarElement::TokenReference { name: r#"IMAG"#.to_string() },
                    GrammarElement::Literal { value: r#"None"#.to_string() },
                    GrammarElement::Literal { value: r#"True"#.to_string() },
                    GrammarElement::Literal { value: r#"False"#.to_string() },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::TokenReference { name: r#"INT"#.to_string() },
                                GrammarElement::TokenReference { name: r#"FLOAT"#.to_string() },
                                GrammarElement::TokenReference { name: r#"IMAG"#.to_string() },
                            ] }) },
                    ] },
                ] },
                line_number: 358,
            },
            GrammarRule {
                name: r#"capture_pattern"#.to_string(),
                body: GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                line_number: 362,
            },
            GrammarRule {
                name: r#"wildcard_pattern"#.to_string(),
                body: GrammarElement::Literal { value: r#"_"#.to_string() },
                line_number: 363,
            },
            GrammarRule {
                name: r#"sequence_pattern"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"pattern_list"#.to_string() }) },
                        GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"pattern_list"#.to_string() }) },
                        GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                    ] },
                ] },
                line_number: 366,
            },
            GrammarRule {
                name: r#"pattern_list"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                ] },
                line_number: 369,
            },
            GrammarRule {
                name: r#"mapping_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"mapping_items"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 372,
            },
            GrammarRule {
                name: r#"mapping_items"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"mapping_item"#.to_string() },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                GrammarElement::RuleReference { name: r#"mapping_item"#.to_string() },
                            ] }) },
                        GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                        GrammarElement::TokenReference { name: r#"DOUBLE_STAR"#.to_string() },
                                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                                    ] }) },
                            ] }) },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"DOUBLE_STAR"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    ] },
                ] },
                line_number: 374,
            },
            GrammarRule {
                name: r#"mapping_item"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::RuleReference { name: r#"literal_pattern"#.to_string() },
                            GrammarElement::RuleReference { name: r#"value_pattern"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                ] },
                line_number: 377,
            },
            GrammarRule {
                name: r#"class_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"dotted_name"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"class_pattern_args"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 380,
            },
            GrammarRule {
                name: r#"class_pattern_args"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                            ] }) },
                        GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                        GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                                GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                                GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                            ] }) },
                        GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                            ] }) },
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                        GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                                GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                                GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                            ] }) },
                        GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                    ] },
                ] },
                line_number: 382,
            },
            GrammarRule {
                name: r#"group_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 387,
            },
            GrammarRule {
                name: r#"value_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 390,
            },
            GrammarRule {
                name: r#"suite"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"simple_stmt"#.to_string() },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                        GrammarElement::TokenReference { name: r#"INDENT"#.to_string() },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"statement"#.to_string() }) },
                        GrammarElement::TokenReference { name: r#"DEDENT"#.to_string() },
                    ] },
                ] },
                line_number: 399,
            },
            GrammarRule {
                name: r#"target_list"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"target"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"target"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                ] },
                line_number: 405,
            },
            GrammarRule {
                name: r#"target"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"target"#.to_string() },
                        GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"target"#.to_string() },
                        GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                        GrammarElement::RuleReference { name: r#"subscript"#.to_string() },
                        GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                        GrammarElement::RuleReference { name: r#"target"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                        GrammarElement::RuleReference { name: r#"target_list"#.to_string() },
                        GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                        GrammarElement::RuleReference { name: r#"target_list"#.to_string() },
                        GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                    ] },
                ] },
                line_number: 407,
            },
            GrammarRule {
                name: r#"expression_list"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                ] },
                line_number: 438,
            },
            GrammarRule {
                name: r#"named_expression"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        GrammarElement::TokenReference { name: r#"WALRUS"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    ] },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 442,
            },
            GrammarRule {
                name: r#"expression"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"lambda_expr"#.to_string() },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"or_expr"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::Literal { value: r#"if"#.to_string() },
                                GrammarElement::RuleReference { name: r#"or_expr"#.to_string() },
                                GrammarElement::Literal { value: r#"else"#.to_string() },
                                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            ] }) },
                    ] },
                    GrammarElement::RuleReference { name: r#"named_expression"#.to_string() },
                ] },
                line_number: 444,
            },
            GrammarRule {
                name: r#"lambda_expr"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"lambda"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"lambda_params"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 448,
            },
            GrammarRule {
                name: r#"lambda_params"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"lambda_param"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"lambda_param"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                ] },
                line_number: 449,
            },
            GrammarRule {
                name: r#"lambda_param"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            ] }) },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"DOUBLE_STAR"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    ] },
                    GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SLASH"#.to_string() },
                ] },
                line_number: 450,
            },
            GrammarRule {
                name: r#"or_expr"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"and_expr"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"or"#.to_string() },
                            GrammarElement::RuleReference { name: r#"and_expr"#.to_string() },
                        ] }) },
                ] },
                line_number: 456,
            },
            GrammarRule {
                name: r#"and_expr"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"not_expr"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"and"#.to_string() },
                            GrammarElement::RuleReference { name: r#"not_expr"#.to_string() },
                        ] }) },
                ] },
                line_number: 457,
            },
            GrammarRule {
                name: r#"not_expr"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"not"#.to_string() },
                        GrammarElement::RuleReference { name: r#"not_expr"#.to_string() },
                    ] },
                    GrammarElement::RuleReference { name: r#"comparison"#.to_string() },
                ] },
                line_number: 458,
            },
            GrammarRule {
                name: r#"comparison"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"bitwise_or"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"comp_op"#.to_string() },
                            GrammarElement::RuleReference { name: r#"bitwise_or"#.to_string() },
                        ] }) },
                ] },
                line_number: 461,
            },
            GrammarRule {
                name: r#"comp_op"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"EQUALS_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NOT_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"GREATER_EQUALS"#.to_string() },
                    GrammarElement::Literal { value: r#"in"#.to_string() },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"not"#.to_string() },
                        GrammarElement::Literal { value: r#"in"#.to_string() },
                    ] },
                    GrammarElement::Literal { value: r#"is"#.to_string() },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"is"#.to_string() },
                        GrammarElement::Literal { value: r#"not"#.to_string() },
                    ] },
                ] },
                line_number: 463,
            },
            GrammarRule {
                name: r#"bitwise_or"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"bitwise_xor"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"PIPE"#.to_string() },
                            GrammarElement::RuleReference { name: r#"bitwise_xor"#.to_string() },
                        ] }) },
                ] },
                line_number: 469,
            },
            GrammarRule {
                name: r#"bitwise_xor"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"bitwise_and"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"CARET"#.to_string() },
                            GrammarElement::RuleReference { name: r#"bitwise_and"#.to_string() },
                        ] }) },
                ] },
                line_number: 470,
            },
            GrammarRule {
                name: r#"bitwise_and"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"shift"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"AMP"#.to_string() },
                            GrammarElement::RuleReference { name: r#"shift"#.to_string() },
                        ] }) },
                ] },
                line_number: 471,
            },
            GrammarRule {
                name: r#"shift"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"arith"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                    GrammarElement::TokenReference { name: r#"LEFT_SHIFT"#.to_string() },
                                    GrammarElement::TokenReference { name: r#"RIGHT_SHIFT"#.to_string() },
                                ] }) },
                            GrammarElement::RuleReference { name: r#"arith"#.to_string() },
                        ] }) },
                ] },
                line_number: 472,
            },
            GrammarRule {
                name: r#"arith"#.to_string(),
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
                line_number: 473,
            },
            GrammarRule {
                name: r#"term"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"factor"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                    GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                                    GrammarElement::TokenReference { name: r#"SLASH"#.to_string() },
                                    GrammarElement::TokenReference { name: r#"FLOOR_DIV"#.to_string() },
                                    GrammarElement::TokenReference { name: r#"PERCENT"#.to_string() },
                                    GrammarElement::TokenReference { name: r#"AT"#.to_string() },
                                ] }) },
                            GrammarElement::RuleReference { name: r#"factor"#.to_string() },
                        ] }) },
                ] },
                line_number: 474,
            },
            GrammarRule {
                name: r#"factor"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                                GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                                GrammarElement::TokenReference { name: r#"TILDE"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"factor"#.to_string() },
                    ] },
                    GrammarElement::RuleReference { name: r#"power"#.to_string() },
                ] },
                line_number: 475,
            },
            GrammarRule {
                name: r#"power"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"await_expr"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"DOUBLE_STAR"#.to_string() },
                            GrammarElement::RuleReference { name: r#"factor"#.to_string() },
                        ] }) },
                ] },
                line_number: 479,
            },
            GrammarRule {
                name: r#"await_expr"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"await"#.to_string() },
                        GrammarElement::RuleReference { name: r#"primary"#.to_string() },
                    ] },
                    GrammarElement::RuleReference { name: r#"primary"#.to_string() },
                ] },
                line_number: 482,
            },
            GrammarRule {
                name: r#"primary"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"atom"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"suffix"#.to_string() }) },
                ] },
                line_number: 490,
            },
            GrammarRule {
                name: r#"suffix"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                        GrammarElement::RuleReference { name: r#"subscript"#.to_string() },
                        GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"arguments"#.to_string() }) },
                        GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                    ] },
                ] },
                line_number: 492,
            },
            GrammarRule {
                name: r#"subscript"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"expression"#.to_string() }) },
                        GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"expression"#.to_string() }) },
                        GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"expression"#.to_string() }) },
                            ] }) },
                    ] },
                ] },
                line_number: 498,
            },
            GrammarRule {
                name: r#"atom"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"INT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"FLOAT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"IMAG"#.to_string() },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"STRING"#.to_string() }) },
                    ] },
                    GrammarElement::RuleReference { name: r#"fstring"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Literal { value: r#"True"#.to_string() },
                    GrammarElement::Literal { value: r#"False"#.to_string() },
                    GrammarElement::Literal { value: r#"None"#.to_string() },
                    GrammarElement::TokenReference { name: r#"ELLIPSIS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"list_expr"#.to_string() },
                    GrammarElement::RuleReference { name: r#"dict_expr"#.to_string() },
                    GrammarElement::RuleReference { name: r#"set_expr"#.to_string() },
                    GrammarElement::RuleReference { name: r#"paren_expr"#.to_string() },
                ] },
                line_number: 505,
            },
            GrammarRule {
                name: r#"fstring"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"FSTRING_START"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"FSTRING_MIDDLE"#.to_string() },
                            GrammarElement::RuleReference { name: r#"fstring_replacement"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"FSTRING_END"#.to_string() },
                ] },
                line_number: 531,
            },
            GrammarRule {
                name: r#"fstring_replacement"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"fstring_conversion"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"fstring_format_spec"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 533,
            },
            GrammarRule {
                name: r#"fstring_conversion"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"!"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                ] },
                line_number: 537,
            },
            GrammarRule {
                name: r#"fstring_format_spec"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"FSTRING_MIDDLE"#.to_string() },
                            GrammarElement::RuleReference { name: r#"fstring_replacement"#.to_string() },
                        ] }) },
                ] },
                line_number: 541,
            },
            GrammarRule {
                name: r#"list_expr"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"list_body"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                ] },
                line_number: 548,
            },
            GrammarRule {
                name: r#"list_body"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        GrammarElement::RuleReference { name: r#"comp_clause"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            ] }) },
                        GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                    ] },
                ] },
                line_number: 550,
            },
            GrammarRule {
                name: r#"dict_expr"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"dict_body"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 555,
            },
            GrammarRule {
                name: r#"dict_body"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"dict_entry"#.to_string() },
                        GrammarElement::RuleReference { name: r#"comp_clause"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"dict_entry"#.to_string() },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                GrammarElement::RuleReference { name: r#"dict_entry"#.to_string() },
                            ] }) },
                        GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                    ] },
                ] },
                line_number: 557,
            },
            GrammarRule {
                name: r#"dict_entry"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"DOUBLE_STAR"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    ] },
                ] },
                line_number: 560,
            },
            GrammarRule {
                name: r#"set_expr"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::RuleReference { name: r#"set_body"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 564,
            },
            GrammarRule {
                name: r#"set_body"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        GrammarElement::RuleReference { name: r#"comp_clause"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                                    ] }) },
                                GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                            ] }) },
                    ] },
                ] },
                line_number: 566,
            },
            GrammarRule {
                name: r#"paren_expr"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"paren_body"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 573,
            },
            GrammarRule {
                name: r#"paren_body"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        GrammarElement::RuleReference { name: r#"comp_clause"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                                    ] }) },
                                GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                            ] }) },
                    ] },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 575,
            },
            GrammarRule {
                name: r#"comp_clause"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"comp_for"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::RuleReference { name: r#"comp_for"#.to_string() },
                            GrammarElement::RuleReference { name: r#"comp_if"#.to_string() },
                        ] }) },
                ] },
                line_number: 581,
            },
            GrammarRule {
                name: r#"comp_for"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"async"#.to_string() }) },
                    GrammarElement::Literal { value: r#"for"#.to_string() },
                    GrammarElement::RuleReference { name: r#"target_list"#.to_string() },
                    GrammarElement::Literal { value: r#"in"#.to_string() },
                    GrammarElement::RuleReference { name: r#"or_expr"#.to_string() },
                ] },
                line_number: 583,
            },
            GrammarRule {
                name: r#"comp_if"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"if"#.to_string() },
                    GrammarElement::RuleReference { name: r#"or_expr"#.to_string() },
                ] },
                line_number: 585,
            },
            GrammarRule {
                name: r#"yield_expr"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"yield"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::Sequence { elements: vec![
                                GrammarElement::Literal { value: r#"from"#.to_string() },
                                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            ] },
                            GrammarElement::RuleReference { name: r#"expression_list"#.to_string() },
                        ] }) },
                ] },
                line_number: 591,
            },
            GrammarRule {
                name: r#"arguments"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"argument"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"argument"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                ] },
                line_number: 600,
            },
            GrammarRule {
                name: r#"argument"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"DOUBLE_STAR"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    ] },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 602,
            },
        ],
            version: 1,
        }
    }
}

