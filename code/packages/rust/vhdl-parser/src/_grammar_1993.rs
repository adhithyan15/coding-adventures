// AUTO-GENERATED FILE — DO NOT EDIT
// Source: vhdl.grammar
// Regenerate with: grammar-tools compile-grammar vhdl.grammar
//
// This file embeds a ParserGrammar as native Rust data structures.
// Call `parser_grammar()` instead of reading and parsing the .grammar file.

use grammar_tools::parser_grammar::{GrammarElement, GrammarRule, ParserGrammar};

pub fn parser_grammar() -> ParserGrammar {
    ParserGrammar {
        rules: vec![
        GrammarRule {
            name: r#"design_file"#.to_string(),
            body: GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"design_unit"#.to_string() }) },
            line_number: 64,
        },
        GrammarRule {
            name: r#"design_unit"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"context_item"#.to_string() }) },
                GrammarElement::RuleReference { name: r#"library_unit"#.to_string() },
            ] },
            line_number: 66,
        },
        GrammarRule {
            name: r#"context_item"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"library_clause"#.to_string() },
                GrammarElement::RuleReference { name: r#"use_clause"#.to_string() },
            ] },
            line_number: 68,
        },
        GrammarRule {
            name: r#"library_clause"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"library"#.to_string() },
                GrammarElement::RuleReference { name: r#"name_list"#.to_string() },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
            ] },
            line_number: 71,
        },
        GrammarRule {
            name: r#"use_clause"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"use"#.to_string() },
                GrammarElement::RuleReference { name: r#"selected_name"#.to_string() },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
            ] },
            line_number: 74,
        },
        GrammarRule {
            name: r#"selected_name"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                                GrammarElement::Literal { value: r#"all"#.to_string() },
                            ] }) },
                    ] }) },
            ] },
            line_number: 77,
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
            line_number: 79,
        },
        GrammarRule {
            name: r#"library_unit"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"entity_declaration"#.to_string() },
                GrammarElement::RuleReference { name: r#"architecture_body"#.to_string() },
                GrammarElement::RuleReference { name: r#"package_declaration"#.to_string() },
                GrammarElement::RuleReference { name: r#"package_body"#.to_string() },
            ] },
            line_number: 81,
        },
        GrammarRule {
            name: r#"entity_declaration"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"entity"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Literal { value: r#"is"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"generic_clause"#.to_string() }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"port_clause"#.to_string() }) },
                GrammarElement::Literal { value: r#"end"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"entity"#.to_string() }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"NAME"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
            ] },
            line_number: 112,
        },
        GrammarRule {
            name: r#"generic_clause"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"generic"#.to_string() },
                GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                GrammarElement::RuleReference { name: r#"interface_list"#.to_string() },
                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
            ] },
            line_number: 117,
        },
        GrammarRule {
            name: r#"port_clause"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"port"#.to_string() },
                GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                GrammarElement::RuleReference { name: r#"interface_list"#.to_string() },
                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
            ] },
            line_number: 118,
        },
        GrammarRule {
            name: r#"interface_list"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"interface_element"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                        GrammarElement::RuleReference { name: r#"interface_element"#.to_string() },
                    ] }) },
            ] },
            line_number: 123,
        },
        GrammarRule {
            name: r#"interface_element"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"name_list"#.to_string() },
                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"mode"#.to_string() }) },
                GrammarElement::RuleReference { name: r#"subtype_indication"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"VAR_ASSIGN"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    ] }) },
            ] },
            line_number: 124,
        },
        GrammarRule {
            name: r#"mode"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Literal { value: r#"in"#.to_string() },
                GrammarElement::Literal { value: r#"out"#.to_string() },
                GrammarElement::Literal { value: r#"inout"#.to_string() },
                GrammarElement::Literal { value: r#"buffer"#.to_string() },
            ] },
            line_number: 132,
        },
        GrammarRule {
            name: r#"architecture_body"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"architecture"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Literal { value: r#"of"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Literal { value: r#"is"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"block_declarative_item"#.to_string() }) },
                GrammarElement::Literal { value: r#"begin"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"concurrent_statement"#.to_string() }) },
                GrammarElement::Literal { value: r#"end"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"architecture"#.to_string() }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"NAME"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
            ] },
            line_number: 154,
        },
        GrammarRule {
            name: r#"block_declarative_item"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"signal_declaration"#.to_string() },
                GrammarElement::RuleReference { name: r#"constant_declaration"#.to_string() },
                GrammarElement::RuleReference { name: r#"type_declaration"#.to_string() },
                GrammarElement::RuleReference { name: r#"subtype_declaration"#.to_string() },
                GrammarElement::RuleReference { name: r#"component_declaration"#.to_string() },
                GrammarElement::RuleReference { name: r#"function_declaration"#.to_string() },
                GrammarElement::RuleReference { name: r#"function_body"#.to_string() },
                GrammarElement::RuleReference { name: r#"procedure_declaration"#.to_string() },
                GrammarElement::RuleReference { name: r#"procedure_body"#.to_string() },
            ] },
            line_number: 160,
        },
        GrammarRule {
            name: r#"signal_declaration"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"signal"#.to_string() },
                GrammarElement::RuleReference { name: r#"name_list"#.to_string() },
                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                GrammarElement::RuleReference { name: r#"subtype_indication"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"VAR_ASSIGN"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    ] }) },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
            ] },
            line_number: 189,
        },
        GrammarRule {
            name: r#"constant_declaration"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"constant"#.to_string() },
                GrammarElement::RuleReference { name: r#"name_list"#.to_string() },
                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                GrammarElement::RuleReference { name: r#"subtype_indication"#.to_string() },
                GrammarElement::TokenReference { name: r#"VAR_ASSIGN"#.to_string() },
                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
            ] },
            line_number: 191,
        },
        GrammarRule {
            name: r#"variable_declaration"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"variable"#.to_string() },
                GrammarElement::RuleReference { name: r#"name_list"#.to_string() },
                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                GrammarElement::RuleReference { name: r#"subtype_indication"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"VAR_ASSIGN"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    ] }) },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
            ] },
            line_number: 193,
        },
        GrammarRule {
            name: r#"type_declaration"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"type"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Literal { value: r#"is"#.to_string() },
                GrammarElement::RuleReference { name: r#"type_definition"#.to_string() },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
            ] },
            line_number: 218,
        },
        GrammarRule {
            name: r#"subtype_declaration"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"subtype"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Literal { value: r#"is"#.to_string() },
                GrammarElement::RuleReference { name: r#"subtype_indication"#.to_string() },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
            ] },
            line_number: 219,
        },
        GrammarRule {
            name: r#"type_definition"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"enumeration_type"#.to_string() },
                GrammarElement::RuleReference { name: r#"array_type"#.to_string() },
                GrammarElement::RuleReference { name: r#"record_type"#.to_string() },
            ] },
            line_number: 221,
        },
        GrammarRule {
            name: r#"enumeration_type"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        GrammarElement::TokenReference { name: r#"CHAR_LITERAL"#.to_string() },
                    ] }) },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                                GrammarElement::TokenReference { name: r#"CHAR_LITERAL"#.to_string() },
                            ] }) },
                    ] }) },
                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
            ] },
            line_number: 227,
        },
        GrammarRule {
            name: r#"array_type"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"array"#.to_string() },
                GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                GrammarElement::RuleReference { name: r#"index_constraint"#.to_string() },
                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                GrammarElement::Literal { value: r#"of"#.to_string() },
                GrammarElement::RuleReference { name: r#"subtype_indication"#.to_string() },
            ] },
            line_number: 232,
        },
        GrammarRule {
            name: r#"index_constraint"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"discrete_range"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::RuleReference { name: r#"discrete_range"#.to_string() },
                    ] }) },
            ] },
            line_number: 234,
        },
        GrammarRule {
            name: r#"discrete_range"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"subtype_indication"#.to_string() },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::Literal { value: r#"to"#.to_string() },
                            GrammarElement::Literal { value: r#"downto"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
            ] },
            line_number: 235,
        },
        GrammarRule {
            name: r#"record_type"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"record"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                        GrammarElement::RuleReference { name: r#"subtype_indication"#.to_string() },
                        GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                    ] }) },
                GrammarElement::Literal { value: r#"end"#.to_string() },
                GrammarElement::Literal { value: r#"record"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"NAME"#.to_string() }) },
            ] },
            line_number: 239,
        },
        GrammarRule {
            name: r#"subtype_indication"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"selected_name"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"constraint"#.to_string() }) },
            ] },
            line_number: 247,
        },
        GrammarRule {
            name: r#"constraint"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::Literal { value: r#"to"#.to_string() },
                            GrammarElement::Literal { value: r#"downto"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"range"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::Literal { value: r#"to"#.to_string() },
                            GrammarElement::Literal { value: r#"downto"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
            ] },
            line_number: 249,
        },
        GrammarRule {
            name: r#"concurrent_statement"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"process_statement"#.to_string() },
                GrammarElement::RuleReference { name: r#"signal_assignment_concurrent"#.to_string() },
                GrammarElement::RuleReference { name: r#"component_instantiation"#.to_string() },
                GrammarElement::RuleReference { name: r#"generate_statement"#.to_string() },
            ] },
            line_number: 264,
        },
        GrammarRule {
            name: r#"signal_assignment_concurrent"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"LESS_EQUALS"#.to_string() },
                GrammarElement::RuleReference { name: r#"waveform"#.to_string() },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
            ] },
            line_number: 272,
        },
        GrammarRule {
            name: r#"waveform"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"waveform_element"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::RuleReference { name: r#"waveform_element"#.to_string() },
                    ] }) },
            ] },
            line_number: 274,
        },
        GrammarRule {
            name: r#"waveform_element"#.to_string(),
            body: GrammarElement::RuleReference { name: r#"expression"#.to_string() },
            line_number: 275,
        },
        GrammarRule {
            name: r#"process_statement"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    ] }) },
                GrammarElement::Literal { value: r#"process"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                        GrammarElement::RuleReference { name: r#"sensitivity_list"#.to_string() },
                        GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                    ] }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"is"#.to_string() }) },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"process_declarative_item"#.to_string() }) },
                GrammarElement::Literal { value: r#"begin"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"sequential_statement"#.to_string() }) },
                GrammarElement::Literal { value: r#"end"#.to_string() },
                GrammarElement::Literal { value: r#"process"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"NAME"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
            ] },
            line_number: 307,
        },
        GrammarRule {
            name: r#"sensitivity_list"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    ] }) },
            ] },
            line_number: 315,
        },
        GrammarRule {
            name: r#"process_declarative_item"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"variable_declaration"#.to_string() },
                GrammarElement::RuleReference { name: r#"constant_declaration"#.to_string() },
                GrammarElement::RuleReference { name: r#"type_declaration"#.to_string() },
                GrammarElement::RuleReference { name: r#"subtype_declaration"#.to_string() },
            ] },
            line_number: 317,
        },
        GrammarRule {
            name: r#"sequential_statement"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"signal_assignment_seq"#.to_string() },
                GrammarElement::RuleReference { name: r#"variable_assignment"#.to_string() },
                GrammarElement::RuleReference { name: r#"if_statement"#.to_string() },
                GrammarElement::RuleReference { name: r#"case_statement"#.to_string() },
                GrammarElement::RuleReference { name: r#"loop_statement"#.to_string() },
                GrammarElement::RuleReference { name: r#"return_statement"#.to_string() },
                GrammarElement::RuleReference { name: r#"null_statement"#.to_string() },
            ] },
            line_number: 329,
        },
        GrammarRule {
            name: r#"signal_assignment_seq"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"LESS_EQUALS"#.to_string() },
                GrammarElement::RuleReference { name: r#"waveform"#.to_string() },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
            ] },
            line_number: 342,
        },
        GrammarRule {
            name: r#"variable_assignment"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"VAR_ASSIGN"#.to_string() },
                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
            ] },
            line_number: 346,
        },
        GrammarRule {
            name: r#"if_statement"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"if"#.to_string() },
                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                GrammarElement::Literal { value: r#"then"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"sequential_statement"#.to_string() }) },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"elsif"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        GrammarElement::Literal { value: r#"then"#.to_string() },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"sequential_statement"#.to_string() }) },
                    ] }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"else"#.to_string() },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"sequential_statement"#.to_string() }) },
                    ] }) },
                GrammarElement::Literal { value: r#"end"#.to_string() },
                GrammarElement::Literal { value: r#"if"#.to_string() },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
            ] },
            line_number: 356,
        },
        GrammarRule {
            name: r#"case_statement"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"case"#.to_string() },
                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                GrammarElement::Literal { value: r#"is"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"when"#.to_string() },
                        GrammarElement::RuleReference { name: r#"choices"#.to_string() },
                        GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"sequential_statement"#.to_string() }) },
                    ] }) },
                GrammarElement::Literal { value: r#"end"#.to_string() },
                GrammarElement::Literal { value: r#"case"#.to_string() },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
            ] },
            line_number: 372,
        },
        GrammarRule {
            name: r#"choices"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"choice"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"PIPE"#.to_string() },
                        GrammarElement::RuleReference { name: r#"choice"#.to_string() },
                    ] }) },
            ] },
            line_number: 376,
        },
        GrammarRule {
            name: r#"choice"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                GrammarElement::RuleReference { name: r#"discrete_range"#.to_string() },
                GrammarElement::Literal { value: r#"others"#.to_string() },
            ] },
            line_number: 377,
        },
        GrammarRule {
            name: r#"loop_statement"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    ] }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"for"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                            GrammarElement::Literal { value: r#"in"#.to_string() },
                            GrammarElement::RuleReference { name: r#"discrete_range"#.to_string() },
                        ] },
                        GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"while"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] },
                    ] }) },
                GrammarElement::Literal { value: r#"loop"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"sequential_statement"#.to_string() }) },
                GrammarElement::Literal { value: r#"end"#.to_string() },
                GrammarElement::Literal { value: r#"loop"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"NAME"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
            ] },
            line_number: 391,
        },
        GrammarRule {
            name: r#"return_statement"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"return"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"expression"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
            ] },
            line_number: 398,
        },
        GrammarRule {
            name: r#"null_statement"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"null"#.to_string() },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
            ] },
            line_number: 399,
        },
        GrammarRule {
            name: r#"component_declaration"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"component"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"is"#.to_string() }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"generic_clause"#.to_string() }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"port_clause"#.to_string() }) },
                GrammarElement::Literal { value: r#"end"#.to_string() },
                GrammarElement::Literal { value: r#"component"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"NAME"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
            ] },
            line_number: 425,
        },
        GrammarRule {
            name: r#"component_instantiation"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"entity"#.to_string() },
                            GrammarElement::RuleReference { name: r#"selected_name"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                                ] }) },
                        ] },
                    ] }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"generic"#.to_string() },
                        GrammarElement::Literal { value: r#"map"#.to_string() },
                        GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                        GrammarElement::RuleReference { name: r#"association_list"#.to_string() },
                        GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                    ] }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"port"#.to_string() },
                        GrammarElement::Literal { value: r#"map"#.to_string() },
                        GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                        GrammarElement::RuleReference { name: r#"association_list"#.to_string() },
                        GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                    ] }) },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
            ] },
            line_number: 430,
        },
        GrammarRule {
            name: r#"association_list"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"association_element"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::RuleReference { name: r#"association_element"#.to_string() },
                    ] }) },
            ] },
            line_number: 437,
        },
        GrammarRule {
            name: r#"association_element"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                            GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                            GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"open"#.to_string() },
                ] },
            ] },
            line_number: 438,
        },
        GrammarRule {
            name: r#"generate_statement"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::RuleReference { name: r#"for_generate"#.to_string() },
                        GrammarElement::RuleReference { name: r#"if_generate"#.to_string() },
                    ] }) },
            ] },
            line_number: 461,
        },
        GrammarRule {
            name: r#"for_generate"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"for"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Literal { value: r#"in"#.to_string() },
                GrammarElement::RuleReference { name: r#"discrete_range"#.to_string() },
                GrammarElement::Literal { value: r#"generate"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"concurrent_statement"#.to_string() }) },
                GrammarElement::Literal { value: r#"end"#.to_string() },
                GrammarElement::Literal { value: r#"generate"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"NAME"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
            ] },
            line_number: 463,
        },
        GrammarRule {
            name: r#"if_generate"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"if"#.to_string() },
                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                GrammarElement::Literal { value: r#"generate"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"concurrent_statement"#.to_string() }) },
                GrammarElement::Literal { value: r#"end"#.to_string() },
                GrammarElement::Literal { value: r#"generate"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"NAME"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
            ] },
            line_number: 467,
        },
        GrammarRule {
            name: r#"package_declaration"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"package"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Literal { value: r#"is"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"package_declarative_item"#.to_string() }) },
                GrammarElement::Literal { value: r#"end"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"package"#.to_string() }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"NAME"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
            ] },
            line_number: 488,
        },
        GrammarRule {
            name: r#"package_body"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"package"#.to_string() },
                GrammarElement::Literal { value: r#"body"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Literal { value: r#"is"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"package_body_declarative_item"#.to_string() }) },
                GrammarElement::Literal { value: r#"end"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"package"#.to_string() },
                        GrammarElement::Literal { value: r#"body"#.to_string() },
                    ] }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"NAME"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
            ] },
            line_number: 492,
        },
        GrammarRule {
            name: r#"package_declarative_item"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"type_declaration"#.to_string() },
                GrammarElement::RuleReference { name: r#"subtype_declaration"#.to_string() },
                GrammarElement::RuleReference { name: r#"constant_declaration"#.to_string() },
                GrammarElement::RuleReference { name: r#"signal_declaration"#.to_string() },
                GrammarElement::RuleReference { name: r#"component_declaration"#.to_string() },
                GrammarElement::RuleReference { name: r#"function_declaration"#.to_string() },
                GrammarElement::RuleReference { name: r#"procedure_declaration"#.to_string() },
            ] },
            line_number: 496,
        },
        GrammarRule {
            name: r#"package_body_declarative_item"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"type_declaration"#.to_string() },
                GrammarElement::RuleReference { name: r#"subtype_declaration"#.to_string() },
                GrammarElement::RuleReference { name: r#"constant_declaration"#.to_string() },
                GrammarElement::RuleReference { name: r#"function_body"#.to_string() },
                GrammarElement::RuleReference { name: r#"procedure_body"#.to_string() },
            ] },
            line_number: 504,
        },
        GrammarRule {
            name: r#"function_declaration"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Optional { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::Literal { value: r#"pure"#.to_string() },
                        GrammarElement::Literal { value: r#"impure"#.to_string() },
                    ] }) },
                GrammarElement::Literal { value: r#"function"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                        GrammarElement::RuleReference { name: r#"interface_list"#.to_string() },
                        GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                    ] }) },
                GrammarElement::Literal { value: r#"return"#.to_string() },
                GrammarElement::RuleReference { name: r#"subtype_indication"#.to_string() },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
            ] },
            line_number: 520,
        },
        GrammarRule {
            name: r#"function_body"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Optional { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::Literal { value: r#"pure"#.to_string() },
                        GrammarElement::Literal { value: r#"impure"#.to_string() },
                    ] }) },
                GrammarElement::Literal { value: r#"function"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                        GrammarElement::RuleReference { name: r#"interface_list"#.to_string() },
                        GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                    ] }) },
                GrammarElement::Literal { value: r#"return"#.to_string() },
                GrammarElement::RuleReference { name: r#"subtype_indication"#.to_string() },
                GrammarElement::Literal { value: r#"is"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"process_declarative_item"#.to_string() }) },
                GrammarElement::Literal { value: r#"begin"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"sequential_statement"#.to_string() }) },
                GrammarElement::Literal { value: r#"end"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"function"#.to_string() }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"NAME"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
            ] },
            line_number: 525,
        },
        GrammarRule {
            name: r#"procedure_declaration"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"procedure"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                        GrammarElement::RuleReference { name: r#"interface_list"#.to_string() },
                        GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                    ] }) },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
            ] },
            line_number: 534,
        },
        GrammarRule {
            name: r#"procedure_body"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"procedure"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                        GrammarElement::RuleReference { name: r#"interface_list"#.to_string() },
                        GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                    ] }) },
                GrammarElement::Literal { value: r#"is"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"process_declarative_item"#.to_string() }) },
                GrammarElement::Literal { value: r#"begin"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"sequential_statement"#.to_string() }) },
                GrammarElement::Literal { value: r#"end"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"procedure"#.to_string() }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"NAME"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
            ] },
            line_number: 537,
        },
        GrammarRule {
            name: r#"expression"#.to_string(),
            body: GrammarElement::RuleReference { name: r#"logical_expr"#.to_string() },
            line_number: 574,
        },
        GrammarRule {
            name: r#"logical_expr"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"relation"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"logical_op"#.to_string() },
                        GrammarElement::RuleReference { name: r#"relation"#.to_string() },
                    ] }) },
            ] },
            line_number: 581,
        },
        GrammarRule {
            name: r#"logical_op"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Literal { value: r#"and"#.to_string() },
                GrammarElement::Literal { value: r#"or"#.to_string() },
                GrammarElement::Literal { value: r#"xor"#.to_string() },
                GrammarElement::Literal { value: r#"nand"#.to_string() },
                GrammarElement::Literal { value: r#"nor"#.to_string() },
                GrammarElement::Literal { value: r#"xnor"#.to_string() },
            ] },
            line_number: 582,
        },
        GrammarRule {
            name: r#"relation"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"shift_expr"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"relational_op"#.to_string() },
                        GrammarElement::RuleReference { name: r#"shift_expr"#.to_string() },
                    ] }) },
            ] },
            line_number: 586,
        },
        GrammarRule {
            name: r#"relational_op"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                GrammarElement::TokenReference { name: r#"NOT_EQUALS"#.to_string() },
                GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                GrammarElement::TokenReference { name: r#"LESS_EQUALS"#.to_string() },
                GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                GrammarElement::TokenReference { name: r#"GREATER_EQUALS"#.to_string() },
            ] },
            line_number: 587,
        },
        GrammarRule {
            name: r#"shift_expr"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"adding_expr"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"shift_op"#.to_string() },
                        GrammarElement::RuleReference { name: r#"adding_expr"#.to_string() },
                    ] }) },
            ] },
            line_number: 592,
        },
        GrammarRule {
            name: r#"shift_op"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Literal { value: r#"sll"#.to_string() },
                GrammarElement::Literal { value: r#"srl"#.to_string() },
                GrammarElement::Literal { value: r#"sla"#.to_string() },
                GrammarElement::Literal { value: r#"sra"#.to_string() },
                GrammarElement::Literal { value: r#"rol"#.to_string() },
                GrammarElement::Literal { value: r#"ror"#.to_string() },
            ] },
            line_number: 593,
        },
        GrammarRule {
            name: r#"adding_expr"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"multiplying_expr"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"adding_op"#.to_string() },
                        GrammarElement::RuleReference { name: r#"multiplying_expr"#.to_string() },
                    ] }) },
            ] },
            line_number: 597,
        },
        GrammarRule {
            name: r#"adding_op"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                GrammarElement::TokenReference { name: r#"AMPERSAND"#.to_string() },
            ] },
            line_number: 598,
        },
        GrammarRule {
            name: r#"multiplying_expr"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"unary_expr"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"multiplying_op"#.to_string() },
                        GrammarElement::RuleReference { name: r#"unary_expr"#.to_string() },
                    ] }) },
            ] },
            line_number: 601,
        },
        GrammarRule {
            name: r#"multiplying_op"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                GrammarElement::TokenReference { name: r#"SLASH"#.to_string() },
                GrammarElement::Literal { value: r#"mod"#.to_string() },
                GrammarElement::Literal { value: r#"rem"#.to_string() },
            ] },
            line_number: 602,
        },
        GrammarRule {
            name: r#"unary_expr"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"abs"#.to_string() },
                    GrammarElement::RuleReference { name: r#"unary_expr"#.to_string() },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"not"#.to_string() },
                    GrammarElement::RuleReference { name: r#"unary_expr"#.to_string() },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                            GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"unary_expr"#.to_string() },
                ] },
                GrammarElement::RuleReference { name: r#"power_expr"#.to_string() },
            ] },
            line_number: 605,
        },
        GrammarRule {
            name: r#"power_expr"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"primary"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"POWER"#.to_string() },
                        GrammarElement::RuleReference { name: r#"primary"#.to_string() },
                    ] }) },
            ] },
            line_number: 611,
        },
        GrammarRule {
            name: r#"primary"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                GrammarElement::TokenReference { name: r#"REAL_NUMBER"#.to_string() },
                GrammarElement::TokenReference { name: r#"BASED_LITERAL"#.to_string() },
                GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                GrammarElement::TokenReference { name: r#"CHAR_LITERAL"#.to_string() },
                GrammarElement::TokenReference { name: r#"BIT_STRING"#.to_string() },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"TICK"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
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
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                GrammarElement::RuleReference { name: r#"aggregate"#.to_string() },
                GrammarElement::Literal { value: r#"null"#.to_string() },
            ] },
            line_number: 619,
        },
        GrammarRule {
            name: r#"aggregate"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                GrammarElement::RuleReference { name: r#"element_association"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::RuleReference { name: r#"element_association"#.to_string() },
                    ] }) },
                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
            ] },
            line_number: 635,
        },
        GrammarRule {
            name: r#"element_association"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"choices"#.to_string() },
                        GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                    ] }) },
                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
            ] },
            line_number: 636,
        },
    ],
        version: 0,
    }
}
