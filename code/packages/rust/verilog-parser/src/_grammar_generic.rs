// AUTO-GENERATED FILE — DO NOT EDIT
// Source: verilog.grammar
// Regenerate with: grammar-tools compile-grammar verilog.grammar
//
// This file embeds a ParserGrammar as native Rust data structures.
// Call `parser_grammar()` instead of reading and parsing the .grammar file.

use grammar_tools::parser_grammar::{GrammarElement, GrammarRule, ParserGrammar};

pub fn parser_grammar() -> ParserGrammar {
    ParserGrammar {
        rules: vec![
        GrammarRule {
            name: r#"source_text"#.to_string(),
            body: GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"description"#.to_string() }) },
            line_number: 42,
        },
        GrammarRule {
            name: r#"description"#.to_string(),
            body: GrammarElement::RuleReference { name: r#"module_declaration"#.to_string() },
            line_number: 44,
        },
        GrammarRule {
            name: r#"module_declaration"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"module"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"parameter_port_list"#.to_string() }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"port_list"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"module_item"#.to_string() }) },
                GrammarElement::Literal { value: r#"endmodule"#.to_string() },
            ] },
            line_number: 73,
        },
        GrammarRule {
            name: r#"parameter_port_list"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"HASH"#.to_string() },
                GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                GrammarElement::RuleReference { name: r#"parameter_declaration"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::RuleReference { name: r#"parameter_declaration"#.to_string() },
                    ] }) },
                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
            ] },
            line_number: 91,
        },
        GrammarRule {
            name: r#"parameter_declaration"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"parameter"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"range"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
            ] },
            line_number: 94,
        },
        GrammarRule {
            name: r#"localparam_declaration"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"localparam"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"range"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
            ] },
            line_number: 95,
        },
        GrammarRule {
            name: r#"port_list"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                GrammarElement::RuleReference { name: r#"port"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::RuleReference { name: r#"port"#.to_string() },
                    ] }) },
                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
            ] },
            line_number: 115,
        },
        GrammarRule {
            name: r#"port"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"port_direction"#.to_string() }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"net_type"#.to_string() }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"signed"#.to_string() }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"range"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
            ] },
            line_number: 117,
        },
        GrammarRule {
            name: r#"port_direction"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Literal { value: r#"input"#.to_string() },
                GrammarElement::Literal { value: r#"output"#.to_string() },
                GrammarElement::Literal { value: r#"inout"#.to_string() },
            ] },
            line_number: 119,
        },
        GrammarRule {
            name: r#"net_type"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Literal { value: r#"wire"#.to_string() },
                GrammarElement::Literal { value: r#"reg"#.to_string() },
                GrammarElement::Literal { value: r#"tri"#.to_string() },
                GrammarElement::Literal { value: r#"supply0"#.to_string() },
                GrammarElement::Literal { value: r#"supply1"#.to_string() },
            ] },
            line_number: 120,
        },
        GrammarRule {
            name: r#"range"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
            ] },
            line_number: 122,
        },
        GrammarRule {
            name: r#"module_item"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"port_declaration"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"net_declaration"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"reg_declaration"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"integer_declaration"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"parameter_declaration"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"localparam_declaration"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                ] },
                GrammarElement::RuleReference { name: r#"continuous_assign"#.to_string() },
                GrammarElement::RuleReference { name: r#"always_construct"#.to_string() },
                GrammarElement::RuleReference { name: r#"initial_construct"#.to_string() },
                GrammarElement::RuleReference { name: r#"module_instantiation"#.to_string() },
                GrammarElement::RuleReference { name: r#"generate_region"#.to_string() },
                GrammarElement::RuleReference { name: r#"function_declaration"#.to_string() },
                GrammarElement::RuleReference { name: r#"task_declaration"#.to_string() },
            ] },
            line_number: 139,
        },
        GrammarRule {
            name: r#"port_declaration"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"port_direction"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"net_type"#.to_string() }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"signed"#.to_string() }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"range"#.to_string() }) },
                GrammarElement::RuleReference { name: r#"name_list"#.to_string() },
            ] },
            line_number: 174,
        },
        GrammarRule {
            name: r#"net_declaration"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"net_type"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"signed"#.to_string() }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"range"#.to_string() }) },
                GrammarElement::RuleReference { name: r#"name_list"#.to_string() },
            ] },
            line_number: 176,
        },
        GrammarRule {
            name: r#"reg_declaration"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"reg"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"signed"#.to_string() }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"range"#.to_string() }) },
                GrammarElement::RuleReference { name: r#"name_list"#.to_string() },
            ] },
            line_number: 177,
        },
        GrammarRule {
            name: r#"integer_declaration"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"integer"#.to_string() },
                GrammarElement::RuleReference { name: r#"name_list"#.to_string() },
            ] },
            line_number: 178,
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
            line_number: 179,
        },
        GrammarRule {
            name: r#"continuous_assign"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"assign"#.to_string() },
                GrammarElement::RuleReference { name: r#"assignment"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::RuleReference { name: r#"assignment"#.to_string() },
                    ] }) },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
            ] },
            line_number: 198,
        },
        GrammarRule {
            name: r#"assignment"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"lvalue"#.to_string() },
                GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
            ] },
            line_number: 199,
        },
        GrammarRule {
            name: r#"lvalue"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"range_select"#.to_string() }) },
                ] },
                GrammarElement::RuleReference { name: r#"concatenation"#.to_string() },
            ] },
            line_number: 203,
        },
        GrammarRule {
            name: r#"range_select"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    ] }) },
                GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
            ] },
            line_number: 206,
        },
        GrammarRule {
            name: r#"always_construct"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"always"#.to_string() },
                GrammarElement::TokenReference { name: r#"AT"#.to_string() },
                GrammarElement::RuleReference { name: r#"sensitivity_list"#.to_string() },
                GrammarElement::RuleReference { name: r#"statement"#.to_string() },
            ] },
            line_number: 243,
        },
        GrammarRule {
            name: r#"initial_construct"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"initial"#.to_string() },
                GrammarElement::RuleReference { name: r#"statement"#.to_string() },
            ] },
            line_number: 244,
        },
        GrammarRule {
            name: r#"sensitivity_list"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"sensitivity_item"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                    GrammarElement::Literal { value: r#"or"#.to_string() },
                                    GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                ] }) },
                            GrammarElement::RuleReference { name: r#"sensitivity_item"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
            ] },
            line_number: 246,
        },
        GrammarRule {
            name: r#"sensitivity_item"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Optional { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::Literal { value: r#"posedge"#.to_string() },
                        GrammarElement::Literal { value: r#"negedge"#.to_string() },
                    ] }) },
                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
            ] },
            line_number: 250,
        },
        GrammarRule {
            name: r#"statement"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"block_statement"#.to_string() },
                GrammarElement::RuleReference { name: r#"if_statement"#.to_string() },
                GrammarElement::RuleReference { name: r#"case_statement"#.to_string() },
                GrammarElement::RuleReference { name: r#"for_statement"#.to_string() },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"blocking_assignment"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"nonblocking_assignment"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"task_call"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                ] },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
            ] },
            line_number: 259,
        },
        GrammarRule {
            name: r#"block_statement"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"begin"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    ] }) },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"statement"#.to_string() }) },
                GrammarElement::Literal { value: r#"end"#.to_string() },
            ] },
            line_number: 275,
        },
        GrammarRule {
            name: r#"if_statement"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"if"#.to_string() },
                GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                GrammarElement::RuleReference { name: r#"statement"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"else"#.to_string() },
                        GrammarElement::RuleReference { name: r#"statement"#.to_string() },
                    ] }) },
            ] },
            line_number: 286,
        },
        GrammarRule {
            name: r#"case_statement"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::Literal { value: r#"case"#.to_string() },
                        GrammarElement::Literal { value: r#"casex"#.to_string() },
                        GrammarElement::Literal { value: r#"casez"#.to_string() },
                    ] }) },
                GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"case_item"#.to_string() }) },
                GrammarElement::Literal { value: r#"endcase"#.to_string() },
            ] },
            line_number: 301,
        },
        GrammarRule {
            name: r#"case_item"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"expression_list"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"statement"#.to_string() },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"default"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COLON"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"statement"#.to_string() },
                ] },
            ] },
            line_number: 306,
        },
        GrammarRule {
            name: r#"expression_list"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    ] }) },
            ] },
            line_number: 309,
        },
        GrammarRule {
            name: r#"for_statement"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"for"#.to_string() },
                GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                GrammarElement::RuleReference { name: r#"blocking_assignment"#.to_string() },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                GrammarElement::RuleReference { name: r#"blocking_assignment"#.to_string() },
                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                GrammarElement::RuleReference { name: r#"statement"#.to_string() },
            ] },
            line_number: 313,
        },
        GrammarRule {
            name: r#"blocking_assignment"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"lvalue"#.to_string() },
                GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
            ] },
            line_number: 317,
        },
        GrammarRule {
            name: r#"nonblocking_assignment"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"lvalue"#.to_string() },
                GrammarElement::TokenReference { name: r#"LESS_EQUALS"#.to_string() },
                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
            ] },
            line_number: 318,
        },
        GrammarRule {
            name: r#"task_call"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            ] }) },
                    ] }) },
                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
            ] },
            line_number: 321,
        },
        GrammarRule {
            name: r#"module_instantiation"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"parameter_value_assignment"#.to_string() }) },
                GrammarElement::RuleReference { name: r#"instance"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::RuleReference { name: r#"instance"#.to_string() },
                    ] }) },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
            ] },
            line_number: 340,
        },
        GrammarRule {
            name: r#"parameter_value_assignment"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"HASH"#.to_string() },
                GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    ] }) },
                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
            ] },
            line_number: 343,
        },
        GrammarRule {
            name: r#"instance"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                GrammarElement::RuleReference { name: r#"port_connections"#.to_string() },
                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
            ] },
            line_number: 345,
        },
        GrammarRule {
            name: r#"port_connections"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"named_port_connection"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"named_port_connection"#.to_string() },
                        ] }) },
                ] },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            ] }) },
                    ] }) },
            ] },
            line_number: 347,
        },
        GrammarRule {
            name: r#"named_port_connection"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"expression"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
            ] },
            line_number: 350,
        },
        GrammarRule {
            name: r#"generate_region"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"generate"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"generate_item"#.to_string() }) },
                GrammarElement::Literal { value: r#"endgenerate"#.to_string() },
            ] },
            line_number: 377,
        },
        GrammarRule {
            name: r#"generate_item"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"genvar_declaration"#.to_string() },
                GrammarElement::RuleReference { name: r#"generate_for"#.to_string() },
                GrammarElement::RuleReference { name: r#"generate_if"#.to_string() },
                GrammarElement::RuleReference { name: r#"module_item"#.to_string() },
            ] },
            line_number: 379,
        },
        GrammarRule {
            name: r#"genvar_declaration"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"genvar"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    ] }) },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
            ] },
            line_number: 384,
        },
        GrammarRule {
            name: r#"generate_for"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"for"#.to_string() },
                GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                GrammarElement::RuleReference { name: r#"genvar_assignment"#.to_string() },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                GrammarElement::RuleReference { name: r#"genvar_assignment"#.to_string() },
                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                GrammarElement::RuleReference { name: r#"generate_block"#.to_string() },
            ] },
            line_number: 386,
        },
        GrammarRule {
            name: r#"generate_if"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"if"#.to_string() },
                GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                GrammarElement::RuleReference { name: r#"generate_block"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"else"#.to_string() },
                        GrammarElement::RuleReference { name: r#"generate_block"#.to_string() },
                    ] }) },
            ] },
            line_number: 390,
        },
        GrammarRule {
            name: r#"generate_block"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"begin"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"generate_item"#.to_string() }) },
                    GrammarElement::Literal { value: r#"end"#.to_string() },
                ] },
                GrammarElement::RuleReference { name: r#"generate_item"#.to_string() },
            ] },
            line_number: 393,
        },
        GrammarRule {
            name: r#"genvar_assignment"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
            ] },
            line_number: 396,
        },
        GrammarRule {
            name: r#"function_declaration"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"function"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"range"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"function_item"#.to_string() }) },
                GrammarElement::RuleReference { name: r#"statement"#.to_string() },
                GrammarElement::Literal { value: r#"endfunction"#.to_string() },
            ] },
            line_number: 415,
        },
        GrammarRule {
            name: r#"function_item"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"port_declaration"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"reg_declaration"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"integer_declaration"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"parameter_declaration"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                ] },
            ] },
            line_number: 420,
        },
        GrammarRule {
            name: r#"task_declaration"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"task"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"task_item"#.to_string() }) },
                GrammarElement::RuleReference { name: r#"statement"#.to_string() },
                GrammarElement::Literal { value: r#"endtask"#.to_string() },
            ] },
            line_number: 425,
        },
        GrammarRule {
            name: r#"task_item"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"port_declaration"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"reg_declaration"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"integer_declaration"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                ] },
            ] },
            line_number: 430,
        },
        GrammarRule {
            name: r#"expression"#.to_string(),
            body: GrammarElement::RuleReference { name: r#"ternary_expr"#.to_string() },
            line_number: 458,
        },
        GrammarRule {
            name: r#"ternary_expr"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"or_expr"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"QUESTION"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                        GrammarElement::RuleReference { name: r#"ternary_expr"#.to_string() },
                    ] }) },
            ] },
            line_number: 464,
        },
        GrammarRule {
            name: r#"or_expr"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"and_expr"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LOGIC_OR"#.to_string() },
                        GrammarElement::RuleReference { name: r#"and_expr"#.to_string() },
                    ] }) },
            ] },
            line_number: 467,
        },
        GrammarRule {
            name: r#"and_expr"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"bit_or_expr"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LOGIC_AND"#.to_string() },
                        GrammarElement::RuleReference { name: r#"bit_or_expr"#.to_string() },
                    ] }) },
            ] },
            line_number: 468,
        },
        GrammarRule {
            name: r#"bit_or_expr"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"bit_xor_expr"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"PIPE"#.to_string() },
                        GrammarElement::RuleReference { name: r#"bit_xor_expr"#.to_string() },
                    ] }) },
            ] },
            line_number: 471,
        },
        GrammarRule {
            name: r#"bit_xor_expr"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"bit_and_expr"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"CARET"#.to_string() },
                        GrammarElement::RuleReference { name: r#"bit_and_expr"#.to_string() },
                    ] }) },
            ] },
            line_number: 472,
        },
        GrammarRule {
            name: r#"bit_and_expr"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"equality_expr"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"AMP"#.to_string() },
                        GrammarElement::RuleReference { name: r#"equality_expr"#.to_string() },
                    ] }) },
            ] },
            line_number: 473,
        },
        GrammarRule {
            name: r#"equality_expr"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"relational_expr"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::TokenReference { name: r#"EQUALS_EQUALS"#.to_string() },
                                GrammarElement::TokenReference { name: r#"NOT_EQUALS"#.to_string() },
                                GrammarElement::TokenReference { name: r#"CASE_EQ"#.to_string() },
                                GrammarElement::TokenReference { name: r#"CASE_NEQ"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"relational_expr"#.to_string() },
                    ] }) },
            ] },
            line_number: 477,
        },
        GrammarRule {
            name: r#"relational_expr"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"shift_expr"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                                GrammarElement::TokenReference { name: r#"LESS_EQUALS"#.to_string() },
                                GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                                GrammarElement::TokenReference { name: r#"GREATER_EQUALS"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"shift_expr"#.to_string() },
                    ] }) },
            ] },
            line_number: 484,
        },
        GrammarRule {
            name: r#"shift_expr"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"additive_expr"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::TokenReference { name: r#"LEFT_SHIFT"#.to_string() },
                                GrammarElement::TokenReference { name: r#"RIGHT_SHIFT"#.to_string() },
                                GrammarElement::TokenReference { name: r#"ARITH_LEFT_SHIFT"#.to_string() },
                                GrammarElement::TokenReference { name: r#"ARITH_RIGHT_SHIFT"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"additive_expr"#.to_string() },
                    ] }) },
            ] },
            line_number: 489,
        },
        GrammarRule {
            name: r#"additive_expr"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"multiplicative_expr"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                                GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"multiplicative_expr"#.to_string() },
                    ] }) },
            ] },
            line_number: 494,
        },
        GrammarRule {
            name: r#"multiplicative_expr"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"power_expr"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                                GrammarElement::TokenReference { name: r#"SLASH"#.to_string() },
                                GrammarElement::TokenReference { name: r#"PERCENT"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"power_expr"#.to_string() },
                    ] }) },
            ] },
            line_number: 495,
        },
        GrammarRule {
            name: r#"power_expr"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"unary_expr"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"POWER"#.to_string() },
                        GrammarElement::RuleReference { name: r#"unary_expr"#.to_string() },
                    ] }) },
            ] },
            line_number: 496,
        },
        GrammarRule {
            name: r#"unary_expr"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                            GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                            GrammarElement::TokenReference { name: r#"BANG"#.to_string() },
                            GrammarElement::TokenReference { name: r#"TILDE"#.to_string() },
                            GrammarElement::TokenReference { name: r#"AMP"#.to_string() },
                            GrammarElement::TokenReference { name: r#"PIPE"#.to_string() },
                            GrammarElement::TokenReference { name: r#"CARET"#.to_string() },
                            GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"TILDE"#.to_string() },
                                GrammarElement::TokenReference { name: r#"AMP"#.to_string() },
                            ] },
                            GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"TILDE"#.to_string() },
                                GrammarElement::TokenReference { name: r#"PIPE"#.to_string() },
                            ] },
                            GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"TILDE"#.to_string() },
                                GrammarElement::TokenReference { name: r#"CARET"#.to_string() },
                            ] },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"unary_expr"#.to_string() },
                ] },
                GrammarElement::RuleReference { name: r#"primary"#.to_string() },
            ] },
            line_number: 508,
        },
        GrammarRule {
            name: r#"primary"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                GrammarElement::TokenReference { name: r#"SIZED_NUMBER"#.to_string() },
                GrammarElement::TokenReference { name: r#"REAL_NUMBER"#.to_string() },
                GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"SYSTEM_ID"#.to_string() },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                GrammarElement::RuleReference { name: r#"concatenation"#.to_string() },
                GrammarElement::RuleReference { name: r#"replication"#.to_string() },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"primary"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
            ] },
            line_number: 518,
        },
        GrammarRule {
            name: r#"concatenation"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    ] }) },
                GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
            ] },
            line_number: 534,
        },
        GrammarRule {
            name: r#"replication"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                GrammarElement::RuleReference { name: r#"concatenation"#.to_string() },
                GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
            ] },
            line_number: 540,
        },
    ],
        version: 0,
    }
}
