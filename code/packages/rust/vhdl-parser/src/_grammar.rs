// AUTO-GENERATED FILE - DO NOT EDIT
// Source family: vhdl
// Regenerate with: grammar-tools generate-rust-compiled-grammars vhdl
//
// This file embeds versioned ParserGrammar values as native Rust data structures.
// Call `parser_grammar` instead of reading and parsing grammar files at runtime.

use grammar_tools::parser_grammar::ParserGrammar;

pub const SUPPORTED_VERSIONS: &[&str] = &[
    "1987",
    "1993",
    "2002",
    "2008",
    "2019",
];

pub fn parser_grammar(version: &str) -> Option<ParserGrammar> {
    match version {
        "1987" => Some(v_1987::parser_grammar()),
        "1993" => Some(v_1993::parser_grammar()),
        "2002" => Some(v_2002::parser_grammar()),
        "2008" => Some(v_2008::parser_grammar()),
        "2019" => Some(v_2019::parser_grammar()),
        _ => None,
    }
}

mod v_1987 {
    // AUTO-GENERATED FILE — DO NOT EDIT
    // Source: vhdl1987.grammar
    // Regenerate with: grammar-tools compile-grammar vhdl1987.grammar
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
                line_number: 67,
            },
            GrammarRule {
                name: r#"design_unit"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"context_item"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"library_unit"#.to_string() },
                ] },
                line_number: 69,
            },
            GrammarRule {
                name: r#"context_item"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"library_clause"#.to_string() },
                    GrammarElement::RuleReference { name: r#"use_clause"#.to_string() },
                ] },
                line_number: 71,
            },
            GrammarRule {
                name: r#"library_clause"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"library"#.to_string() },
                    GrammarElement::RuleReference { name: r#"name_list"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                ] },
                line_number: 74,
            },
            GrammarRule {
                name: r#"use_clause"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"use"#.to_string() },
                    GrammarElement::RuleReference { name: r#"selected_name"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                ] },
                line_number: 77,
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
                line_number: 80,
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
                line_number: 82,
            },
            GrammarRule {
                name: r#"library_unit"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"entity_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"architecture_body"#.to_string() },
                    GrammarElement::RuleReference { name: r#"package_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"package_body"#.to_string() },
                ] },
                line_number: 84,
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
                line_number: 115,
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
                line_number: 120,
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
                line_number: 121,
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
                line_number: 126,
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
                line_number: 127,
            },
            GrammarRule {
                name: r#"mode"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"in"#.to_string() },
                    GrammarElement::Literal { value: r#"out"#.to_string() },
                    GrammarElement::Literal { value: r#"inout"#.to_string() },
                    GrammarElement::Literal { value: r#"buffer"#.to_string() },
                ] },
                line_number: 135,
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
                line_number: 157,
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
                line_number: 163,
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
                line_number: 192,
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
                line_number: 194,
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
                line_number: 196,
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
                line_number: 221,
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
                line_number: 222,
            },
            GrammarRule {
                name: r#"type_definition"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"enumeration_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"array_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_type"#.to_string() },
                ] },
                line_number: 224,
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
                line_number: 230,
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
                line_number: 235,
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
                line_number: 237,
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
                line_number: 238,
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
                line_number: 242,
            },
            GrammarRule {
                name: r#"subtype_indication"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"selected_name"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"constraint"#.to_string() }) },
                ] },
                line_number: 250,
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
                line_number: 252,
            },
            GrammarRule {
                name: r#"concurrent_statement"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"process_statement"#.to_string() },
                    GrammarElement::RuleReference { name: r#"signal_assignment_concurrent"#.to_string() },
                    GrammarElement::RuleReference { name: r#"component_instantiation"#.to_string() },
                    GrammarElement::RuleReference { name: r#"generate_statement"#.to_string() },
                ] },
                line_number: 267,
            },
            GrammarRule {
                name: r#"signal_assignment_concurrent"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"waveform"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                ] },
                line_number: 275,
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
                line_number: 277,
            },
            GrammarRule {
                name: r#"waveform_element"#.to_string(),
                body: GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                line_number: 278,
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
                line_number: 310,
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
                line_number: 318,
            },
            GrammarRule {
                name: r#"process_declarative_item"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"variable_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"constant_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"subtype_declaration"#.to_string() },
                ] },
                line_number: 320,
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
                line_number: 332,
            },
            GrammarRule {
                name: r#"signal_assignment_seq"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"waveform"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                ] },
                line_number: 345,
            },
            GrammarRule {
                name: r#"variable_assignment"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"VAR_ASSIGN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                ] },
                line_number: 349,
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
                line_number: 359,
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
                line_number: 375,
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
                line_number: 379,
            },
            GrammarRule {
                name: r#"choice"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"discrete_range"#.to_string() },
                    GrammarElement::Literal { value: r#"others"#.to_string() },
                ] },
                line_number: 380,
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
                line_number: 394,
            },
            GrammarRule {
                name: r#"return_statement"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"return"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"expression"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                ] },
                line_number: 401,
            },
            GrammarRule {
                name: r#"null_statement"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"null"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                ] },
                line_number: 402,
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
                line_number: 428,
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
                line_number: 433,
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
                line_number: 440,
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
                line_number: 441,
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
                line_number: 464,
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
                line_number: 466,
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
                line_number: 470,
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
                line_number: 491,
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
                line_number: 495,
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
                line_number: 499,
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
                line_number: 507,
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
                line_number: 523,
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
                line_number: 528,
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
                line_number: 537,
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
                line_number: 540,
            },
            GrammarRule {
                name: r#"expression"#.to_string(),
                body: GrammarElement::RuleReference { name: r#"logical_expr"#.to_string() },
                line_number: 577,
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
                line_number: 584,
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
                line_number: 585,
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
                line_number: 589,
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
                line_number: 590,
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
                line_number: 595,
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
                line_number: 596,
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
                line_number: 600,
            },
            GrammarRule {
                name: r#"adding_op"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"AMPERSAND"#.to_string() },
                ] },
                line_number: 601,
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
                line_number: 604,
            },
            GrammarRule {
                name: r#"multiplying_op"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SLASH"#.to_string() },
                    GrammarElement::Literal { value: r#"mod"#.to_string() },
                    GrammarElement::Literal { value: r#"rem"#.to_string() },
                ] },
                line_number: 605,
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
                line_number: 608,
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
                line_number: 614,
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
                line_number: 622,
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
                line_number: 638,
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
                line_number: 639,
            },
        ],
            version: 0,
        }
    }
}

mod v_1993 {
    // AUTO-GENERATED FILE — DO NOT EDIT
    // Source: vhdl1993.grammar
    // Regenerate with: grammar-tools compile-grammar vhdl1993.grammar
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
                line_number: 67,
            },
            GrammarRule {
                name: r#"design_unit"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"context_item"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"library_unit"#.to_string() },
                ] },
                line_number: 69,
            },
            GrammarRule {
                name: r#"context_item"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"library_clause"#.to_string() },
                    GrammarElement::RuleReference { name: r#"use_clause"#.to_string() },
                ] },
                line_number: 71,
            },
            GrammarRule {
                name: r#"library_clause"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"library"#.to_string() },
                    GrammarElement::RuleReference { name: r#"name_list"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                ] },
                line_number: 74,
            },
            GrammarRule {
                name: r#"use_clause"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"use"#.to_string() },
                    GrammarElement::RuleReference { name: r#"selected_name"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                ] },
                line_number: 77,
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
                line_number: 80,
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
                line_number: 82,
            },
            GrammarRule {
                name: r#"library_unit"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"entity_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"architecture_body"#.to_string() },
                    GrammarElement::RuleReference { name: r#"package_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"package_body"#.to_string() },
                ] },
                line_number: 84,
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
                line_number: 115,
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
                line_number: 120,
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
                line_number: 121,
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
                line_number: 126,
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
                line_number: 127,
            },
            GrammarRule {
                name: r#"mode"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"in"#.to_string() },
                    GrammarElement::Literal { value: r#"out"#.to_string() },
                    GrammarElement::Literal { value: r#"inout"#.to_string() },
                    GrammarElement::Literal { value: r#"buffer"#.to_string() },
                ] },
                line_number: 135,
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
                line_number: 157,
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
                line_number: 163,
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
                line_number: 192,
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
                line_number: 194,
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
                line_number: 196,
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
                line_number: 221,
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
                line_number: 222,
            },
            GrammarRule {
                name: r#"type_definition"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"enumeration_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"array_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_type"#.to_string() },
                ] },
                line_number: 224,
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
                line_number: 230,
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
                line_number: 235,
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
                line_number: 237,
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
                line_number: 238,
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
                line_number: 242,
            },
            GrammarRule {
                name: r#"subtype_indication"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"selected_name"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"constraint"#.to_string() }) },
                ] },
                line_number: 250,
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
                line_number: 252,
            },
            GrammarRule {
                name: r#"concurrent_statement"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"process_statement"#.to_string() },
                    GrammarElement::RuleReference { name: r#"signal_assignment_concurrent"#.to_string() },
                    GrammarElement::RuleReference { name: r#"component_instantiation"#.to_string() },
                    GrammarElement::RuleReference { name: r#"generate_statement"#.to_string() },
                ] },
                line_number: 267,
            },
            GrammarRule {
                name: r#"signal_assignment_concurrent"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"waveform"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                ] },
                line_number: 275,
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
                line_number: 277,
            },
            GrammarRule {
                name: r#"waveform_element"#.to_string(),
                body: GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                line_number: 278,
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
                line_number: 310,
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
                line_number: 318,
            },
            GrammarRule {
                name: r#"process_declarative_item"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"variable_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"constant_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"subtype_declaration"#.to_string() },
                ] },
                line_number: 320,
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
                line_number: 332,
            },
            GrammarRule {
                name: r#"signal_assignment_seq"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"waveform"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                ] },
                line_number: 345,
            },
            GrammarRule {
                name: r#"variable_assignment"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"VAR_ASSIGN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                ] },
                line_number: 349,
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
                line_number: 359,
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
                line_number: 375,
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
                line_number: 379,
            },
            GrammarRule {
                name: r#"choice"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"discrete_range"#.to_string() },
                    GrammarElement::Literal { value: r#"others"#.to_string() },
                ] },
                line_number: 380,
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
                line_number: 394,
            },
            GrammarRule {
                name: r#"return_statement"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"return"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"expression"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                ] },
                line_number: 401,
            },
            GrammarRule {
                name: r#"null_statement"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"null"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                ] },
                line_number: 402,
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
                line_number: 428,
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
                line_number: 433,
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
                line_number: 440,
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
                line_number: 441,
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
                line_number: 464,
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
                line_number: 466,
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
                line_number: 470,
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
                line_number: 491,
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
                line_number: 495,
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
                line_number: 499,
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
                line_number: 507,
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
                line_number: 523,
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
                line_number: 528,
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
                line_number: 537,
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
                line_number: 540,
            },
            GrammarRule {
                name: r#"expression"#.to_string(),
                body: GrammarElement::RuleReference { name: r#"logical_expr"#.to_string() },
                line_number: 577,
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
                line_number: 584,
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
                line_number: 585,
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
                line_number: 589,
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
                line_number: 590,
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
                line_number: 595,
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
                line_number: 596,
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
                line_number: 600,
            },
            GrammarRule {
                name: r#"adding_op"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"AMPERSAND"#.to_string() },
                ] },
                line_number: 601,
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
                line_number: 604,
            },
            GrammarRule {
                name: r#"multiplying_op"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SLASH"#.to_string() },
                    GrammarElement::Literal { value: r#"mod"#.to_string() },
                    GrammarElement::Literal { value: r#"rem"#.to_string() },
                ] },
                line_number: 605,
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
                line_number: 608,
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
                line_number: 614,
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
                line_number: 622,
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
                line_number: 638,
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
                line_number: 639,
            },
        ],
            version: 0,
        }
    }
}

mod v_2002 {
    // AUTO-GENERATED FILE — DO NOT EDIT
    // Source: vhdl2002.grammar
    // Regenerate with: grammar-tools compile-grammar vhdl2002.grammar
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
                line_number: 67,
            },
            GrammarRule {
                name: r#"design_unit"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"context_item"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"library_unit"#.to_string() },
                ] },
                line_number: 69,
            },
            GrammarRule {
                name: r#"context_item"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"library_clause"#.to_string() },
                    GrammarElement::RuleReference { name: r#"use_clause"#.to_string() },
                ] },
                line_number: 71,
            },
            GrammarRule {
                name: r#"library_clause"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"library"#.to_string() },
                    GrammarElement::RuleReference { name: r#"name_list"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                ] },
                line_number: 74,
            },
            GrammarRule {
                name: r#"use_clause"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"use"#.to_string() },
                    GrammarElement::RuleReference { name: r#"selected_name"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                ] },
                line_number: 77,
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
                line_number: 80,
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
                line_number: 82,
            },
            GrammarRule {
                name: r#"library_unit"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"entity_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"architecture_body"#.to_string() },
                    GrammarElement::RuleReference { name: r#"package_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"package_body"#.to_string() },
                ] },
                line_number: 84,
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
                line_number: 115,
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
                line_number: 120,
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
                line_number: 121,
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
                line_number: 126,
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
                line_number: 127,
            },
            GrammarRule {
                name: r#"mode"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"in"#.to_string() },
                    GrammarElement::Literal { value: r#"out"#.to_string() },
                    GrammarElement::Literal { value: r#"inout"#.to_string() },
                    GrammarElement::Literal { value: r#"buffer"#.to_string() },
                ] },
                line_number: 135,
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
                line_number: 157,
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
                line_number: 163,
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
                line_number: 192,
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
                line_number: 194,
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
                line_number: 196,
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
                line_number: 221,
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
                line_number: 222,
            },
            GrammarRule {
                name: r#"type_definition"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"enumeration_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"array_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_type"#.to_string() },
                ] },
                line_number: 224,
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
                line_number: 230,
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
                line_number: 235,
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
                line_number: 237,
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
                line_number: 238,
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
                line_number: 242,
            },
            GrammarRule {
                name: r#"subtype_indication"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"selected_name"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"constraint"#.to_string() }) },
                ] },
                line_number: 250,
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
                line_number: 252,
            },
            GrammarRule {
                name: r#"concurrent_statement"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"process_statement"#.to_string() },
                    GrammarElement::RuleReference { name: r#"signal_assignment_concurrent"#.to_string() },
                    GrammarElement::RuleReference { name: r#"component_instantiation"#.to_string() },
                    GrammarElement::RuleReference { name: r#"generate_statement"#.to_string() },
                ] },
                line_number: 267,
            },
            GrammarRule {
                name: r#"signal_assignment_concurrent"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"waveform"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                ] },
                line_number: 275,
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
                line_number: 277,
            },
            GrammarRule {
                name: r#"waveform_element"#.to_string(),
                body: GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                line_number: 278,
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
                line_number: 310,
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
                line_number: 318,
            },
            GrammarRule {
                name: r#"process_declarative_item"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"variable_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"constant_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"subtype_declaration"#.to_string() },
                ] },
                line_number: 320,
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
                line_number: 332,
            },
            GrammarRule {
                name: r#"signal_assignment_seq"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"waveform"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                ] },
                line_number: 345,
            },
            GrammarRule {
                name: r#"variable_assignment"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"VAR_ASSIGN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                ] },
                line_number: 349,
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
                line_number: 359,
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
                line_number: 375,
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
                line_number: 379,
            },
            GrammarRule {
                name: r#"choice"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"discrete_range"#.to_string() },
                    GrammarElement::Literal { value: r#"others"#.to_string() },
                ] },
                line_number: 380,
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
                line_number: 394,
            },
            GrammarRule {
                name: r#"return_statement"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"return"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"expression"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                ] },
                line_number: 401,
            },
            GrammarRule {
                name: r#"null_statement"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"null"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                ] },
                line_number: 402,
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
                line_number: 428,
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
                line_number: 433,
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
                line_number: 440,
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
                line_number: 441,
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
                line_number: 464,
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
                line_number: 466,
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
                line_number: 470,
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
                line_number: 491,
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
                line_number: 495,
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
                line_number: 499,
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
                line_number: 507,
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
                line_number: 523,
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
                line_number: 528,
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
                line_number: 537,
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
                line_number: 540,
            },
            GrammarRule {
                name: r#"expression"#.to_string(),
                body: GrammarElement::RuleReference { name: r#"logical_expr"#.to_string() },
                line_number: 577,
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
                line_number: 584,
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
                line_number: 585,
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
                line_number: 589,
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
                line_number: 590,
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
                line_number: 595,
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
                line_number: 596,
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
                line_number: 600,
            },
            GrammarRule {
                name: r#"adding_op"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"AMPERSAND"#.to_string() },
                ] },
                line_number: 601,
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
                line_number: 604,
            },
            GrammarRule {
                name: r#"multiplying_op"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SLASH"#.to_string() },
                    GrammarElement::Literal { value: r#"mod"#.to_string() },
                    GrammarElement::Literal { value: r#"rem"#.to_string() },
                ] },
                line_number: 605,
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
                line_number: 608,
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
                line_number: 614,
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
                line_number: 622,
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
                line_number: 638,
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
                line_number: 639,
            },
        ],
            version: 0,
        }
    }
}

mod v_2008 {
    // AUTO-GENERATED FILE — DO NOT EDIT
    // Source: vhdl2008.grammar
    // Regenerate with: grammar-tools compile-grammar vhdl2008.grammar
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
                line_number: 67,
            },
            GrammarRule {
                name: r#"design_unit"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"context_item"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"library_unit"#.to_string() },
                ] },
                line_number: 69,
            },
            GrammarRule {
                name: r#"context_item"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"library_clause"#.to_string() },
                    GrammarElement::RuleReference { name: r#"use_clause"#.to_string() },
                ] },
                line_number: 71,
            },
            GrammarRule {
                name: r#"library_clause"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"library"#.to_string() },
                    GrammarElement::RuleReference { name: r#"name_list"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                ] },
                line_number: 74,
            },
            GrammarRule {
                name: r#"use_clause"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"use"#.to_string() },
                    GrammarElement::RuleReference { name: r#"selected_name"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                ] },
                line_number: 77,
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
                line_number: 80,
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
                line_number: 82,
            },
            GrammarRule {
                name: r#"library_unit"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"entity_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"architecture_body"#.to_string() },
                    GrammarElement::RuleReference { name: r#"package_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"package_body"#.to_string() },
                ] },
                line_number: 84,
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
                line_number: 115,
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
                line_number: 120,
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
                line_number: 121,
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
                line_number: 126,
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
                line_number: 127,
            },
            GrammarRule {
                name: r#"mode"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"in"#.to_string() },
                    GrammarElement::Literal { value: r#"out"#.to_string() },
                    GrammarElement::Literal { value: r#"inout"#.to_string() },
                    GrammarElement::Literal { value: r#"buffer"#.to_string() },
                ] },
                line_number: 135,
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
                line_number: 157,
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
                line_number: 163,
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
                line_number: 192,
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
                line_number: 194,
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
                line_number: 196,
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
                line_number: 221,
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
                line_number: 222,
            },
            GrammarRule {
                name: r#"type_definition"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"enumeration_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"array_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_type"#.to_string() },
                ] },
                line_number: 224,
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
                line_number: 230,
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
                line_number: 235,
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
                line_number: 237,
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
                line_number: 238,
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
                line_number: 242,
            },
            GrammarRule {
                name: r#"subtype_indication"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"selected_name"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"constraint"#.to_string() }) },
                ] },
                line_number: 250,
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
                line_number: 252,
            },
            GrammarRule {
                name: r#"concurrent_statement"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"process_statement"#.to_string() },
                    GrammarElement::RuleReference { name: r#"signal_assignment_concurrent"#.to_string() },
                    GrammarElement::RuleReference { name: r#"component_instantiation"#.to_string() },
                    GrammarElement::RuleReference { name: r#"generate_statement"#.to_string() },
                ] },
                line_number: 267,
            },
            GrammarRule {
                name: r#"signal_assignment_concurrent"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"waveform"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                ] },
                line_number: 275,
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
                line_number: 277,
            },
            GrammarRule {
                name: r#"waveform_element"#.to_string(),
                body: GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                line_number: 278,
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
                line_number: 310,
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
                line_number: 318,
            },
            GrammarRule {
                name: r#"process_declarative_item"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"variable_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"constant_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"subtype_declaration"#.to_string() },
                ] },
                line_number: 320,
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
                line_number: 332,
            },
            GrammarRule {
                name: r#"signal_assignment_seq"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"waveform"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                ] },
                line_number: 345,
            },
            GrammarRule {
                name: r#"variable_assignment"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"VAR_ASSIGN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                ] },
                line_number: 349,
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
                line_number: 359,
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
                line_number: 375,
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
                line_number: 379,
            },
            GrammarRule {
                name: r#"choice"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"discrete_range"#.to_string() },
                    GrammarElement::Literal { value: r#"others"#.to_string() },
                ] },
                line_number: 380,
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
                line_number: 394,
            },
            GrammarRule {
                name: r#"return_statement"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"return"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"expression"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                ] },
                line_number: 401,
            },
            GrammarRule {
                name: r#"null_statement"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"null"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                ] },
                line_number: 402,
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
                line_number: 428,
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
                line_number: 433,
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
                line_number: 440,
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
                line_number: 441,
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
                line_number: 464,
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
                line_number: 466,
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
                line_number: 470,
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
                line_number: 491,
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
                line_number: 495,
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
                line_number: 499,
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
                line_number: 507,
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
                line_number: 523,
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
                line_number: 528,
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
                line_number: 537,
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
                line_number: 540,
            },
            GrammarRule {
                name: r#"expression"#.to_string(),
                body: GrammarElement::RuleReference { name: r#"logical_expr"#.to_string() },
                line_number: 577,
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
                line_number: 584,
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
                line_number: 585,
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
                line_number: 589,
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
                line_number: 590,
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
                line_number: 595,
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
                line_number: 596,
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
                line_number: 600,
            },
            GrammarRule {
                name: r#"adding_op"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"AMPERSAND"#.to_string() },
                ] },
                line_number: 601,
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
                line_number: 604,
            },
            GrammarRule {
                name: r#"multiplying_op"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SLASH"#.to_string() },
                    GrammarElement::Literal { value: r#"mod"#.to_string() },
                    GrammarElement::Literal { value: r#"rem"#.to_string() },
                ] },
                line_number: 605,
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
                line_number: 608,
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
                line_number: 614,
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
                line_number: 622,
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
                line_number: 638,
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
                line_number: 639,
            },
        ],
            version: 0,
        }
    }
}

mod v_2019 {
    // AUTO-GENERATED FILE — DO NOT EDIT
    // Source: vhdl2019.grammar
    // Regenerate with: grammar-tools compile-grammar vhdl2019.grammar
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
                line_number: 67,
            },
            GrammarRule {
                name: r#"design_unit"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"context_item"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"library_unit"#.to_string() },
                ] },
                line_number: 69,
            },
            GrammarRule {
                name: r#"context_item"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"library_clause"#.to_string() },
                    GrammarElement::RuleReference { name: r#"use_clause"#.to_string() },
                ] },
                line_number: 71,
            },
            GrammarRule {
                name: r#"library_clause"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"library"#.to_string() },
                    GrammarElement::RuleReference { name: r#"name_list"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                ] },
                line_number: 74,
            },
            GrammarRule {
                name: r#"use_clause"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"use"#.to_string() },
                    GrammarElement::RuleReference { name: r#"selected_name"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                ] },
                line_number: 77,
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
                line_number: 80,
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
                line_number: 82,
            },
            GrammarRule {
                name: r#"library_unit"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"entity_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"architecture_body"#.to_string() },
                    GrammarElement::RuleReference { name: r#"package_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"package_body"#.to_string() },
                ] },
                line_number: 84,
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
                line_number: 115,
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
                line_number: 120,
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
                line_number: 121,
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
                line_number: 126,
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
                line_number: 127,
            },
            GrammarRule {
                name: r#"mode"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"in"#.to_string() },
                    GrammarElement::Literal { value: r#"out"#.to_string() },
                    GrammarElement::Literal { value: r#"inout"#.to_string() },
                    GrammarElement::Literal { value: r#"buffer"#.to_string() },
                ] },
                line_number: 135,
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
                line_number: 157,
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
                line_number: 163,
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
                line_number: 192,
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
                line_number: 194,
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
                line_number: 196,
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
                line_number: 221,
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
                line_number: 222,
            },
            GrammarRule {
                name: r#"type_definition"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"enumeration_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"array_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_type"#.to_string() },
                ] },
                line_number: 224,
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
                line_number: 230,
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
                line_number: 235,
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
                line_number: 237,
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
                line_number: 238,
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
                line_number: 242,
            },
            GrammarRule {
                name: r#"subtype_indication"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"selected_name"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"constraint"#.to_string() }) },
                ] },
                line_number: 250,
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
                line_number: 252,
            },
            GrammarRule {
                name: r#"concurrent_statement"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"process_statement"#.to_string() },
                    GrammarElement::RuleReference { name: r#"signal_assignment_concurrent"#.to_string() },
                    GrammarElement::RuleReference { name: r#"component_instantiation"#.to_string() },
                    GrammarElement::RuleReference { name: r#"generate_statement"#.to_string() },
                ] },
                line_number: 267,
            },
            GrammarRule {
                name: r#"signal_assignment_concurrent"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"waveform"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                ] },
                line_number: 275,
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
                line_number: 277,
            },
            GrammarRule {
                name: r#"waveform_element"#.to_string(),
                body: GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                line_number: 278,
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
                line_number: 310,
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
                line_number: 318,
            },
            GrammarRule {
                name: r#"process_declarative_item"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"variable_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"constant_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"subtype_declaration"#.to_string() },
                ] },
                line_number: 320,
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
                line_number: 332,
            },
            GrammarRule {
                name: r#"signal_assignment_seq"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"waveform"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                ] },
                line_number: 345,
            },
            GrammarRule {
                name: r#"variable_assignment"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"VAR_ASSIGN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                ] },
                line_number: 349,
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
                line_number: 359,
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
                line_number: 375,
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
                line_number: 379,
            },
            GrammarRule {
                name: r#"choice"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"discrete_range"#.to_string() },
                    GrammarElement::Literal { value: r#"others"#.to_string() },
                ] },
                line_number: 380,
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
                line_number: 394,
            },
            GrammarRule {
                name: r#"return_statement"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"return"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"expression"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                ] },
                line_number: 401,
            },
            GrammarRule {
                name: r#"null_statement"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"null"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                ] },
                line_number: 402,
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
                line_number: 428,
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
                line_number: 433,
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
                line_number: 440,
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
                line_number: 441,
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
                line_number: 464,
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
                line_number: 466,
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
                line_number: 470,
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
                line_number: 491,
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
                line_number: 495,
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
                line_number: 499,
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
                line_number: 507,
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
                line_number: 523,
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
                line_number: 528,
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
                line_number: 537,
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
                line_number: 540,
            },
            GrammarRule {
                name: r#"expression"#.to_string(),
                body: GrammarElement::RuleReference { name: r#"logical_expr"#.to_string() },
                line_number: 577,
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
                line_number: 584,
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
                line_number: 585,
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
                line_number: 589,
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
                line_number: 590,
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
                line_number: 595,
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
                line_number: 596,
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
                line_number: 600,
            },
            GrammarRule {
                name: r#"adding_op"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"AMPERSAND"#.to_string() },
                ] },
                line_number: 601,
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
                line_number: 604,
            },
            GrammarRule {
                name: r#"multiplying_op"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SLASH"#.to_string() },
                    GrammarElement::Literal { value: r#"mod"#.to_string() },
                    GrammarElement::Literal { value: r#"rem"#.to_string() },
                ] },
                line_number: 605,
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
                line_number: 608,
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
                line_number: 614,
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
                line_number: 622,
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
                line_number: 638,
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
                line_number: 639,
            },
        ],
            version: 0,
        }
    }
}

