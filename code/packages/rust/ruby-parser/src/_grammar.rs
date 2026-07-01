// AUTO-GENERATED FILE — DO NOT EDIT
// Source: ruby.grammar
// Regenerate with: grammar-tools compile-grammar ruby.grammar
//
// This file embeds a ParserGrammar as native Rust data structures.
// Call `parser_grammar()` instead of reading and parsing the .grammar file.

use grammar_tools::parser_grammar::{GrammarElement, GrammarRule, ParserGrammar};

pub fn parser_grammar() -> ParserGrammar {
    ParserGrammar {
        rules: vec![
        GrammarRule {
            name: r#"program"#.to_string(),
            body: GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"statement"#.to_string() }) },
            line_number: 27,
        },
        GrammarRule {
            name: r#"statement"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"endless_def_statement"#.to_string() },
                GrammarElement::RuleReference { name: r#"def_statement"#.to_string() },
                GrammarElement::RuleReference { name: r#"class_statement"#.to_string() },
                GrammarElement::RuleReference { name: r#"module_statement"#.to_string() },
                GrammarElement::RuleReference { name: r#"if_statement"#.to_string() },
                GrammarElement::RuleReference { name: r#"unless_statement"#.to_string() },
                GrammarElement::RuleReference { name: r#"while_statement"#.to_string() },
                GrammarElement::RuleReference { name: r#"until_statement"#.to_string() },
                GrammarElement::RuleReference { name: r#"case_statement"#.to_string() },
                GrammarElement::RuleReference { name: r#"begin_statement"#.to_string() },
                GrammarElement::RuleReference { name: r#"return_statement"#.to_string() },
                GrammarElement::RuleReference { name: r#"break_statement"#.to_string() },
                GrammarElement::RuleReference { name: r#"next_statement"#.to_string() },
                GrammarElement::RuleReference { name: r#"redo_statement"#.to_string() },
                GrammarElement::RuleReference { name: r#"retry_statement"#.to_string() },
                GrammarElement::RuleReference { name: r#"yield_statement"#.to_string() },
                GrammarElement::RuleReference { name: r#"super_statement"#.to_string() },
                GrammarElement::RuleReference { name: r#"alias_statement"#.to_string() },
                GrammarElement::RuleReference { name: r#"undef_statement"#.to_string() },
                GrammarElement::RuleReference { name: r#"multi_assignment"#.to_string() },
                GrammarElement::RuleReference { name: r#"modifier_statement"#.to_string() },
                GrammarElement::RuleReference { name: r#"rightward_assignment"#.to_string() },
                GrammarElement::RuleReference { name: r#"assignment"#.to_string() },
                GrammarElement::RuleReference { name: r#"defined_expression"#.to_string() },
                GrammarElement::RuleReference { name: r#"method_with_block"#.to_string() },
                GrammarElement::RuleReference { name: r#"method_call"#.to_string() },
                GrammarElement::RuleReference { name: r#"method_call_no_paren"#.to_string() },
                GrammarElement::RuleReference { name: r#"expression_stmt"#.to_string() },
            ] },
            line_number: 28,
        },
        GrammarRule {
            name: r#"multi_assignment"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"mlhs_target"#.to_string() },
                GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                GrammarElement::RuleReference { name: r#"mlhs_target"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::RuleReference { name: r#"mlhs_target"#.to_string() },
                    ] }) },
                GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    ] }) },
            ] },
            line_number: 71,
        },
        GrammarRule {
            name: r#"mlhs_target"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"*"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
            ] },
            line_number: 72,
        },
        GrammarRule {
            name: r#"modifier_statement"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::RuleReference { name: r#"assignment"#.to_string() },
                        GrammarElement::RuleReference { name: r#"method_call_no_paren"#.to_string() },
                        GrammarElement::RuleReference { name: r#"method_call"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expression_stmt"#.to_string() },
                    ] }) },
                GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::Literal { value: r#"if_modifier"#.to_string() },
                        GrammarElement::Literal { value: r#"unless_modifier"#.to_string() },
                        GrammarElement::Literal { value: r#"while_modifier"#.to_string() },
                        GrammarElement::Literal { value: r#"until_modifier"#.to_string() },
                    ] }) },
                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
            ] },
            line_number: 108,
        },
        GrammarRule {
            name: r#"def_statement"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"def"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"params"#.to_string() }) },
                        GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                    ] }) },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::NegativeLookahead { element: Box::new(GrammarElement::Literal { value: r#"rescue"#.to_string() }) },
                        GrammarElement::NegativeLookahead { element: Box::new(GrammarElement::Literal { value: r#"ensure"#.to_string() }) },
                        GrammarElement::NegativeLookahead { element: Box::new(GrammarElement::Literal { value: r#"end"#.to_string() }) },
                        GrammarElement::RuleReference { name: r#"statement"#.to_string() },
                    ] }) },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"rescue_clause"#.to_string() }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"ensure_clause"#.to_string() }) },
                GrammarElement::Literal { value: r#"end"#.to_string() },
            ] },
            line_number: 115,
        },
        GrammarRule {
            name: r#"endless_def_statement"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"def"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"params"#.to_string() }) },
                        GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                    ] }) },
                GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
            ] },
            line_number: 121,
        },
        GrammarRule {
            name: r#"class_statement"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"class"#.to_string() },
                    GrammarElement::Literal { value: r#"<<"#.to_string() },
                    GrammarElement::RuleReference { name: r#"singleton_receiver"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::NegativeLookahead { element: Box::new(GrammarElement::Literal { value: r#"end"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"statement"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"end"#.to_string() },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"class"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"<"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::NegativeLookahead { element: Box::new(GrammarElement::Literal { value: r#"end"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"statement"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"end"#.to_string() },
                ] },
            ] },
            line_number: 142,
        },
        GrammarRule {
            name: r#"singleton_receiver"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Literal { value: r#"self"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
            ] },
            line_number: 144,
        },
        GrammarRule {
            name: r#"module_statement"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"module"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::NegativeLookahead { element: Box::new(GrammarElement::Literal { value: r#"end"#.to_string() }) },
                        GrammarElement::RuleReference { name: r#"statement"#.to_string() },
                    ] }) },
                GrammarElement::Literal { value: r#"end"#.to_string() },
            ] },
            line_number: 145,
        },
        GrammarRule {
            name: r#"method_with_block"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        GrammarElement::TokenReference { name: r#"KEYWORD"#.to_string() },
                    ] }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                                    ] }) },
                            ] }) },
                        GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                    ] }) },
                GrammarElement::RuleReference { name: r#"block"#.to_string() },
            ] },
            line_number: 147,
        },
        GrammarRule {
            name: r#"block"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"do_block"#.to_string() },
                GrammarElement::RuleReference { name: r#"brace_block"#.to_string() },
            ] },
            line_number: 148,
        },
        GrammarRule {
            name: r#"do_block"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"do"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"block_params"#.to_string() }) },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::NegativeLookahead { element: Box::new(GrammarElement::Literal { value: r#"end"#.to_string() }) },
                        GrammarElement::RuleReference { name: r#"statement"#.to_string() },
                    ] }) },
                GrammarElement::Literal { value: r#"end"#.to_string() },
            ] },
            line_number: 149,
        },
        GrammarRule {
            name: r#"brace_block"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"block_params"#.to_string() }) },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"statement"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
            ] },
            line_number: 150,
        },
        GrammarRule {
            name: r#"block_params"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"|"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    ] }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#";"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                            ] }) },
                    ] }) },
                GrammarElement::Literal { value: r#"|"#.to_string() },
            ] },
            line_number: 160,
        },
        GrammarRule {
            name: r#"return_statement"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"return"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"expression"#.to_string() }) },
            ] },
            line_number: 162,
        },
        GrammarRule {
            name: r#"break_statement"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"break"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"expression"#.to_string() }) },
            ] },
            line_number: 163,
        },
        GrammarRule {
            name: r#"next_statement"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"next"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"expression"#.to_string() }) },
            ] },
            line_number: 164,
        },
        GrammarRule {
            name: r#"redo_statement"#.to_string(),
            body: GrammarElement::Literal { value: r#"redo"#.to_string() },
            line_number: 168,
        },
        GrammarRule {
            name: r#"retry_statement"#.to_string(),
            body: GrammarElement::Literal { value: r#"retry"#.to_string() },
            line_number: 172,
        },
        GrammarRule {
            name: r#"alias_statement"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"alias"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
            ] },
            line_number: 183,
        },
        GrammarRule {
            name: r#"undef_statement"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"undef"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
            ] },
            line_number: 195,
        },
        GrammarRule {
            name: r#"yield_statement"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"yield"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"yield_args"#.to_string() }) },
            ] },
            line_number: 217,
        },
        GrammarRule {
            name: r#"yield_args"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"call_arg"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"call_arg"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"call_arg"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"call_arg"#.to_string() },
                        ] }) },
                ] },
            ] },
            line_number: 218,
        },
        GrammarRule {
            name: r#"super_statement"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"super"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"super_args"#.to_string() }) },
            ] },
            line_number: 237,
        },
        GrammarRule {
            name: r#"super_args"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"call_arg"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"call_arg"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"call_arg"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"call_arg"#.to_string() },
                        ] }) },
                ] },
            ] },
            line_number: 238,
        },
        GrammarRule {
            name: r#"params"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Literal { value: r#"..."#.to_string() },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"param"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"param"#.to_string() },
                        ] }) },
                ] },
            ] },
            line_number: 267,
        },
        GrammarRule {
            name: r#"param"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Optional { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::Literal { value: r#"*"#.to_string() },
                        GrammarElement::Literal { value: r#"**"#.to_string() },
                    ] }) },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    ] }) },
            ] },
            line_number: 280,
        },
        GrammarRule {
            name: r#"if_statement"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"if"#.to_string() },
                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::NegativeLookahead { element: Box::new(GrammarElement::Literal { value: r#"else"#.to_string() }) },
                        GrammarElement::NegativeLookahead { element: Box::new(GrammarElement::Literal { value: r#"elsif"#.to_string() }) },
                        GrammarElement::NegativeLookahead { element: Box::new(GrammarElement::Literal { value: r#"end"#.to_string() }) },
                        GrammarElement::RuleReference { name: r#"statement"#.to_string() },
                    ] }) },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"elsif_clause"#.to_string() }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"else_clause"#.to_string() }) },
                GrammarElement::Literal { value: r#"end"#.to_string() },
            ] },
            line_number: 281,
        },
        GrammarRule {
            name: r#"elsif_clause"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"elsif"#.to_string() },
                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::NegativeLookahead { element: Box::new(GrammarElement::Literal { value: r#"else"#.to_string() }) },
                        GrammarElement::NegativeLookahead { element: Box::new(GrammarElement::Literal { value: r#"elsif"#.to_string() }) },
                        GrammarElement::NegativeLookahead { element: Box::new(GrammarElement::Literal { value: r#"end"#.to_string() }) },
                        GrammarElement::RuleReference { name: r#"statement"#.to_string() },
                    ] }) },
            ] },
            line_number: 282,
        },
        GrammarRule {
            name: r#"else_clause"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"else"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::NegativeLookahead { element: Box::new(GrammarElement::Literal { value: r#"end"#.to_string() }) },
                        GrammarElement::RuleReference { name: r#"statement"#.to_string() },
                    ] }) },
            ] },
            line_number: 283,
        },
        GrammarRule {
            name: r#"unless_statement"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"unless"#.to_string() },
                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::NegativeLookahead { element: Box::new(GrammarElement::Literal { value: r#"else"#.to_string() }) },
                        GrammarElement::NegativeLookahead { element: Box::new(GrammarElement::Literal { value: r#"end"#.to_string() }) },
                        GrammarElement::RuleReference { name: r#"statement"#.to_string() },
                    ] }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"else_clause"#.to_string() }) },
                GrammarElement::Literal { value: r#"end"#.to_string() },
            ] },
            line_number: 284,
        },
        GrammarRule {
            name: r#"while_statement"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"while"#.to_string() },
                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::NegativeLookahead { element: Box::new(GrammarElement::Literal { value: r#"end"#.to_string() }) },
                        GrammarElement::RuleReference { name: r#"statement"#.to_string() },
                    ] }) },
                GrammarElement::Literal { value: r#"end"#.to_string() },
            ] },
            line_number: 285,
        },
        GrammarRule {
            name: r#"until_statement"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"until"#.to_string() },
                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::NegativeLookahead { element: Box::new(GrammarElement::Literal { value: r#"end"#.to_string() }) },
                        GrammarElement::RuleReference { name: r#"statement"#.to_string() },
                    ] }) },
                GrammarElement::Literal { value: r#"end"#.to_string() },
            ] },
            line_number: 286,
        },
        GrammarRule {
            name: r#"case_statement"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"case"#.to_string() },
                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::RuleReference { name: r#"when_clause"#.to_string() },
                        GrammarElement::RuleReference { name: r#"in_clause"#.to_string() },
                    ] }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"else_clause"#.to_string() }) },
                GrammarElement::Literal { value: r#"end"#.to_string() },
            ] },
            line_number: 309,
        },
        GrammarRule {
            name: r#"when_clause"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"when"#.to_string() },
                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    ] }) },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::NegativeLookahead { element: Box::new(GrammarElement::Literal { value: r#"when"#.to_string() }) },
                        GrammarElement::NegativeLookahead { element: Box::new(GrammarElement::Literal { value: r#"in"#.to_string() }) },
                        GrammarElement::NegativeLookahead { element: Box::new(GrammarElement::Literal { value: r#"else"#.to_string() }) },
                        GrammarElement::NegativeLookahead { element: Box::new(GrammarElement::Literal { value: r#"end"#.to_string() }) },
                        GrammarElement::RuleReference { name: r#"statement"#.to_string() },
                    ] }) },
            ] },
            line_number: 310,
        },
        GrammarRule {
            name: r#"in_clause"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"in"#.to_string() },
                GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::NegativeLookahead { element: Box::new(GrammarElement::Literal { value: r#"when"#.to_string() }) },
                        GrammarElement::NegativeLookahead { element: Box::new(GrammarElement::Literal { value: r#"in"#.to_string() }) },
                        GrammarElement::NegativeLookahead { element: Box::new(GrammarElement::Literal { value: r#"else"#.to_string() }) },
                        GrammarElement::NegativeLookahead { element: Box::new(GrammarElement::Literal { value: r#"end"#.to_string() }) },
                        GrammarElement::RuleReference { name: r#"statement"#.to_string() },
                    ] }) },
            ] },
            line_number: 332,
        },
        GrammarRule {
            name: r#"pattern"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"array_pattern"#.to_string() },
                GrammarElement::RuleReference { name: r#"hash_pattern"#.to_string() },
                GrammarElement::RuleReference { name: r#"class_pattern"#.to_string() },
                GrammarElement::RuleReference { name: r#"pin_pattern"#.to_string() },
                GrammarElement::RuleReference { name: r#"literal_pattern"#.to_string() },
                GrammarElement::RuleReference { name: r#"binding_pattern"#.to_string() },
            ] },
            line_number: 333,
        },
        GrammarRule {
            name: r#"literal_pattern"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                GrammarElement::RuleReference { name: r#"symbol_literal"#.to_string() },
                GrammarElement::TokenReference { name: r#"KEYWORD"#.to_string() },
            ] },
            line_number: 334,
        },
        GrammarRule {
            name: r#"binding_pattern"#.to_string(),
            body: GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
            line_number: 335,
        },
        GrammarRule {
            name: r#"array_pattern"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::RuleReference { name: r#"splat_pattern"#.to_string() },
                                GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                            ] }) },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                        GrammarElement::RuleReference { name: r#"splat_pattern"#.to_string() },
                                        GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                                    ] }) },
                            ] }) },
                    ] }) },
                GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
            ] },
            line_number: 336,
        },
        GrammarRule {
            name: r#"hash_pattern"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"hash_pattern_pair"#.to_string() },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                GrammarElement::RuleReference { name: r#"hash_pattern_pair"#.to_string() },
                            ] }) },
                    ] }) },
                GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
            ] },
            line_number: 337,
        },
        GrammarRule {
            name: r#"hash_pattern_pair"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"pattern"#.to_string() }) },
            ] },
            line_number: 338,
        },
        GrammarRule {
            name: r#"splat_pattern"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"*"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"NAME"#.to_string() }) },
            ] },
            line_number: 345,
        },
        GrammarRule {
            name: r#"pin_pattern"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"^"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
            ] },
            line_number: 350,
        },
        GrammarRule {
            name: r#"class_pattern"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                            ] }) },
                    ] }) },
                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
            ] },
            line_number: 356,
        },
        GrammarRule {
            name: r#"begin_statement"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"begin"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::NegativeLookahead { element: Box::new(GrammarElement::Literal { value: r#"rescue"#.to_string() }) },
                        GrammarElement::NegativeLookahead { element: Box::new(GrammarElement::Literal { value: r#"ensure"#.to_string() }) },
                        GrammarElement::NegativeLookahead { element: Box::new(GrammarElement::Literal { value: r#"end"#.to_string() }) },
                        GrammarElement::RuleReference { name: r#"statement"#.to_string() },
                    ] }) },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"rescue_clause"#.to_string() }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"ensure_clause"#.to_string() }) },
                GrammarElement::Literal { value: r#"end"#.to_string() },
            ] },
            line_number: 377,
        },
        GrammarRule {
            name: r#"rescue_clause"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"rescue"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"exception_list"#.to_string() },
                        GrammarElement::Literal { value: r#"=>"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    ] }) },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::NegativeLookahead { element: Box::new(GrammarElement::Literal { value: r#"rescue"#.to_string() }) },
                        GrammarElement::NegativeLookahead { element: Box::new(GrammarElement::Literal { value: r#"ensure"#.to_string() }) },
                        GrammarElement::NegativeLookahead { element: Box::new(GrammarElement::Literal { value: r#"end"#.to_string() }) },
                        GrammarElement::RuleReference { name: r#"statement"#.to_string() },
                    ] }) },
            ] },
            line_number: 386,
        },
        GrammarRule {
            name: r#"exception_list"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    ] }) },
            ] },
            line_number: 387,
        },
        GrammarRule {
            name: r#"ensure_clause"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"ensure"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::NegativeLookahead { element: Box::new(GrammarElement::Literal { value: r#"end"#.to_string() }) },
                        GrammarElement::RuleReference { name: r#"statement"#.to_string() },
                    ] }) },
            ] },
            line_number: 388,
        },
        GrammarRule {
            name: r#"assignment"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                        GrammarElement::Literal { value: r#"+="#.to_string() },
                        GrammarElement::Literal { value: r#"-="#.to_string() },
                        GrammarElement::Literal { value: r#"*="#.to_string() },
                        GrammarElement::Literal { value: r#"/="#.to_string() },
                        GrammarElement::Literal { value: r#"%="#.to_string() },
                        GrammarElement::Literal { value: r#"**="#.to_string() },
                        GrammarElement::Literal { value: r#"<<="#.to_string() },
                        GrammarElement::Literal { value: r#">>="#.to_string() },
                        GrammarElement::Literal { value: r#"&="#.to_string() },
                        GrammarElement::Literal { value: r#"|="#.to_string() },
                        GrammarElement::Literal { value: r#"^="#.to_string() },
                        GrammarElement::Literal { value: r#"||="#.to_string() },
                        GrammarElement::Literal { value: r#"&&="#.to_string() },
                    ] }) },
                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
            ] },
            line_number: 408,
        },
        GrammarRule {
            name: r#"rightward_assignment"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                GrammarElement::Literal { value: r#"=>"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
            ] },
            line_number: 427,
        },
        GrammarRule {
            name: r#"method_call"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        GrammarElement::TokenReference { name: r#"KEYWORD"#.to_string() },
                    ] }) },
                GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"call_arg"#.to_string() },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                GrammarElement::RuleReference { name: r#"call_arg"#.to_string() },
                            ] }) },
                    ] }) },
                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"dot_call"#.to_string() }) },
            ] },
            line_number: 438,
        },
        GrammarRule {
            name: r#"dot_call"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"."#.to_string() },
                GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        GrammarElement::TokenReference { name: r#"KEYWORD"#.to_string() },
                    ] }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::RuleReference { name: r#"call_arg"#.to_string() },
                                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                        GrammarElement::RuleReference { name: r#"call_arg"#.to_string() },
                                    ] }) },
                            ] }) },
                        GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                    ] }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"block"#.to_string() }) },
            ] },
            line_number: 439,
        },
        GrammarRule {
            name: r#"scope_resolution"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"::"#.to_string() },
                GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        GrammarElement::TokenReference { name: r#"KEYWORD"#.to_string() },
                    ] }) },
            ] },
            line_number: 447,
        },
        GrammarRule {
            name: r#"call_arg"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Optional { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::Literal { value: r#"*"#.to_string() },
                        GrammarElement::Literal { value: r#"**"#.to_string() },
                        GrammarElement::Literal { value: r#"&"#.to_string() },
                    ] }) },
                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
            ] },
            line_number: 477,
        },
        GrammarRule {
            name: r#"method_call_no_paren"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        GrammarElement::TokenReference { name: r#"KEYWORD"#.to_string() },
                    ] }) },
                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    ] }) },
            ] },
            line_number: 491,
        },
        GrammarRule {
            name: r#"expression_stmt"#.to_string(),
            body: GrammarElement::RuleReference { name: r#"expression"#.to_string() },
            line_number: 492,
        },
        GrammarRule {
            name: r#"expression"#.to_string(),
            body: GrammarElement::RuleReference { name: r#"ternary"#.to_string() },
            line_number: 599,
        },
        GrammarRule {
            name: r#"ternary"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"range"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"?"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        GrammarElement::Literal { value: r#":"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    ] }) },
            ] },
            line_number: 600,
        },
        GrammarRule {
            name: r#"range"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::Literal { value: r#"..."#.to_string() },
                            GrammarElement::Literal { value: r#".."#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"logical_or"#.to_string() },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"logical_or"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                    GrammarElement::Literal { value: r#"..."#.to_string() },
                                    GrammarElement::Literal { value: r#".."#.to_string() },
                                ] }) },
                            GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"logical_or"#.to_string() }) },
                        ] }) },
                ] },
            ] },
            line_number: 601,
        },
        GrammarRule {
            name: r#"logical_or"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"logical_and"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::Literal { value: r#"||"#.to_string() },
                                GrammarElement::Literal { value: r#"or"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"logical_and"#.to_string() },
                    ] }) },
            ] },
            line_number: 602,
        },
        GrammarRule {
            name: r#"logical_and"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"logical_not"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::Literal { value: r#"&&"#.to_string() },
                                GrammarElement::Literal { value: r#"and"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"logical_not"#.to_string() },
                    ] }) },
            ] },
            line_number: 603,
        },
        GrammarRule {
            name: r#"logical_not"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Repetition { element: Box::new(GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::Literal { value: r#"!"#.to_string() },
                            GrammarElement::Literal { value: r#"not"#.to_string() },
                        ] }) }) },
                GrammarElement::RuleReference { name: r#"comparison"#.to_string() },
            ] },
            line_number: 610,
        },
        GrammarRule {
            name: r#"comparison"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"sum"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::Literal { value: r#"=="#.to_string() },
                                GrammarElement::Literal { value: r#"!="#.to_string() },
                                GrammarElement::Literal { value: r#"<="#.to_string() },
                                GrammarElement::Literal { value: r#">="#.to_string() },
                                GrammarElement::Literal { value: r#"<"#.to_string() },
                                GrammarElement::Literal { value: r#">"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"sum"#.to_string() },
                    ] }) },
            ] },
            line_number: 611,
        },
        GrammarRule {
            name: r#"sum"#.to_string(),
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
            line_number: 612,
        },
        GrammarRule {
            name: r#"term"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"factor"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                                GrammarElement::TokenReference { name: r#"SLASH"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"factor"#.to_string() },
                    ] }) },
            ] },
            line_number: 613,
        },
        GrammarRule {
            name: r#"factor"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::RuleReference { name: r#"defined_expression"#.to_string() },
                        GrammarElement::RuleReference { name: r#"lambda_literal"#.to_string() },
                        GrammarElement::RuleReference { name: r#"method_call"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                        GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        GrammarElement::Group { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::NegativeLookahead { element: Box::new(GrammarElement::Literal { value: r#"end"#.to_string() }) },
                                GrammarElement::NegativeLookahead { element: Box::new(GrammarElement::Literal { value: r#"rescue"#.to_string() }) },
                                GrammarElement::NegativeLookahead { element: Box::new(GrammarElement::Literal { value: r#"ensure"#.to_string() }) },
                                GrammarElement::NegativeLookahead { element: Box::new(GrammarElement::Literal { value: r#"else"#.to_string() }) },
                                GrammarElement::NegativeLookahead { element: Box::new(GrammarElement::Literal { value: r#"elsif"#.to_string() }) },
                                GrammarElement::NegativeLookahead { element: Box::new(GrammarElement::Literal { value: r#"when"#.to_string() }) },
                                GrammarElement::NegativeLookahead { element: Box::new(GrammarElement::Literal { value: r#"then"#.to_string() }) },
                                GrammarElement::NegativeLookahead { element: Box::new(GrammarElement::Literal { value: r#"in"#.to_string() }) },
                                GrammarElement::NegativeLookahead { element: Box::new(GrammarElement::Literal { value: r#"do"#.to_string() }) },
                                GrammarElement::TokenReference { name: r#"KEYWORD"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"symbol_literal"#.to_string() },
                        GrammarElement::RuleReference { name: r#"array_literal"#.to_string() },
                        GrammarElement::RuleReference { name: r#"hash_literal"#.to_string() },
                        GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                        ] },
                        GrammarElement::RuleReference { name: r#"unary_minus"#.to_string() },
                    ] }) },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::RuleReference { name: r#"dot_call"#.to_string() },
                        GrammarElement::RuleReference { name: r#"scope_resolution"#.to_string() },
                    ] }) },
            ] },
            line_number: 649,
        },
        GrammarRule {
            name: r#"lambda_literal"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"->"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"params"#.to_string() }) },
                        GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                    ] }) },
                GrammarElement::RuleReference { name: r#"block"#.to_string() },
            ] },
            line_number: 668,
        },
        GrammarRule {
            name: r#"unary_minus"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                GrammarElement::RuleReference { name: r#"factor"#.to_string() },
            ] },
            line_number: 669,
        },
        GrammarRule {
            name: r#"defined_expression"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"defined?"#.to_string() },
                GrammarElement::RuleReference { name: r#"factor"#.to_string() },
            ] },
            line_number: 680,
        },
        GrammarRule {
            name: r#"symbol_literal"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#":"#.to_string() },
                GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        GrammarElement::TokenReference { name: r#"KEYWORD"#.to_string() },
                        GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                    ] }) },
            ] },
            line_number: 687,
        },
        GrammarRule {
            name: r#"array_literal"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            ] }) },
                    ] }) },
                GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
            ] },
            line_number: 688,
        },
        GrammarRule {
            name: r#"hash_literal"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"hash_entry"#.to_string() },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                GrammarElement::RuleReference { name: r#"hash_entry"#.to_string() },
                            ] }) },
                    ] }) },
                GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
            ] },
            line_number: 689,
        },
        GrammarRule {
            name: r#"hash_entry"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Literal { value: r#"=>"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
            ] },
            line_number: 690,
        },
    ],
        version: 1,
    }
}
