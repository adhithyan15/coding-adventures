// AUTO-GENERATED FILE — DO NOT EDIT
// Source: starlark.grammar
// Regenerate with: grammar-tools compile-grammar starlark.grammar
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
            line_number: 48,
        },
        GrammarRule {
            name: r#"statement"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"compound_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"simple_stmt"#.to_string() },
            ] },
            line_number: 62,
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
            line_number: 66,
        },
        GrammarRule {
            name: r#"small_stmt"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"return_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"break_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"continue_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"pass_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"load_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"assign_stmt"#.to_string() },
            ] },
            line_number: 68,
        },
        GrammarRule {
            name: r#"return_stmt"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"return"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"expression"#.to_string() }) },
            ] },
            line_number: 82,
        },
        GrammarRule {
            name: r#"break_stmt"#.to_string(),
            body: GrammarElement::Literal { value: r#"break"#.to_string() },
            line_number: 85,
        },
        GrammarRule {
            name: r#"continue_stmt"#.to_string(),
            body: GrammarElement::Literal { value: r#"continue"#.to_string() },
            line_number: 88,
        },
        GrammarRule {
            name: r#"pass_stmt"#.to_string(),
            body: GrammarElement::Literal { value: r#"pass"#.to_string() },
            line_number: 93,
        },
        GrammarRule {
            name: r#"load_stmt"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"load"#.to_string() },
                GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::RuleReference { name: r#"load_arg"#.to_string() },
                    ] }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMA"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
            ] },
            line_number: 102,
        },
        GrammarRule {
            name: r#"load_arg"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                ] },
                GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
            ] },
            line_number: 103,
        },
        GrammarRule {
            name: r#"assign_stmt"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"expression_list"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::RuleReference { name: r#"assign_op"#.to_string() },
                                GrammarElement::RuleReference { name: r#"augmented_assign_op"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"expression_list"#.to_string() },
                    ] }) },
            ] },
            line_number: 124,
        },
        GrammarRule {
            name: r#"assign_op"#.to_string(),
            body: GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
            line_number: 127,
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
            line_number: 129,
        },
        GrammarRule {
            name: r#"compound_stmt"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"if_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"for_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"def_stmt"#.to_string() },
            ] },
            line_number: 138,
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
            line_number: 150,
        },
        GrammarRule {
            name: r#"for_stmt"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"for"#.to_string() },
                GrammarElement::RuleReference { name: r#"loop_vars"#.to_string() },
                GrammarElement::Literal { value: r#"in"#.to_string() },
                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                GrammarElement::RuleReference { name: r#"suite"#.to_string() },
            ] },
            line_number: 164,
        },
        GrammarRule {
            name: r#"loop_vars"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    ] }) },
            ] },
            line_number: 170,
        },
        GrammarRule {
            name: r#"def_stmt"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"def"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"parameters"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                GrammarElement::RuleReference { name: r#"suite"#.to_string() },
            ] },
            line_number: 180,
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
            line_number: 191,
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
            line_number: 212,
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
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
            ] },
            line_number: 214,
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
            line_number: 248,
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
            line_number: 253,
        },
        GrammarRule {
            name: r#"lambda_expr"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"lambda"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"lambda_params"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
            ] },
            line_number: 258,
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
            line_number: 259,
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
            line_number: 260,
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
            line_number: 264,
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
            line_number: 268,
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
            line_number: 272,
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
            line_number: 281,
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
            ] },
            line_number: 283,
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
            line_number: 289,
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
            line_number: 290,
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
            line_number: 291,
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
            line_number: 294,
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
            line_number: 298,
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
            line_number: 303,
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
            line_number: 309,
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
            line_number: 317,
        },
        GrammarRule {
            name: r#"primary"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"atom"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"suffix"#.to_string() }) },
            ] },
            line_number: 334,
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
            line_number: 336,
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
            line_number: 348,
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
                GrammarElement::Literal { value: r#"True"#.to_string() },
                GrammarElement::Literal { value: r#"False"#.to_string() },
                GrammarElement::Literal { value: r#"None"#.to_string() },
                GrammarElement::RuleReference { name: r#"list_expr"#.to_string() },
                GrammarElement::RuleReference { name: r#"dict_expr"#.to_string() },
                GrammarElement::RuleReference { name: r#"paren_expr"#.to_string() },
            ] },
            line_number: 357,
        },
        GrammarRule {
            name: r#"list_expr"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"list_body"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
            ] },
            line_number: 373,
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
            line_number: 375,
        },
        GrammarRule {
            name: r#"dict_expr"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"dict_body"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
            ] },
            line_number: 381,
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
            line_number: 383,
        },
        GrammarRule {
            name: r#"dict_entry"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
            ] },
            line_number: 386,
        },
        GrammarRule {
            name: r#"paren_expr"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"paren_body"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
            ] },
            line_number: 393,
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
            line_number: 395,
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
            line_number: 411,
        },
        GrammarRule {
            name: r#"comp_for"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"for"#.to_string() },
                GrammarElement::RuleReference { name: r#"loop_vars"#.to_string() },
                GrammarElement::Literal { value: r#"in"#.to_string() },
                GrammarElement::RuleReference { name: r#"or_expr"#.to_string() },
            ] },
            line_number: 413,
        },
        GrammarRule {
            name: r#"comp_if"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"if"#.to_string() },
                GrammarElement::RuleReference { name: r#"or_expr"#.to_string() },
            ] },
            line_number: 415,
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
            line_number: 434,
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
            line_number: 436,
        },
    ],
        version: 1,
    }
}
