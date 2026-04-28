// AUTO-GENERATED FILE - DO NOT EDIT
// Source family: haskell
// Regenerate with: grammar-tools generate-rust-compiled-grammars haskell
//
// This file embeds versioned ParserGrammar values as native Rust data structures.
// Call `parser_grammar` instead of reading and parsing grammar files at runtime.

use grammar_tools::parser_grammar::ParserGrammar;

pub const SUPPORTED_VERSIONS: &[&str] = &[
    "1.0",
    "1.1",
    "1.2",
    "1.3",
    "1.4",
    "98",
    "2010",
];

pub fn parser_grammar(version: &str) -> Option<ParserGrammar> {
    match version {
        "1.0" => Some(v_1_0::parser_grammar()),
        "1.1" => Some(v_1_1::parser_grammar()),
        "1.2" => Some(v_1_2::parser_grammar()),
        "1.3" => Some(v_1_3::parser_grammar()),
        "1.4" => Some(v_1_4::parser_grammar()),
        "98" => Some(v_98::parser_grammar()),
        "2010" => Some(v_2010::parser_grammar()),
        _ => None,
    }
}

mod v_1_0 {
    // AUTO-GENERATED FILE — DO NOT EDIT
    // Source: haskell1.0.grammar
    // Regenerate with: grammar-tools compile-grammar haskell1.0.grammar
    //
    // This file embeds a ParserGrammar as native Rust data structures.
    // Call `parser_grammar()` instead of reading and parsing the .grammar file.

    use grammar_tools::parser_grammar::{GrammarElement, GrammarRule, ParserGrammar};

    pub fn parser_grammar() -> ParserGrammar {
        ParserGrammar {
            rules: vec![
            GrammarRule {
                name: r#"file"#.to_string(),
                body: GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"declaration"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"layout_sep"#.to_string() }) },
                    ] }) },
                line_number: 10,
            },
            GrammarRule {
                name: r#"declaration"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"module_decl"#.to_string() },
                    GrammarElement::RuleReference { name: r#"let_decl"#.to_string() },
                    GrammarElement::RuleReference { name: r#"do_decl"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expr_decl"#.to_string() },
                ] },
                line_number: 11,
            },
            GrammarRule {
                name: r#"layout_open"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"VIRTUAL_LBRACE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Literal { value: r#"{"#.to_string() },
                ] },
                line_number: 18,
            },
            GrammarRule {
                name: r#"layout_close"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"VIRTUAL_RBRACE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                    GrammarElement::Literal { value: r#"}"#.to_string() },
                ] },
                line_number: 19,
            },
            GrammarRule {
                name: r#"layout_sep"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"VIRTUAL_SEMICOLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                ] },
                line_number: 20,
            },
            GrammarRule {
                name: r#"module_decl"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"module"#.to_string() },
                    GrammarElement::RuleReference { name: r#"module_name"#.to_string() },
                    GrammarElement::Literal { value: r#"where"#.to_string() },
                    GrammarElement::RuleReference { name: r#"layout_open"#.to_string() },
                    GrammarElement::RuleReference { name: r#"module_body"#.to_string() },
                    GrammarElement::RuleReference { name: r#"layout_close"#.to_string() },
                ] },
                line_number: 22,
            },
            GrammarRule {
                name: r#"module_name"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 23,
            },
            GrammarRule {
                name: r#"module_body"#.to_string(),
                body: GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"declaration"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"layout_sep"#.to_string() }) },
                    ] }) },
                line_number: 24,
            },
            GrammarRule {
                name: r#"let_decl"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"let"#.to_string() },
                    GrammarElement::RuleReference { name: r#"layout_open"#.to_string() },
                    GrammarElement::RuleReference { name: r#"let_bindings"#.to_string() },
                    GrammarElement::RuleReference { name: r#"layout_close"#.to_string() },
                    GrammarElement::Literal { value: r#"in"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expr_decl"#.to_string() },
                ] },
                line_number: 26,
            },
            GrammarRule {
                name: r#"let_bindings"#.to_string(),
                body: GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"binding"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"layout_sep"#.to_string() }) },
                    ] }) },
                line_number: 27,
            },
            GrammarRule {
                name: r#"binding"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expr_decl"#.to_string() },
                ] },
                line_number: 28,
            },
            GrammarRule {
                name: r#"do_decl"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"do"#.to_string() },
                    GrammarElement::RuleReference { name: r#"layout_open"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"expr_decl"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"layout_sep"#.to_string() }) },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"layout_close"#.to_string() },
                ] },
                line_number: 30,
            },
            GrammarRule {
                name: r#"expr_decl"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"lambda_expr"#.to_string() },
                    GrammarElement::RuleReference { name: r#"app_expr"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"INTEGER"#.to_string() },
                    GrammarElement::TokenReference { name: r#"FLOAT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CHARACTER"#.to_string() },
                ] },
                line_number: 32,
            },
            GrammarRule {
                name: r#"lambda_expr"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LAMBDA"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NAME"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"RARROW"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expr_decl"#.to_string() },
                ] },
                line_number: 34,
            },
            GrammarRule {
                name: r#"app_expr"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"atom_expr"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"atom_expr"#.to_string() }) },
                ] },
                line_number: 35,
            },
            GrammarRule {
                name: r#"atom_expr"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"INTEGER"#.to_string() },
                    GrammarElement::TokenReference { name: r#"FLOAT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CHARACTER"#.to_string() },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expr_decl"#.to_string() },
                        GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expr_list"#.to_string() },
                        GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"expr_list"#.to_string() }) },
                        GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                    ] },
                ] },
                line_number: 36,
            },
            GrammarRule {
                name: r#"expr_list"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"expr_decl"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expr_decl"#.to_string() },
                        ] }) },
                ] },
                line_number: 45,
            },
        ],
            version: 1,
        }
    }
}

mod v_1_1 {
    // AUTO-GENERATED FILE — DO NOT EDIT
    // Source: haskell1.1.grammar
    // Regenerate with: grammar-tools compile-grammar haskell1.1.grammar
    //
    // This file embeds a ParserGrammar as native Rust data structures.
    // Call `parser_grammar()` instead of reading and parsing the .grammar file.

    use grammar_tools::parser_grammar::{GrammarElement, GrammarRule, ParserGrammar};

    pub fn parser_grammar() -> ParserGrammar {
        ParserGrammar {
            rules: vec![
            GrammarRule {
                name: r#"file"#.to_string(),
                body: GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"declaration"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"layout_sep"#.to_string() }) },
                    ] }) },
                line_number: 10,
            },
            GrammarRule {
                name: r#"declaration"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"module_decl"#.to_string() },
                    GrammarElement::RuleReference { name: r#"let_decl"#.to_string() },
                    GrammarElement::RuleReference { name: r#"do_decl"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expr_decl"#.to_string() },
                ] },
                line_number: 11,
            },
            GrammarRule {
                name: r#"layout_open"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"VIRTUAL_LBRACE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Literal { value: r#"{"#.to_string() },
                ] },
                line_number: 18,
            },
            GrammarRule {
                name: r#"layout_close"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"VIRTUAL_RBRACE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                    GrammarElement::Literal { value: r#"}"#.to_string() },
                ] },
                line_number: 19,
            },
            GrammarRule {
                name: r#"layout_sep"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"VIRTUAL_SEMICOLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                ] },
                line_number: 20,
            },
            GrammarRule {
                name: r#"module_decl"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"module"#.to_string() },
                    GrammarElement::RuleReference { name: r#"module_name"#.to_string() },
                    GrammarElement::Literal { value: r#"where"#.to_string() },
                    GrammarElement::RuleReference { name: r#"layout_open"#.to_string() },
                    GrammarElement::RuleReference { name: r#"module_body"#.to_string() },
                    GrammarElement::RuleReference { name: r#"layout_close"#.to_string() },
                ] },
                line_number: 22,
            },
            GrammarRule {
                name: r#"module_name"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 23,
            },
            GrammarRule {
                name: r#"module_body"#.to_string(),
                body: GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"declaration"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"layout_sep"#.to_string() }) },
                    ] }) },
                line_number: 24,
            },
            GrammarRule {
                name: r#"let_decl"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"let"#.to_string() },
                    GrammarElement::RuleReference { name: r#"layout_open"#.to_string() },
                    GrammarElement::RuleReference { name: r#"let_bindings"#.to_string() },
                    GrammarElement::RuleReference { name: r#"layout_close"#.to_string() },
                    GrammarElement::Literal { value: r#"in"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expr_decl"#.to_string() },
                ] },
                line_number: 26,
            },
            GrammarRule {
                name: r#"let_bindings"#.to_string(),
                body: GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"binding"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"layout_sep"#.to_string() }) },
                    ] }) },
                line_number: 27,
            },
            GrammarRule {
                name: r#"binding"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expr_decl"#.to_string() },
                ] },
                line_number: 28,
            },
            GrammarRule {
                name: r#"do_decl"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"do"#.to_string() },
                    GrammarElement::RuleReference { name: r#"layout_open"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"expr_decl"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"layout_sep"#.to_string() }) },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"layout_close"#.to_string() },
                ] },
                line_number: 30,
            },
            GrammarRule {
                name: r#"expr_decl"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"lambda_expr"#.to_string() },
                    GrammarElement::RuleReference { name: r#"app_expr"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"INTEGER"#.to_string() },
                    GrammarElement::TokenReference { name: r#"FLOAT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CHARACTER"#.to_string() },
                ] },
                line_number: 32,
            },
            GrammarRule {
                name: r#"lambda_expr"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LAMBDA"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NAME"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"RARROW"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expr_decl"#.to_string() },
                ] },
                line_number: 34,
            },
            GrammarRule {
                name: r#"app_expr"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"atom_expr"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"atom_expr"#.to_string() }) },
                ] },
                line_number: 35,
            },
            GrammarRule {
                name: r#"atom_expr"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"INTEGER"#.to_string() },
                    GrammarElement::TokenReference { name: r#"FLOAT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CHARACTER"#.to_string() },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expr_decl"#.to_string() },
                        GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expr_list"#.to_string() },
                        GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"expr_list"#.to_string() }) },
                        GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                    ] },
                ] },
                line_number: 36,
            },
            GrammarRule {
                name: r#"expr_list"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"expr_decl"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expr_decl"#.to_string() },
                        ] }) },
                ] },
                line_number: 45,
            },
        ],
            version: 1,
        }
    }
}

mod v_1_2 {
    // AUTO-GENERATED FILE — DO NOT EDIT
    // Source: haskell1.2.grammar
    // Regenerate with: grammar-tools compile-grammar haskell1.2.grammar
    //
    // This file embeds a ParserGrammar as native Rust data structures.
    // Call `parser_grammar()` instead of reading and parsing the .grammar file.

    use grammar_tools::parser_grammar::{GrammarElement, GrammarRule, ParserGrammar};

    pub fn parser_grammar() -> ParserGrammar {
        ParserGrammar {
            rules: vec![
            GrammarRule {
                name: r#"file"#.to_string(),
                body: GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"declaration"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"layout_sep"#.to_string() }) },
                    ] }) },
                line_number: 10,
            },
            GrammarRule {
                name: r#"declaration"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"module_decl"#.to_string() },
                    GrammarElement::RuleReference { name: r#"let_decl"#.to_string() },
                    GrammarElement::RuleReference { name: r#"do_decl"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expr_decl"#.to_string() },
                ] },
                line_number: 11,
            },
            GrammarRule {
                name: r#"layout_open"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"VIRTUAL_LBRACE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Literal { value: r#"{"#.to_string() },
                ] },
                line_number: 18,
            },
            GrammarRule {
                name: r#"layout_close"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"VIRTUAL_RBRACE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                    GrammarElement::Literal { value: r#"}"#.to_string() },
                ] },
                line_number: 19,
            },
            GrammarRule {
                name: r#"layout_sep"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"VIRTUAL_SEMICOLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                ] },
                line_number: 20,
            },
            GrammarRule {
                name: r#"module_decl"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"module"#.to_string() },
                    GrammarElement::RuleReference { name: r#"module_name"#.to_string() },
                    GrammarElement::Literal { value: r#"where"#.to_string() },
                    GrammarElement::RuleReference { name: r#"layout_open"#.to_string() },
                    GrammarElement::RuleReference { name: r#"module_body"#.to_string() },
                    GrammarElement::RuleReference { name: r#"layout_close"#.to_string() },
                ] },
                line_number: 22,
            },
            GrammarRule {
                name: r#"module_name"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 23,
            },
            GrammarRule {
                name: r#"module_body"#.to_string(),
                body: GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"declaration"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"layout_sep"#.to_string() }) },
                    ] }) },
                line_number: 24,
            },
            GrammarRule {
                name: r#"let_decl"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"let"#.to_string() },
                    GrammarElement::RuleReference { name: r#"layout_open"#.to_string() },
                    GrammarElement::RuleReference { name: r#"let_bindings"#.to_string() },
                    GrammarElement::RuleReference { name: r#"layout_close"#.to_string() },
                    GrammarElement::Literal { value: r#"in"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expr_decl"#.to_string() },
                ] },
                line_number: 26,
            },
            GrammarRule {
                name: r#"let_bindings"#.to_string(),
                body: GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"binding"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"layout_sep"#.to_string() }) },
                    ] }) },
                line_number: 27,
            },
            GrammarRule {
                name: r#"binding"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expr_decl"#.to_string() },
                ] },
                line_number: 28,
            },
            GrammarRule {
                name: r#"do_decl"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"do"#.to_string() },
                    GrammarElement::RuleReference { name: r#"layout_open"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"expr_decl"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"layout_sep"#.to_string() }) },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"layout_close"#.to_string() },
                ] },
                line_number: 30,
            },
            GrammarRule {
                name: r#"expr_decl"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"lambda_expr"#.to_string() },
                    GrammarElement::RuleReference { name: r#"app_expr"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"INTEGER"#.to_string() },
                    GrammarElement::TokenReference { name: r#"FLOAT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CHARACTER"#.to_string() },
                ] },
                line_number: 32,
            },
            GrammarRule {
                name: r#"lambda_expr"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LAMBDA"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NAME"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"RARROW"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expr_decl"#.to_string() },
                ] },
                line_number: 34,
            },
            GrammarRule {
                name: r#"app_expr"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"atom_expr"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"atom_expr"#.to_string() }) },
                ] },
                line_number: 35,
            },
            GrammarRule {
                name: r#"atom_expr"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"INTEGER"#.to_string() },
                    GrammarElement::TokenReference { name: r#"FLOAT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CHARACTER"#.to_string() },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expr_decl"#.to_string() },
                        GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expr_list"#.to_string() },
                        GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"expr_list"#.to_string() }) },
                        GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                    ] },
                ] },
                line_number: 36,
            },
            GrammarRule {
                name: r#"expr_list"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"expr_decl"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expr_decl"#.to_string() },
                        ] }) },
                ] },
                line_number: 45,
            },
        ],
            version: 1,
        }
    }
}

mod v_1_3 {
    // AUTO-GENERATED FILE — DO NOT EDIT
    // Source: haskell1.3.grammar
    // Regenerate with: grammar-tools compile-grammar haskell1.3.grammar
    //
    // This file embeds a ParserGrammar as native Rust data structures.
    // Call `parser_grammar()` instead of reading and parsing the .grammar file.

    use grammar_tools::parser_grammar::{GrammarElement, GrammarRule, ParserGrammar};

    pub fn parser_grammar() -> ParserGrammar {
        ParserGrammar {
            rules: vec![
            GrammarRule {
                name: r#"file"#.to_string(),
                body: GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"declaration"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"layout_sep"#.to_string() }) },
                    ] }) },
                line_number: 10,
            },
            GrammarRule {
                name: r#"declaration"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"module_decl"#.to_string() },
                    GrammarElement::RuleReference { name: r#"let_decl"#.to_string() },
                    GrammarElement::RuleReference { name: r#"do_decl"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expr_decl"#.to_string() },
                ] },
                line_number: 11,
            },
            GrammarRule {
                name: r#"layout_open"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"VIRTUAL_LBRACE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Literal { value: r#"{"#.to_string() },
                ] },
                line_number: 18,
            },
            GrammarRule {
                name: r#"layout_close"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"VIRTUAL_RBRACE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                    GrammarElement::Literal { value: r#"}"#.to_string() },
                ] },
                line_number: 19,
            },
            GrammarRule {
                name: r#"layout_sep"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"VIRTUAL_SEMICOLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                ] },
                line_number: 20,
            },
            GrammarRule {
                name: r#"module_decl"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"module"#.to_string() },
                    GrammarElement::RuleReference { name: r#"module_name"#.to_string() },
                    GrammarElement::Literal { value: r#"where"#.to_string() },
                    GrammarElement::RuleReference { name: r#"layout_open"#.to_string() },
                    GrammarElement::RuleReference { name: r#"module_body"#.to_string() },
                    GrammarElement::RuleReference { name: r#"layout_close"#.to_string() },
                ] },
                line_number: 22,
            },
            GrammarRule {
                name: r#"module_name"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 23,
            },
            GrammarRule {
                name: r#"module_body"#.to_string(),
                body: GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"declaration"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"layout_sep"#.to_string() }) },
                    ] }) },
                line_number: 24,
            },
            GrammarRule {
                name: r#"let_decl"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"let"#.to_string() },
                    GrammarElement::RuleReference { name: r#"layout_open"#.to_string() },
                    GrammarElement::RuleReference { name: r#"let_bindings"#.to_string() },
                    GrammarElement::RuleReference { name: r#"layout_close"#.to_string() },
                    GrammarElement::Literal { value: r#"in"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expr_decl"#.to_string() },
                ] },
                line_number: 26,
            },
            GrammarRule {
                name: r#"let_bindings"#.to_string(),
                body: GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"binding"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"layout_sep"#.to_string() }) },
                    ] }) },
                line_number: 27,
            },
            GrammarRule {
                name: r#"binding"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expr_decl"#.to_string() },
                ] },
                line_number: 28,
            },
            GrammarRule {
                name: r#"do_decl"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"do"#.to_string() },
                    GrammarElement::RuleReference { name: r#"layout_open"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"expr_decl"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"layout_sep"#.to_string() }) },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"layout_close"#.to_string() },
                ] },
                line_number: 30,
            },
            GrammarRule {
                name: r#"expr_decl"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"lambda_expr"#.to_string() },
                    GrammarElement::RuleReference { name: r#"app_expr"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"INTEGER"#.to_string() },
                    GrammarElement::TokenReference { name: r#"FLOAT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CHARACTER"#.to_string() },
                ] },
                line_number: 32,
            },
            GrammarRule {
                name: r#"lambda_expr"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LAMBDA"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NAME"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"RARROW"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expr_decl"#.to_string() },
                ] },
                line_number: 34,
            },
            GrammarRule {
                name: r#"app_expr"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"atom_expr"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"atom_expr"#.to_string() }) },
                ] },
                line_number: 35,
            },
            GrammarRule {
                name: r#"atom_expr"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"INTEGER"#.to_string() },
                    GrammarElement::TokenReference { name: r#"FLOAT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CHARACTER"#.to_string() },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expr_decl"#.to_string() },
                        GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expr_list"#.to_string() },
                        GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"expr_list"#.to_string() }) },
                        GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                    ] },
                ] },
                line_number: 36,
            },
            GrammarRule {
                name: r#"expr_list"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"expr_decl"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expr_decl"#.to_string() },
                        ] }) },
                ] },
                line_number: 45,
            },
        ],
            version: 1,
        }
    }
}

mod v_1_4 {
    // AUTO-GENERATED FILE — DO NOT EDIT
    // Source: haskell1.4.grammar
    // Regenerate with: grammar-tools compile-grammar haskell1.4.grammar
    //
    // This file embeds a ParserGrammar as native Rust data structures.
    // Call `parser_grammar()` instead of reading and parsing the .grammar file.

    use grammar_tools::parser_grammar::{GrammarElement, GrammarRule, ParserGrammar};

    pub fn parser_grammar() -> ParserGrammar {
        ParserGrammar {
            rules: vec![
            GrammarRule {
                name: r#"file"#.to_string(),
                body: GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"declaration"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"layout_sep"#.to_string() }) },
                    ] }) },
                line_number: 10,
            },
            GrammarRule {
                name: r#"declaration"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"module_decl"#.to_string() },
                    GrammarElement::RuleReference { name: r#"let_decl"#.to_string() },
                    GrammarElement::RuleReference { name: r#"do_decl"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expr_decl"#.to_string() },
                ] },
                line_number: 11,
            },
            GrammarRule {
                name: r#"layout_open"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"VIRTUAL_LBRACE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Literal { value: r#"{"#.to_string() },
                ] },
                line_number: 18,
            },
            GrammarRule {
                name: r#"layout_close"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"VIRTUAL_RBRACE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                    GrammarElement::Literal { value: r#"}"#.to_string() },
                ] },
                line_number: 19,
            },
            GrammarRule {
                name: r#"layout_sep"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"VIRTUAL_SEMICOLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                ] },
                line_number: 20,
            },
            GrammarRule {
                name: r#"module_decl"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"module"#.to_string() },
                    GrammarElement::RuleReference { name: r#"module_name"#.to_string() },
                    GrammarElement::Literal { value: r#"where"#.to_string() },
                    GrammarElement::RuleReference { name: r#"layout_open"#.to_string() },
                    GrammarElement::RuleReference { name: r#"module_body"#.to_string() },
                    GrammarElement::RuleReference { name: r#"layout_close"#.to_string() },
                ] },
                line_number: 22,
            },
            GrammarRule {
                name: r#"module_name"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 23,
            },
            GrammarRule {
                name: r#"module_body"#.to_string(),
                body: GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"declaration"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"layout_sep"#.to_string() }) },
                    ] }) },
                line_number: 24,
            },
            GrammarRule {
                name: r#"let_decl"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"let"#.to_string() },
                    GrammarElement::RuleReference { name: r#"layout_open"#.to_string() },
                    GrammarElement::RuleReference { name: r#"let_bindings"#.to_string() },
                    GrammarElement::RuleReference { name: r#"layout_close"#.to_string() },
                    GrammarElement::Literal { value: r#"in"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expr_decl"#.to_string() },
                ] },
                line_number: 26,
            },
            GrammarRule {
                name: r#"let_bindings"#.to_string(),
                body: GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"binding"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"layout_sep"#.to_string() }) },
                    ] }) },
                line_number: 27,
            },
            GrammarRule {
                name: r#"binding"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expr_decl"#.to_string() },
                ] },
                line_number: 28,
            },
            GrammarRule {
                name: r#"do_decl"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"do"#.to_string() },
                    GrammarElement::RuleReference { name: r#"layout_open"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"expr_decl"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"layout_sep"#.to_string() }) },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"layout_close"#.to_string() },
                ] },
                line_number: 30,
            },
            GrammarRule {
                name: r#"expr_decl"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"lambda_expr"#.to_string() },
                    GrammarElement::RuleReference { name: r#"app_expr"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"INTEGER"#.to_string() },
                    GrammarElement::TokenReference { name: r#"FLOAT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CHARACTER"#.to_string() },
                ] },
                line_number: 32,
            },
            GrammarRule {
                name: r#"lambda_expr"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LAMBDA"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NAME"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"RARROW"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expr_decl"#.to_string() },
                ] },
                line_number: 34,
            },
            GrammarRule {
                name: r#"app_expr"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"atom_expr"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"atom_expr"#.to_string() }) },
                ] },
                line_number: 35,
            },
            GrammarRule {
                name: r#"atom_expr"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"INTEGER"#.to_string() },
                    GrammarElement::TokenReference { name: r#"FLOAT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CHARACTER"#.to_string() },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expr_decl"#.to_string() },
                        GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expr_list"#.to_string() },
                        GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"expr_list"#.to_string() }) },
                        GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                    ] },
                ] },
                line_number: 36,
            },
            GrammarRule {
                name: r#"expr_list"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"expr_decl"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expr_decl"#.to_string() },
                        ] }) },
                ] },
                line_number: 45,
            },
        ],
            version: 1,
        }
    }
}

mod v_98 {
    // AUTO-GENERATED FILE — DO NOT EDIT
    // Source: haskell98.grammar
    // Regenerate with: grammar-tools compile-grammar haskell98.grammar
    //
    // This file embeds a ParserGrammar as native Rust data structures.
    // Call `parser_grammar()` instead of reading and parsing the .grammar file.

    use grammar_tools::parser_grammar::{GrammarElement, GrammarRule, ParserGrammar};

    pub fn parser_grammar() -> ParserGrammar {
        ParserGrammar {
            rules: vec![
            GrammarRule {
                name: r#"file"#.to_string(),
                body: GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"declaration"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"layout_sep"#.to_string() }) },
                    ] }) },
                line_number: 10,
            },
            GrammarRule {
                name: r#"declaration"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"module_decl"#.to_string() },
                    GrammarElement::RuleReference { name: r#"let_decl"#.to_string() },
                    GrammarElement::RuleReference { name: r#"do_decl"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expr_decl"#.to_string() },
                ] },
                line_number: 11,
            },
            GrammarRule {
                name: r#"layout_open"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"VIRTUAL_LBRACE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Literal { value: r#"{"#.to_string() },
                ] },
                line_number: 18,
            },
            GrammarRule {
                name: r#"layout_close"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"VIRTUAL_RBRACE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                    GrammarElement::Literal { value: r#"}"#.to_string() },
                ] },
                line_number: 19,
            },
            GrammarRule {
                name: r#"layout_sep"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"VIRTUAL_SEMICOLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                ] },
                line_number: 20,
            },
            GrammarRule {
                name: r#"module_decl"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"module"#.to_string() },
                    GrammarElement::RuleReference { name: r#"module_name"#.to_string() },
                    GrammarElement::Literal { value: r#"where"#.to_string() },
                    GrammarElement::RuleReference { name: r#"layout_open"#.to_string() },
                    GrammarElement::RuleReference { name: r#"module_body"#.to_string() },
                    GrammarElement::RuleReference { name: r#"layout_close"#.to_string() },
                ] },
                line_number: 22,
            },
            GrammarRule {
                name: r#"module_name"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 23,
            },
            GrammarRule {
                name: r#"module_body"#.to_string(),
                body: GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"declaration"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"layout_sep"#.to_string() }) },
                    ] }) },
                line_number: 24,
            },
            GrammarRule {
                name: r#"let_decl"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"let"#.to_string() },
                    GrammarElement::RuleReference { name: r#"layout_open"#.to_string() },
                    GrammarElement::RuleReference { name: r#"let_bindings"#.to_string() },
                    GrammarElement::RuleReference { name: r#"layout_close"#.to_string() },
                    GrammarElement::Literal { value: r#"in"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expr_decl"#.to_string() },
                ] },
                line_number: 26,
            },
            GrammarRule {
                name: r#"let_bindings"#.to_string(),
                body: GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"binding"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"layout_sep"#.to_string() }) },
                    ] }) },
                line_number: 27,
            },
            GrammarRule {
                name: r#"binding"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expr_decl"#.to_string() },
                ] },
                line_number: 28,
            },
            GrammarRule {
                name: r#"do_decl"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"do"#.to_string() },
                    GrammarElement::RuleReference { name: r#"layout_open"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"expr_decl"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"layout_sep"#.to_string() }) },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"layout_close"#.to_string() },
                ] },
                line_number: 30,
            },
            GrammarRule {
                name: r#"expr_decl"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"lambda_expr"#.to_string() },
                    GrammarElement::RuleReference { name: r#"app_expr"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"INTEGER"#.to_string() },
                    GrammarElement::TokenReference { name: r#"FLOAT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CHARACTER"#.to_string() },
                ] },
                line_number: 32,
            },
            GrammarRule {
                name: r#"lambda_expr"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LAMBDA"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NAME"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"RARROW"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expr_decl"#.to_string() },
                ] },
                line_number: 34,
            },
            GrammarRule {
                name: r#"app_expr"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"atom_expr"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"atom_expr"#.to_string() }) },
                ] },
                line_number: 35,
            },
            GrammarRule {
                name: r#"atom_expr"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"INTEGER"#.to_string() },
                    GrammarElement::TokenReference { name: r#"FLOAT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CHARACTER"#.to_string() },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expr_decl"#.to_string() },
                        GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expr_list"#.to_string() },
                        GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"expr_list"#.to_string() }) },
                        GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                    ] },
                ] },
                line_number: 36,
            },
            GrammarRule {
                name: r#"expr_list"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"expr_decl"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expr_decl"#.to_string() },
                        ] }) },
                ] },
                line_number: 45,
            },
        ],
            version: 1,
        }
    }
}

mod v_2010 {
    // AUTO-GENERATED FILE — DO NOT EDIT
    // Source: haskell2010.grammar
    // Regenerate with: grammar-tools compile-grammar haskell2010.grammar
    //
    // This file embeds a ParserGrammar as native Rust data structures.
    // Call `parser_grammar()` instead of reading and parsing the .grammar file.

    use grammar_tools::parser_grammar::{GrammarElement, GrammarRule, ParserGrammar};

    pub fn parser_grammar() -> ParserGrammar {
        ParserGrammar {
            rules: vec![
            GrammarRule {
                name: r#"file"#.to_string(),
                body: GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"declaration"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"layout_sep"#.to_string() }) },
                    ] }) },
                line_number: 10,
            },
            GrammarRule {
                name: r#"declaration"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"module_decl"#.to_string() },
                    GrammarElement::RuleReference { name: r#"let_decl"#.to_string() },
                    GrammarElement::RuleReference { name: r#"do_decl"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expr_decl"#.to_string() },
                ] },
                line_number: 11,
            },
            GrammarRule {
                name: r#"layout_open"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"VIRTUAL_LBRACE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Literal { value: r#"{"#.to_string() },
                ] },
                line_number: 18,
            },
            GrammarRule {
                name: r#"layout_close"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"VIRTUAL_RBRACE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                    GrammarElement::Literal { value: r#"}"#.to_string() },
                ] },
                line_number: 19,
            },
            GrammarRule {
                name: r#"layout_sep"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"VIRTUAL_SEMICOLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                ] },
                line_number: 20,
            },
            GrammarRule {
                name: r#"module_decl"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"module"#.to_string() },
                    GrammarElement::RuleReference { name: r#"module_name"#.to_string() },
                    GrammarElement::Literal { value: r#"where"#.to_string() },
                    GrammarElement::RuleReference { name: r#"layout_open"#.to_string() },
                    GrammarElement::RuleReference { name: r#"module_body"#.to_string() },
                    GrammarElement::RuleReference { name: r#"layout_close"#.to_string() },
                ] },
                line_number: 22,
            },
            GrammarRule {
                name: r#"module_name"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 23,
            },
            GrammarRule {
                name: r#"module_body"#.to_string(),
                body: GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"declaration"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"layout_sep"#.to_string() }) },
                    ] }) },
                line_number: 24,
            },
            GrammarRule {
                name: r#"let_decl"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"let"#.to_string() },
                    GrammarElement::RuleReference { name: r#"layout_open"#.to_string() },
                    GrammarElement::RuleReference { name: r#"let_bindings"#.to_string() },
                    GrammarElement::RuleReference { name: r#"layout_close"#.to_string() },
                    GrammarElement::Literal { value: r#"in"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expr_decl"#.to_string() },
                ] },
                line_number: 26,
            },
            GrammarRule {
                name: r#"let_bindings"#.to_string(),
                body: GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"binding"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"layout_sep"#.to_string() }) },
                    ] }) },
                line_number: 27,
            },
            GrammarRule {
                name: r#"binding"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expr_decl"#.to_string() },
                ] },
                line_number: 28,
            },
            GrammarRule {
                name: r#"do_decl"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"do"#.to_string() },
                    GrammarElement::RuleReference { name: r#"layout_open"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"expr_decl"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"layout_sep"#.to_string() }) },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"layout_close"#.to_string() },
                ] },
                line_number: 30,
            },
            GrammarRule {
                name: r#"expr_decl"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"lambda_expr"#.to_string() },
                    GrammarElement::RuleReference { name: r#"app_expr"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"INTEGER"#.to_string() },
                    GrammarElement::TokenReference { name: r#"FLOAT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CHARACTER"#.to_string() },
                ] },
                line_number: 32,
            },
            GrammarRule {
                name: r#"lambda_expr"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LAMBDA"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NAME"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"RARROW"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expr_decl"#.to_string() },
                ] },
                line_number: 34,
            },
            GrammarRule {
                name: r#"app_expr"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"atom_expr"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"atom_expr"#.to_string() }) },
                ] },
                line_number: 35,
            },
            GrammarRule {
                name: r#"atom_expr"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"INTEGER"#.to_string() },
                    GrammarElement::TokenReference { name: r#"FLOAT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CHARACTER"#.to_string() },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expr_decl"#.to_string() },
                        GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expr_list"#.to_string() },
                        GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                    ] },
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"expr_list"#.to_string() }) },
                        GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                    ] },
                ] },
                line_number: 36,
            },
            GrammarRule {
                name: r#"expr_list"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"expr_decl"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expr_decl"#.to_string() },
                        ] }) },
                ] },
                line_number: 45,
            },
        ],
            version: 1,
        }
    }
}

