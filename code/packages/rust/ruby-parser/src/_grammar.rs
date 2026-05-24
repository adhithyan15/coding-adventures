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
                GrammarElement::RuleReference { name: r#"def_statement"#.to_string() },
                GrammarElement::RuleReference { name: r#"class_statement"#.to_string() },
                GrammarElement::RuleReference { name: r#"module_statement"#.to_string() },
                GrammarElement::RuleReference { name: r#"if_statement"#.to_string() },
                GrammarElement::RuleReference { name: r#"unless_statement"#.to_string() },
                GrammarElement::RuleReference { name: r#"while_statement"#.to_string() },
                GrammarElement::RuleReference { name: r#"until_statement"#.to_string() },
                GrammarElement::RuleReference { name: r#"return_statement"#.to_string() },
                GrammarElement::RuleReference { name: r#"break_statement"#.to_string() },
                GrammarElement::RuleReference { name: r#"next_statement"#.to_string() },
                GrammarElement::RuleReference { name: r#"assignment"#.to_string() },
                GrammarElement::RuleReference { name: r#"method_with_block"#.to_string() },
                GrammarElement::RuleReference { name: r#"method_call"#.to_string() },
                GrammarElement::RuleReference { name: r#"method_call_no_paren"#.to_string() },
                GrammarElement::RuleReference { name: r#"expression_stmt"#.to_string() },
            ] },
            line_number: 28,
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
                        GrammarElement::NegativeLookahead { element: Box::new(GrammarElement::Literal { value: r#"end"#.to_string() }) },
                        GrammarElement::RuleReference { name: r#"statement"#.to_string() },
                    ] }) },
                GrammarElement::Literal { value: r#"end"#.to_string() },
            ] },
            line_number: 29,
        },
        GrammarRule {
            name: r#"class_statement"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"class"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::NegativeLookahead { element: Box::new(GrammarElement::Literal { value: r#"end"#.to_string() }) },
                        GrammarElement::RuleReference { name: r#"statement"#.to_string() },
                    ] }) },
                GrammarElement::Literal { value: r#"end"#.to_string() },
            ] },
            line_number: 34,
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
            line_number: 35,
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
            line_number: 37,
        },
        GrammarRule {
            name: r#"block"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"do_block"#.to_string() },
                GrammarElement::RuleReference { name: r#"brace_block"#.to_string() },
            ] },
            line_number: 38,
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
            line_number: 39,
        },
        GrammarRule {
            name: r#"brace_block"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"block_params"#.to_string() }) },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"statement"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
            ] },
            line_number: 40,
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
                GrammarElement::Literal { value: r#"|"#.to_string() },
            ] },
            line_number: 41,
        },
        GrammarRule {
            name: r#"return_statement"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"return"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"expression"#.to_string() }) },
            ] },
            line_number: 43,
        },
        GrammarRule {
            name: r#"break_statement"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"break"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"expression"#.to_string() }) },
            ] },
            line_number: 44,
        },
        GrammarRule {
            name: r#"next_statement"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"next"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"expression"#.to_string() }) },
            ] },
            line_number: 45,
        },
        GrammarRule {
            name: r#"params"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    ] }) },
            ] },
            line_number: 46,
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
            line_number: 47,
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
            line_number: 48,
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
            line_number: 49,
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
            line_number: 50,
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
            line_number: 51,
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
            line_number: 52,
        },
        GrammarRule {
            name: r#"assignment"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
            ] },
            line_number: 53,
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
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            ] }) },
                    ] }) },
                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"dot_call"#.to_string() }) },
            ] },
            line_number: 64,
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
                                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                                    ] }) },
                            ] }) },
                        GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                    ] }) },
            ] },
            line_number: 65,
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
            line_number: 79,
        },
        GrammarRule {
            name: r#"expression_stmt"#.to_string(),
            body: GrammarElement::RuleReference { name: r#"expression"#.to_string() },
            line_number: 80,
        },
        GrammarRule {
            name: r#"expression"#.to_string(),
            body: GrammarElement::RuleReference { name: r#"range"#.to_string() },
            line_number: 145,
        },
        GrammarRule {
            name: r#"range"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"logical_or"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::Literal { value: r#"..."#.to_string() },
                                GrammarElement::Literal { value: r#".."#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"logical_or"#.to_string() },
                    ] }) },
            ] },
            line_number: 146,
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
            line_number: 147,
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
            line_number: 148,
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
            line_number: 155,
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
            line_number: 156,
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
            line_number: 157,
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
            line_number: 158,
        },
        GrammarRule {
            name: r#"factor"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::RuleReference { name: r#"method_call"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                        GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        GrammarElement::TokenReference { name: r#"KEYWORD"#.to_string() },
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
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"dot_call"#.to_string() }) },
            ] },
            line_number: 168,
        },
        GrammarRule {
            name: r#"unary_minus"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                GrammarElement::RuleReference { name: r#"factor"#.to_string() },
            ] },
            line_number: 169,
        },
        GrammarRule {
            name: r#"symbol_literal"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        GrammarElement::TokenReference { name: r#"KEYWORD"#.to_string() },
                        GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                    ] }) },
            ] },
            line_number: 170,
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
            line_number: 171,
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
            line_number: 172,
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
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Literal { value: r#"=>"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
            ] },
            line_number: 173,
        },
    ],
        version: 1,
    }
}
