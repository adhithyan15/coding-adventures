// AUTO-GENERATED FILE - DO NOT EDIT
// Source family: fsharp
// Regenerate with: grammar-tools generate-rust-compiled-grammars fsharp
//
// This file embeds versioned ParserGrammar values as native Rust data structures.
// Call `parser_grammar` instead of reading and parsing grammar files at runtime.

use grammar_tools::parser_grammar::ParserGrammar;

pub const SUPPORTED_VERSIONS: &[&str] = &[
    "1.0",
    "2.0",
    "3.0",
    "3.1",
    "4.0",
    "4.1",
    "4.5",
    "4.6",
    "4.7",
    "5",
    "6",
    "7",
    "8",
    "9",
    "10",
];

pub fn parser_grammar(version: &str) -> Option<ParserGrammar> {
    match version {
        "1.0" => Some(v_1_0::parser_grammar()),
        "2.0" => Some(v_2_0::parser_grammar()),
        "3.0" => Some(v_3_0::parser_grammar()),
        "3.1" => Some(v_3_1::parser_grammar()),
        "4.0" => Some(v_4_0::parser_grammar()),
        "4.1" => Some(v_4_1::parser_grammar()),
        "4.5" => Some(v_4_5::parser_grammar()),
        "4.6" => Some(v_4_6::parser_grammar()),
        "4.7" => Some(v_4_7::parser_grammar()),
        "5" => Some(v_5::parser_grammar()),
        "6" => Some(v_6::parser_grammar()),
        "7" => Some(v_7::parser_grammar()),
        "8" => Some(v_8::parser_grammar()),
        "9" => Some(v_9::parser_grammar()),
        "10" => Some(v_10::parser_grammar()),
        _ => None,
    }
}

mod v_1_0 {
    // AUTO-GENERATED FILE — DO NOT EDIT
    // Source: fsharp1.0.grammar
    // Regenerate with: grammar-tools compile-grammar fsharp1.0.grammar
    //
    // This file embeds a ParserGrammar as native Rust data structures.
    // Call `parser_grammar()` instead of reading and parsing the .grammar file.

    use grammar_tools::parser_grammar::{GrammarElement, GrammarRule, ParserGrammar};

    pub fn parser_grammar() -> ParserGrammar {
        ParserGrammar {
            rules: vec![
            GrammarRule {
                name: r#"compilation_unit"#.to_string(),
                body: GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                        GrammarElement::RuleReference { name: r#"decorated_declaration"#.to_string() },
                    ] }) },
                line_number: 9,
            },
            GrammarRule {
                name: r#"decorated_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"attribute_section"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"declaration_body"#.to_string() },
                ] },
                line_number: 11,
            },
            GrammarRule {
                name: r#"declaration_body"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"module_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"namespace_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"open_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"let_binding"#.to_string() },
                    GrammarElement::RuleReference { name: r#"use_binding"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"member_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"do_binding"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 13,
            },
            GrammarRule {
                name: r#"module_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"module"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"module_modifier"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"rec"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                        ] }) },
                ] },
                line_number: 23,
            },
            GrammarRule {
                name: r#"module_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"private"#.to_string() },
                    GrammarElement::Literal { value: r#"internal"#.to_string() },
                ] },
                line_number: 25,
            },
            GrammarRule {
                name: r#"namespace_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"namespace"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"namespace_modifier"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                ] },
                line_number: 28,
            },
            GrammarRule {
                name: r#"namespace_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"private"#.to_string() },
                    GrammarElement::Literal { value: r#"internal"#.to_string() },
                ] },
                line_number: 30,
            },
            GrammarRule {
                name: r#"open_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"open"#.to_string() },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                ] },
                line_number: 33,
            },
            GrammarRule {
                name: r#"let_binding"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"let"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"rec"#.to_string() }) },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"and"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                        ] }) },
                ] },
                line_number: 35,
            },
            GrammarRule {
                name: r#"use_binding"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"use"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"rec"#.to_string() }) },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"and"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                        ] }) },
                ] },
                line_number: 37,
            },
            GrammarRule {
                name: r#"binding_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"inline"#.to_string() },
                    GrammarElement::Literal { value: r#"mutable"#.to_string() },
                ] },
                line_number: 39,
            },
            GrammarRule {
                name: r#"binding_clause"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"pattern"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"type_annotation"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 42,
            },
            GrammarRule {
                name: r#"do_binding"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"do"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 44,
            },
            GrammarRule {
                name: r#"member_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"member_modifier"#.to_string() }) },
                    GrammarElement::Literal { value: r#"member"#.to_string() },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"parameter_list"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"type_annotation"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                ] },
                line_number: 46,
            },
            GrammarRule {
                name: r#"member_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"static"#.to_string() },
                    GrammarElement::Literal { value: r#"override"#.to_string() },
                    GrammarElement::Literal { value: r#"default"#.to_string() },
                    GrammarElement::Literal { value: r#"abstract"#.to_string() },
                    GrammarElement::Literal { value: r#"new"#.to_string() },
                ] },
                line_number: 48,
            },
            GrammarRule {
                name: r#"type_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"type"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"type_modifier"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"generic_parameters"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"parameter_list"#.to_string() }) },
                            GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"type_definition"#.to_string() },
                ] },
                line_number: 54,
            },
            GrammarRule {
                name: r#"type_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"private"#.to_string() },
                    GrammarElement::Literal { value: r#"public"#.to_string() },
                    GrammarElement::Literal { value: r#"internal"#.to_string() },
                ] },
                line_number: 56,
            },
            GrammarRule {
                name: r#"generic_parameters"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_parameter"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_parameter"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                ] },
                line_number: 60,
            },
            GrammarRule {
                name: r#"type_parameter"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"TYPEVAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                ] },
                line_number: 62,
            },
            GrammarRule {
                name: r#"type_definition"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"class_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"interface_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"struct_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"union_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"alias_type"#.to_string() },
                ] },
                line_number: 65,
            },
            GrammarRule {
                name: r#"class_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"class"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                            GrammarElement::RuleReference { name: r#"declaration_body"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"end"#.to_string() },
                ] },
                line_number: 72,
            },
            GrammarRule {
                name: r#"interface_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"interface"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                            GrammarElement::RuleReference { name: r#"declaration_body"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"end"#.to_string() },
                ] },
                line_number: 74,
            },
            GrammarRule {
                name: r#"struct_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"struct"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                            GrammarElement::RuleReference { name: r#"declaration_body"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"end"#.to_string() },
                ] },
                line_number: 76,
            },
            GrammarRule {
                name: r#"record_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"field_declaration"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"field_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"field_declaration"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 78,
            },
            GrammarRule {
                name: r#"field_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                ] },
                line_number: 80,
            },
            GrammarRule {
                name: r#"union_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"union_case"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"case_separator"#.to_string() },
                            GrammarElement::RuleReference { name: r#"union_case"#.to_string() },
                        ] }) },
                ] },
                line_number: 82,
            },
            GrammarRule {
                name: r#"union_case"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"of"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                                ] }) },
                        ] }) },
                ] },
                line_number: 84,
            },
            GrammarRule {
                name: r#"alias_type"#.to_string(),
                body: GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                line_number: 86,
            },
            GrammarRule {
                name: r#"field_separator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                ] },
                line_number: 88,
            },
            GrammarRule {
                name: r#"case_separator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"PIPE"#.to_string() }) },
                    ] },
                    GrammarElement::TokenReference { name: r#"PIPE"#.to_string() },
                ] },
                line_number: 91,
            },
            GrammarRule {
                name: r#"attribute_section"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"attribute"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"attribute"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                ] },
                line_number: 94,
            },
            GrammarRule {
                name: r#"attribute"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"attribute_target"#.to_string() },
                            GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"attribute_arguments"#.to_string() }) },
                            GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                        ] }) },
                ] },
                line_number: 96,
            },
            GrammarRule {
                name: r#"attribute_target"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"assembly"#.to_string() },
                    GrammarElement::Literal { value: r#"field"#.to_string() },
                    GrammarElement::Literal { value: r#"method"#.to_string() },
                    GrammarElement::Literal { value: r#"module"#.to_string() },
                    GrammarElement::Literal { value: r#"param"#.to_string() },
                    GrammarElement::Literal { value: r#"property"#.to_string() },
                    GrammarElement::Literal { value: r#"return"#.to_string() },
                    GrammarElement::Literal { value: r#"type"#.to_string() },
                    GrammarElement::Literal { value: r#"event"#.to_string() },
                ] },
                line_number: 98,
            },
            GrammarRule {
                name: r#"attribute_arguments"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"attribute_argument"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"attribute_argument"#.to_string() },
                        ] }) },
                ] },
                line_number: 108,
            },
            GrammarRule {
                name: r#"attribute_argument"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 110,
            },
            GrammarRule {
                name: r#"parameter_list"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"pattern"#.to_string() }) },
                ] },
                line_number: 112,
            },
            GrammarRule {
                name: r#"type_annotation"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                ] },
                line_number: 114,
            },
            GrammarRule {
                name: r#"type_expression"#.to_string(),
                body: GrammarElement::RuleReference { name: r#"function_type"#.to_string() },
                line_number: 116,
            },
            GrammarRule {
                name: r#"function_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"type_product"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                            GrammarElement::RuleReference { name: r#"function_type"#.to_string() },
                        ] }) },
                ] },
                line_number: 118,
            },
            GrammarRule {
                name: r#"type_product"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"type_application"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_application"#.to_string() },
                        ] }) },
                ] },
                line_number: 120,
            },
            GrammarRule {
                name: r#"type_application"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"type_atom"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"type_atom"#.to_string() }) },
                ] },
                line_number: 122,
            },
            GrammarRule {
                name: r#"generic_type_arguments"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                ] },
                line_number: 124,
            },
            GrammarRule {
                name: r#"type_atom"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"generic_type_arguments"#.to_string() }) },
                    ] },
                    GrammarElement::TokenReference { name: r#"TYPEVAR"#.to_string() },
                    GrammarElement::RuleReference { name: r#"tuple_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"unit_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"parenthesized_type"#.to_string() },
                ] },
                line_number: 126,
            },
            GrammarRule {
                name: r#"tuple_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 133,
            },
            GrammarRule {
                name: r#"unit_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 135,
            },
            GrammarRule {
                name: r#"parenthesized_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 137,
            },
            GrammarRule {
                name: r#"qualified_name"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 139,
            },
            GrammarRule {
                name: r#"expression"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"if_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"match_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"let_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"function_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"fun_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"for_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"while_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"sequence_expression"#.to_string() },
                ] },
                line_number: 141,
            },
            GrammarRule {
                name: r#"sequence_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"infix_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                                GrammarElement::RuleReference { name: r#"infix_expression"#.to_string() },
                            ] },
                            GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                                GrammarElement::RuleReference { name: r#"infix_expression"#.to_string() },
                            ] },
                        ] }) },
                ] },
                line_number: 150,
            },
            GrammarRule {
                name: r#"infix_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"application_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"infix_operator"#.to_string() },
                            GrammarElement::RuleReference { name: r#"application_expression"#.to_string() },
                        ] }) },
                ] },
                line_number: 152,
            },
            GrammarRule {
                name: r#"infix_operator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"PIPE_RIGHT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PIPE_LEFT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMPOSE_RIGHT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMPOSE_LEFT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NOT_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"GREATER_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"DOUBLE_COLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LARROW"#.to_string() },
                    GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                    GrammarElement::TokenReference { name: r#"DOT_DOT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"AND_AND"#.to_string() },
                    GrammarElement::TokenReference { name: r#"OR_OR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SLASH"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PERCENT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"AMPERSAND"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PIPE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CARET"#.to_string() },
                ] },
                line_number: 154,
            },
            GrammarRule {
                name: r#"application_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"prefix_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"prefix_expression"#.to_string() }) },
                ] },
                line_number: 181,
            },
            GrammarRule {
                name: r#"prefix_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"unary_operator"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"atomic_expression"#.to_string() },
                ] },
                line_number: 183,
            },
            GrammarRule {
                name: r#"unary_operator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"BANG"#.to_string() },
                ] },
                line_number: 185,
            },
            GrammarRule {
                name: r#"atomic_expression"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"computation_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"if_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"match_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"let_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"function_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"fun_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"for_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"while_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"unit_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"tuple_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"list_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"array_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"parenthesized_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CHAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"TRUE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"FALSE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NULL"#.to_string() },
                ] },
                line_number: 189,
            },
            GrammarRule {
                name: r#"computation_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"expression"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 211,
            },
            GrammarRule {
                name: r#"unit_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 213,
            },
            GrammarRule {
                name: r#"tuple_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 215,
            },
            GrammarRule {
                name: r#"list_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                    GrammarElement::NegativeLookahead { element: Box::new(GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"element_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                ] },
                line_number: 217,
            },
            GrammarRule {
                name: r#"array_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"ARRAY_LBRACKET"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"element_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"ARRAY_RBRACKET"#.to_string() },
                ] },
                line_number: 219,
            },
            GrammarRule {
                name: r#"record_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"field_assignment"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"field_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"field_assignment"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 221,
            },
            GrammarRule {
                name: r#"field_assignment"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 223,
            },
            GrammarRule {
                name: r#"parenthesized_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 225,
            },
            GrammarRule {
                name: r#"element_separator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                ] },
                line_number: 227,
            },
            GrammarRule {
                name: r#"if_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"if"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Literal { value: r#"then"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"else"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                ] },
                line_number: 230,
            },
            GrammarRule {
                name: r#"match_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"match"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Literal { value: r#"with"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"case_separator"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"match_case"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"case_separator"#.to_string() },
                            GrammarElement::RuleReference { name: r#"match_case"#.to_string() },
                        ] }) },
                ] },
                line_number: 232,
            },
            GrammarRule {
                name: r#"match_case"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"when"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 234,
            },
            GrammarRule {
                name: r#"fun_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"fun"#.to_string() },
                    GrammarElement::RuleReference { name: r#"parameter_list"#.to_string() },
                    GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 236,
            },
            GrammarRule {
                name: r#"function_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"function"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"case_separator"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"match_case"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"case_separator"#.to_string() },
                            GrammarElement::RuleReference { name: r#"match_case"#.to_string() },
                        ] }) },
                ] },
                line_number: 238,
            },
            GrammarRule {
                name: r#"let_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"let"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"rec"#.to_string() }) },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"and"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"in"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 240,
            },
            GrammarRule {
                name: r#"for_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"for"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::Literal { value: r#"to"#.to_string() },
                            GrammarElement::Literal { value: r#"downto"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Literal { value: r#"do"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 242,
            },
            GrammarRule {
                name: r#"while_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"while"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Literal { value: r#"do"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 244,
            },
            GrammarRule {
                name: r#"pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"pattern_atom"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"as"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 246,
            },
            GrammarRule {
                name: r#"pattern_atom"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"wildcard_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"literal_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"tuple_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"list_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"array_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"unit_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"parenthesized_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::TokenReference { name: r#"TYPEVAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                ] },
                line_number: 248,
            },
            GrammarRule {
                name: r#"wildcard_pattern"#.to_string(),
                body: GrammarElement::TokenReference { name: r#"UNDERSCORE"#.to_string() },
                line_number: 260,
            },
            GrammarRule {
                name: r#"literal_pattern"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CHAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"TRUE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"FALSE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NULL"#.to_string() },
                ] },
                line_number: 262,
            },
            GrammarRule {
                name: r#"tuple_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 269,
            },
            GrammarRule {
                name: r#"list_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"element_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                ] },
                line_number: 271,
            },
            GrammarRule {
                name: r#"array_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"ARRAY_LBRACKET"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"element_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"ARRAY_RBRACKET"#.to_string() },
                ] },
                line_number: 273,
            },
            GrammarRule {
                name: r#"record_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"field_pattern"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"field_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"field_pattern"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 275,
            },
            GrammarRule {
                name: r#"field_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                ] },
                line_number: 277,
            },
            GrammarRule {
                name: r#"unit_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 279,
            },
            GrammarRule {
                name: r#"parenthesized_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 281,
            },
        ],
            version: 1,
        }
    }
}

mod v_2_0 {
    // AUTO-GENERATED FILE — DO NOT EDIT
    // Source: fsharp2.0.grammar
    // Regenerate with: grammar-tools compile-grammar fsharp2.0.grammar
    //
    // This file embeds a ParserGrammar as native Rust data structures.
    // Call `parser_grammar()` instead of reading and parsing the .grammar file.

    use grammar_tools::parser_grammar::{GrammarElement, GrammarRule, ParserGrammar};

    pub fn parser_grammar() -> ParserGrammar {
        ParserGrammar {
            rules: vec![
            GrammarRule {
                name: r#"compilation_unit"#.to_string(),
                body: GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                        GrammarElement::RuleReference { name: r#"decorated_declaration"#.to_string() },
                    ] }) },
                line_number: 9,
            },
            GrammarRule {
                name: r#"decorated_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"attribute_section"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"declaration_body"#.to_string() },
                ] },
                line_number: 11,
            },
            GrammarRule {
                name: r#"declaration_body"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"module_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"namespace_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"open_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"let_binding"#.to_string() },
                    GrammarElement::RuleReference { name: r#"use_binding"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"member_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"do_binding"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 13,
            },
            GrammarRule {
                name: r#"module_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"module"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"module_modifier"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"rec"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                        ] }) },
                ] },
                line_number: 23,
            },
            GrammarRule {
                name: r#"module_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"private"#.to_string() },
                    GrammarElement::Literal { value: r#"internal"#.to_string() },
                ] },
                line_number: 25,
            },
            GrammarRule {
                name: r#"namespace_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"namespace"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"namespace_modifier"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                ] },
                line_number: 28,
            },
            GrammarRule {
                name: r#"namespace_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"private"#.to_string() },
                    GrammarElement::Literal { value: r#"internal"#.to_string() },
                ] },
                line_number: 30,
            },
            GrammarRule {
                name: r#"open_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"open"#.to_string() },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                ] },
                line_number: 33,
            },
            GrammarRule {
                name: r#"let_binding"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"let"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"rec"#.to_string() }) },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"and"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                        ] }) },
                ] },
                line_number: 35,
            },
            GrammarRule {
                name: r#"use_binding"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"use"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"rec"#.to_string() }) },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"and"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                        ] }) },
                ] },
                line_number: 37,
            },
            GrammarRule {
                name: r#"binding_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"inline"#.to_string() },
                    GrammarElement::Literal { value: r#"mutable"#.to_string() },
                ] },
                line_number: 39,
            },
            GrammarRule {
                name: r#"binding_clause"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"pattern"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"type_annotation"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 42,
            },
            GrammarRule {
                name: r#"do_binding"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"do"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 44,
            },
            GrammarRule {
                name: r#"member_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"member_modifier"#.to_string() }) },
                    GrammarElement::Literal { value: r#"member"#.to_string() },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"parameter_list"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"type_annotation"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                ] },
                line_number: 46,
            },
            GrammarRule {
                name: r#"member_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"static"#.to_string() },
                    GrammarElement::Literal { value: r#"override"#.to_string() },
                    GrammarElement::Literal { value: r#"default"#.to_string() },
                    GrammarElement::Literal { value: r#"abstract"#.to_string() },
                    GrammarElement::Literal { value: r#"new"#.to_string() },
                ] },
                line_number: 48,
            },
            GrammarRule {
                name: r#"type_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"type"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"type_modifier"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"generic_parameters"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"parameter_list"#.to_string() }) },
                            GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"type_definition"#.to_string() },
                ] },
                line_number: 54,
            },
            GrammarRule {
                name: r#"type_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"private"#.to_string() },
                    GrammarElement::Literal { value: r#"public"#.to_string() },
                    GrammarElement::Literal { value: r#"internal"#.to_string() },
                ] },
                line_number: 56,
            },
            GrammarRule {
                name: r#"generic_parameters"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_parameter"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_parameter"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                ] },
                line_number: 60,
            },
            GrammarRule {
                name: r#"type_parameter"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"TYPEVAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                ] },
                line_number: 62,
            },
            GrammarRule {
                name: r#"type_definition"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"class_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"interface_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"struct_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"union_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"alias_type"#.to_string() },
                ] },
                line_number: 65,
            },
            GrammarRule {
                name: r#"class_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"class"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                            GrammarElement::RuleReference { name: r#"declaration_body"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"end"#.to_string() },
                ] },
                line_number: 72,
            },
            GrammarRule {
                name: r#"interface_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"interface"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                            GrammarElement::RuleReference { name: r#"declaration_body"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"end"#.to_string() },
                ] },
                line_number: 74,
            },
            GrammarRule {
                name: r#"struct_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"struct"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                            GrammarElement::RuleReference { name: r#"declaration_body"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"end"#.to_string() },
                ] },
                line_number: 76,
            },
            GrammarRule {
                name: r#"record_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"field_declaration"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"field_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"field_declaration"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 78,
            },
            GrammarRule {
                name: r#"field_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                ] },
                line_number: 80,
            },
            GrammarRule {
                name: r#"union_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"union_case"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"case_separator"#.to_string() },
                            GrammarElement::RuleReference { name: r#"union_case"#.to_string() },
                        ] }) },
                ] },
                line_number: 82,
            },
            GrammarRule {
                name: r#"union_case"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"of"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                                ] }) },
                        ] }) },
                ] },
                line_number: 84,
            },
            GrammarRule {
                name: r#"alias_type"#.to_string(),
                body: GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                line_number: 86,
            },
            GrammarRule {
                name: r#"field_separator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                ] },
                line_number: 88,
            },
            GrammarRule {
                name: r#"case_separator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"PIPE"#.to_string() }) },
                    ] },
                    GrammarElement::TokenReference { name: r#"PIPE"#.to_string() },
                ] },
                line_number: 91,
            },
            GrammarRule {
                name: r#"attribute_section"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"attribute"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"attribute"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                ] },
                line_number: 94,
            },
            GrammarRule {
                name: r#"attribute"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"attribute_target"#.to_string() },
                            GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"attribute_arguments"#.to_string() }) },
                            GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                        ] }) },
                ] },
                line_number: 96,
            },
            GrammarRule {
                name: r#"attribute_target"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"assembly"#.to_string() },
                    GrammarElement::Literal { value: r#"field"#.to_string() },
                    GrammarElement::Literal { value: r#"method"#.to_string() },
                    GrammarElement::Literal { value: r#"module"#.to_string() },
                    GrammarElement::Literal { value: r#"param"#.to_string() },
                    GrammarElement::Literal { value: r#"property"#.to_string() },
                    GrammarElement::Literal { value: r#"return"#.to_string() },
                    GrammarElement::Literal { value: r#"type"#.to_string() },
                    GrammarElement::Literal { value: r#"event"#.to_string() },
                ] },
                line_number: 98,
            },
            GrammarRule {
                name: r#"attribute_arguments"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"attribute_argument"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"attribute_argument"#.to_string() },
                        ] }) },
                ] },
                line_number: 108,
            },
            GrammarRule {
                name: r#"attribute_argument"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 110,
            },
            GrammarRule {
                name: r#"parameter_list"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"pattern"#.to_string() }) },
                ] },
                line_number: 112,
            },
            GrammarRule {
                name: r#"type_annotation"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                ] },
                line_number: 114,
            },
            GrammarRule {
                name: r#"type_expression"#.to_string(),
                body: GrammarElement::RuleReference { name: r#"function_type"#.to_string() },
                line_number: 116,
            },
            GrammarRule {
                name: r#"function_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"type_product"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                            GrammarElement::RuleReference { name: r#"function_type"#.to_string() },
                        ] }) },
                ] },
                line_number: 118,
            },
            GrammarRule {
                name: r#"type_product"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"type_application"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_application"#.to_string() },
                        ] }) },
                ] },
                line_number: 120,
            },
            GrammarRule {
                name: r#"type_application"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"type_atom"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"type_atom"#.to_string() }) },
                ] },
                line_number: 122,
            },
            GrammarRule {
                name: r#"generic_type_arguments"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                ] },
                line_number: 124,
            },
            GrammarRule {
                name: r#"type_atom"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"generic_type_arguments"#.to_string() }) },
                    ] },
                    GrammarElement::TokenReference { name: r#"TYPEVAR"#.to_string() },
                    GrammarElement::RuleReference { name: r#"tuple_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"unit_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"parenthesized_type"#.to_string() },
                ] },
                line_number: 126,
            },
            GrammarRule {
                name: r#"tuple_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 133,
            },
            GrammarRule {
                name: r#"unit_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 135,
            },
            GrammarRule {
                name: r#"parenthesized_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 137,
            },
            GrammarRule {
                name: r#"qualified_name"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 139,
            },
            GrammarRule {
                name: r#"expression"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"if_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"match_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"let_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"function_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"fun_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"for_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"while_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"sequence_expression"#.to_string() },
                ] },
                line_number: 141,
            },
            GrammarRule {
                name: r#"sequence_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"infix_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                                GrammarElement::RuleReference { name: r#"infix_expression"#.to_string() },
                            ] },
                            GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                                GrammarElement::RuleReference { name: r#"infix_expression"#.to_string() },
                            ] },
                        ] }) },
                ] },
                line_number: 150,
            },
            GrammarRule {
                name: r#"infix_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"application_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"infix_operator"#.to_string() },
                            GrammarElement::RuleReference { name: r#"application_expression"#.to_string() },
                        ] }) },
                ] },
                line_number: 152,
            },
            GrammarRule {
                name: r#"infix_operator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"PIPE_RIGHT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PIPE_LEFT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMPOSE_RIGHT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMPOSE_LEFT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NOT_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"GREATER_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"DOUBLE_COLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LARROW"#.to_string() },
                    GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                    GrammarElement::TokenReference { name: r#"DOT_DOT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"AND_AND"#.to_string() },
                    GrammarElement::TokenReference { name: r#"OR_OR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SLASH"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PERCENT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"AMPERSAND"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PIPE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CARET"#.to_string() },
                ] },
                line_number: 154,
            },
            GrammarRule {
                name: r#"application_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"prefix_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"prefix_expression"#.to_string() }) },
                ] },
                line_number: 181,
            },
            GrammarRule {
                name: r#"prefix_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"unary_operator"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"atomic_expression"#.to_string() },
                ] },
                line_number: 183,
            },
            GrammarRule {
                name: r#"unary_operator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"BANG"#.to_string() },
                ] },
                line_number: 185,
            },
            GrammarRule {
                name: r#"atomic_expression"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"computation_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"if_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"match_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"let_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"function_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"fun_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"for_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"while_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"unit_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"tuple_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"list_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"array_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"parenthesized_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CHAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"TRUE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"FALSE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NULL"#.to_string() },
                ] },
                line_number: 189,
            },
            GrammarRule {
                name: r#"computation_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"expression"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 211,
            },
            GrammarRule {
                name: r#"unit_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 213,
            },
            GrammarRule {
                name: r#"tuple_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 215,
            },
            GrammarRule {
                name: r#"list_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                    GrammarElement::NegativeLookahead { element: Box::new(GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"element_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                ] },
                line_number: 217,
            },
            GrammarRule {
                name: r#"array_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"ARRAY_LBRACKET"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"element_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"ARRAY_RBRACKET"#.to_string() },
                ] },
                line_number: 219,
            },
            GrammarRule {
                name: r#"record_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"field_assignment"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"field_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"field_assignment"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 221,
            },
            GrammarRule {
                name: r#"field_assignment"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 223,
            },
            GrammarRule {
                name: r#"parenthesized_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 225,
            },
            GrammarRule {
                name: r#"element_separator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                ] },
                line_number: 227,
            },
            GrammarRule {
                name: r#"if_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"if"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Literal { value: r#"then"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"else"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                ] },
                line_number: 230,
            },
            GrammarRule {
                name: r#"match_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"match"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Literal { value: r#"with"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"case_separator"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"match_case"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"case_separator"#.to_string() },
                            GrammarElement::RuleReference { name: r#"match_case"#.to_string() },
                        ] }) },
                ] },
                line_number: 232,
            },
            GrammarRule {
                name: r#"match_case"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"when"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 234,
            },
            GrammarRule {
                name: r#"fun_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"fun"#.to_string() },
                    GrammarElement::RuleReference { name: r#"parameter_list"#.to_string() },
                    GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 236,
            },
            GrammarRule {
                name: r#"function_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"function"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"case_separator"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"match_case"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"case_separator"#.to_string() },
                            GrammarElement::RuleReference { name: r#"match_case"#.to_string() },
                        ] }) },
                ] },
                line_number: 238,
            },
            GrammarRule {
                name: r#"let_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"let"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"rec"#.to_string() }) },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"and"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"in"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 240,
            },
            GrammarRule {
                name: r#"for_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"for"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::Literal { value: r#"to"#.to_string() },
                            GrammarElement::Literal { value: r#"downto"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Literal { value: r#"do"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 242,
            },
            GrammarRule {
                name: r#"while_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"while"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Literal { value: r#"do"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 244,
            },
            GrammarRule {
                name: r#"pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"pattern_atom"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"as"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 246,
            },
            GrammarRule {
                name: r#"pattern_atom"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"wildcard_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"literal_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"tuple_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"list_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"array_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"unit_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"parenthesized_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::TokenReference { name: r#"TYPEVAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                ] },
                line_number: 248,
            },
            GrammarRule {
                name: r#"wildcard_pattern"#.to_string(),
                body: GrammarElement::TokenReference { name: r#"UNDERSCORE"#.to_string() },
                line_number: 260,
            },
            GrammarRule {
                name: r#"literal_pattern"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CHAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"TRUE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"FALSE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NULL"#.to_string() },
                ] },
                line_number: 262,
            },
            GrammarRule {
                name: r#"tuple_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 269,
            },
            GrammarRule {
                name: r#"list_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"element_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                ] },
                line_number: 271,
            },
            GrammarRule {
                name: r#"array_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"ARRAY_LBRACKET"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"element_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"ARRAY_RBRACKET"#.to_string() },
                ] },
                line_number: 273,
            },
            GrammarRule {
                name: r#"record_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"field_pattern"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"field_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"field_pattern"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 275,
            },
            GrammarRule {
                name: r#"field_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                ] },
                line_number: 277,
            },
            GrammarRule {
                name: r#"unit_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 279,
            },
            GrammarRule {
                name: r#"parenthesized_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 281,
            },
        ],
            version: 1,
        }
    }
}

mod v_3_0 {
    // AUTO-GENERATED FILE — DO NOT EDIT
    // Source: fsharp3.0.grammar
    // Regenerate with: grammar-tools compile-grammar fsharp3.0.grammar
    //
    // This file embeds a ParserGrammar as native Rust data structures.
    // Call `parser_grammar()` instead of reading and parsing the .grammar file.

    use grammar_tools::parser_grammar::{GrammarElement, GrammarRule, ParserGrammar};

    pub fn parser_grammar() -> ParserGrammar {
        ParserGrammar {
            rules: vec![
            GrammarRule {
                name: r#"compilation_unit"#.to_string(),
                body: GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                        GrammarElement::RuleReference { name: r#"decorated_declaration"#.to_string() },
                    ] }) },
                line_number: 9,
            },
            GrammarRule {
                name: r#"decorated_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"attribute_section"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"declaration_body"#.to_string() },
                ] },
                line_number: 11,
            },
            GrammarRule {
                name: r#"declaration_body"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"module_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"namespace_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"open_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"let_binding"#.to_string() },
                    GrammarElement::RuleReference { name: r#"use_binding"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"member_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"do_binding"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 13,
            },
            GrammarRule {
                name: r#"module_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"module"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"module_modifier"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"rec"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                        ] }) },
                ] },
                line_number: 23,
            },
            GrammarRule {
                name: r#"module_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"private"#.to_string() },
                    GrammarElement::Literal { value: r#"internal"#.to_string() },
                ] },
                line_number: 25,
            },
            GrammarRule {
                name: r#"namespace_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"namespace"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"namespace_modifier"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                ] },
                line_number: 28,
            },
            GrammarRule {
                name: r#"namespace_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"private"#.to_string() },
                    GrammarElement::Literal { value: r#"internal"#.to_string() },
                ] },
                line_number: 30,
            },
            GrammarRule {
                name: r#"open_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"open"#.to_string() },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                ] },
                line_number: 33,
            },
            GrammarRule {
                name: r#"let_binding"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"let"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"rec"#.to_string() }) },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"and"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                        ] }) },
                ] },
                line_number: 35,
            },
            GrammarRule {
                name: r#"use_binding"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"use"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"rec"#.to_string() }) },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"and"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                        ] }) },
                ] },
                line_number: 37,
            },
            GrammarRule {
                name: r#"binding_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"inline"#.to_string() },
                    GrammarElement::Literal { value: r#"mutable"#.to_string() },
                ] },
                line_number: 39,
            },
            GrammarRule {
                name: r#"binding_clause"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"pattern"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"type_annotation"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 42,
            },
            GrammarRule {
                name: r#"do_binding"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"do"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 44,
            },
            GrammarRule {
                name: r#"member_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"member_modifier"#.to_string() }) },
                    GrammarElement::Literal { value: r#"member"#.to_string() },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"parameter_list"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"type_annotation"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                ] },
                line_number: 46,
            },
            GrammarRule {
                name: r#"member_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"static"#.to_string() },
                    GrammarElement::Literal { value: r#"override"#.to_string() },
                    GrammarElement::Literal { value: r#"default"#.to_string() },
                    GrammarElement::Literal { value: r#"abstract"#.to_string() },
                    GrammarElement::Literal { value: r#"new"#.to_string() },
                ] },
                line_number: 48,
            },
            GrammarRule {
                name: r#"type_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"type"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"type_modifier"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"generic_parameters"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"parameter_list"#.to_string() }) },
                            GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"type_definition"#.to_string() },
                ] },
                line_number: 54,
            },
            GrammarRule {
                name: r#"type_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"private"#.to_string() },
                    GrammarElement::Literal { value: r#"public"#.to_string() },
                    GrammarElement::Literal { value: r#"internal"#.to_string() },
                ] },
                line_number: 56,
            },
            GrammarRule {
                name: r#"generic_parameters"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_parameter"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_parameter"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                ] },
                line_number: 60,
            },
            GrammarRule {
                name: r#"type_parameter"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"TYPEVAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                ] },
                line_number: 62,
            },
            GrammarRule {
                name: r#"type_definition"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"class_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"interface_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"struct_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"union_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"alias_type"#.to_string() },
                ] },
                line_number: 65,
            },
            GrammarRule {
                name: r#"class_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"class"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                            GrammarElement::RuleReference { name: r#"declaration_body"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"end"#.to_string() },
                ] },
                line_number: 72,
            },
            GrammarRule {
                name: r#"interface_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"interface"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                            GrammarElement::RuleReference { name: r#"declaration_body"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"end"#.to_string() },
                ] },
                line_number: 74,
            },
            GrammarRule {
                name: r#"struct_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"struct"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                            GrammarElement::RuleReference { name: r#"declaration_body"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"end"#.to_string() },
                ] },
                line_number: 76,
            },
            GrammarRule {
                name: r#"record_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"field_declaration"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"field_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"field_declaration"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 78,
            },
            GrammarRule {
                name: r#"field_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                ] },
                line_number: 80,
            },
            GrammarRule {
                name: r#"union_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"union_case"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"case_separator"#.to_string() },
                            GrammarElement::RuleReference { name: r#"union_case"#.to_string() },
                        ] }) },
                ] },
                line_number: 82,
            },
            GrammarRule {
                name: r#"union_case"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"of"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                                ] }) },
                        ] }) },
                ] },
                line_number: 84,
            },
            GrammarRule {
                name: r#"alias_type"#.to_string(),
                body: GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                line_number: 86,
            },
            GrammarRule {
                name: r#"field_separator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                ] },
                line_number: 88,
            },
            GrammarRule {
                name: r#"case_separator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"PIPE"#.to_string() }) },
                    ] },
                    GrammarElement::TokenReference { name: r#"PIPE"#.to_string() },
                ] },
                line_number: 91,
            },
            GrammarRule {
                name: r#"attribute_section"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"attribute"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"attribute"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                ] },
                line_number: 94,
            },
            GrammarRule {
                name: r#"attribute"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"attribute_target"#.to_string() },
                            GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"attribute_arguments"#.to_string() }) },
                            GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                        ] }) },
                ] },
                line_number: 96,
            },
            GrammarRule {
                name: r#"attribute_target"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"assembly"#.to_string() },
                    GrammarElement::Literal { value: r#"field"#.to_string() },
                    GrammarElement::Literal { value: r#"method"#.to_string() },
                    GrammarElement::Literal { value: r#"module"#.to_string() },
                    GrammarElement::Literal { value: r#"param"#.to_string() },
                    GrammarElement::Literal { value: r#"property"#.to_string() },
                    GrammarElement::Literal { value: r#"return"#.to_string() },
                    GrammarElement::Literal { value: r#"type"#.to_string() },
                    GrammarElement::Literal { value: r#"event"#.to_string() },
                ] },
                line_number: 98,
            },
            GrammarRule {
                name: r#"attribute_arguments"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"attribute_argument"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"attribute_argument"#.to_string() },
                        ] }) },
                ] },
                line_number: 108,
            },
            GrammarRule {
                name: r#"attribute_argument"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 110,
            },
            GrammarRule {
                name: r#"parameter_list"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"pattern"#.to_string() }) },
                ] },
                line_number: 112,
            },
            GrammarRule {
                name: r#"type_annotation"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                ] },
                line_number: 114,
            },
            GrammarRule {
                name: r#"type_expression"#.to_string(),
                body: GrammarElement::RuleReference { name: r#"function_type"#.to_string() },
                line_number: 116,
            },
            GrammarRule {
                name: r#"function_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"type_product"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                            GrammarElement::RuleReference { name: r#"function_type"#.to_string() },
                        ] }) },
                ] },
                line_number: 118,
            },
            GrammarRule {
                name: r#"type_product"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"type_application"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_application"#.to_string() },
                        ] }) },
                ] },
                line_number: 120,
            },
            GrammarRule {
                name: r#"type_application"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"type_atom"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"type_atom"#.to_string() }) },
                ] },
                line_number: 122,
            },
            GrammarRule {
                name: r#"generic_type_arguments"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                ] },
                line_number: 124,
            },
            GrammarRule {
                name: r#"type_atom"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"generic_type_arguments"#.to_string() }) },
                    ] },
                    GrammarElement::TokenReference { name: r#"TYPEVAR"#.to_string() },
                    GrammarElement::RuleReference { name: r#"tuple_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"unit_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"parenthesized_type"#.to_string() },
                ] },
                line_number: 126,
            },
            GrammarRule {
                name: r#"tuple_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 133,
            },
            GrammarRule {
                name: r#"unit_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 135,
            },
            GrammarRule {
                name: r#"parenthesized_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 137,
            },
            GrammarRule {
                name: r#"qualified_name"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 139,
            },
            GrammarRule {
                name: r#"expression"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"if_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"match_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"let_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"function_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"fun_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"for_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"while_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"sequence_expression"#.to_string() },
                ] },
                line_number: 141,
            },
            GrammarRule {
                name: r#"sequence_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"infix_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                                GrammarElement::RuleReference { name: r#"infix_expression"#.to_string() },
                            ] },
                            GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                                GrammarElement::RuleReference { name: r#"infix_expression"#.to_string() },
                            ] },
                        ] }) },
                ] },
                line_number: 150,
            },
            GrammarRule {
                name: r#"infix_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"application_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"infix_operator"#.to_string() },
                            GrammarElement::RuleReference { name: r#"application_expression"#.to_string() },
                        ] }) },
                ] },
                line_number: 152,
            },
            GrammarRule {
                name: r#"infix_operator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"PIPE_RIGHT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PIPE_LEFT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMPOSE_RIGHT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMPOSE_LEFT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NOT_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"GREATER_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"DOUBLE_COLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LARROW"#.to_string() },
                    GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                    GrammarElement::TokenReference { name: r#"DOT_DOT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"AND_AND"#.to_string() },
                    GrammarElement::TokenReference { name: r#"OR_OR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SLASH"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PERCENT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"AMPERSAND"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PIPE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CARET"#.to_string() },
                ] },
                line_number: 154,
            },
            GrammarRule {
                name: r#"application_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"prefix_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"prefix_expression"#.to_string() }) },
                ] },
                line_number: 181,
            },
            GrammarRule {
                name: r#"prefix_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"unary_operator"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"atomic_expression"#.to_string() },
                ] },
                line_number: 183,
            },
            GrammarRule {
                name: r#"unary_operator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"BANG"#.to_string() },
                ] },
                line_number: 185,
            },
            GrammarRule {
                name: r#"atomic_expression"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"computation_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"if_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"match_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"let_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"function_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"fun_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"for_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"while_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"unit_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"tuple_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"list_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"array_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"parenthesized_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CHAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"TRUE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"FALSE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NULL"#.to_string() },
                ] },
                line_number: 189,
            },
            GrammarRule {
                name: r#"computation_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"expression"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 211,
            },
            GrammarRule {
                name: r#"unit_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 213,
            },
            GrammarRule {
                name: r#"tuple_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 215,
            },
            GrammarRule {
                name: r#"list_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                    GrammarElement::NegativeLookahead { element: Box::new(GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"element_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                ] },
                line_number: 217,
            },
            GrammarRule {
                name: r#"array_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"ARRAY_LBRACKET"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"element_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"ARRAY_RBRACKET"#.to_string() },
                ] },
                line_number: 219,
            },
            GrammarRule {
                name: r#"record_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"field_assignment"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"field_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"field_assignment"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 221,
            },
            GrammarRule {
                name: r#"field_assignment"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 223,
            },
            GrammarRule {
                name: r#"parenthesized_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 225,
            },
            GrammarRule {
                name: r#"element_separator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                ] },
                line_number: 227,
            },
            GrammarRule {
                name: r#"if_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"if"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Literal { value: r#"then"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"else"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                ] },
                line_number: 230,
            },
            GrammarRule {
                name: r#"match_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"match"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Literal { value: r#"with"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"case_separator"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"match_case"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"case_separator"#.to_string() },
                            GrammarElement::RuleReference { name: r#"match_case"#.to_string() },
                        ] }) },
                ] },
                line_number: 232,
            },
            GrammarRule {
                name: r#"match_case"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"when"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 234,
            },
            GrammarRule {
                name: r#"fun_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"fun"#.to_string() },
                    GrammarElement::RuleReference { name: r#"parameter_list"#.to_string() },
                    GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 236,
            },
            GrammarRule {
                name: r#"function_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"function"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"case_separator"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"match_case"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"case_separator"#.to_string() },
                            GrammarElement::RuleReference { name: r#"match_case"#.to_string() },
                        ] }) },
                ] },
                line_number: 238,
            },
            GrammarRule {
                name: r#"let_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"let"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"rec"#.to_string() }) },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"and"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"in"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 240,
            },
            GrammarRule {
                name: r#"for_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"for"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::Literal { value: r#"to"#.to_string() },
                            GrammarElement::Literal { value: r#"downto"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Literal { value: r#"do"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 242,
            },
            GrammarRule {
                name: r#"while_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"while"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Literal { value: r#"do"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 244,
            },
            GrammarRule {
                name: r#"pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"pattern_atom"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"as"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 246,
            },
            GrammarRule {
                name: r#"pattern_atom"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"wildcard_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"literal_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"tuple_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"list_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"array_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"unit_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"parenthesized_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::TokenReference { name: r#"TYPEVAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                ] },
                line_number: 248,
            },
            GrammarRule {
                name: r#"wildcard_pattern"#.to_string(),
                body: GrammarElement::TokenReference { name: r#"UNDERSCORE"#.to_string() },
                line_number: 260,
            },
            GrammarRule {
                name: r#"literal_pattern"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CHAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"TRUE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"FALSE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NULL"#.to_string() },
                ] },
                line_number: 262,
            },
            GrammarRule {
                name: r#"tuple_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 269,
            },
            GrammarRule {
                name: r#"list_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"element_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                ] },
                line_number: 271,
            },
            GrammarRule {
                name: r#"array_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"ARRAY_LBRACKET"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"element_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"ARRAY_RBRACKET"#.to_string() },
                ] },
                line_number: 273,
            },
            GrammarRule {
                name: r#"record_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"field_pattern"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"field_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"field_pattern"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 275,
            },
            GrammarRule {
                name: r#"field_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                ] },
                line_number: 277,
            },
            GrammarRule {
                name: r#"unit_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 279,
            },
            GrammarRule {
                name: r#"parenthesized_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 281,
            },
        ],
            version: 1,
        }
    }
}

mod v_3_1 {
    // AUTO-GENERATED FILE — DO NOT EDIT
    // Source: fsharp3.1.grammar
    // Regenerate with: grammar-tools compile-grammar fsharp3.1.grammar
    //
    // This file embeds a ParserGrammar as native Rust data structures.
    // Call `parser_grammar()` instead of reading and parsing the .grammar file.

    use grammar_tools::parser_grammar::{GrammarElement, GrammarRule, ParserGrammar};

    pub fn parser_grammar() -> ParserGrammar {
        ParserGrammar {
            rules: vec![
            GrammarRule {
                name: r#"compilation_unit"#.to_string(),
                body: GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                        GrammarElement::RuleReference { name: r#"decorated_declaration"#.to_string() },
                    ] }) },
                line_number: 9,
            },
            GrammarRule {
                name: r#"decorated_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"attribute_section"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"declaration_body"#.to_string() },
                ] },
                line_number: 11,
            },
            GrammarRule {
                name: r#"declaration_body"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"module_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"namespace_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"open_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"let_binding"#.to_string() },
                    GrammarElement::RuleReference { name: r#"use_binding"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"member_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"do_binding"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 13,
            },
            GrammarRule {
                name: r#"module_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"module"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"module_modifier"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"rec"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                        ] }) },
                ] },
                line_number: 23,
            },
            GrammarRule {
                name: r#"module_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"private"#.to_string() },
                    GrammarElement::Literal { value: r#"internal"#.to_string() },
                ] },
                line_number: 25,
            },
            GrammarRule {
                name: r#"namespace_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"namespace"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"namespace_modifier"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                ] },
                line_number: 28,
            },
            GrammarRule {
                name: r#"namespace_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"private"#.to_string() },
                    GrammarElement::Literal { value: r#"internal"#.to_string() },
                ] },
                line_number: 30,
            },
            GrammarRule {
                name: r#"open_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"open"#.to_string() },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                ] },
                line_number: 33,
            },
            GrammarRule {
                name: r#"let_binding"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"let"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"rec"#.to_string() }) },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"and"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                        ] }) },
                ] },
                line_number: 35,
            },
            GrammarRule {
                name: r#"use_binding"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"use"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"rec"#.to_string() }) },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"and"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                        ] }) },
                ] },
                line_number: 37,
            },
            GrammarRule {
                name: r#"binding_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"inline"#.to_string() },
                    GrammarElement::Literal { value: r#"mutable"#.to_string() },
                ] },
                line_number: 39,
            },
            GrammarRule {
                name: r#"binding_clause"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"pattern"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"type_annotation"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 42,
            },
            GrammarRule {
                name: r#"do_binding"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"do"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 44,
            },
            GrammarRule {
                name: r#"member_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"member_modifier"#.to_string() }) },
                    GrammarElement::Literal { value: r#"member"#.to_string() },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"parameter_list"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"type_annotation"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                ] },
                line_number: 46,
            },
            GrammarRule {
                name: r#"member_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"static"#.to_string() },
                    GrammarElement::Literal { value: r#"override"#.to_string() },
                    GrammarElement::Literal { value: r#"default"#.to_string() },
                    GrammarElement::Literal { value: r#"abstract"#.to_string() },
                    GrammarElement::Literal { value: r#"new"#.to_string() },
                ] },
                line_number: 48,
            },
            GrammarRule {
                name: r#"type_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"type"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"type_modifier"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"generic_parameters"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"parameter_list"#.to_string() }) },
                            GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"type_definition"#.to_string() },
                ] },
                line_number: 54,
            },
            GrammarRule {
                name: r#"type_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"private"#.to_string() },
                    GrammarElement::Literal { value: r#"public"#.to_string() },
                    GrammarElement::Literal { value: r#"internal"#.to_string() },
                ] },
                line_number: 56,
            },
            GrammarRule {
                name: r#"generic_parameters"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_parameter"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_parameter"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                ] },
                line_number: 60,
            },
            GrammarRule {
                name: r#"type_parameter"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"TYPEVAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                ] },
                line_number: 62,
            },
            GrammarRule {
                name: r#"type_definition"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"class_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"interface_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"struct_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"union_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"alias_type"#.to_string() },
                ] },
                line_number: 65,
            },
            GrammarRule {
                name: r#"class_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"class"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                            GrammarElement::RuleReference { name: r#"declaration_body"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"end"#.to_string() },
                ] },
                line_number: 72,
            },
            GrammarRule {
                name: r#"interface_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"interface"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                            GrammarElement::RuleReference { name: r#"declaration_body"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"end"#.to_string() },
                ] },
                line_number: 74,
            },
            GrammarRule {
                name: r#"struct_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"struct"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                            GrammarElement::RuleReference { name: r#"declaration_body"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"end"#.to_string() },
                ] },
                line_number: 76,
            },
            GrammarRule {
                name: r#"record_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"field_declaration"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"field_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"field_declaration"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 78,
            },
            GrammarRule {
                name: r#"field_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                ] },
                line_number: 80,
            },
            GrammarRule {
                name: r#"union_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"union_case"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"case_separator"#.to_string() },
                            GrammarElement::RuleReference { name: r#"union_case"#.to_string() },
                        ] }) },
                ] },
                line_number: 82,
            },
            GrammarRule {
                name: r#"union_case"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"of"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                                ] }) },
                        ] }) },
                ] },
                line_number: 84,
            },
            GrammarRule {
                name: r#"alias_type"#.to_string(),
                body: GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                line_number: 86,
            },
            GrammarRule {
                name: r#"field_separator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                ] },
                line_number: 88,
            },
            GrammarRule {
                name: r#"case_separator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"PIPE"#.to_string() }) },
                    ] },
                    GrammarElement::TokenReference { name: r#"PIPE"#.to_string() },
                ] },
                line_number: 91,
            },
            GrammarRule {
                name: r#"attribute_section"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"attribute"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"attribute"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                ] },
                line_number: 94,
            },
            GrammarRule {
                name: r#"attribute"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"attribute_target"#.to_string() },
                            GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"attribute_arguments"#.to_string() }) },
                            GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                        ] }) },
                ] },
                line_number: 96,
            },
            GrammarRule {
                name: r#"attribute_target"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"assembly"#.to_string() },
                    GrammarElement::Literal { value: r#"field"#.to_string() },
                    GrammarElement::Literal { value: r#"method"#.to_string() },
                    GrammarElement::Literal { value: r#"module"#.to_string() },
                    GrammarElement::Literal { value: r#"param"#.to_string() },
                    GrammarElement::Literal { value: r#"property"#.to_string() },
                    GrammarElement::Literal { value: r#"return"#.to_string() },
                    GrammarElement::Literal { value: r#"type"#.to_string() },
                    GrammarElement::Literal { value: r#"event"#.to_string() },
                ] },
                line_number: 98,
            },
            GrammarRule {
                name: r#"attribute_arguments"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"attribute_argument"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"attribute_argument"#.to_string() },
                        ] }) },
                ] },
                line_number: 108,
            },
            GrammarRule {
                name: r#"attribute_argument"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 110,
            },
            GrammarRule {
                name: r#"parameter_list"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"pattern"#.to_string() }) },
                ] },
                line_number: 112,
            },
            GrammarRule {
                name: r#"type_annotation"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                ] },
                line_number: 114,
            },
            GrammarRule {
                name: r#"type_expression"#.to_string(),
                body: GrammarElement::RuleReference { name: r#"function_type"#.to_string() },
                line_number: 116,
            },
            GrammarRule {
                name: r#"function_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"type_product"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                            GrammarElement::RuleReference { name: r#"function_type"#.to_string() },
                        ] }) },
                ] },
                line_number: 118,
            },
            GrammarRule {
                name: r#"type_product"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"type_application"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_application"#.to_string() },
                        ] }) },
                ] },
                line_number: 120,
            },
            GrammarRule {
                name: r#"type_application"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"type_atom"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"type_atom"#.to_string() }) },
                ] },
                line_number: 122,
            },
            GrammarRule {
                name: r#"generic_type_arguments"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                ] },
                line_number: 124,
            },
            GrammarRule {
                name: r#"type_atom"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"generic_type_arguments"#.to_string() }) },
                    ] },
                    GrammarElement::TokenReference { name: r#"TYPEVAR"#.to_string() },
                    GrammarElement::RuleReference { name: r#"tuple_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"unit_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"parenthesized_type"#.to_string() },
                ] },
                line_number: 126,
            },
            GrammarRule {
                name: r#"tuple_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 133,
            },
            GrammarRule {
                name: r#"unit_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 135,
            },
            GrammarRule {
                name: r#"parenthesized_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 137,
            },
            GrammarRule {
                name: r#"qualified_name"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 139,
            },
            GrammarRule {
                name: r#"expression"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"if_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"match_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"let_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"function_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"fun_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"for_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"while_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"sequence_expression"#.to_string() },
                ] },
                line_number: 141,
            },
            GrammarRule {
                name: r#"sequence_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"infix_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                                GrammarElement::RuleReference { name: r#"infix_expression"#.to_string() },
                            ] },
                            GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                                GrammarElement::RuleReference { name: r#"infix_expression"#.to_string() },
                            ] },
                        ] }) },
                ] },
                line_number: 150,
            },
            GrammarRule {
                name: r#"infix_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"application_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"infix_operator"#.to_string() },
                            GrammarElement::RuleReference { name: r#"application_expression"#.to_string() },
                        ] }) },
                ] },
                line_number: 152,
            },
            GrammarRule {
                name: r#"infix_operator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"PIPE_RIGHT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PIPE_LEFT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMPOSE_RIGHT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMPOSE_LEFT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NOT_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"GREATER_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"DOUBLE_COLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LARROW"#.to_string() },
                    GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                    GrammarElement::TokenReference { name: r#"DOT_DOT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"AND_AND"#.to_string() },
                    GrammarElement::TokenReference { name: r#"OR_OR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SLASH"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PERCENT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"AMPERSAND"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PIPE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CARET"#.to_string() },
                ] },
                line_number: 154,
            },
            GrammarRule {
                name: r#"application_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"prefix_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"prefix_expression"#.to_string() }) },
                ] },
                line_number: 181,
            },
            GrammarRule {
                name: r#"prefix_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"unary_operator"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"atomic_expression"#.to_string() },
                ] },
                line_number: 183,
            },
            GrammarRule {
                name: r#"unary_operator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"BANG"#.to_string() },
                ] },
                line_number: 185,
            },
            GrammarRule {
                name: r#"atomic_expression"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"computation_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"if_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"match_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"let_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"function_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"fun_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"for_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"while_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"unit_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"tuple_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"list_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"array_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"parenthesized_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CHAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"TRUE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"FALSE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NULL"#.to_string() },
                ] },
                line_number: 189,
            },
            GrammarRule {
                name: r#"computation_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"expression"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 211,
            },
            GrammarRule {
                name: r#"unit_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 213,
            },
            GrammarRule {
                name: r#"tuple_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 215,
            },
            GrammarRule {
                name: r#"list_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                    GrammarElement::NegativeLookahead { element: Box::new(GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"element_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                ] },
                line_number: 217,
            },
            GrammarRule {
                name: r#"array_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"ARRAY_LBRACKET"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"element_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"ARRAY_RBRACKET"#.to_string() },
                ] },
                line_number: 219,
            },
            GrammarRule {
                name: r#"record_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"field_assignment"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"field_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"field_assignment"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 221,
            },
            GrammarRule {
                name: r#"field_assignment"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 223,
            },
            GrammarRule {
                name: r#"parenthesized_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 225,
            },
            GrammarRule {
                name: r#"element_separator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                ] },
                line_number: 227,
            },
            GrammarRule {
                name: r#"if_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"if"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Literal { value: r#"then"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"else"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                ] },
                line_number: 230,
            },
            GrammarRule {
                name: r#"match_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"match"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Literal { value: r#"with"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"case_separator"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"match_case"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"case_separator"#.to_string() },
                            GrammarElement::RuleReference { name: r#"match_case"#.to_string() },
                        ] }) },
                ] },
                line_number: 232,
            },
            GrammarRule {
                name: r#"match_case"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"when"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 234,
            },
            GrammarRule {
                name: r#"fun_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"fun"#.to_string() },
                    GrammarElement::RuleReference { name: r#"parameter_list"#.to_string() },
                    GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 236,
            },
            GrammarRule {
                name: r#"function_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"function"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"case_separator"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"match_case"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"case_separator"#.to_string() },
                            GrammarElement::RuleReference { name: r#"match_case"#.to_string() },
                        ] }) },
                ] },
                line_number: 238,
            },
            GrammarRule {
                name: r#"let_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"let"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"rec"#.to_string() }) },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"and"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"in"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 240,
            },
            GrammarRule {
                name: r#"for_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"for"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::Literal { value: r#"to"#.to_string() },
                            GrammarElement::Literal { value: r#"downto"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Literal { value: r#"do"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 242,
            },
            GrammarRule {
                name: r#"while_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"while"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Literal { value: r#"do"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 244,
            },
            GrammarRule {
                name: r#"pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"pattern_atom"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"as"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 246,
            },
            GrammarRule {
                name: r#"pattern_atom"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"wildcard_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"literal_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"tuple_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"list_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"array_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"unit_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"parenthesized_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::TokenReference { name: r#"TYPEVAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                ] },
                line_number: 248,
            },
            GrammarRule {
                name: r#"wildcard_pattern"#.to_string(),
                body: GrammarElement::TokenReference { name: r#"UNDERSCORE"#.to_string() },
                line_number: 260,
            },
            GrammarRule {
                name: r#"literal_pattern"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CHAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"TRUE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"FALSE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NULL"#.to_string() },
                ] },
                line_number: 262,
            },
            GrammarRule {
                name: r#"tuple_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 269,
            },
            GrammarRule {
                name: r#"list_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"element_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                ] },
                line_number: 271,
            },
            GrammarRule {
                name: r#"array_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"ARRAY_LBRACKET"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"element_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"ARRAY_RBRACKET"#.to_string() },
                ] },
                line_number: 273,
            },
            GrammarRule {
                name: r#"record_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"field_pattern"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"field_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"field_pattern"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 275,
            },
            GrammarRule {
                name: r#"field_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                ] },
                line_number: 277,
            },
            GrammarRule {
                name: r#"unit_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 279,
            },
            GrammarRule {
                name: r#"parenthesized_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 281,
            },
        ],
            version: 1,
        }
    }
}

mod v_4_0 {
    // AUTO-GENERATED FILE — DO NOT EDIT
    // Source: fsharp4.0.grammar
    // Regenerate with: grammar-tools compile-grammar fsharp4.0.grammar
    //
    // This file embeds a ParserGrammar as native Rust data structures.
    // Call `parser_grammar()` instead of reading and parsing the .grammar file.

    use grammar_tools::parser_grammar::{GrammarElement, GrammarRule, ParserGrammar};

    pub fn parser_grammar() -> ParserGrammar {
        ParserGrammar {
            rules: vec![
            GrammarRule {
                name: r#"compilation_unit"#.to_string(),
                body: GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                        GrammarElement::RuleReference { name: r#"decorated_declaration"#.to_string() },
                    ] }) },
                line_number: 9,
            },
            GrammarRule {
                name: r#"decorated_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"attribute_section"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"declaration_body"#.to_string() },
                ] },
                line_number: 11,
            },
            GrammarRule {
                name: r#"declaration_body"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"module_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"namespace_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"open_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"let_binding"#.to_string() },
                    GrammarElement::RuleReference { name: r#"use_binding"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"member_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"do_binding"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 13,
            },
            GrammarRule {
                name: r#"module_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"module"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"module_modifier"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"rec"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                        ] }) },
                ] },
                line_number: 23,
            },
            GrammarRule {
                name: r#"module_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"private"#.to_string() },
                    GrammarElement::Literal { value: r#"internal"#.to_string() },
                ] },
                line_number: 25,
            },
            GrammarRule {
                name: r#"namespace_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"namespace"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"namespace_modifier"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                ] },
                line_number: 28,
            },
            GrammarRule {
                name: r#"namespace_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"private"#.to_string() },
                    GrammarElement::Literal { value: r#"internal"#.to_string() },
                ] },
                line_number: 30,
            },
            GrammarRule {
                name: r#"open_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"open"#.to_string() },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                ] },
                line_number: 33,
            },
            GrammarRule {
                name: r#"let_binding"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"let"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"rec"#.to_string() }) },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"and"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                        ] }) },
                ] },
                line_number: 35,
            },
            GrammarRule {
                name: r#"use_binding"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"use"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"rec"#.to_string() }) },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"and"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                        ] }) },
                ] },
                line_number: 37,
            },
            GrammarRule {
                name: r#"binding_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"inline"#.to_string() },
                    GrammarElement::Literal { value: r#"mutable"#.to_string() },
                ] },
                line_number: 39,
            },
            GrammarRule {
                name: r#"binding_clause"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"pattern"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"type_annotation"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 42,
            },
            GrammarRule {
                name: r#"do_binding"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"do"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 44,
            },
            GrammarRule {
                name: r#"member_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"member_modifier"#.to_string() }) },
                    GrammarElement::Literal { value: r#"member"#.to_string() },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"parameter_list"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"type_annotation"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                ] },
                line_number: 46,
            },
            GrammarRule {
                name: r#"member_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"static"#.to_string() },
                    GrammarElement::Literal { value: r#"override"#.to_string() },
                    GrammarElement::Literal { value: r#"default"#.to_string() },
                    GrammarElement::Literal { value: r#"abstract"#.to_string() },
                    GrammarElement::Literal { value: r#"new"#.to_string() },
                ] },
                line_number: 48,
            },
            GrammarRule {
                name: r#"type_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"type"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"type_modifier"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"generic_parameters"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"parameter_list"#.to_string() }) },
                            GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"type_definition"#.to_string() },
                ] },
                line_number: 54,
            },
            GrammarRule {
                name: r#"type_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"private"#.to_string() },
                    GrammarElement::Literal { value: r#"public"#.to_string() },
                    GrammarElement::Literal { value: r#"internal"#.to_string() },
                ] },
                line_number: 56,
            },
            GrammarRule {
                name: r#"generic_parameters"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_parameter"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_parameter"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                ] },
                line_number: 60,
            },
            GrammarRule {
                name: r#"type_parameter"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"TYPEVAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                ] },
                line_number: 62,
            },
            GrammarRule {
                name: r#"type_definition"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"class_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"interface_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"struct_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"union_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"alias_type"#.to_string() },
                ] },
                line_number: 65,
            },
            GrammarRule {
                name: r#"class_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"class"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                            GrammarElement::RuleReference { name: r#"declaration_body"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"end"#.to_string() },
                ] },
                line_number: 72,
            },
            GrammarRule {
                name: r#"interface_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"interface"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                            GrammarElement::RuleReference { name: r#"declaration_body"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"end"#.to_string() },
                ] },
                line_number: 74,
            },
            GrammarRule {
                name: r#"struct_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"struct"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                            GrammarElement::RuleReference { name: r#"declaration_body"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"end"#.to_string() },
                ] },
                line_number: 76,
            },
            GrammarRule {
                name: r#"record_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"field_declaration"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"field_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"field_declaration"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 78,
            },
            GrammarRule {
                name: r#"field_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                ] },
                line_number: 80,
            },
            GrammarRule {
                name: r#"union_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"union_case"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"case_separator"#.to_string() },
                            GrammarElement::RuleReference { name: r#"union_case"#.to_string() },
                        ] }) },
                ] },
                line_number: 82,
            },
            GrammarRule {
                name: r#"union_case"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"of"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                                ] }) },
                        ] }) },
                ] },
                line_number: 84,
            },
            GrammarRule {
                name: r#"alias_type"#.to_string(),
                body: GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                line_number: 86,
            },
            GrammarRule {
                name: r#"field_separator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                ] },
                line_number: 88,
            },
            GrammarRule {
                name: r#"case_separator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"PIPE"#.to_string() }) },
                    ] },
                    GrammarElement::TokenReference { name: r#"PIPE"#.to_string() },
                ] },
                line_number: 91,
            },
            GrammarRule {
                name: r#"attribute_section"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"attribute"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"attribute"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                ] },
                line_number: 94,
            },
            GrammarRule {
                name: r#"attribute"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"attribute_target"#.to_string() },
                            GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"attribute_arguments"#.to_string() }) },
                            GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                        ] }) },
                ] },
                line_number: 96,
            },
            GrammarRule {
                name: r#"attribute_target"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"assembly"#.to_string() },
                    GrammarElement::Literal { value: r#"field"#.to_string() },
                    GrammarElement::Literal { value: r#"method"#.to_string() },
                    GrammarElement::Literal { value: r#"module"#.to_string() },
                    GrammarElement::Literal { value: r#"param"#.to_string() },
                    GrammarElement::Literal { value: r#"property"#.to_string() },
                    GrammarElement::Literal { value: r#"return"#.to_string() },
                    GrammarElement::Literal { value: r#"type"#.to_string() },
                    GrammarElement::Literal { value: r#"event"#.to_string() },
                ] },
                line_number: 98,
            },
            GrammarRule {
                name: r#"attribute_arguments"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"attribute_argument"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"attribute_argument"#.to_string() },
                        ] }) },
                ] },
                line_number: 108,
            },
            GrammarRule {
                name: r#"attribute_argument"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 110,
            },
            GrammarRule {
                name: r#"parameter_list"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"pattern"#.to_string() }) },
                ] },
                line_number: 112,
            },
            GrammarRule {
                name: r#"type_annotation"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                ] },
                line_number: 114,
            },
            GrammarRule {
                name: r#"type_expression"#.to_string(),
                body: GrammarElement::RuleReference { name: r#"function_type"#.to_string() },
                line_number: 116,
            },
            GrammarRule {
                name: r#"function_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"type_product"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                            GrammarElement::RuleReference { name: r#"function_type"#.to_string() },
                        ] }) },
                ] },
                line_number: 118,
            },
            GrammarRule {
                name: r#"type_product"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"type_application"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_application"#.to_string() },
                        ] }) },
                ] },
                line_number: 120,
            },
            GrammarRule {
                name: r#"type_application"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"type_atom"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"type_atom"#.to_string() }) },
                ] },
                line_number: 122,
            },
            GrammarRule {
                name: r#"generic_type_arguments"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                ] },
                line_number: 124,
            },
            GrammarRule {
                name: r#"type_atom"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"generic_type_arguments"#.to_string() }) },
                    ] },
                    GrammarElement::TokenReference { name: r#"TYPEVAR"#.to_string() },
                    GrammarElement::RuleReference { name: r#"tuple_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"unit_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"parenthesized_type"#.to_string() },
                ] },
                line_number: 126,
            },
            GrammarRule {
                name: r#"tuple_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 133,
            },
            GrammarRule {
                name: r#"unit_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 135,
            },
            GrammarRule {
                name: r#"parenthesized_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 137,
            },
            GrammarRule {
                name: r#"qualified_name"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 139,
            },
            GrammarRule {
                name: r#"expression"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"if_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"match_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"let_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"function_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"fun_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"for_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"while_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"sequence_expression"#.to_string() },
                ] },
                line_number: 141,
            },
            GrammarRule {
                name: r#"sequence_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"infix_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                                GrammarElement::RuleReference { name: r#"infix_expression"#.to_string() },
                            ] },
                            GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                                GrammarElement::RuleReference { name: r#"infix_expression"#.to_string() },
                            ] },
                        ] }) },
                ] },
                line_number: 150,
            },
            GrammarRule {
                name: r#"infix_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"application_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"infix_operator"#.to_string() },
                            GrammarElement::RuleReference { name: r#"application_expression"#.to_string() },
                        ] }) },
                ] },
                line_number: 152,
            },
            GrammarRule {
                name: r#"infix_operator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"PIPE_RIGHT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PIPE_LEFT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMPOSE_RIGHT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMPOSE_LEFT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NOT_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"GREATER_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"DOUBLE_COLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LARROW"#.to_string() },
                    GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                    GrammarElement::TokenReference { name: r#"DOT_DOT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"AND_AND"#.to_string() },
                    GrammarElement::TokenReference { name: r#"OR_OR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SLASH"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PERCENT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"AMPERSAND"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PIPE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CARET"#.to_string() },
                ] },
                line_number: 154,
            },
            GrammarRule {
                name: r#"application_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"prefix_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"prefix_expression"#.to_string() }) },
                ] },
                line_number: 181,
            },
            GrammarRule {
                name: r#"prefix_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"unary_operator"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"atomic_expression"#.to_string() },
                ] },
                line_number: 183,
            },
            GrammarRule {
                name: r#"unary_operator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"BANG"#.to_string() },
                ] },
                line_number: 185,
            },
            GrammarRule {
                name: r#"atomic_expression"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"computation_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"if_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"match_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"let_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"function_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"fun_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"for_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"while_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"unit_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"tuple_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"list_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"array_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"parenthesized_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CHAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"TRUE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"FALSE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NULL"#.to_string() },
                ] },
                line_number: 189,
            },
            GrammarRule {
                name: r#"computation_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"expression"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 211,
            },
            GrammarRule {
                name: r#"unit_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 213,
            },
            GrammarRule {
                name: r#"tuple_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 215,
            },
            GrammarRule {
                name: r#"list_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                    GrammarElement::NegativeLookahead { element: Box::new(GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"element_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                ] },
                line_number: 217,
            },
            GrammarRule {
                name: r#"array_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"ARRAY_LBRACKET"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"element_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"ARRAY_RBRACKET"#.to_string() },
                ] },
                line_number: 219,
            },
            GrammarRule {
                name: r#"record_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"field_assignment"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"field_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"field_assignment"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 221,
            },
            GrammarRule {
                name: r#"field_assignment"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 223,
            },
            GrammarRule {
                name: r#"parenthesized_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 225,
            },
            GrammarRule {
                name: r#"element_separator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                ] },
                line_number: 227,
            },
            GrammarRule {
                name: r#"if_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"if"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Literal { value: r#"then"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"else"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                ] },
                line_number: 230,
            },
            GrammarRule {
                name: r#"match_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"match"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Literal { value: r#"with"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"case_separator"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"match_case"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"case_separator"#.to_string() },
                            GrammarElement::RuleReference { name: r#"match_case"#.to_string() },
                        ] }) },
                ] },
                line_number: 232,
            },
            GrammarRule {
                name: r#"match_case"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"when"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 234,
            },
            GrammarRule {
                name: r#"fun_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"fun"#.to_string() },
                    GrammarElement::RuleReference { name: r#"parameter_list"#.to_string() },
                    GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 236,
            },
            GrammarRule {
                name: r#"function_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"function"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"case_separator"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"match_case"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"case_separator"#.to_string() },
                            GrammarElement::RuleReference { name: r#"match_case"#.to_string() },
                        ] }) },
                ] },
                line_number: 238,
            },
            GrammarRule {
                name: r#"let_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"let"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"rec"#.to_string() }) },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"and"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"in"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 240,
            },
            GrammarRule {
                name: r#"for_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"for"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::Literal { value: r#"to"#.to_string() },
                            GrammarElement::Literal { value: r#"downto"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Literal { value: r#"do"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 242,
            },
            GrammarRule {
                name: r#"while_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"while"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Literal { value: r#"do"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 244,
            },
            GrammarRule {
                name: r#"pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"pattern_atom"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"as"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 246,
            },
            GrammarRule {
                name: r#"pattern_atom"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"wildcard_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"literal_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"tuple_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"list_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"array_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"unit_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"parenthesized_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::TokenReference { name: r#"TYPEVAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                ] },
                line_number: 248,
            },
            GrammarRule {
                name: r#"wildcard_pattern"#.to_string(),
                body: GrammarElement::TokenReference { name: r#"UNDERSCORE"#.to_string() },
                line_number: 260,
            },
            GrammarRule {
                name: r#"literal_pattern"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CHAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"TRUE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"FALSE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NULL"#.to_string() },
                ] },
                line_number: 262,
            },
            GrammarRule {
                name: r#"tuple_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 269,
            },
            GrammarRule {
                name: r#"list_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"element_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                ] },
                line_number: 271,
            },
            GrammarRule {
                name: r#"array_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"ARRAY_LBRACKET"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"element_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"ARRAY_RBRACKET"#.to_string() },
                ] },
                line_number: 273,
            },
            GrammarRule {
                name: r#"record_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"field_pattern"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"field_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"field_pattern"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 275,
            },
            GrammarRule {
                name: r#"field_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                ] },
                line_number: 277,
            },
            GrammarRule {
                name: r#"unit_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 279,
            },
            GrammarRule {
                name: r#"parenthesized_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 281,
            },
        ],
            version: 1,
        }
    }
}

mod v_4_1 {
    // AUTO-GENERATED FILE — DO NOT EDIT
    // Source: fsharp4.1.grammar
    // Regenerate with: grammar-tools compile-grammar fsharp4.1.grammar
    //
    // This file embeds a ParserGrammar as native Rust data structures.
    // Call `parser_grammar()` instead of reading and parsing the .grammar file.

    use grammar_tools::parser_grammar::{GrammarElement, GrammarRule, ParserGrammar};

    pub fn parser_grammar() -> ParserGrammar {
        ParserGrammar {
            rules: vec![
            GrammarRule {
                name: r#"compilation_unit"#.to_string(),
                body: GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                        GrammarElement::RuleReference { name: r#"decorated_declaration"#.to_string() },
                    ] }) },
                line_number: 9,
            },
            GrammarRule {
                name: r#"decorated_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"attribute_section"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"declaration_body"#.to_string() },
                ] },
                line_number: 11,
            },
            GrammarRule {
                name: r#"declaration_body"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"module_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"namespace_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"open_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"let_binding"#.to_string() },
                    GrammarElement::RuleReference { name: r#"use_binding"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"member_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"do_binding"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 13,
            },
            GrammarRule {
                name: r#"module_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"module"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"module_modifier"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"rec"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                        ] }) },
                ] },
                line_number: 23,
            },
            GrammarRule {
                name: r#"module_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"private"#.to_string() },
                    GrammarElement::Literal { value: r#"internal"#.to_string() },
                ] },
                line_number: 25,
            },
            GrammarRule {
                name: r#"namespace_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"namespace"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"namespace_modifier"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                ] },
                line_number: 28,
            },
            GrammarRule {
                name: r#"namespace_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"private"#.to_string() },
                    GrammarElement::Literal { value: r#"internal"#.to_string() },
                ] },
                line_number: 30,
            },
            GrammarRule {
                name: r#"open_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"open"#.to_string() },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                ] },
                line_number: 33,
            },
            GrammarRule {
                name: r#"let_binding"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"let"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"rec"#.to_string() }) },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"and"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                        ] }) },
                ] },
                line_number: 35,
            },
            GrammarRule {
                name: r#"use_binding"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"use"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"rec"#.to_string() }) },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"and"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                        ] }) },
                ] },
                line_number: 37,
            },
            GrammarRule {
                name: r#"binding_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"inline"#.to_string() },
                    GrammarElement::Literal { value: r#"mutable"#.to_string() },
                ] },
                line_number: 39,
            },
            GrammarRule {
                name: r#"binding_clause"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"pattern"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"type_annotation"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 42,
            },
            GrammarRule {
                name: r#"do_binding"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"do"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 44,
            },
            GrammarRule {
                name: r#"member_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"member_modifier"#.to_string() }) },
                    GrammarElement::Literal { value: r#"member"#.to_string() },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"parameter_list"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"type_annotation"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                ] },
                line_number: 46,
            },
            GrammarRule {
                name: r#"member_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"static"#.to_string() },
                    GrammarElement::Literal { value: r#"override"#.to_string() },
                    GrammarElement::Literal { value: r#"default"#.to_string() },
                    GrammarElement::Literal { value: r#"abstract"#.to_string() },
                    GrammarElement::Literal { value: r#"new"#.to_string() },
                ] },
                line_number: 48,
            },
            GrammarRule {
                name: r#"type_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"type"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"type_modifier"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"generic_parameters"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"parameter_list"#.to_string() }) },
                            GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"type_definition"#.to_string() },
                ] },
                line_number: 54,
            },
            GrammarRule {
                name: r#"type_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"private"#.to_string() },
                    GrammarElement::Literal { value: r#"public"#.to_string() },
                    GrammarElement::Literal { value: r#"internal"#.to_string() },
                ] },
                line_number: 56,
            },
            GrammarRule {
                name: r#"generic_parameters"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_parameter"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_parameter"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                ] },
                line_number: 60,
            },
            GrammarRule {
                name: r#"type_parameter"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"TYPEVAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                ] },
                line_number: 62,
            },
            GrammarRule {
                name: r#"type_definition"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"class_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"interface_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"struct_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"union_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"alias_type"#.to_string() },
                ] },
                line_number: 65,
            },
            GrammarRule {
                name: r#"class_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"class"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                            GrammarElement::RuleReference { name: r#"declaration_body"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"end"#.to_string() },
                ] },
                line_number: 72,
            },
            GrammarRule {
                name: r#"interface_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"interface"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                            GrammarElement::RuleReference { name: r#"declaration_body"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"end"#.to_string() },
                ] },
                line_number: 74,
            },
            GrammarRule {
                name: r#"struct_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"struct"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                            GrammarElement::RuleReference { name: r#"declaration_body"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"end"#.to_string() },
                ] },
                line_number: 76,
            },
            GrammarRule {
                name: r#"record_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"field_declaration"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"field_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"field_declaration"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 78,
            },
            GrammarRule {
                name: r#"field_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                ] },
                line_number: 80,
            },
            GrammarRule {
                name: r#"union_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"union_case"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"case_separator"#.to_string() },
                            GrammarElement::RuleReference { name: r#"union_case"#.to_string() },
                        ] }) },
                ] },
                line_number: 82,
            },
            GrammarRule {
                name: r#"union_case"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"of"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                                ] }) },
                        ] }) },
                ] },
                line_number: 84,
            },
            GrammarRule {
                name: r#"alias_type"#.to_string(),
                body: GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                line_number: 86,
            },
            GrammarRule {
                name: r#"field_separator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                ] },
                line_number: 88,
            },
            GrammarRule {
                name: r#"case_separator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"PIPE"#.to_string() }) },
                    ] },
                    GrammarElement::TokenReference { name: r#"PIPE"#.to_string() },
                ] },
                line_number: 91,
            },
            GrammarRule {
                name: r#"attribute_section"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"attribute"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"attribute"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                ] },
                line_number: 94,
            },
            GrammarRule {
                name: r#"attribute"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"attribute_target"#.to_string() },
                            GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"attribute_arguments"#.to_string() }) },
                            GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                        ] }) },
                ] },
                line_number: 96,
            },
            GrammarRule {
                name: r#"attribute_target"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"assembly"#.to_string() },
                    GrammarElement::Literal { value: r#"field"#.to_string() },
                    GrammarElement::Literal { value: r#"method"#.to_string() },
                    GrammarElement::Literal { value: r#"module"#.to_string() },
                    GrammarElement::Literal { value: r#"param"#.to_string() },
                    GrammarElement::Literal { value: r#"property"#.to_string() },
                    GrammarElement::Literal { value: r#"return"#.to_string() },
                    GrammarElement::Literal { value: r#"type"#.to_string() },
                    GrammarElement::Literal { value: r#"event"#.to_string() },
                ] },
                line_number: 98,
            },
            GrammarRule {
                name: r#"attribute_arguments"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"attribute_argument"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"attribute_argument"#.to_string() },
                        ] }) },
                ] },
                line_number: 108,
            },
            GrammarRule {
                name: r#"attribute_argument"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 110,
            },
            GrammarRule {
                name: r#"parameter_list"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"pattern"#.to_string() }) },
                ] },
                line_number: 112,
            },
            GrammarRule {
                name: r#"type_annotation"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                ] },
                line_number: 114,
            },
            GrammarRule {
                name: r#"type_expression"#.to_string(),
                body: GrammarElement::RuleReference { name: r#"function_type"#.to_string() },
                line_number: 116,
            },
            GrammarRule {
                name: r#"function_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"type_product"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                            GrammarElement::RuleReference { name: r#"function_type"#.to_string() },
                        ] }) },
                ] },
                line_number: 118,
            },
            GrammarRule {
                name: r#"type_product"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"type_application"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_application"#.to_string() },
                        ] }) },
                ] },
                line_number: 120,
            },
            GrammarRule {
                name: r#"type_application"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"type_atom"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"type_atom"#.to_string() }) },
                ] },
                line_number: 122,
            },
            GrammarRule {
                name: r#"generic_type_arguments"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                ] },
                line_number: 124,
            },
            GrammarRule {
                name: r#"type_atom"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"generic_type_arguments"#.to_string() }) },
                    ] },
                    GrammarElement::TokenReference { name: r#"TYPEVAR"#.to_string() },
                    GrammarElement::RuleReference { name: r#"tuple_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"unit_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"parenthesized_type"#.to_string() },
                ] },
                line_number: 126,
            },
            GrammarRule {
                name: r#"tuple_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 133,
            },
            GrammarRule {
                name: r#"unit_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 135,
            },
            GrammarRule {
                name: r#"parenthesized_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 137,
            },
            GrammarRule {
                name: r#"qualified_name"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 139,
            },
            GrammarRule {
                name: r#"expression"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"if_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"match_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"let_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"function_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"fun_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"for_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"while_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"sequence_expression"#.to_string() },
                ] },
                line_number: 141,
            },
            GrammarRule {
                name: r#"sequence_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"infix_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                                GrammarElement::RuleReference { name: r#"infix_expression"#.to_string() },
                            ] },
                            GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                                GrammarElement::RuleReference { name: r#"infix_expression"#.to_string() },
                            ] },
                        ] }) },
                ] },
                line_number: 150,
            },
            GrammarRule {
                name: r#"infix_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"application_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"infix_operator"#.to_string() },
                            GrammarElement::RuleReference { name: r#"application_expression"#.to_string() },
                        ] }) },
                ] },
                line_number: 152,
            },
            GrammarRule {
                name: r#"infix_operator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"PIPE_RIGHT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PIPE_LEFT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMPOSE_RIGHT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMPOSE_LEFT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NOT_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"GREATER_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"DOUBLE_COLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LARROW"#.to_string() },
                    GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                    GrammarElement::TokenReference { name: r#"DOT_DOT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"AND_AND"#.to_string() },
                    GrammarElement::TokenReference { name: r#"OR_OR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SLASH"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PERCENT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"AMPERSAND"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PIPE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CARET"#.to_string() },
                ] },
                line_number: 154,
            },
            GrammarRule {
                name: r#"application_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"prefix_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"prefix_expression"#.to_string() }) },
                ] },
                line_number: 181,
            },
            GrammarRule {
                name: r#"prefix_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"unary_operator"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"atomic_expression"#.to_string() },
                ] },
                line_number: 183,
            },
            GrammarRule {
                name: r#"unary_operator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"BANG"#.to_string() },
                ] },
                line_number: 185,
            },
            GrammarRule {
                name: r#"atomic_expression"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"computation_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"if_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"match_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"let_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"function_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"fun_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"for_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"while_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"unit_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"tuple_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"list_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"array_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"parenthesized_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CHAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"TRUE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"FALSE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NULL"#.to_string() },
                ] },
                line_number: 189,
            },
            GrammarRule {
                name: r#"computation_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"expression"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 211,
            },
            GrammarRule {
                name: r#"unit_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 213,
            },
            GrammarRule {
                name: r#"tuple_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 215,
            },
            GrammarRule {
                name: r#"list_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                    GrammarElement::NegativeLookahead { element: Box::new(GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"element_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                ] },
                line_number: 217,
            },
            GrammarRule {
                name: r#"array_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"ARRAY_LBRACKET"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"element_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"ARRAY_RBRACKET"#.to_string() },
                ] },
                line_number: 219,
            },
            GrammarRule {
                name: r#"record_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"field_assignment"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"field_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"field_assignment"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 221,
            },
            GrammarRule {
                name: r#"field_assignment"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 223,
            },
            GrammarRule {
                name: r#"parenthesized_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 225,
            },
            GrammarRule {
                name: r#"element_separator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                ] },
                line_number: 227,
            },
            GrammarRule {
                name: r#"if_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"if"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Literal { value: r#"then"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"else"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                ] },
                line_number: 230,
            },
            GrammarRule {
                name: r#"match_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"match"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Literal { value: r#"with"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"case_separator"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"match_case"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"case_separator"#.to_string() },
                            GrammarElement::RuleReference { name: r#"match_case"#.to_string() },
                        ] }) },
                ] },
                line_number: 232,
            },
            GrammarRule {
                name: r#"match_case"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"when"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 234,
            },
            GrammarRule {
                name: r#"fun_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"fun"#.to_string() },
                    GrammarElement::RuleReference { name: r#"parameter_list"#.to_string() },
                    GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 236,
            },
            GrammarRule {
                name: r#"function_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"function"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"case_separator"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"match_case"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"case_separator"#.to_string() },
                            GrammarElement::RuleReference { name: r#"match_case"#.to_string() },
                        ] }) },
                ] },
                line_number: 238,
            },
            GrammarRule {
                name: r#"let_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"let"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"rec"#.to_string() }) },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"and"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"in"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 240,
            },
            GrammarRule {
                name: r#"for_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"for"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::Literal { value: r#"to"#.to_string() },
                            GrammarElement::Literal { value: r#"downto"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Literal { value: r#"do"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 242,
            },
            GrammarRule {
                name: r#"while_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"while"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Literal { value: r#"do"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 244,
            },
            GrammarRule {
                name: r#"pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"pattern_atom"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"as"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 246,
            },
            GrammarRule {
                name: r#"pattern_atom"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"wildcard_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"literal_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"tuple_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"list_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"array_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"unit_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"parenthesized_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::TokenReference { name: r#"TYPEVAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                ] },
                line_number: 248,
            },
            GrammarRule {
                name: r#"wildcard_pattern"#.to_string(),
                body: GrammarElement::TokenReference { name: r#"UNDERSCORE"#.to_string() },
                line_number: 260,
            },
            GrammarRule {
                name: r#"literal_pattern"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CHAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"TRUE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"FALSE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NULL"#.to_string() },
                ] },
                line_number: 262,
            },
            GrammarRule {
                name: r#"tuple_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 269,
            },
            GrammarRule {
                name: r#"list_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"element_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                ] },
                line_number: 271,
            },
            GrammarRule {
                name: r#"array_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"ARRAY_LBRACKET"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"element_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"ARRAY_RBRACKET"#.to_string() },
                ] },
                line_number: 273,
            },
            GrammarRule {
                name: r#"record_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"field_pattern"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"field_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"field_pattern"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 275,
            },
            GrammarRule {
                name: r#"field_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                ] },
                line_number: 277,
            },
            GrammarRule {
                name: r#"unit_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 279,
            },
            GrammarRule {
                name: r#"parenthesized_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 281,
            },
        ],
            version: 1,
        }
    }
}

mod v_4_5 {
    // AUTO-GENERATED FILE — DO NOT EDIT
    // Source: fsharp4.5.grammar
    // Regenerate with: grammar-tools compile-grammar fsharp4.5.grammar
    //
    // This file embeds a ParserGrammar as native Rust data structures.
    // Call `parser_grammar()` instead of reading and parsing the .grammar file.

    use grammar_tools::parser_grammar::{GrammarElement, GrammarRule, ParserGrammar};

    pub fn parser_grammar() -> ParserGrammar {
        ParserGrammar {
            rules: vec![
            GrammarRule {
                name: r#"compilation_unit"#.to_string(),
                body: GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                        GrammarElement::RuleReference { name: r#"decorated_declaration"#.to_string() },
                    ] }) },
                line_number: 9,
            },
            GrammarRule {
                name: r#"decorated_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"attribute_section"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"declaration_body"#.to_string() },
                ] },
                line_number: 11,
            },
            GrammarRule {
                name: r#"declaration_body"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"module_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"namespace_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"open_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"let_binding"#.to_string() },
                    GrammarElement::RuleReference { name: r#"use_binding"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"member_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"do_binding"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 13,
            },
            GrammarRule {
                name: r#"module_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"module"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"module_modifier"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"rec"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                        ] }) },
                ] },
                line_number: 23,
            },
            GrammarRule {
                name: r#"module_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"private"#.to_string() },
                    GrammarElement::Literal { value: r#"internal"#.to_string() },
                ] },
                line_number: 25,
            },
            GrammarRule {
                name: r#"namespace_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"namespace"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"namespace_modifier"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                ] },
                line_number: 28,
            },
            GrammarRule {
                name: r#"namespace_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"private"#.to_string() },
                    GrammarElement::Literal { value: r#"internal"#.to_string() },
                ] },
                line_number: 30,
            },
            GrammarRule {
                name: r#"open_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"open"#.to_string() },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                ] },
                line_number: 33,
            },
            GrammarRule {
                name: r#"let_binding"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"let"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"rec"#.to_string() }) },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"and"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                        ] }) },
                ] },
                line_number: 35,
            },
            GrammarRule {
                name: r#"use_binding"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"use"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"rec"#.to_string() }) },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"and"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                        ] }) },
                ] },
                line_number: 37,
            },
            GrammarRule {
                name: r#"binding_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"inline"#.to_string() },
                    GrammarElement::Literal { value: r#"mutable"#.to_string() },
                ] },
                line_number: 39,
            },
            GrammarRule {
                name: r#"binding_clause"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"pattern"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"type_annotation"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 42,
            },
            GrammarRule {
                name: r#"do_binding"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"do"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 44,
            },
            GrammarRule {
                name: r#"member_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"member_modifier"#.to_string() }) },
                    GrammarElement::Literal { value: r#"member"#.to_string() },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"parameter_list"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"type_annotation"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                ] },
                line_number: 46,
            },
            GrammarRule {
                name: r#"member_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"static"#.to_string() },
                    GrammarElement::Literal { value: r#"override"#.to_string() },
                    GrammarElement::Literal { value: r#"default"#.to_string() },
                    GrammarElement::Literal { value: r#"abstract"#.to_string() },
                    GrammarElement::Literal { value: r#"new"#.to_string() },
                ] },
                line_number: 48,
            },
            GrammarRule {
                name: r#"type_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"type"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"type_modifier"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"generic_parameters"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"parameter_list"#.to_string() }) },
                            GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"type_definition"#.to_string() },
                ] },
                line_number: 54,
            },
            GrammarRule {
                name: r#"type_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"private"#.to_string() },
                    GrammarElement::Literal { value: r#"public"#.to_string() },
                    GrammarElement::Literal { value: r#"internal"#.to_string() },
                ] },
                line_number: 56,
            },
            GrammarRule {
                name: r#"generic_parameters"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_parameter"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_parameter"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                ] },
                line_number: 60,
            },
            GrammarRule {
                name: r#"type_parameter"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"TYPEVAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                ] },
                line_number: 62,
            },
            GrammarRule {
                name: r#"type_definition"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"class_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"interface_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"struct_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"union_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"alias_type"#.to_string() },
                ] },
                line_number: 65,
            },
            GrammarRule {
                name: r#"class_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"class"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                            GrammarElement::RuleReference { name: r#"declaration_body"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"end"#.to_string() },
                ] },
                line_number: 72,
            },
            GrammarRule {
                name: r#"interface_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"interface"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                            GrammarElement::RuleReference { name: r#"declaration_body"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"end"#.to_string() },
                ] },
                line_number: 74,
            },
            GrammarRule {
                name: r#"struct_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"struct"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                            GrammarElement::RuleReference { name: r#"declaration_body"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"end"#.to_string() },
                ] },
                line_number: 76,
            },
            GrammarRule {
                name: r#"record_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"field_declaration"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"field_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"field_declaration"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 78,
            },
            GrammarRule {
                name: r#"field_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                ] },
                line_number: 80,
            },
            GrammarRule {
                name: r#"union_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"union_case"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"case_separator"#.to_string() },
                            GrammarElement::RuleReference { name: r#"union_case"#.to_string() },
                        ] }) },
                ] },
                line_number: 82,
            },
            GrammarRule {
                name: r#"union_case"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"of"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                                ] }) },
                        ] }) },
                ] },
                line_number: 84,
            },
            GrammarRule {
                name: r#"alias_type"#.to_string(),
                body: GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                line_number: 86,
            },
            GrammarRule {
                name: r#"field_separator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                ] },
                line_number: 88,
            },
            GrammarRule {
                name: r#"case_separator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"PIPE"#.to_string() }) },
                    ] },
                    GrammarElement::TokenReference { name: r#"PIPE"#.to_string() },
                ] },
                line_number: 91,
            },
            GrammarRule {
                name: r#"attribute_section"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"attribute"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"attribute"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                ] },
                line_number: 94,
            },
            GrammarRule {
                name: r#"attribute"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"attribute_target"#.to_string() },
                            GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"attribute_arguments"#.to_string() }) },
                            GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                        ] }) },
                ] },
                line_number: 96,
            },
            GrammarRule {
                name: r#"attribute_target"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"assembly"#.to_string() },
                    GrammarElement::Literal { value: r#"field"#.to_string() },
                    GrammarElement::Literal { value: r#"method"#.to_string() },
                    GrammarElement::Literal { value: r#"module"#.to_string() },
                    GrammarElement::Literal { value: r#"param"#.to_string() },
                    GrammarElement::Literal { value: r#"property"#.to_string() },
                    GrammarElement::Literal { value: r#"return"#.to_string() },
                    GrammarElement::Literal { value: r#"type"#.to_string() },
                    GrammarElement::Literal { value: r#"event"#.to_string() },
                ] },
                line_number: 98,
            },
            GrammarRule {
                name: r#"attribute_arguments"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"attribute_argument"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"attribute_argument"#.to_string() },
                        ] }) },
                ] },
                line_number: 108,
            },
            GrammarRule {
                name: r#"attribute_argument"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 110,
            },
            GrammarRule {
                name: r#"parameter_list"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"pattern"#.to_string() }) },
                ] },
                line_number: 112,
            },
            GrammarRule {
                name: r#"type_annotation"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                ] },
                line_number: 114,
            },
            GrammarRule {
                name: r#"type_expression"#.to_string(),
                body: GrammarElement::RuleReference { name: r#"function_type"#.to_string() },
                line_number: 116,
            },
            GrammarRule {
                name: r#"function_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"type_product"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                            GrammarElement::RuleReference { name: r#"function_type"#.to_string() },
                        ] }) },
                ] },
                line_number: 118,
            },
            GrammarRule {
                name: r#"type_product"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"type_application"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_application"#.to_string() },
                        ] }) },
                ] },
                line_number: 120,
            },
            GrammarRule {
                name: r#"type_application"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"type_atom"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"type_atom"#.to_string() }) },
                ] },
                line_number: 122,
            },
            GrammarRule {
                name: r#"generic_type_arguments"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                ] },
                line_number: 124,
            },
            GrammarRule {
                name: r#"type_atom"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"generic_type_arguments"#.to_string() }) },
                    ] },
                    GrammarElement::TokenReference { name: r#"TYPEVAR"#.to_string() },
                    GrammarElement::RuleReference { name: r#"tuple_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"unit_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"parenthesized_type"#.to_string() },
                ] },
                line_number: 126,
            },
            GrammarRule {
                name: r#"tuple_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 133,
            },
            GrammarRule {
                name: r#"unit_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 135,
            },
            GrammarRule {
                name: r#"parenthesized_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 137,
            },
            GrammarRule {
                name: r#"qualified_name"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 139,
            },
            GrammarRule {
                name: r#"expression"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"if_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"match_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"let_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"function_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"fun_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"for_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"while_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"sequence_expression"#.to_string() },
                ] },
                line_number: 141,
            },
            GrammarRule {
                name: r#"sequence_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"infix_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                                GrammarElement::RuleReference { name: r#"infix_expression"#.to_string() },
                            ] },
                            GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                                GrammarElement::RuleReference { name: r#"infix_expression"#.to_string() },
                            ] },
                        ] }) },
                ] },
                line_number: 150,
            },
            GrammarRule {
                name: r#"infix_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"application_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"infix_operator"#.to_string() },
                            GrammarElement::RuleReference { name: r#"application_expression"#.to_string() },
                        ] }) },
                ] },
                line_number: 152,
            },
            GrammarRule {
                name: r#"infix_operator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"PIPE_RIGHT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PIPE_LEFT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMPOSE_RIGHT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMPOSE_LEFT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NOT_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"GREATER_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"DOUBLE_COLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LARROW"#.to_string() },
                    GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                    GrammarElement::TokenReference { name: r#"DOT_DOT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"AND_AND"#.to_string() },
                    GrammarElement::TokenReference { name: r#"OR_OR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SLASH"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PERCENT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"AMPERSAND"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PIPE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CARET"#.to_string() },
                ] },
                line_number: 154,
            },
            GrammarRule {
                name: r#"application_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"prefix_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"prefix_expression"#.to_string() }) },
                ] },
                line_number: 181,
            },
            GrammarRule {
                name: r#"prefix_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"unary_operator"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"atomic_expression"#.to_string() },
                ] },
                line_number: 183,
            },
            GrammarRule {
                name: r#"unary_operator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"BANG"#.to_string() },
                ] },
                line_number: 185,
            },
            GrammarRule {
                name: r#"atomic_expression"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"computation_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"if_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"match_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"let_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"function_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"fun_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"for_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"while_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"unit_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"tuple_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"list_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"array_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"parenthesized_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CHAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"TRUE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"FALSE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NULL"#.to_string() },
                ] },
                line_number: 189,
            },
            GrammarRule {
                name: r#"computation_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"expression"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 211,
            },
            GrammarRule {
                name: r#"unit_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 213,
            },
            GrammarRule {
                name: r#"tuple_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 215,
            },
            GrammarRule {
                name: r#"list_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                    GrammarElement::NegativeLookahead { element: Box::new(GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"element_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                ] },
                line_number: 217,
            },
            GrammarRule {
                name: r#"array_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"ARRAY_LBRACKET"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"element_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"ARRAY_RBRACKET"#.to_string() },
                ] },
                line_number: 219,
            },
            GrammarRule {
                name: r#"record_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"field_assignment"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"field_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"field_assignment"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 221,
            },
            GrammarRule {
                name: r#"field_assignment"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 223,
            },
            GrammarRule {
                name: r#"parenthesized_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 225,
            },
            GrammarRule {
                name: r#"element_separator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                ] },
                line_number: 227,
            },
            GrammarRule {
                name: r#"if_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"if"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Literal { value: r#"then"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"else"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                ] },
                line_number: 230,
            },
            GrammarRule {
                name: r#"match_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"match"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Literal { value: r#"with"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"case_separator"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"match_case"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"case_separator"#.to_string() },
                            GrammarElement::RuleReference { name: r#"match_case"#.to_string() },
                        ] }) },
                ] },
                line_number: 232,
            },
            GrammarRule {
                name: r#"match_case"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"when"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 234,
            },
            GrammarRule {
                name: r#"fun_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"fun"#.to_string() },
                    GrammarElement::RuleReference { name: r#"parameter_list"#.to_string() },
                    GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 236,
            },
            GrammarRule {
                name: r#"function_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"function"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"case_separator"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"match_case"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"case_separator"#.to_string() },
                            GrammarElement::RuleReference { name: r#"match_case"#.to_string() },
                        ] }) },
                ] },
                line_number: 238,
            },
            GrammarRule {
                name: r#"let_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"let"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"rec"#.to_string() }) },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"and"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"in"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 240,
            },
            GrammarRule {
                name: r#"for_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"for"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::Literal { value: r#"to"#.to_string() },
                            GrammarElement::Literal { value: r#"downto"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Literal { value: r#"do"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 242,
            },
            GrammarRule {
                name: r#"while_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"while"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Literal { value: r#"do"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 244,
            },
            GrammarRule {
                name: r#"pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"pattern_atom"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"as"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 246,
            },
            GrammarRule {
                name: r#"pattern_atom"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"wildcard_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"literal_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"tuple_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"list_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"array_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"unit_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"parenthesized_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::TokenReference { name: r#"TYPEVAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                ] },
                line_number: 248,
            },
            GrammarRule {
                name: r#"wildcard_pattern"#.to_string(),
                body: GrammarElement::TokenReference { name: r#"UNDERSCORE"#.to_string() },
                line_number: 260,
            },
            GrammarRule {
                name: r#"literal_pattern"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CHAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"TRUE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"FALSE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NULL"#.to_string() },
                ] },
                line_number: 262,
            },
            GrammarRule {
                name: r#"tuple_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 269,
            },
            GrammarRule {
                name: r#"list_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"element_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                ] },
                line_number: 271,
            },
            GrammarRule {
                name: r#"array_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"ARRAY_LBRACKET"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"element_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"ARRAY_RBRACKET"#.to_string() },
                ] },
                line_number: 273,
            },
            GrammarRule {
                name: r#"record_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"field_pattern"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"field_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"field_pattern"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 275,
            },
            GrammarRule {
                name: r#"field_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                ] },
                line_number: 277,
            },
            GrammarRule {
                name: r#"unit_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 279,
            },
            GrammarRule {
                name: r#"parenthesized_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 281,
            },
        ],
            version: 1,
        }
    }
}

mod v_4_6 {
    // AUTO-GENERATED FILE — DO NOT EDIT
    // Source: fsharp4.6.grammar
    // Regenerate with: grammar-tools compile-grammar fsharp4.6.grammar
    //
    // This file embeds a ParserGrammar as native Rust data structures.
    // Call `parser_grammar()` instead of reading and parsing the .grammar file.

    use grammar_tools::parser_grammar::{GrammarElement, GrammarRule, ParserGrammar};

    pub fn parser_grammar() -> ParserGrammar {
        ParserGrammar {
            rules: vec![
            GrammarRule {
                name: r#"compilation_unit"#.to_string(),
                body: GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                        GrammarElement::RuleReference { name: r#"decorated_declaration"#.to_string() },
                    ] }) },
                line_number: 9,
            },
            GrammarRule {
                name: r#"decorated_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"attribute_section"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"declaration_body"#.to_string() },
                ] },
                line_number: 11,
            },
            GrammarRule {
                name: r#"declaration_body"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"module_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"namespace_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"open_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"let_binding"#.to_string() },
                    GrammarElement::RuleReference { name: r#"use_binding"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"member_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"do_binding"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 13,
            },
            GrammarRule {
                name: r#"module_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"module"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"module_modifier"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"rec"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                        ] }) },
                ] },
                line_number: 23,
            },
            GrammarRule {
                name: r#"module_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"private"#.to_string() },
                    GrammarElement::Literal { value: r#"internal"#.to_string() },
                ] },
                line_number: 25,
            },
            GrammarRule {
                name: r#"namespace_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"namespace"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"namespace_modifier"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                ] },
                line_number: 28,
            },
            GrammarRule {
                name: r#"namespace_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"private"#.to_string() },
                    GrammarElement::Literal { value: r#"internal"#.to_string() },
                ] },
                line_number: 30,
            },
            GrammarRule {
                name: r#"open_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"open"#.to_string() },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                ] },
                line_number: 33,
            },
            GrammarRule {
                name: r#"let_binding"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"let"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"rec"#.to_string() }) },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"and"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                        ] }) },
                ] },
                line_number: 35,
            },
            GrammarRule {
                name: r#"use_binding"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"use"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"rec"#.to_string() }) },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"and"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                        ] }) },
                ] },
                line_number: 37,
            },
            GrammarRule {
                name: r#"binding_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"inline"#.to_string() },
                    GrammarElement::Literal { value: r#"mutable"#.to_string() },
                ] },
                line_number: 39,
            },
            GrammarRule {
                name: r#"binding_clause"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"pattern"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"type_annotation"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 42,
            },
            GrammarRule {
                name: r#"do_binding"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"do"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 44,
            },
            GrammarRule {
                name: r#"member_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"member_modifier"#.to_string() }) },
                    GrammarElement::Literal { value: r#"member"#.to_string() },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"parameter_list"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"type_annotation"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                ] },
                line_number: 46,
            },
            GrammarRule {
                name: r#"member_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"static"#.to_string() },
                    GrammarElement::Literal { value: r#"override"#.to_string() },
                    GrammarElement::Literal { value: r#"default"#.to_string() },
                    GrammarElement::Literal { value: r#"abstract"#.to_string() },
                    GrammarElement::Literal { value: r#"new"#.to_string() },
                ] },
                line_number: 48,
            },
            GrammarRule {
                name: r#"type_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"type"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"type_modifier"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"generic_parameters"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"parameter_list"#.to_string() }) },
                            GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"type_definition"#.to_string() },
                ] },
                line_number: 54,
            },
            GrammarRule {
                name: r#"type_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"private"#.to_string() },
                    GrammarElement::Literal { value: r#"public"#.to_string() },
                    GrammarElement::Literal { value: r#"internal"#.to_string() },
                ] },
                line_number: 56,
            },
            GrammarRule {
                name: r#"generic_parameters"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_parameter"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_parameter"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                ] },
                line_number: 60,
            },
            GrammarRule {
                name: r#"type_parameter"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"TYPEVAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                ] },
                line_number: 62,
            },
            GrammarRule {
                name: r#"type_definition"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"class_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"interface_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"struct_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"union_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"alias_type"#.to_string() },
                ] },
                line_number: 65,
            },
            GrammarRule {
                name: r#"class_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"class"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                            GrammarElement::RuleReference { name: r#"declaration_body"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"end"#.to_string() },
                ] },
                line_number: 72,
            },
            GrammarRule {
                name: r#"interface_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"interface"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                            GrammarElement::RuleReference { name: r#"declaration_body"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"end"#.to_string() },
                ] },
                line_number: 74,
            },
            GrammarRule {
                name: r#"struct_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"struct"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                            GrammarElement::RuleReference { name: r#"declaration_body"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"end"#.to_string() },
                ] },
                line_number: 76,
            },
            GrammarRule {
                name: r#"record_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"field_declaration"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"field_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"field_declaration"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 78,
            },
            GrammarRule {
                name: r#"field_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                ] },
                line_number: 80,
            },
            GrammarRule {
                name: r#"union_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"union_case"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"case_separator"#.to_string() },
                            GrammarElement::RuleReference { name: r#"union_case"#.to_string() },
                        ] }) },
                ] },
                line_number: 82,
            },
            GrammarRule {
                name: r#"union_case"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"of"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                                ] }) },
                        ] }) },
                ] },
                line_number: 84,
            },
            GrammarRule {
                name: r#"alias_type"#.to_string(),
                body: GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                line_number: 86,
            },
            GrammarRule {
                name: r#"field_separator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                ] },
                line_number: 88,
            },
            GrammarRule {
                name: r#"case_separator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"PIPE"#.to_string() }) },
                    ] },
                    GrammarElement::TokenReference { name: r#"PIPE"#.to_string() },
                ] },
                line_number: 91,
            },
            GrammarRule {
                name: r#"attribute_section"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"attribute"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"attribute"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                ] },
                line_number: 94,
            },
            GrammarRule {
                name: r#"attribute"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"attribute_target"#.to_string() },
                            GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"attribute_arguments"#.to_string() }) },
                            GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                        ] }) },
                ] },
                line_number: 96,
            },
            GrammarRule {
                name: r#"attribute_target"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"assembly"#.to_string() },
                    GrammarElement::Literal { value: r#"field"#.to_string() },
                    GrammarElement::Literal { value: r#"method"#.to_string() },
                    GrammarElement::Literal { value: r#"module"#.to_string() },
                    GrammarElement::Literal { value: r#"param"#.to_string() },
                    GrammarElement::Literal { value: r#"property"#.to_string() },
                    GrammarElement::Literal { value: r#"return"#.to_string() },
                    GrammarElement::Literal { value: r#"type"#.to_string() },
                    GrammarElement::Literal { value: r#"event"#.to_string() },
                ] },
                line_number: 98,
            },
            GrammarRule {
                name: r#"attribute_arguments"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"attribute_argument"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"attribute_argument"#.to_string() },
                        ] }) },
                ] },
                line_number: 108,
            },
            GrammarRule {
                name: r#"attribute_argument"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 110,
            },
            GrammarRule {
                name: r#"parameter_list"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"pattern"#.to_string() }) },
                ] },
                line_number: 112,
            },
            GrammarRule {
                name: r#"type_annotation"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                ] },
                line_number: 114,
            },
            GrammarRule {
                name: r#"type_expression"#.to_string(),
                body: GrammarElement::RuleReference { name: r#"function_type"#.to_string() },
                line_number: 116,
            },
            GrammarRule {
                name: r#"function_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"type_product"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                            GrammarElement::RuleReference { name: r#"function_type"#.to_string() },
                        ] }) },
                ] },
                line_number: 118,
            },
            GrammarRule {
                name: r#"type_product"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"type_application"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_application"#.to_string() },
                        ] }) },
                ] },
                line_number: 120,
            },
            GrammarRule {
                name: r#"type_application"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"type_atom"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"type_atom"#.to_string() }) },
                ] },
                line_number: 122,
            },
            GrammarRule {
                name: r#"generic_type_arguments"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                ] },
                line_number: 124,
            },
            GrammarRule {
                name: r#"type_atom"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"generic_type_arguments"#.to_string() }) },
                    ] },
                    GrammarElement::TokenReference { name: r#"TYPEVAR"#.to_string() },
                    GrammarElement::RuleReference { name: r#"tuple_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"unit_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"parenthesized_type"#.to_string() },
                ] },
                line_number: 126,
            },
            GrammarRule {
                name: r#"tuple_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 133,
            },
            GrammarRule {
                name: r#"unit_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 135,
            },
            GrammarRule {
                name: r#"parenthesized_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 137,
            },
            GrammarRule {
                name: r#"qualified_name"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 139,
            },
            GrammarRule {
                name: r#"expression"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"if_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"match_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"let_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"function_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"fun_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"for_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"while_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"sequence_expression"#.to_string() },
                ] },
                line_number: 141,
            },
            GrammarRule {
                name: r#"sequence_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"infix_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                                GrammarElement::RuleReference { name: r#"infix_expression"#.to_string() },
                            ] },
                            GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                                GrammarElement::RuleReference { name: r#"infix_expression"#.to_string() },
                            ] },
                        ] }) },
                ] },
                line_number: 150,
            },
            GrammarRule {
                name: r#"infix_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"application_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"infix_operator"#.to_string() },
                            GrammarElement::RuleReference { name: r#"application_expression"#.to_string() },
                        ] }) },
                ] },
                line_number: 152,
            },
            GrammarRule {
                name: r#"infix_operator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"PIPE_RIGHT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PIPE_LEFT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMPOSE_RIGHT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMPOSE_LEFT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NOT_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"GREATER_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"DOUBLE_COLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LARROW"#.to_string() },
                    GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                    GrammarElement::TokenReference { name: r#"DOT_DOT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"AND_AND"#.to_string() },
                    GrammarElement::TokenReference { name: r#"OR_OR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SLASH"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PERCENT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"AMPERSAND"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PIPE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CARET"#.to_string() },
                ] },
                line_number: 154,
            },
            GrammarRule {
                name: r#"application_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"prefix_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"prefix_expression"#.to_string() }) },
                ] },
                line_number: 181,
            },
            GrammarRule {
                name: r#"prefix_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"unary_operator"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"atomic_expression"#.to_string() },
                ] },
                line_number: 183,
            },
            GrammarRule {
                name: r#"unary_operator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"BANG"#.to_string() },
                ] },
                line_number: 185,
            },
            GrammarRule {
                name: r#"atomic_expression"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"computation_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"if_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"match_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"let_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"function_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"fun_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"for_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"while_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"unit_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"tuple_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"list_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"array_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"parenthesized_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CHAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"TRUE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"FALSE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NULL"#.to_string() },
                ] },
                line_number: 189,
            },
            GrammarRule {
                name: r#"computation_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"expression"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 211,
            },
            GrammarRule {
                name: r#"unit_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 213,
            },
            GrammarRule {
                name: r#"tuple_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 215,
            },
            GrammarRule {
                name: r#"list_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                    GrammarElement::NegativeLookahead { element: Box::new(GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"element_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                ] },
                line_number: 217,
            },
            GrammarRule {
                name: r#"array_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"ARRAY_LBRACKET"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"element_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"ARRAY_RBRACKET"#.to_string() },
                ] },
                line_number: 219,
            },
            GrammarRule {
                name: r#"record_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"field_assignment"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"field_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"field_assignment"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 221,
            },
            GrammarRule {
                name: r#"field_assignment"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 223,
            },
            GrammarRule {
                name: r#"parenthesized_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 225,
            },
            GrammarRule {
                name: r#"element_separator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                ] },
                line_number: 227,
            },
            GrammarRule {
                name: r#"if_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"if"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Literal { value: r#"then"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"else"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                ] },
                line_number: 230,
            },
            GrammarRule {
                name: r#"match_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"match"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Literal { value: r#"with"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"case_separator"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"match_case"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"case_separator"#.to_string() },
                            GrammarElement::RuleReference { name: r#"match_case"#.to_string() },
                        ] }) },
                ] },
                line_number: 232,
            },
            GrammarRule {
                name: r#"match_case"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"when"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 234,
            },
            GrammarRule {
                name: r#"fun_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"fun"#.to_string() },
                    GrammarElement::RuleReference { name: r#"parameter_list"#.to_string() },
                    GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 236,
            },
            GrammarRule {
                name: r#"function_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"function"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"case_separator"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"match_case"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"case_separator"#.to_string() },
                            GrammarElement::RuleReference { name: r#"match_case"#.to_string() },
                        ] }) },
                ] },
                line_number: 238,
            },
            GrammarRule {
                name: r#"let_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"let"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"rec"#.to_string() }) },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"and"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"in"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 240,
            },
            GrammarRule {
                name: r#"for_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"for"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::Literal { value: r#"to"#.to_string() },
                            GrammarElement::Literal { value: r#"downto"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Literal { value: r#"do"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 242,
            },
            GrammarRule {
                name: r#"while_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"while"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Literal { value: r#"do"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 244,
            },
            GrammarRule {
                name: r#"pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"pattern_atom"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"as"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 246,
            },
            GrammarRule {
                name: r#"pattern_atom"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"wildcard_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"literal_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"tuple_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"list_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"array_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"unit_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"parenthesized_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::TokenReference { name: r#"TYPEVAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                ] },
                line_number: 248,
            },
            GrammarRule {
                name: r#"wildcard_pattern"#.to_string(),
                body: GrammarElement::TokenReference { name: r#"UNDERSCORE"#.to_string() },
                line_number: 260,
            },
            GrammarRule {
                name: r#"literal_pattern"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CHAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"TRUE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"FALSE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NULL"#.to_string() },
                ] },
                line_number: 262,
            },
            GrammarRule {
                name: r#"tuple_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 269,
            },
            GrammarRule {
                name: r#"list_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"element_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                ] },
                line_number: 271,
            },
            GrammarRule {
                name: r#"array_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"ARRAY_LBRACKET"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"element_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"ARRAY_RBRACKET"#.to_string() },
                ] },
                line_number: 273,
            },
            GrammarRule {
                name: r#"record_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"field_pattern"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"field_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"field_pattern"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 275,
            },
            GrammarRule {
                name: r#"field_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                ] },
                line_number: 277,
            },
            GrammarRule {
                name: r#"unit_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 279,
            },
            GrammarRule {
                name: r#"parenthesized_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 281,
            },
        ],
            version: 1,
        }
    }
}

mod v_4_7 {
    // AUTO-GENERATED FILE — DO NOT EDIT
    // Source: fsharp4.7.grammar
    // Regenerate with: grammar-tools compile-grammar fsharp4.7.grammar
    //
    // This file embeds a ParserGrammar as native Rust data structures.
    // Call `parser_grammar()` instead of reading and parsing the .grammar file.

    use grammar_tools::parser_grammar::{GrammarElement, GrammarRule, ParserGrammar};

    pub fn parser_grammar() -> ParserGrammar {
        ParserGrammar {
            rules: vec![
            GrammarRule {
                name: r#"compilation_unit"#.to_string(),
                body: GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                        GrammarElement::RuleReference { name: r#"decorated_declaration"#.to_string() },
                    ] }) },
                line_number: 9,
            },
            GrammarRule {
                name: r#"decorated_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"attribute_section"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"declaration_body"#.to_string() },
                ] },
                line_number: 11,
            },
            GrammarRule {
                name: r#"declaration_body"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"module_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"namespace_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"open_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"let_binding"#.to_string() },
                    GrammarElement::RuleReference { name: r#"use_binding"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"member_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"do_binding"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 13,
            },
            GrammarRule {
                name: r#"module_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"module"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"module_modifier"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"rec"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                        ] }) },
                ] },
                line_number: 23,
            },
            GrammarRule {
                name: r#"module_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"private"#.to_string() },
                    GrammarElement::Literal { value: r#"internal"#.to_string() },
                ] },
                line_number: 25,
            },
            GrammarRule {
                name: r#"namespace_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"namespace"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"namespace_modifier"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                ] },
                line_number: 28,
            },
            GrammarRule {
                name: r#"namespace_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"private"#.to_string() },
                    GrammarElement::Literal { value: r#"internal"#.to_string() },
                ] },
                line_number: 30,
            },
            GrammarRule {
                name: r#"open_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"open"#.to_string() },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                ] },
                line_number: 33,
            },
            GrammarRule {
                name: r#"let_binding"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"let"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"rec"#.to_string() }) },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"and"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                        ] }) },
                ] },
                line_number: 35,
            },
            GrammarRule {
                name: r#"use_binding"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"use"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"rec"#.to_string() }) },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"and"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                        ] }) },
                ] },
                line_number: 37,
            },
            GrammarRule {
                name: r#"binding_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"inline"#.to_string() },
                    GrammarElement::Literal { value: r#"mutable"#.to_string() },
                ] },
                line_number: 39,
            },
            GrammarRule {
                name: r#"binding_clause"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"pattern"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"type_annotation"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 42,
            },
            GrammarRule {
                name: r#"do_binding"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"do"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 44,
            },
            GrammarRule {
                name: r#"member_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"member_modifier"#.to_string() }) },
                    GrammarElement::Literal { value: r#"member"#.to_string() },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"parameter_list"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"type_annotation"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                ] },
                line_number: 46,
            },
            GrammarRule {
                name: r#"member_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"static"#.to_string() },
                    GrammarElement::Literal { value: r#"override"#.to_string() },
                    GrammarElement::Literal { value: r#"default"#.to_string() },
                    GrammarElement::Literal { value: r#"abstract"#.to_string() },
                    GrammarElement::Literal { value: r#"new"#.to_string() },
                ] },
                line_number: 48,
            },
            GrammarRule {
                name: r#"type_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"type"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"type_modifier"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"generic_parameters"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"parameter_list"#.to_string() }) },
                            GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"type_definition"#.to_string() },
                ] },
                line_number: 54,
            },
            GrammarRule {
                name: r#"type_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"private"#.to_string() },
                    GrammarElement::Literal { value: r#"public"#.to_string() },
                    GrammarElement::Literal { value: r#"internal"#.to_string() },
                ] },
                line_number: 56,
            },
            GrammarRule {
                name: r#"generic_parameters"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_parameter"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_parameter"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                ] },
                line_number: 60,
            },
            GrammarRule {
                name: r#"type_parameter"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"TYPEVAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                ] },
                line_number: 62,
            },
            GrammarRule {
                name: r#"type_definition"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"class_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"interface_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"struct_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"union_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"alias_type"#.to_string() },
                ] },
                line_number: 65,
            },
            GrammarRule {
                name: r#"class_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"class"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                            GrammarElement::RuleReference { name: r#"declaration_body"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"end"#.to_string() },
                ] },
                line_number: 72,
            },
            GrammarRule {
                name: r#"interface_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"interface"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                            GrammarElement::RuleReference { name: r#"declaration_body"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"end"#.to_string() },
                ] },
                line_number: 74,
            },
            GrammarRule {
                name: r#"struct_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"struct"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                            GrammarElement::RuleReference { name: r#"declaration_body"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"end"#.to_string() },
                ] },
                line_number: 76,
            },
            GrammarRule {
                name: r#"record_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"field_declaration"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"field_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"field_declaration"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 78,
            },
            GrammarRule {
                name: r#"field_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                ] },
                line_number: 80,
            },
            GrammarRule {
                name: r#"union_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"union_case"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"case_separator"#.to_string() },
                            GrammarElement::RuleReference { name: r#"union_case"#.to_string() },
                        ] }) },
                ] },
                line_number: 82,
            },
            GrammarRule {
                name: r#"union_case"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"of"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                                ] }) },
                        ] }) },
                ] },
                line_number: 84,
            },
            GrammarRule {
                name: r#"alias_type"#.to_string(),
                body: GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                line_number: 86,
            },
            GrammarRule {
                name: r#"field_separator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                ] },
                line_number: 88,
            },
            GrammarRule {
                name: r#"case_separator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"PIPE"#.to_string() }) },
                    ] },
                    GrammarElement::TokenReference { name: r#"PIPE"#.to_string() },
                ] },
                line_number: 91,
            },
            GrammarRule {
                name: r#"attribute_section"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"attribute"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"attribute"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                ] },
                line_number: 94,
            },
            GrammarRule {
                name: r#"attribute"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"attribute_target"#.to_string() },
                            GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"attribute_arguments"#.to_string() }) },
                            GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                        ] }) },
                ] },
                line_number: 96,
            },
            GrammarRule {
                name: r#"attribute_target"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"assembly"#.to_string() },
                    GrammarElement::Literal { value: r#"field"#.to_string() },
                    GrammarElement::Literal { value: r#"method"#.to_string() },
                    GrammarElement::Literal { value: r#"module"#.to_string() },
                    GrammarElement::Literal { value: r#"param"#.to_string() },
                    GrammarElement::Literal { value: r#"property"#.to_string() },
                    GrammarElement::Literal { value: r#"return"#.to_string() },
                    GrammarElement::Literal { value: r#"type"#.to_string() },
                    GrammarElement::Literal { value: r#"event"#.to_string() },
                ] },
                line_number: 98,
            },
            GrammarRule {
                name: r#"attribute_arguments"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"attribute_argument"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"attribute_argument"#.to_string() },
                        ] }) },
                ] },
                line_number: 108,
            },
            GrammarRule {
                name: r#"attribute_argument"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 110,
            },
            GrammarRule {
                name: r#"parameter_list"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"pattern"#.to_string() }) },
                ] },
                line_number: 112,
            },
            GrammarRule {
                name: r#"type_annotation"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                ] },
                line_number: 114,
            },
            GrammarRule {
                name: r#"type_expression"#.to_string(),
                body: GrammarElement::RuleReference { name: r#"function_type"#.to_string() },
                line_number: 116,
            },
            GrammarRule {
                name: r#"function_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"type_product"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                            GrammarElement::RuleReference { name: r#"function_type"#.to_string() },
                        ] }) },
                ] },
                line_number: 118,
            },
            GrammarRule {
                name: r#"type_product"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"type_application"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_application"#.to_string() },
                        ] }) },
                ] },
                line_number: 120,
            },
            GrammarRule {
                name: r#"type_application"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"type_atom"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"type_atom"#.to_string() }) },
                ] },
                line_number: 122,
            },
            GrammarRule {
                name: r#"generic_type_arguments"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                ] },
                line_number: 124,
            },
            GrammarRule {
                name: r#"type_atom"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"generic_type_arguments"#.to_string() }) },
                    ] },
                    GrammarElement::TokenReference { name: r#"TYPEVAR"#.to_string() },
                    GrammarElement::RuleReference { name: r#"tuple_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"unit_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"parenthesized_type"#.to_string() },
                ] },
                line_number: 126,
            },
            GrammarRule {
                name: r#"tuple_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 133,
            },
            GrammarRule {
                name: r#"unit_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 135,
            },
            GrammarRule {
                name: r#"parenthesized_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 137,
            },
            GrammarRule {
                name: r#"qualified_name"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 139,
            },
            GrammarRule {
                name: r#"expression"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"if_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"match_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"let_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"function_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"fun_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"for_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"while_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"sequence_expression"#.to_string() },
                ] },
                line_number: 141,
            },
            GrammarRule {
                name: r#"sequence_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"infix_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                                GrammarElement::RuleReference { name: r#"infix_expression"#.to_string() },
                            ] },
                            GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                                GrammarElement::RuleReference { name: r#"infix_expression"#.to_string() },
                            ] },
                        ] }) },
                ] },
                line_number: 150,
            },
            GrammarRule {
                name: r#"infix_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"application_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"infix_operator"#.to_string() },
                            GrammarElement::RuleReference { name: r#"application_expression"#.to_string() },
                        ] }) },
                ] },
                line_number: 152,
            },
            GrammarRule {
                name: r#"infix_operator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"PIPE_RIGHT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PIPE_LEFT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMPOSE_RIGHT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMPOSE_LEFT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NOT_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"GREATER_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"DOUBLE_COLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LARROW"#.to_string() },
                    GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                    GrammarElement::TokenReference { name: r#"DOT_DOT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"AND_AND"#.to_string() },
                    GrammarElement::TokenReference { name: r#"OR_OR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SLASH"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PERCENT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"AMPERSAND"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PIPE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CARET"#.to_string() },
                ] },
                line_number: 154,
            },
            GrammarRule {
                name: r#"application_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"prefix_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"prefix_expression"#.to_string() }) },
                ] },
                line_number: 181,
            },
            GrammarRule {
                name: r#"prefix_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"unary_operator"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"atomic_expression"#.to_string() },
                ] },
                line_number: 183,
            },
            GrammarRule {
                name: r#"unary_operator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"BANG"#.to_string() },
                ] },
                line_number: 185,
            },
            GrammarRule {
                name: r#"atomic_expression"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"computation_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"if_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"match_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"let_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"function_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"fun_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"for_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"while_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"unit_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"tuple_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"list_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"array_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"parenthesized_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CHAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"TRUE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"FALSE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NULL"#.to_string() },
                ] },
                line_number: 189,
            },
            GrammarRule {
                name: r#"computation_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"expression"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 211,
            },
            GrammarRule {
                name: r#"unit_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 213,
            },
            GrammarRule {
                name: r#"tuple_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 215,
            },
            GrammarRule {
                name: r#"list_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                    GrammarElement::NegativeLookahead { element: Box::new(GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"element_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                ] },
                line_number: 217,
            },
            GrammarRule {
                name: r#"array_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"ARRAY_LBRACKET"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"element_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"ARRAY_RBRACKET"#.to_string() },
                ] },
                line_number: 219,
            },
            GrammarRule {
                name: r#"record_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"field_assignment"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"field_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"field_assignment"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 221,
            },
            GrammarRule {
                name: r#"field_assignment"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 223,
            },
            GrammarRule {
                name: r#"parenthesized_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 225,
            },
            GrammarRule {
                name: r#"element_separator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                ] },
                line_number: 227,
            },
            GrammarRule {
                name: r#"if_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"if"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Literal { value: r#"then"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"else"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                ] },
                line_number: 230,
            },
            GrammarRule {
                name: r#"match_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"match"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Literal { value: r#"with"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"case_separator"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"match_case"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"case_separator"#.to_string() },
                            GrammarElement::RuleReference { name: r#"match_case"#.to_string() },
                        ] }) },
                ] },
                line_number: 232,
            },
            GrammarRule {
                name: r#"match_case"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"when"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 234,
            },
            GrammarRule {
                name: r#"fun_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"fun"#.to_string() },
                    GrammarElement::RuleReference { name: r#"parameter_list"#.to_string() },
                    GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 236,
            },
            GrammarRule {
                name: r#"function_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"function"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"case_separator"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"match_case"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"case_separator"#.to_string() },
                            GrammarElement::RuleReference { name: r#"match_case"#.to_string() },
                        ] }) },
                ] },
                line_number: 238,
            },
            GrammarRule {
                name: r#"let_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"let"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"rec"#.to_string() }) },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"and"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"in"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 240,
            },
            GrammarRule {
                name: r#"for_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"for"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::Literal { value: r#"to"#.to_string() },
                            GrammarElement::Literal { value: r#"downto"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Literal { value: r#"do"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 242,
            },
            GrammarRule {
                name: r#"while_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"while"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Literal { value: r#"do"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 244,
            },
            GrammarRule {
                name: r#"pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"pattern_atom"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"as"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 246,
            },
            GrammarRule {
                name: r#"pattern_atom"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"wildcard_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"literal_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"tuple_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"list_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"array_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"unit_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"parenthesized_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::TokenReference { name: r#"TYPEVAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                ] },
                line_number: 248,
            },
            GrammarRule {
                name: r#"wildcard_pattern"#.to_string(),
                body: GrammarElement::TokenReference { name: r#"UNDERSCORE"#.to_string() },
                line_number: 260,
            },
            GrammarRule {
                name: r#"literal_pattern"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CHAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"TRUE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"FALSE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NULL"#.to_string() },
                ] },
                line_number: 262,
            },
            GrammarRule {
                name: r#"tuple_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 269,
            },
            GrammarRule {
                name: r#"list_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"element_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                ] },
                line_number: 271,
            },
            GrammarRule {
                name: r#"array_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"ARRAY_LBRACKET"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"element_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"ARRAY_RBRACKET"#.to_string() },
                ] },
                line_number: 273,
            },
            GrammarRule {
                name: r#"record_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"field_pattern"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"field_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"field_pattern"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 275,
            },
            GrammarRule {
                name: r#"field_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                ] },
                line_number: 277,
            },
            GrammarRule {
                name: r#"unit_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 279,
            },
            GrammarRule {
                name: r#"parenthesized_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 281,
            },
        ],
            version: 1,
        }
    }
}

mod v_5 {
    // AUTO-GENERATED FILE — DO NOT EDIT
    // Source: fsharp5.grammar
    // Regenerate with: grammar-tools compile-grammar fsharp5.grammar
    //
    // This file embeds a ParserGrammar as native Rust data structures.
    // Call `parser_grammar()` instead of reading and parsing the .grammar file.

    use grammar_tools::parser_grammar::{GrammarElement, GrammarRule, ParserGrammar};

    pub fn parser_grammar() -> ParserGrammar {
        ParserGrammar {
            rules: vec![
            GrammarRule {
                name: r#"compilation_unit"#.to_string(),
                body: GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                        GrammarElement::RuleReference { name: r#"decorated_declaration"#.to_string() },
                    ] }) },
                line_number: 9,
            },
            GrammarRule {
                name: r#"decorated_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"attribute_section"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"declaration_body"#.to_string() },
                ] },
                line_number: 11,
            },
            GrammarRule {
                name: r#"declaration_body"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"module_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"namespace_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"open_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"let_binding"#.to_string() },
                    GrammarElement::RuleReference { name: r#"use_binding"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"member_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"do_binding"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 13,
            },
            GrammarRule {
                name: r#"module_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"module"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"module_modifier"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"rec"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                        ] }) },
                ] },
                line_number: 23,
            },
            GrammarRule {
                name: r#"module_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"private"#.to_string() },
                    GrammarElement::Literal { value: r#"internal"#.to_string() },
                ] },
                line_number: 25,
            },
            GrammarRule {
                name: r#"namespace_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"namespace"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"namespace_modifier"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                ] },
                line_number: 28,
            },
            GrammarRule {
                name: r#"namespace_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"private"#.to_string() },
                    GrammarElement::Literal { value: r#"internal"#.to_string() },
                ] },
                line_number: 30,
            },
            GrammarRule {
                name: r#"open_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"open"#.to_string() },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                ] },
                line_number: 33,
            },
            GrammarRule {
                name: r#"let_binding"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"let"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"rec"#.to_string() }) },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"and"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                        ] }) },
                ] },
                line_number: 35,
            },
            GrammarRule {
                name: r#"use_binding"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"use"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"rec"#.to_string() }) },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"and"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                        ] }) },
                ] },
                line_number: 37,
            },
            GrammarRule {
                name: r#"binding_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"inline"#.to_string() },
                    GrammarElement::Literal { value: r#"mutable"#.to_string() },
                ] },
                line_number: 39,
            },
            GrammarRule {
                name: r#"binding_clause"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"pattern"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"type_annotation"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 42,
            },
            GrammarRule {
                name: r#"do_binding"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"do"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 44,
            },
            GrammarRule {
                name: r#"member_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"member_modifier"#.to_string() }) },
                    GrammarElement::Literal { value: r#"member"#.to_string() },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"parameter_list"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"type_annotation"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                ] },
                line_number: 46,
            },
            GrammarRule {
                name: r#"member_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"static"#.to_string() },
                    GrammarElement::Literal { value: r#"override"#.to_string() },
                    GrammarElement::Literal { value: r#"default"#.to_string() },
                    GrammarElement::Literal { value: r#"abstract"#.to_string() },
                    GrammarElement::Literal { value: r#"new"#.to_string() },
                ] },
                line_number: 48,
            },
            GrammarRule {
                name: r#"type_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"type"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"type_modifier"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"generic_parameters"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"parameter_list"#.to_string() }) },
                            GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"type_definition"#.to_string() },
                ] },
                line_number: 54,
            },
            GrammarRule {
                name: r#"type_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"private"#.to_string() },
                    GrammarElement::Literal { value: r#"public"#.to_string() },
                    GrammarElement::Literal { value: r#"internal"#.to_string() },
                ] },
                line_number: 56,
            },
            GrammarRule {
                name: r#"generic_parameters"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_parameter"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_parameter"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                ] },
                line_number: 60,
            },
            GrammarRule {
                name: r#"type_parameter"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"TYPEVAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                ] },
                line_number: 62,
            },
            GrammarRule {
                name: r#"type_definition"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"class_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"interface_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"struct_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"union_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"alias_type"#.to_string() },
                ] },
                line_number: 65,
            },
            GrammarRule {
                name: r#"class_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"class"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                            GrammarElement::RuleReference { name: r#"declaration_body"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"end"#.to_string() },
                ] },
                line_number: 72,
            },
            GrammarRule {
                name: r#"interface_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"interface"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                            GrammarElement::RuleReference { name: r#"declaration_body"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"end"#.to_string() },
                ] },
                line_number: 74,
            },
            GrammarRule {
                name: r#"struct_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"struct"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                            GrammarElement::RuleReference { name: r#"declaration_body"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"end"#.to_string() },
                ] },
                line_number: 76,
            },
            GrammarRule {
                name: r#"record_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"field_declaration"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"field_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"field_declaration"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 78,
            },
            GrammarRule {
                name: r#"field_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                ] },
                line_number: 80,
            },
            GrammarRule {
                name: r#"union_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"union_case"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"case_separator"#.to_string() },
                            GrammarElement::RuleReference { name: r#"union_case"#.to_string() },
                        ] }) },
                ] },
                line_number: 82,
            },
            GrammarRule {
                name: r#"union_case"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"of"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                                ] }) },
                        ] }) },
                ] },
                line_number: 84,
            },
            GrammarRule {
                name: r#"alias_type"#.to_string(),
                body: GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                line_number: 86,
            },
            GrammarRule {
                name: r#"field_separator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                ] },
                line_number: 88,
            },
            GrammarRule {
                name: r#"case_separator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"PIPE"#.to_string() }) },
                    ] },
                    GrammarElement::TokenReference { name: r#"PIPE"#.to_string() },
                ] },
                line_number: 91,
            },
            GrammarRule {
                name: r#"attribute_section"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"attribute"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"attribute"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                ] },
                line_number: 94,
            },
            GrammarRule {
                name: r#"attribute"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"attribute_target"#.to_string() },
                            GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"attribute_arguments"#.to_string() }) },
                            GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                        ] }) },
                ] },
                line_number: 96,
            },
            GrammarRule {
                name: r#"attribute_target"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"assembly"#.to_string() },
                    GrammarElement::Literal { value: r#"field"#.to_string() },
                    GrammarElement::Literal { value: r#"method"#.to_string() },
                    GrammarElement::Literal { value: r#"module"#.to_string() },
                    GrammarElement::Literal { value: r#"param"#.to_string() },
                    GrammarElement::Literal { value: r#"property"#.to_string() },
                    GrammarElement::Literal { value: r#"return"#.to_string() },
                    GrammarElement::Literal { value: r#"type"#.to_string() },
                    GrammarElement::Literal { value: r#"event"#.to_string() },
                ] },
                line_number: 98,
            },
            GrammarRule {
                name: r#"attribute_arguments"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"attribute_argument"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"attribute_argument"#.to_string() },
                        ] }) },
                ] },
                line_number: 108,
            },
            GrammarRule {
                name: r#"attribute_argument"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 110,
            },
            GrammarRule {
                name: r#"parameter_list"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"pattern"#.to_string() }) },
                ] },
                line_number: 112,
            },
            GrammarRule {
                name: r#"type_annotation"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                ] },
                line_number: 114,
            },
            GrammarRule {
                name: r#"type_expression"#.to_string(),
                body: GrammarElement::RuleReference { name: r#"function_type"#.to_string() },
                line_number: 116,
            },
            GrammarRule {
                name: r#"function_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"type_product"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                            GrammarElement::RuleReference { name: r#"function_type"#.to_string() },
                        ] }) },
                ] },
                line_number: 118,
            },
            GrammarRule {
                name: r#"type_product"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"type_application"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_application"#.to_string() },
                        ] }) },
                ] },
                line_number: 120,
            },
            GrammarRule {
                name: r#"type_application"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"type_atom"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"type_atom"#.to_string() }) },
                ] },
                line_number: 122,
            },
            GrammarRule {
                name: r#"generic_type_arguments"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                ] },
                line_number: 124,
            },
            GrammarRule {
                name: r#"type_atom"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"generic_type_arguments"#.to_string() }) },
                    ] },
                    GrammarElement::TokenReference { name: r#"TYPEVAR"#.to_string() },
                    GrammarElement::RuleReference { name: r#"tuple_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"unit_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"parenthesized_type"#.to_string() },
                ] },
                line_number: 126,
            },
            GrammarRule {
                name: r#"tuple_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 133,
            },
            GrammarRule {
                name: r#"unit_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 135,
            },
            GrammarRule {
                name: r#"parenthesized_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 137,
            },
            GrammarRule {
                name: r#"qualified_name"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 139,
            },
            GrammarRule {
                name: r#"expression"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"if_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"match_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"let_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"function_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"fun_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"for_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"while_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"sequence_expression"#.to_string() },
                ] },
                line_number: 141,
            },
            GrammarRule {
                name: r#"sequence_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"infix_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                                GrammarElement::RuleReference { name: r#"infix_expression"#.to_string() },
                            ] },
                            GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                                GrammarElement::RuleReference { name: r#"infix_expression"#.to_string() },
                            ] },
                        ] }) },
                ] },
                line_number: 150,
            },
            GrammarRule {
                name: r#"infix_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"application_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"infix_operator"#.to_string() },
                            GrammarElement::RuleReference { name: r#"application_expression"#.to_string() },
                        ] }) },
                ] },
                line_number: 152,
            },
            GrammarRule {
                name: r#"infix_operator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"PIPE_RIGHT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PIPE_LEFT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMPOSE_RIGHT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMPOSE_LEFT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NOT_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"GREATER_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"DOUBLE_COLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LARROW"#.to_string() },
                    GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                    GrammarElement::TokenReference { name: r#"DOT_DOT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"AND_AND"#.to_string() },
                    GrammarElement::TokenReference { name: r#"OR_OR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SLASH"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PERCENT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"AMPERSAND"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PIPE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CARET"#.to_string() },
                ] },
                line_number: 154,
            },
            GrammarRule {
                name: r#"application_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"prefix_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"prefix_expression"#.to_string() }) },
                ] },
                line_number: 181,
            },
            GrammarRule {
                name: r#"prefix_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"unary_operator"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"atomic_expression"#.to_string() },
                ] },
                line_number: 183,
            },
            GrammarRule {
                name: r#"unary_operator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"BANG"#.to_string() },
                ] },
                line_number: 185,
            },
            GrammarRule {
                name: r#"atomic_expression"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"computation_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"if_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"match_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"let_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"function_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"fun_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"for_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"while_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"unit_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"tuple_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"list_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"array_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"parenthesized_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CHAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"TRUE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"FALSE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NULL"#.to_string() },
                ] },
                line_number: 189,
            },
            GrammarRule {
                name: r#"computation_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"expression"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 211,
            },
            GrammarRule {
                name: r#"unit_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 213,
            },
            GrammarRule {
                name: r#"tuple_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 215,
            },
            GrammarRule {
                name: r#"list_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                    GrammarElement::NegativeLookahead { element: Box::new(GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"element_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                ] },
                line_number: 217,
            },
            GrammarRule {
                name: r#"array_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"ARRAY_LBRACKET"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"element_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"ARRAY_RBRACKET"#.to_string() },
                ] },
                line_number: 219,
            },
            GrammarRule {
                name: r#"record_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"field_assignment"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"field_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"field_assignment"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 221,
            },
            GrammarRule {
                name: r#"field_assignment"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 223,
            },
            GrammarRule {
                name: r#"parenthesized_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 225,
            },
            GrammarRule {
                name: r#"element_separator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                ] },
                line_number: 227,
            },
            GrammarRule {
                name: r#"if_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"if"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Literal { value: r#"then"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"else"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                ] },
                line_number: 230,
            },
            GrammarRule {
                name: r#"match_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"match"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Literal { value: r#"with"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"case_separator"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"match_case"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"case_separator"#.to_string() },
                            GrammarElement::RuleReference { name: r#"match_case"#.to_string() },
                        ] }) },
                ] },
                line_number: 232,
            },
            GrammarRule {
                name: r#"match_case"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"when"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 234,
            },
            GrammarRule {
                name: r#"fun_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"fun"#.to_string() },
                    GrammarElement::RuleReference { name: r#"parameter_list"#.to_string() },
                    GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 236,
            },
            GrammarRule {
                name: r#"function_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"function"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"case_separator"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"match_case"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"case_separator"#.to_string() },
                            GrammarElement::RuleReference { name: r#"match_case"#.to_string() },
                        ] }) },
                ] },
                line_number: 238,
            },
            GrammarRule {
                name: r#"let_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"let"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"rec"#.to_string() }) },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"and"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"in"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 240,
            },
            GrammarRule {
                name: r#"for_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"for"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::Literal { value: r#"to"#.to_string() },
                            GrammarElement::Literal { value: r#"downto"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Literal { value: r#"do"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 242,
            },
            GrammarRule {
                name: r#"while_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"while"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Literal { value: r#"do"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 244,
            },
            GrammarRule {
                name: r#"pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"pattern_atom"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"as"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 246,
            },
            GrammarRule {
                name: r#"pattern_atom"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"wildcard_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"literal_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"tuple_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"list_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"array_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"unit_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"parenthesized_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::TokenReference { name: r#"TYPEVAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                ] },
                line_number: 248,
            },
            GrammarRule {
                name: r#"wildcard_pattern"#.to_string(),
                body: GrammarElement::TokenReference { name: r#"UNDERSCORE"#.to_string() },
                line_number: 260,
            },
            GrammarRule {
                name: r#"literal_pattern"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CHAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"TRUE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"FALSE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NULL"#.to_string() },
                ] },
                line_number: 262,
            },
            GrammarRule {
                name: r#"tuple_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 269,
            },
            GrammarRule {
                name: r#"list_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"element_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                ] },
                line_number: 271,
            },
            GrammarRule {
                name: r#"array_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"ARRAY_LBRACKET"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"element_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"ARRAY_RBRACKET"#.to_string() },
                ] },
                line_number: 273,
            },
            GrammarRule {
                name: r#"record_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"field_pattern"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"field_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"field_pattern"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 275,
            },
            GrammarRule {
                name: r#"field_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                ] },
                line_number: 277,
            },
            GrammarRule {
                name: r#"unit_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 279,
            },
            GrammarRule {
                name: r#"parenthesized_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 281,
            },
        ],
            version: 1,
        }
    }
}

mod v_6 {
    // AUTO-GENERATED FILE — DO NOT EDIT
    // Source: fsharp6.grammar
    // Regenerate with: grammar-tools compile-grammar fsharp6.grammar
    //
    // This file embeds a ParserGrammar as native Rust data structures.
    // Call `parser_grammar()` instead of reading and parsing the .grammar file.

    use grammar_tools::parser_grammar::{GrammarElement, GrammarRule, ParserGrammar};

    pub fn parser_grammar() -> ParserGrammar {
        ParserGrammar {
            rules: vec![
            GrammarRule {
                name: r#"compilation_unit"#.to_string(),
                body: GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                        GrammarElement::RuleReference { name: r#"decorated_declaration"#.to_string() },
                    ] }) },
                line_number: 9,
            },
            GrammarRule {
                name: r#"decorated_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"attribute_section"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"declaration_body"#.to_string() },
                ] },
                line_number: 11,
            },
            GrammarRule {
                name: r#"declaration_body"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"module_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"namespace_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"open_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"let_binding"#.to_string() },
                    GrammarElement::RuleReference { name: r#"use_binding"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"member_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"do_binding"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 13,
            },
            GrammarRule {
                name: r#"module_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"module"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"module_modifier"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"rec"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                        ] }) },
                ] },
                line_number: 23,
            },
            GrammarRule {
                name: r#"module_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"private"#.to_string() },
                    GrammarElement::Literal { value: r#"internal"#.to_string() },
                ] },
                line_number: 25,
            },
            GrammarRule {
                name: r#"namespace_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"namespace"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"namespace_modifier"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                ] },
                line_number: 28,
            },
            GrammarRule {
                name: r#"namespace_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"private"#.to_string() },
                    GrammarElement::Literal { value: r#"internal"#.to_string() },
                ] },
                line_number: 30,
            },
            GrammarRule {
                name: r#"open_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"open"#.to_string() },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                ] },
                line_number: 33,
            },
            GrammarRule {
                name: r#"let_binding"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"let"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"rec"#.to_string() }) },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"and"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                        ] }) },
                ] },
                line_number: 35,
            },
            GrammarRule {
                name: r#"use_binding"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"use"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"rec"#.to_string() }) },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"and"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                        ] }) },
                ] },
                line_number: 37,
            },
            GrammarRule {
                name: r#"binding_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"inline"#.to_string() },
                    GrammarElement::Literal { value: r#"mutable"#.to_string() },
                ] },
                line_number: 39,
            },
            GrammarRule {
                name: r#"binding_clause"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"pattern"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"type_annotation"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 42,
            },
            GrammarRule {
                name: r#"do_binding"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"do"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 44,
            },
            GrammarRule {
                name: r#"member_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"member_modifier"#.to_string() }) },
                    GrammarElement::Literal { value: r#"member"#.to_string() },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"parameter_list"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"type_annotation"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                ] },
                line_number: 46,
            },
            GrammarRule {
                name: r#"member_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"static"#.to_string() },
                    GrammarElement::Literal { value: r#"override"#.to_string() },
                    GrammarElement::Literal { value: r#"default"#.to_string() },
                    GrammarElement::Literal { value: r#"abstract"#.to_string() },
                    GrammarElement::Literal { value: r#"new"#.to_string() },
                ] },
                line_number: 48,
            },
            GrammarRule {
                name: r#"type_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"type"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"type_modifier"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"generic_parameters"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"parameter_list"#.to_string() }) },
                            GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"type_definition"#.to_string() },
                ] },
                line_number: 54,
            },
            GrammarRule {
                name: r#"type_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"private"#.to_string() },
                    GrammarElement::Literal { value: r#"public"#.to_string() },
                    GrammarElement::Literal { value: r#"internal"#.to_string() },
                ] },
                line_number: 56,
            },
            GrammarRule {
                name: r#"generic_parameters"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_parameter"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_parameter"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                ] },
                line_number: 60,
            },
            GrammarRule {
                name: r#"type_parameter"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"TYPEVAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                ] },
                line_number: 62,
            },
            GrammarRule {
                name: r#"type_definition"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"class_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"interface_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"struct_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"union_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"alias_type"#.to_string() },
                ] },
                line_number: 65,
            },
            GrammarRule {
                name: r#"class_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"class"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                            GrammarElement::RuleReference { name: r#"declaration_body"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"end"#.to_string() },
                ] },
                line_number: 72,
            },
            GrammarRule {
                name: r#"interface_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"interface"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                            GrammarElement::RuleReference { name: r#"declaration_body"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"end"#.to_string() },
                ] },
                line_number: 74,
            },
            GrammarRule {
                name: r#"struct_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"struct"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                            GrammarElement::RuleReference { name: r#"declaration_body"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"end"#.to_string() },
                ] },
                line_number: 76,
            },
            GrammarRule {
                name: r#"record_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"field_declaration"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"field_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"field_declaration"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 78,
            },
            GrammarRule {
                name: r#"field_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                ] },
                line_number: 80,
            },
            GrammarRule {
                name: r#"union_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"union_case"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"case_separator"#.to_string() },
                            GrammarElement::RuleReference { name: r#"union_case"#.to_string() },
                        ] }) },
                ] },
                line_number: 82,
            },
            GrammarRule {
                name: r#"union_case"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"of"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                                ] }) },
                        ] }) },
                ] },
                line_number: 84,
            },
            GrammarRule {
                name: r#"alias_type"#.to_string(),
                body: GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                line_number: 86,
            },
            GrammarRule {
                name: r#"field_separator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                ] },
                line_number: 88,
            },
            GrammarRule {
                name: r#"case_separator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"PIPE"#.to_string() }) },
                    ] },
                    GrammarElement::TokenReference { name: r#"PIPE"#.to_string() },
                ] },
                line_number: 91,
            },
            GrammarRule {
                name: r#"attribute_section"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"attribute"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"attribute"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                ] },
                line_number: 94,
            },
            GrammarRule {
                name: r#"attribute"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"attribute_target"#.to_string() },
                            GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"attribute_arguments"#.to_string() }) },
                            GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                        ] }) },
                ] },
                line_number: 96,
            },
            GrammarRule {
                name: r#"attribute_target"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"assembly"#.to_string() },
                    GrammarElement::Literal { value: r#"field"#.to_string() },
                    GrammarElement::Literal { value: r#"method"#.to_string() },
                    GrammarElement::Literal { value: r#"module"#.to_string() },
                    GrammarElement::Literal { value: r#"param"#.to_string() },
                    GrammarElement::Literal { value: r#"property"#.to_string() },
                    GrammarElement::Literal { value: r#"return"#.to_string() },
                    GrammarElement::Literal { value: r#"type"#.to_string() },
                    GrammarElement::Literal { value: r#"event"#.to_string() },
                ] },
                line_number: 98,
            },
            GrammarRule {
                name: r#"attribute_arguments"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"attribute_argument"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"attribute_argument"#.to_string() },
                        ] }) },
                ] },
                line_number: 108,
            },
            GrammarRule {
                name: r#"attribute_argument"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 110,
            },
            GrammarRule {
                name: r#"parameter_list"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"pattern"#.to_string() }) },
                ] },
                line_number: 112,
            },
            GrammarRule {
                name: r#"type_annotation"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                ] },
                line_number: 114,
            },
            GrammarRule {
                name: r#"type_expression"#.to_string(),
                body: GrammarElement::RuleReference { name: r#"function_type"#.to_string() },
                line_number: 116,
            },
            GrammarRule {
                name: r#"function_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"type_product"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                            GrammarElement::RuleReference { name: r#"function_type"#.to_string() },
                        ] }) },
                ] },
                line_number: 118,
            },
            GrammarRule {
                name: r#"type_product"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"type_application"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_application"#.to_string() },
                        ] }) },
                ] },
                line_number: 120,
            },
            GrammarRule {
                name: r#"type_application"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"type_atom"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"type_atom"#.to_string() }) },
                ] },
                line_number: 122,
            },
            GrammarRule {
                name: r#"generic_type_arguments"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                ] },
                line_number: 124,
            },
            GrammarRule {
                name: r#"type_atom"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"generic_type_arguments"#.to_string() }) },
                    ] },
                    GrammarElement::TokenReference { name: r#"TYPEVAR"#.to_string() },
                    GrammarElement::RuleReference { name: r#"tuple_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"unit_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"parenthesized_type"#.to_string() },
                ] },
                line_number: 126,
            },
            GrammarRule {
                name: r#"tuple_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 133,
            },
            GrammarRule {
                name: r#"unit_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 135,
            },
            GrammarRule {
                name: r#"parenthesized_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 137,
            },
            GrammarRule {
                name: r#"qualified_name"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 139,
            },
            GrammarRule {
                name: r#"expression"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"if_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"match_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"let_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"function_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"fun_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"for_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"while_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"sequence_expression"#.to_string() },
                ] },
                line_number: 141,
            },
            GrammarRule {
                name: r#"sequence_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"infix_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                                GrammarElement::RuleReference { name: r#"infix_expression"#.to_string() },
                            ] },
                            GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                                GrammarElement::RuleReference { name: r#"infix_expression"#.to_string() },
                            ] },
                        ] }) },
                ] },
                line_number: 150,
            },
            GrammarRule {
                name: r#"infix_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"application_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"infix_operator"#.to_string() },
                            GrammarElement::RuleReference { name: r#"application_expression"#.to_string() },
                        ] }) },
                ] },
                line_number: 152,
            },
            GrammarRule {
                name: r#"infix_operator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"PIPE_RIGHT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PIPE_LEFT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMPOSE_RIGHT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMPOSE_LEFT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NOT_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"GREATER_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"DOUBLE_COLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LARROW"#.to_string() },
                    GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                    GrammarElement::TokenReference { name: r#"DOT_DOT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"AND_AND"#.to_string() },
                    GrammarElement::TokenReference { name: r#"OR_OR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SLASH"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PERCENT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"AMPERSAND"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PIPE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CARET"#.to_string() },
                ] },
                line_number: 154,
            },
            GrammarRule {
                name: r#"application_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"prefix_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"prefix_expression"#.to_string() }) },
                ] },
                line_number: 181,
            },
            GrammarRule {
                name: r#"prefix_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"unary_operator"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"atomic_expression"#.to_string() },
                ] },
                line_number: 183,
            },
            GrammarRule {
                name: r#"unary_operator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"BANG"#.to_string() },
                ] },
                line_number: 185,
            },
            GrammarRule {
                name: r#"atomic_expression"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"computation_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"if_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"match_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"let_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"function_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"fun_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"for_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"while_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"unit_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"tuple_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"list_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"array_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"parenthesized_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CHAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"TRUE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"FALSE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NULL"#.to_string() },
                ] },
                line_number: 189,
            },
            GrammarRule {
                name: r#"computation_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"expression"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 211,
            },
            GrammarRule {
                name: r#"unit_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 213,
            },
            GrammarRule {
                name: r#"tuple_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 215,
            },
            GrammarRule {
                name: r#"list_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                    GrammarElement::NegativeLookahead { element: Box::new(GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"element_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                ] },
                line_number: 217,
            },
            GrammarRule {
                name: r#"array_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"ARRAY_LBRACKET"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"element_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"ARRAY_RBRACKET"#.to_string() },
                ] },
                line_number: 219,
            },
            GrammarRule {
                name: r#"record_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"field_assignment"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"field_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"field_assignment"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 221,
            },
            GrammarRule {
                name: r#"field_assignment"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 223,
            },
            GrammarRule {
                name: r#"parenthesized_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 225,
            },
            GrammarRule {
                name: r#"element_separator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                ] },
                line_number: 227,
            },
            GrammarRule {
                name: r#"if_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"if"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Literal { value: r#"then"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"else"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                ] },
                line_number: 230,
            },
            GrammarRule {
                name: r#"match_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"match"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Literal { value: r#"with"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"case_separator"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"match_case"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"case_separator"#.to_string() },
                            GrammarElement::RuleReference { name: r#"match_case"#.to_string() },
                        ] }) },
                ] },
                line_number: 232,
            },
            GrammarRule {
                name: r#"match_case"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"when"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 234,
            },
            GrammarRule {
                name: r#"fun_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"fun"#.to_string() },
                    GrammarElement::RuleReference { name: r#"parameter_list"#.to_string() },
                    GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 236,
            },
            GrammarRule {
                name: r#"function_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"function"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"case_separator"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"match_case"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"case_separator"#.to_string() },
                            GrammarElement::RuleReference { name: r#"match_case"#.to_string() },
                        ] }) },
                ] },
                line_number: 238,
            },
            GrammarRule {
                name: r#"let_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"let"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"rec"#.to_string() }) },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"and"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"in"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 240,
            },
            GrammarRule {
                name: r#"for_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"for"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::Literal { value: r#"to"#.to_string() },
                            GrammarElement::Literal { value: r#"downto"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Literal { value: r#"do"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 242,
            },
            GrammarRule {
                name: r#"while_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"while"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Literal { value: r#"do"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 244,
            },
            GrammarRule {
                name: r#"pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"pattern_atom"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"as"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 246,
            },
            GrammarRule {
                name: r#"pattern_atom"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"wildcard_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"literal_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"tuple_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"list_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"array_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"unit_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"parenthesized_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::TokenReference { name: r#"TYPEVAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                ] },
                line_number: 248,
            },
            GrammarRule {
                name: r#"wildcard_pattern"#.to_string(),
                body: GrammarElement::TokenReference { name: r#"UNDERSCORE"#.to_string() },
                line_number: 260,
            },
            GrammarRule {
                name: r#"literal_pattern"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CHAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"TRUE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"FALSE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NULL"#.to_string() },
                ] },
                line_number: 262,
            },
            GrammarRule {
                name: r#"tuple_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 269,
            },
            GrammarRule {
                name: r#"list_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"element_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                ] },
                line_number: 271,
            },
            GrammarRule {
                name: r#"array_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"ARRAY_LBRACKET"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"element_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"ARRAY_RBRACKET"#.to_string() },
                ] },
                line_number: 273,
            },
            GrammarRule {
                name: r#"record_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"field_pattern"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"field_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"field_pattern"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 275,
            },
            GrammarRule {
                name: r#"field_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                ] },
                line_number: 277,
            },
            GrammarRule {
                name: r#"unit_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 279,
            },
            GrammarRule {
                name: r#"parenthesized_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 281,
            },
        ],
            version: 1,
        }
    }
}

mod v_7 {
    // AUTO-GENERATED FILE — DO NOT EDIT
    // Source: fsharp7.grammar
    // Regenerate with: grammar-tools compile-grammar fsharp7.grammar
    //
    // This file embeds a ParserGrammar as native Rust data structures.
    // Call `parser_grammar()` instead of reading and parsing the .grammar file.

    use grammar_tools::parser_grammar::{GrammarElement, GrammarRule, ParserGrammar};

    pub fn parser_grammar() -> ParserGrammar {
        ParserGrammar {
            rules: vec![
            GrammarRule {
                name: r#"compilation_unit"#.to_string(),
                body: GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                        GrammarElement::RuleReference { name: r#"decorated_declaration"#.to_string() },
                    ] }) },
                line_number: 9,
            },
            GrammarRule {
                name: r#"decorated_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"attribute_section"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"declaration_body"#.to_string() },
                ] },
                line_number: 11,
            },
            GrammarRule {
                name: r#"declaration_body"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"module_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"namespace_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"open_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"let_binding"#.to_string() },
                    GrammarElement::RuleReference { name: r#"use_binding"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"member_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"do_binding"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 13,
            },
            GrammarRule {
                name: r#"module_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"module"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"module_modifier"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"rec"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                        ] }) },
                ] },
                line_number: 23,
            },
            GrammarRule {
                name: r#"module_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"private"#.to_string() },
                    GrammarElement::Literal { value: r#"internal"#.to_string() },
                ] },
                line_number: 25,
            },
            GrammarRule {
                name: r#"namespace_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"namespace"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"namespace_modifier"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                ] },
                line_number: 28,
            },
            GrammarRule {
                name: r#"namespace_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"private"#.to_string() },
                    GrammarElement::Literal { value: r#"internal"#.to_string() },
                ] },
                line_number: 30,
            },
            GrammarRule {
                name: r#"open_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"open"#.to_string() },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                ] },
                line_number: 33,
            },
            GrammarRule {
                name: r#"let_binding"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"let"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"rec"#.to_string() }) },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"and"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                        ] }) },
                ] },
                line_number: 35,
            },
            GrammarRule {
                name: r#"use_binding"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"use"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"rec"#.to_string() }) },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"and"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                        ] }) },
                ] },
                line_number: 37,
            },
            GrammarRule {
                name: r#"binding_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"inline"#.to_string() },
                    GrammarElement::Literal { value: r#"mutable"#.to_string() },
                ] },
                line_number: 39,
            },
            GrammarRule {
                name: r#"binding_clause"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"pattern"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"type_annotation"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 42,
            },
            GrammarRule {
                name: r#"do_binding"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"do"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 44,
            },
            GrammarRule {
                name: r#"member_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"member_modifier"#.to_string() }) },
                    GrammarElement::Literal { value: r#"member"#.to_string() },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"parameter_list"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"type_annotation"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                ] },
                line_number: 46,
            },
            GrammarRule {
                name: r#"member_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"static"#.to_string() },
                    GrammarElement::Literal { value: r#"override"#.to_string() },
                    GrammarElement::Literal { value: r#"default"#.to_string() },
                    GrammarElement::Literal { value: r#"abstract"#.to_string() },
                    GrammarElement::Literal { value: r#"new"#.to_string() },
                ] },
                line_number: 48,
            },
            GrammarRule {
                name: r#"type_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"type"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"type_modifier"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"generic_parameters"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"parameter_list"#.to_string() }) },
                            GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"type_definition"#.to_string() },
                ] },
                line_number: 54,
            },
            GrammarRule {
                name: r#"type_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"private"#.to_string() },
                    GrammarElement::Literal { value: r#"public"#.to_string() },
                    GrammarElement::Literal { value: r#"internal"#.to_string() },
                ] },
                line_number: 56,
            },
            GrammarRule {
                name: r#"generic_parameters"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_parameter"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_parameter"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                ] },
                line_number: 60,
            },
            GrammarRule {
                name: r#"type_parameter"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"TYPEVAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                ] },
                line_number: 62,
            },
            GrammarRule {
                name: r#"type_definition"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"class_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"interface_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"struct_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"union_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"alias_type"#.to_string() },
                ] },
                line_number: 65,
            },
            GrammarRule {
                name: r#"class_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"class"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                            GrammarElement::RuleReference { name: r#"declaration_body"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"end"#.to_string() },
                ] },
                line_number: 72,
            },
            GrammarRule {
                name: r#"interface_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"interface"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                            GrammarElement::RuleReference { name: r#"declaration_body"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"end"#.to_string() },
                ] },
                line_number: 74,
            },
            GrammarRule {
                name: r#"struct_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"struct"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                            GrammarElement::RuleReference { name: r#"declaration_body"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"end"#.to_string() },
                ] },
                line_number: 76,
            },
            GrammarRule {
                name: r#"record_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"field_declaration"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"field_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"field_declaration"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 78,
            },
            GrammarRule {
                name: r#"field_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                ] },
                line_number: 80,
            },
            GrammarRule {
                name: r#"union_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"union_case"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"case_separator"#.to_string() },
                            GrammarElement::RuleReference { name: r#"union_case"#.to_string() },
                        ] }) },
                ] },
                line_number: 82,
            },
            GrammarRule {
                name: r#"union_case"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"of"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                                ] }) },
                        ] }) },
                ] },
                line_number: 84,
            },
            GrammarRule {
                name: r#"alias_type"#.to_string(),
                body: GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                line_number: 86,
            },
            GrammarRule {
                name: r#"field_separator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                ] },
                line_number: 88,
            },
            GrammarRule {
                name: r#"case_separator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"PIPE"#.to_string() }) },
                    ] },
                    GrammarElement::TokenReference { name: r#"PIPE"#.to_string() },
                ] },
                line_number: 91,
            },
            GrammarRule {
                name: r#"attribute_section"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"attribute"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"attribute"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                ] },
                line_number: 94,
            },
            GrammarRule {
                name: r#"attribute"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"attribute_target"#.to_string() },
                            GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"attribute_arguments"#.to_string() }) },
                            GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                        ] }) },
                ] },
                line_number: 96,
            },
            GrammarRule {
                name: r#"attribute_target"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"assembly"#.to_string() },
                    GrammarElement::Literal { value: r#"field"#.to_string() },
                    GrammarElement::Literal { value: r#"method"#.to_string() },
                    GrammarElement::Literal { value: r#"module"#.to_string() },
                    GrammarElement::Literal { value: r#"param"#.to_string() },
                    GrammarElement::Literal { value: r#"property"#.to_string() },
                    GrammarElement::Literal { value: r#"return"#.to_string() },
                    GrammarElement::Literal { value: r#"type"#.to_string() },
                    GrammarElement::Literal { value: r#"event"#.to_string() },
                ] },
                line_number: 98,
            },
            GrammarRule {
                name: r#"attribute_arguments"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"attribute_argument"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"attribute_argument"#.to_string() },
                        ] }) },
                ] },
                line_number: 108,
            },
            GrammarRule {
                name: r#"attribute_argument"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 110,
            },
            GrammarRule {
                name: r#"parameter_list"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"pattern"#.to_string() }) },
                ] },
                line_number: 112,
            },
            GrammarRule {
                name: r#"type_annotation"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                ] },
                line_number: 114,
            },
            GrammarRule {
                name: r#"type_expression"#.to_string(),
                body: GrammarElement::RuleReference { name: r#"function_type"#.to_string() },
                line_number: 116,
            },
            GrammarRule {
                name: r#"function_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"type_product"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                            GrammarElement::RuleReference { name: r#"function_type"#.to_string() },
                        ] }) },
                ] },
                line_number: 118,
            },
            GrammarRule {
                name: r#"type_product"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"type_application"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_application"#.to_string() },
                        ] }) },
                ] },
                line_number: 120,
            },
            GrammarRule {
                name: r#"type_application"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"type_atom"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"type_atom"#.to_string() }) },
                ] },
                line_number: 122,
            },
            GrammarRule {
                name: r#"generic_type_arguments"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                ] },
                line_number: 124,
            },
            GrammarRule {
                name: r#"type_atom"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"generic_type_arguments"#.to_string() }) },
                    ] },
                    GrammarElement::TokenReference { name: r#"TYPEVAR"#.to_string() },
                    GrammarElement::RuleReference { name: r#"tuple_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"unit_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"parenthesized_type"#.to_string() },
                ] },
                line_number: 126,
            },
            GrammarRule {
                name: r#"tuple_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 133,
            },
            GrammarRule {
                name: r#"unit_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 135,
            },
            GrammarRule {
                name: r#"parenthesized_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 137,
            },
            GrammarRule {
                name: r#"qualified_name"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 139,
            },
            GrammarRule {
                name: r#"expression"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"if_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"match_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"let_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"function_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"fun_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"for_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"while_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"sequence_expression"#.to_string() },
                ] },
                line_number: 141,
            },
            GrammarRule {
                name: r#"sequence_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"infix_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                                GrammarElement::RuleReference { name: r#"infix_expression"#.to_string() },
                            ] },
                            GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                                GrammarElement::RuleReference { name: r#"infix_expression"#.to_string() },
                            ] },
                        ] }) },
                ] },
                line_number: 150,
            },
            GrammarRule {
                name: r#"infix_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"application_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"infix_operator"#.to_string() },
                            GrammarElement::RuleReference { name: r#"application_expression"#.to_string() },
                        ] }) },
                ] },
                line_number: 152,
            },
            GrammarRule {
                name: r#"infix_operator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"PIPE_RIGHT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PIPE_LEFT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMPOSE_RIGHT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMPOSE_LEFT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NOT_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"GREATER_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"DOUBLE_COLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LARROW"#.to_string() },
                    GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                    GrammarElement::TokenReference { name: r#"DOT_DOT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"AND_AND"#.to_string() },
                    GrammarElement::TokenReference { name: r#"OR_OR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SLASH"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PERCENT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"AMPERSAND"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PIPE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CARET"#.to_string() },
                ] },
                line_number: 154,
            },
            GrammarRule {
                name: r#"application_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"prefix_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"prefix_expression"#.to_string() }) },
                ] },
                line_number: 181,
            },
            GrammarRule {
                name: r#"prefix_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"unary_operator"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"atomic_expression"#.to_string() },
                ] },
                line_number: 183,
            },
            GrammarRule {
                name: r#"unary_operator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"BANG"#.to_string() },
                ] },
                line_number: 185,
            },
            GrammarRule {
                name: r#"atomic_expression"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"computation_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"if_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"match_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"let_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"function_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"fun_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"for_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"while_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"unit_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"tuple_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"list_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"array_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"parenthesized_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CHAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"TRUE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"FALSE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NULL"#.to_string() },
                ] },
                line_number: 189,
            },
            GrammarRule {
                name: r#"computation_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"expression"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 211,
            },
            GrammarRule {
                name: r#"unit_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 213,
            },
            GrammarRule {
                name: r#"tuple_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 215,
            },
            GrammarRule {
                name: r#"list_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                    GrammarElement::NegativeLookahead { element: Box::new(GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"element_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                ] },
                line_number: 217,
            },
            GrammarRule {
                name: r#"array_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"ARRAY_LBRACKET"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"element_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"ARRAY_RBRACKET"#.to_string() },
                ] },
                line_number: 219,
            },
            GrammarRule {
                name: r#"record_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"field_assignment"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"field_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"field_assignment"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 221,
            },
            GrammarRule {
                name: r#"field_assignment"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 223,
            },
            GrammarRule {
                name: r#"parenthesized_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 225,
            },
            GrammarRule {
                name: r#"element_separator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                ] },
                line_number: 227,
            },
            GrammarRule {
                name: r#"if_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"if"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Literal { value: r#"then"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"else"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                ] },
                line_number: 230,
            },
            GrammarRule {
                name: r#"match_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"match"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Literal { value: r#"with"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"case_separator"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"match_case"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"case_separator"#.to_string() },
                            GrammarElement::RuleReference { name: r#"match_case"#.to_string() },
                        ] }) },
                ] },
                line_number: 232,
            },
            GrammarRule {
                name: r#"match_case"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"when"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 234,
            },
            GrammarRule {
                name: r#"fun_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"fun"#.to_string() },
                    GrammarElement::RuleReference { name: r#"parameter_list"#.to_string() },
                    GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 236,
            },
            GrammarRule {
                name: r#"function_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"function"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"case_separator"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"match_case"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"case_separator"#.to_string() },
                            GrammarElement::RuleReference { name: r#"match_case"#.to_string() },
                        ] }) },
                ] },
                line_number: 238,
            },
            GrammarRule {
                name: r#"let_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"let"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"rec"#.to_string() }) },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"and"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"in"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 240,
            },
            GrammarRule {
                name: r#"for_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"for"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::Literal { value: r#"to"#.to_string() },
                            GrammarElement::Literal { value: r#"downto"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Literal { value: r#"do"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 242,
            },
            GrammarRule {
                name: r#"while_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"while"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Literal { value: r#"do"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 244,
            },
            GrammarRule {
                name: r#"pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"pattern_atom"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"as"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 246,
            },
            GrammarRule {
                name: r#"pattern_atom"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"wildcard_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"literal_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"tuple_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"list_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"array_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"unit_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"parenthesized_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::TokenReference { name: r#"TYPEVAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                ] },
                line_number: 248,
            },
            GrammarRule {
                name: r#"wildcard_pattern"#.to_string(),
                body: GrammarElement::TokenReference { name: r#"UNDERSCORE"#.to_string() },
                line_number: 260,
            },
            GrammarRule {
                name: r#"literal_pattern"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CHAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"TRUE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"FALSE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NULL"#.to_string() },
                ] },
                line_number: 262,
            },
            GrammarRule {
                name: r#"tuple_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 269,
            },
            GrammarRule {
                name: r#"list_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"element_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                ] },
                line_number: 271,
            },
            GrammarRule {
                name: r#"array_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"ARRAY_LBRACKET"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"element_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"ARRAY_RBRACKET"#.to_string() },
                ] },
                line_number: 273,
            },
            GrammarRule {
                name: r#"record_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"field_pattern"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"field_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"field_pattern"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 275,
            },
            GrammarRule {
                name: r#"field_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                ] },
                line_number: 277,
            },
            GrammarRule {
                name: r#"unit_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 279,
            },
            GrammarRule {
                name: r#"parenthesized_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 281,
            },
        ],
            version: 1,
        }
    }
}

mod v_8 {
    // AUTO-GENERATED FILE — DO NOT EDIT
    // Source: fsharp8.grammar
    // Regenerate with: grammar-tools compile-grammar fsharp8.grammar
    //
    // This file embeds a ParserGrammar as native Rust data structures.
    // Call `parser_grammar()` instead of reading and parsing the .grammar file.

    use grammar_tools::parser_grammar::{GrammarElement, GrammarRule, ParserGrammar};

    pub fn parser_grammar() -> ParserGrammar {
        ParserGrammar {
            rules: vec![
            GrammarRule {
                name: r#"compilation_unit"#.to_string(),
                body: GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                        GrammarElement::RuleReference { name: r#"decorated_declaration"#.to_string() },
                    ] }) },
                line_number: 9,
            },
            GrammarRule {
                name: r#"decorated_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"attribute_section"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"declaration_body"#.to_string() },
                ] },
                line_number: 11,
            },
            GrammarRule {
                name: r#"declaration_body"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"module_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"namespace_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"open_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"let_binding"#.to_string() },
                    GrammarElement::RuleReference { name: r#"use_binding"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"member_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"do_binding"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 13,
            },
            GrammarRule {
                name: r#"module_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"module"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"module_modifier"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"rec"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                        ] }) },
                ] },
                line_number: 23,
            },
            GrammarRule {
                name: r#"module_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"private"#.to_string() },
                    GrammarElement::Literal { value: r#"internal"#.to_string() },
                ] },
                line_number: 25,
            },
            GrammarRule {
                name: r#"namespace_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"namespace"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"namespace_modifier"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                ] },
                line_number: 28,
            },
            GrammarRule {
                name: r#"namespace_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"private"#.to_string() },
                    GrammarElement::Literal { value: r#"internal"#.to_string() },
                ] },
                line_number: 30,
            },
            GrammarRule {
                name: r#"open_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"open"#.to_string() },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                ] },
                line_number: 33,
            },
            GrammarRule {
                name: r#"let_binding"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"let"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"rec"#.to_string() }) },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"and"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                        ] }) },
                ] },
                line_number: 35,
            },
            GrammarRule {
                name: r#"use_binding"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"use"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"rec"#.to_string() }) },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"and"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                        ] }) },
                ] },
                line_number: 37,
            },
            GrammarRule {
                name: r#"binding_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"inline"#.to_string() },
                    GrammarElement::Literal { value: r#"mutable"#.to_string() },
                ] },
                line_number: 39,
            },
            GrammarRule {
                name: r#"binding_clause"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"pattern"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"type_annotation"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 42,
            },
            GrammarRule {
                name: r#"do_binding"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"do"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 44,
            },
            GrammarRule {
                name: r#"member_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"member_modifier"#.to_string() }) },
                    GrammarElement::Literal { value: r#"member"#.to_string() },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"parameter_list"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"type_annotation"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                ] },
                line_number: 46,
            },
            GrammarRule {
                name: r#"member_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"static"#.to_string() },
                    GrammarElement::Literal { value: r#"override"#.to_string() },
                    GrammarElement::Literal { value: r#"default"#.to_string() },
                    GrammarElement::Literal { value: r#"abstract"#.to_string() },
                    GrammarElement::Literal { value: r#"new"#.to_string() },
                ] },
                line_number: 48,
            },
            GrammarRule {
                name: r#"type_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"type"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"type_modifier"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"generic_parameters"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"parameter_list"#.to_string() }) },
                            GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"type_definition"#.to_string() },
                ] },
                line_number: 54,
            },
            GrammarRule {
                name: r#"type_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"private"#.to_string() },
                    GrammarElement::Literal { value: r#"public"#.to_string() },
                    GrammarElement::Literal { value: r#"internal"#.to_string() },
                ] },
                line_number: 56,
            },
            GrammarRule {
                name: r#"generic_parameters"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_parameter"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_parameter"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                ] },
                line_number: 60,
            },
            GrammarRule {
                name: r#"type_parameter"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"TYPEVAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                ] },
                line_number: 62,
            },
            GrammarRule {
                name: r#"type_definition"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"class_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"interface_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"struct_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"union_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"alias_type"#.to_string() },
                ] },
                line_number: 65,
            },
            GrammarRule {
                name: r#"class_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"class"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                            GrammarElement::RuleReference { name: r#"declaration_body"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"end"#.to_string() },
                ] },
                line_number: 72,
            },
            GrammarRule {
                name: r#"interface_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"interface"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                            GrammarElement::RuleReference { name: r#"declaration_body"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"end"#.to_string() },
                ] },
                line_number: 74,
            },
            GrammarRule {
                name: r#"struct_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"struct"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                            GrammarElement::RuleReference { name: r#"declaration_body"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"end"#.to_string() },
                ] },
                line_number: 76,
            },
            GrammarRule {
                name: r#"record_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"field_declaration"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"field_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"field_declaration"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 78,
            },
            GrammarRule {
                name: r#"field_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                ] },
                line_number: 80,
            },
            GrammarRule {
                name: r#"union_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"union_case"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"case_separator"#.to_string() },
                            GrammarElement::RuleReference { name: r#"union_case"#.to_string() },
                        ] }) },
                ] },
                line_number: 82,
            },
            GrammarRule {
                name: r#"union_case"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"of"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                                ] }) },
                        ] }) },
                ] },
                line_number: 84,
            },
            GrammarRule {
                name: r#"alias_type"#.to_string(),
                body: GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                line_number: 86,
            },
            GrammarRule {
                name: r#"field_separator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                ] },
                line_number: 88,
            },
            GrammarRule {
                name: r#"case_separator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"PIPE"#.to_string() }) },
                    ] },
                    GrammarElement::TokenReference { name: r#"PIPE"#.to_string() },
                ] },
                line_number: 91,
            },
            GrammarRule {
                name: r#"attribute_section"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"attribute"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"attribute"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                ] },
                line_number: 94,
            },
            GrammarRule {
                name: r#"attribute"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"attribute_target"#.to_string() },
                            GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"attribute_arguments"#.to_string() }) },
                            GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                        ] }) },
                ] },
                line_number: 96,
            },
            GrammarRule {
                name: r#"attribute_target"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"assembly"#.to_string() },
                    GrammarElement::Literal { value: r#"field"#.to_string() },
                    GrammarElement::Literal { value: r#"method"#.to_string() },
                    GrammarElement::Literal { value: r#"module"#.to_string() },
                    GrammarElement::Literal { value: r#"param"#.to_string() },
                    GrammarElement::Literal { value: r#"property"#.to_string() },
                    GrammarElement::Literal { value: r#"return"#.to_string() },
                    GrammarElement::Literal { value: r#"type"#.to_string() },
                    GrammarElement::Literal { value: r#"event"#.to_string() },
                ] },
                line_number: 98,
            },
            GrammarRule {
                name: r#"attribute_arguments"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"attribute_argument"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"attribute_argument"#.to_string() },
                        ] }) },
                ] },
                line_number: 108,
            },
            GrammarRule {
                name: r#"attribute_argument"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 110,
            },
            GrammarRule {
                name: r#"parameter_list"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"pattern"#.to_string() }) },
                ] },
                line_number: 112,
            },
            GrammarRule {
                name: r#"type_annotation"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                ] },
                line_number: 114,
            },
            GrammarRule {
                name: r#"type_expression"#.to_string(),
                body: GrammarElement::RuleReference { name: r#"function_type"#.to_string() },
                line_number: 116,
            },
            GrammarRule {
                name: r#"function_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"type_product"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                            GrammarElement::RuleReference { name: r#"function_type"#.to_string() },
                        ] }) },
                ] },
                line_number: 118,
            },
            GrammarRule {
                name: r#"type_product"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"type_application"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_application"#.to_string() },
                        ] }) },
                ] },
                line_number: 120,
            },
            GrammarRule {
                name: r#"type_application"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"type_atom"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"type_atom"#.to_string() }) },
                ] },
                line_number: 122,
            },
            GrammarRule {
                name: r#"generic_type_arguments"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                ] },
                line_number: 124,
            },
            GrammarRule {
                name: r#"type_atom"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"generic_type_arguments"#.to_string() }) },
                    ] },
                    GrammarElement::TokenReference { name: r#"TYPEVAR"#.to_string() },
                    GrammarElement::RuleReference { name: r#"tuple_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"unit_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"parenthesized_type"#.to_string() },
                ] },
                line_number: 126,
            },
            GrammarRule {
                name: r#"tuple_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 133,
            },
            GrammarRule {
                name: r#"unit_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 135,
            },
            GrammarRule {
                name: r#"parenthesized_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 137,
            },
            GrammarRule {
                name: r#"qualified_name"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 139,
            },
            GrammarRule {
                name: r#"expression"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"if_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"match_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"let_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"function_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"fun_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"for_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"while_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"sequence_expression"#.to_string() },
                ] },
                line_number: 141,
            },
            GrammarRule {
                name: r#"sequence_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"infix_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                                GrammarElement::RuleReference { name: r#"infix_expression"#.to_string() },
                            ] },
                            GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                                GrammarElement::RuleReference { name: r#"infix_expression"#.to_string() },
                            ] },
                        ] }) },
                ] },
                line_number: 150,
            },
            GrammarRule {
                name: r#"infix_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"application_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"infix_operator"#.to_string() },
                            GrammarElement::RuleReference { name: r#"application_expression"#.to_string() },
                        ] }) },
                ] },
                line_number: 152,
            },
            GrammarRule {
                name: r#"infix_operator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"PIPE_RIGHT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PIPE_LEFT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMPOSE_RIGHT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMPOSE_LEFT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NOT_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"GREATER_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"DOUBLE_COLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LARROW"#.to_string() },
                    GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                    GrammarElement::TokenReference { name: r#"DOT_DOT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"AND_AND"#.to_string() },
                    GrammarElement::TokenReference { name: r#"OR_OR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SLASH"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PERCENT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"AMPERSAND"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PIPE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CARET"#.to_string() },
                ] },
                line_number: 154,
            },
            GrammarRule {
                name: r#"application_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"prefix_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"prefix_expression"#.to_string() }) },
                ] },
                line_number: 181,
            },
            GrammarRule {
                name: r#"prefix_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"unary_operator"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"atomic_expression"#.to_string() },
                ] },
                line_number: 183,
            },
            GrammarRule {
                name: r#"unary_operator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"BANG"#.to_string() },
                ] },
                line_number: 185,
            },
            GrammarRule {
                name: r#"atomic_expression"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"computation_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"if_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"match_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"let_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"function_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"fun_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"for_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"while_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"unit_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"tuple_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"list_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"array_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"parenthesized_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CHAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"TRUE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"FALSE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NULL"#.to_string() },
                ] },
                line_number: 189,
            },
            GrammarRule {
                name: r#"computation_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"expression"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 211,
            },
            GrammarRule {
                name: r#"unit_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 213,
            },
            GrammarRule {
                name: r#"tuple_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 215,
            },
            GrammarRule {
                name: r#"list_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                    GrammarElement::NegativeLookahead { element: Box::new(GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"element_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                ] },
                line_number: 217,
            },
            GrammarRule {
                name: r#"array_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"ARRAY_LBRACKET"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"element_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"ARRAY_RBRACKET"#.to_string() },
                ] },
                line_number: 219,
            },
            GrammarRule {
                name: r#"record_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"field_assignment"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"field_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"field_assignment"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 221,
            },
            GrammarRule {
                name: r#"field_assignment"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 223,
            },
            GrammarRule {
                name: r#"parenthesized_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 225,
            },
            GrammarRule {
                name: r#"element_separator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                ] },
                line_number: 227,
            },
            GrammarRule {
                name: r#"if_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"if"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Literal { value: r#"then"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"else"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                ] },
                line_number: 230,
            },
            GrammarRule {
                name: r#"match_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"match"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Literal { value: r#"with"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"case_separator"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"match_case"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"case_separator"#.to_string() },
                            GrammarElement::RuleReference { name: r#"match_case"#.to_string() },
                        ] }) },
                ] },
                line_number: 232,
            },
            GrammarRule {
                name: r#"match_case"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"when"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 234,
            },
            GrammarRule {
                name: r#"fun_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"fun"#.to_string() },
                    GrammarElement::RuleReference { name: r#"parameter_list"#.to_string() },
                    GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 236,
            },
            GrammarRule {
                name: r#"function_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"function"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"case_separator"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"match_case"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"case_separator"#.to_string() },
                            GrammarElement::RuleReference { name: r#"match_case"#.to_string() },
                        ] }) },
                ] },
                line_number: 238,
            },
            GrammarRule {
                name: r#"let_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"let"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"rec"#.to_string() }) },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"and"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"in"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 240,
            },
            GrammarRule {
                name: r#"for_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"for"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::Literal { value: r#"to"#.to_string() },
                            GrammarElement::Literal { value: r#"downto"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Literal { value: r#"do"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 242,
            },
            GrammarRule {
                name: r#"while_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"while"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Literal { value: r#"do"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 244,
            },
            GrammarRule {
                name: r#"pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"pattern_atom"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"as"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 246,
            },
            GrammarRule {
                name: r#"pattern_atom"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"wildcard_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"literal_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"tuple_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"list_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"array_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"unit_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"parenthesized_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::TokenReference { name: r#"TYPEVAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                ] },
                line_number: 248,
            },
            GrammarRule {
                name: r#"wildcard_pattern"#.to_string(),
                body: GrammarElement::TokenReference { name: r#"UNDERSCORE"#.to_string() },
                line_number: 260,
            },
            GrammarRule {
                name: r#"literal_pattern"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CHAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"TRUE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"FALSE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NULL"#.to_string() },
                ] },
                line_number: 262,
            },
            GrammarRule {
                name: r#"tuple_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 269,
            },
            GrammarRule {
                name: r#"list_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"element_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                ] },
                line_number: 271,
            },
            GrammarRule {
                name: r#"array_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"ARRAY_LBRACKET"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"element_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"ARRAY_RBRACKET"#.to_string() },
                ] },
                line_number: 273,
            },
            GrammarRule {
                name: r#"record_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"field_pattern"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"field_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"field_pattern"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 275,
            },
            GrammarRule {
                name: r#"field_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                ] },
                line_number: 277,
            },
            GrammarRule {
                name: r#"unit_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 279,
            },
            GrammarRule {
                name: r#"parenthesized_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 281,
            },
        ],
            version: 1,
        }
    }
}

mod v_9 {
    // AUTO-GENERATED FILE — DO NOT EDIT
    // Source: fsharp9.grammar
    // Regenerate with: grammar-tools compile-grammar fsharp9.grammar
    //
    // This file embeds a ParserGrammar as native Rust data structures.
    // Call `parser_grammar()` instead of reading and parsing the .grammar file.

    use grammar_tools::parser_grammar::{GrammarElement, GrammarRule, ParserGrammar};

    pub fn parser_grammar() -> ParserGrammar {
        ParserGrammar {
            rules: vec![
            GrammarRule {
                name: r#"compilation_unit"#.to_string(),
                body: GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                        GrammarElement::RuleReference { name: r#"decorated_declaration"#.to_string() },
                    ] }) },
                line_number: 9,
            },
            GrammarRule {
                name: r#"decorated_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"attribute_section"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"declaration_body"#.to_string() },
                ] },
                line_number: 11,
            },
            GrammarRule {
                name: r#"declaration_body"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"module_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"namespace_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"open_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"let_binding"#.to_string() },
                    GrammarElement::RuleReference { name: r#"use_binding"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"member_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"do_binding"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 13,
            },
            GrammarRule {
                name: r#"module_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"module"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"module_modifier"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"rec"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                        ] }) },
                ] },
                line_number: 23,
            },
            GrammarRule {
                name: r#"module_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"private"#.to_string() },
                    GrammarElement::Literal { value: r#"internal"#.to_string() },
                ] },
                line_number: 25,
            },
            GrammarRule {
                name: r#"namespace_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"namespace"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"namespace_modifier"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                ] },
                line_number: 28,
            },
            GrammarRule {
                name: r#"namespace_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"private"#.to_string() },
                    GrammarElement::Literal { value: r#"internal"#.to_string() },
                ] },
                line_number: 30,
            },
            GrammarRule {
                name: r#"open_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"open"#.to_string() },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                ] },
                line_number: 33,
            },
            GrammarRule {
                name: r#"let_binding"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"let"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"rec"#.to_string() }) },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"and"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                        ] }) },
                ] },
                line_number: 35,
            },
            GrammarRule {
                name: r#"use_binding"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"use"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"rec"#.to_string() }) },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"and"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                        ] }) },
                ] },
                line_number: 37,
            },
            GrammarRule {
                name: r#"binding_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"inline"#.to_string() },
                    GrammarElement::Literal { value: r#"mutable"#.to_string() },
                ] },
                line_number: 39,
            },
            GrammarRule {
                name: r#"binding_clause"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"pattern"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"type_annotation"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 42,
            },
            GrammarRule {
                name: r#"do_binding"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"do"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 44,
            },
            GrammarRule {
                name: r#"member_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"member_modifier"#.to_string() }) },
                    GrammarElement::Literal { value: r#"member"#.to_string() },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"parameter_list"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"type_annotation"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                ] },
                line_number: 46,
            },
            GrammarRule {
                name: r#"member_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"static"#.to_string() },
                    GrammarElement::Literal { value: r#"override"#.to_string() },
                    GrammarElement::Literal { value: r#"default"#.to_string() },
                    GrammarElement::Literal { value: r#"abstract"#.to_string() },
                    GrammarElement::Literal { value: r#"new"#.to_string() },
                ] },
                line_number: 48,
            },
            GrammarRule {
                name: r#"type_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"type"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"type_modifier"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"generic_parameters"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"parameter_list"#.to_string() }) },
                            GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"type_definition"#.to_string() },
                ] },
                line_number: 54,
            },
            GrammarRule {
                name: r#"type_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"private"#.to_string() },
                    GrammarElement::Literal { value: r#"public"#.to_string() },
                    GrammarElement::Literal { value: r#"internal"#.to_string() },
                ] },
                line_number: 56,
            },
            GrammarRule {
                name: r#"generic_parameters"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_parameter"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_parameter"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                ] },
                line_number: 60,
            },
            GrammarRule {
                name: r#"type_parameter"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"TYPEVAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                ] },
                line_number: 62,
            },
            GrammarRule {
                name: r#"type_definition"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"class_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"interface_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"struct_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"union_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"alias_type"#.to_string() },
                ] },
                line_number: 65,
            },
            GrammarRule {
                name: r#"class_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"class"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                            GrammarElement::RuleReference { name: r#"declaration_body"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"end"#.to_string() },
                ] },
                line_number: 72,
            },
            GrammarRule {
                name: r#"interface_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"interface"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                            GrammarElement::RuleReference { name: r#"declaration_body"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"end"#.to_string() },
                ] },
                line_number: 74,
            },
            GrammarRule {
                name: r#"struct_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"struct"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                            GrammarElement::RuleReference { name: r#"declaration_body"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"end"#.to_string() },
                ] },
                line_number: 76,
            },
            GrammarRule {
                name: r#"record_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"field_declaration"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"field_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"field_declaration"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 78,
            },
            GrammarRule {
                name: r#"field_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                ] },
                line_number: 80,
            },
            GrammarRule {
                name: r#"union_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"union_case"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"case_separator"#.to_string() },
                            GrammarElement::RuleReference { name: r#"union_case"#.to_string() },
                        ] }) },
                ] },
                line_number: 82,
            },
            GrammarRule {
                name: r#"union_case"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"of"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                                ] }) },
                        ] }) },
                ] },
                line_number: 84,
            },
            GrammarRule {
                name: r#"alias_type"#.to_string(),
                body: GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                line_number: 86,
            },
            GrammarRule {
                name: r#"field_separator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                ] },
                line_number: 88,
            },
            GrammarRule {
                name: r#"case_separator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"PIPE"#.to_string() }) },
                    ] },
                    GrammarElement::TokenReference { name: r#"PIPE"#.to_string() },
                ] },
                line_number: 91,
            },
            GrammarRule {
                name: r#"attribute_section"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"attribute"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"attribute"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                ] },
                line_number: 94,
            },
            GrammarRule {
                name: r#"attribute"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"attribute_target"#.to_string() },
                            GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"attribute_arguments"#.to_string() }) },
                            GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                        ] }) },
                ] },
                line_number: 96,
            },
            GrammarRule {
                name: r#"attribute_target"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"assembly"#.to_string() },
                    GrammarElement::Literal { value: r#"field"#.to_string() },
                    GrammarElement::Literal { value: r#"method"#.to_string() },
                    GrammarElement::Literal { value: r#"module"#.to_string() },
                    GrammarElement::Literal { value: r#"param"#.to_string() },
                    GrammarElement::Literal { value: r#"property"#.to_string() },
                    GrammarElement::Literal { value: r#"return"#.to_string() },
                    GrammarElement::Literal { value: r#"type"#.to_string() },
                    GrammarElement::Literal { value: r#"event"#.to_string() },
                ] },
                line_number: 98,
            },
            GrammarRule {
                name: r#"attribute_arguments"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"attribute_argument"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"attribute_argument"#.to_string() },
                        ] }) },
                ] },
                line_number: 108,
            },
            GrammarRule {
                name: r#"attribute_argument"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 110,
            },
            GrammarRule {
                name: r#"parameter_list"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"pattern"#.to_string() }) },
                ] },
                line_number: 112,
            },
            GrammarRule {
                name: r#"type_annotation"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                ] },
                line_number: 114,
            },
            GrammarRule {
                name: r#"type_expression"#.to_string(),
                body: GrammarElement::RuleReference { name: r#"function_type"#.to_string() },
                line_number: 116,
            },
            GrammarRule {
                name: r#"function_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"type_product"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                            GrammarElement::RuleReference { name: r#"function_type"#.to_string() },
                        ] }) },
                ] },
                line_number: 118,
            },
            GrammarRule {
                name: r#"type_product"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"type_application"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_application"#.to_string() },
                        ] }) },
                ] },
                line_number: 120,
            },
            GrammarRule {
                name: r#"type_application"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"type_atom"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"type_atom"#.to_string() }) },
                ] },
                line_number: 122,
            },
            GrammarRule {
                name: r#"generic_type_arguments"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                ] },
                line_number: 124,
            },
            GrammarRule {
                name: r#"type_atom"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"generic_type_arguments"#.to_string() }) },
                    ] },
                    GrammarElement::TokenReference { name: r#"TYPEVAR"#.to_string() },
                    GrammarElement::RuleReference { name: r#"tuple_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"unit_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"parenthesized_type"#.to_string() },
                ] },
                line_number: 126,
            },
            GrammarRule {
                name: r#"tuple_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 133,
            },
            GrammarRule {
                name: r#"unit_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 135,
            },
            GrammarRule {
                name: r#"parenthesized_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 137,
            },
            GrammarRule {
                name: r#"qualified_name"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 139,
            },
            GrammarRule {
                name: r#"expression"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"if_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"match_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"let_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"function_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"fun_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"for_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"while_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"sequence_expression"#.to_string() },
                ] },
                line_number: 141,
            },
            GrammarRule {
                name: r#"sequence_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"infix_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                                GrammarElement::RuleReference { name: r#"infix_expression"#.to_string() },
                            ] },
                            GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                                GrammarElement::RuleReference { name: r#"infix_expression"#.to_string() },
                            ] },
                        ] }) },
                ] },
                line_number: 150,
            },
            GrammarRule {
                name: r#"infix_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"application_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"infix_operator"#.to_string() },
                            GrammarElement::RuleReference { name: r#"application_expression"#.to_string() },
                        ] }) },
                ] },
                line_number: 152,
            },
            GrammarRule {
                name: r#"infix_operator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"PIPE_RIGHT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PIPE_LEFT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMPOSE_RIGHT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMPOSE_LEFT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NOT_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"GREATER_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"DOUBLE_COLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LARROW"#.to_string() },
                    GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                    GrammarElement::TokenReference { name: r#"DOT_DOT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"AND_AND"#.to_string() },
                    GrammarElement::TokenReference { name: r#"OR_OR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SLASH"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PERCENT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"AMPERSAND"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PIPE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CARET"#.to_string() },
                ] },
                line_number: 154,
            },
            GrammarRule {
                name: r#"application_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"prefix_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"prefix_expression"#.to_string() }) },
                ] },
                line_number: 181,
            },
            GrammarRule {
                name: r#"prefix_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"unary_operator"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"atomic_expression"#.to_string() },
                ] },
                line_number: 183,
            },
            GrammarRule {
                name: r#"unary_operator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"BANG"#.to_string() },
                ] },
                line_number: 185,
            },
            GrammarRule {
                name: r#"atomic_expression"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"computation_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"if_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"match_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"let_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"function_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"fun_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"for_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"while_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"unit_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"tuple_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"list_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"array_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"parenthesized_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CHAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"TRUE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"FALSE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NULL"#.to_string() },
                ] },
                line_number: 189,
            },
            GrammarRule {
                name: r#"computation_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"expression"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 211,
            },
            GrammarRule {
                name: r#"unit_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 213,
            },
            GrammarRule {
                name: r#"tuple_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 215,
            },
            GrammarRule {
                name: r#"list_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                    GrammarElement::NegativeLookahead { element: Box::new(GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"element_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                ] },
                line_number: 217,
            },
            GrammarRule {
                name: r#"array_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"ARRAY_LBRACKET"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"element_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"ARRAY_RBRACKET"#.to_string() },
                ] },
                line_number: 219,
            },
            GrammarRule {
                name: r#"record_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"field_assignment"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"field_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"field_assignment"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 221,
            },
            GrammarRule {
                name: r#"field_assignment"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 223,
            },
            GrammarRule {
                name: r#"parenthesized_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 225,
            },
            GrammarRule {
                name: r#"element_separator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                ] },
                line_number: 227,
            },
            GrammarRule {
                name: r#"if_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"if"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Literal { value: r#"then"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"else"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                ] },
                line_number: 230,
            },
            GrammarRule {
                name: r#"match_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"match"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Literal { value: r#"with"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"case_separator"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"match_case"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"case_separator"#.to_string() },
                            GrammarElement::RuleReference { name: r#"match_case"#.to_string() },
                        ] }) },
                ] },
                line_number: 232,
            },
            GrammarRule {
                name: r#"match_case"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"when"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 234,
            },
            GrammarRule {
                name: r#"fun_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"fun"#.to_string() },
                    GrammarElement::RuleReference { name: r#"parameter_list"#.to_string() },
                    GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 236,
            },
            GrammarRule {
                name: r#"function_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"function"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"case_separator"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"match_case"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"case_separator"#.to_string() },
                            GrammarElement::RuleReference { name: r#"match_case"#.to_string() },
                        ] }) },
                ] },
                line_number: 238,
            },
            GrammarRule {
                name: r#"let_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"let"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"rec"#.to_string() }) },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"and"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"in"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 240,
            },
            GrammarRule {
                name: r#"for_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"for"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::Literal { value: r#"to"#.to_string() },
                            GrammarElement::Literal { value: r#"downto"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Literal { value: r#"do"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 242,
            },
            GrammarRule {
                name: r#"while_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"while"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Literal { value: r#"do"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 244,
            },
            GrammarRule {
                name: r#"pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"pattern_atom"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"as"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 246,
            },
            GrammarRule {
                name: r#"pattern_atom"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"wildcard_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"literal_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"tuple_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"list_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"array_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"unit_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"parenthesized_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::TokenReference { name: r#"TYPEVAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                ] },
                line_number: 248,
            },
            GrammarRule {
                name: r#"wildcard_pattern"#.to_string(),
                body: GrammarElement::TokenReference { name: r#"UNDERSCORE"#.to_string() },
                line_number: 260,
            },
            GrammarRule {
                name: r#"literal_pattern"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CHAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"TRUE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"FALSE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NULL"#.to_string() },
                ] },
                line_number: 262,
            },
            GrammarRule {
                name: r#"tuple_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 269,
            },
            GrammarRule {
                name: r#"list_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"element_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                ] },
                line_number: 271,
            },
            GrammarRule {
                name: r#"array_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"ARRAY_LBRACKET"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"element_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"ARRAY_RBRACKET"#.to_string() },
                ] },
                line_number: 273,
            },
            GrammarRule {
                name: r#"record_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"field_pattern"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"field_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"field_pattern"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 275,
            },
            GrammarRule {
                name: r#"field_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                ] },
                line_number: 277,
            },
            GrammarRule {
                name: r#"unit_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 279,
            },
            GrammarRule {
                name: r#"parenthesized_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 281,
            },
        ],
            version: 1,
        }
    }
}

mod v_10 {
    // AUTO-GENERATED FILE — DO NOT EDIT
    // Source: fsharp10.grammar
    // Regenerate with: grammar-tools compile-grammar fsharp10.grammar
    //
    // This file embeds a ParserGrammar as native Rust data structures.
    // Call `parser_grammar()` instead of reading and parsing the .grammar file.

    use grammar_tools::parser_grammar::{GrammarElement, GrammarRule, ParserGrammar};

    pub fn parser_grammar() -> ParserGrammar {
        ParserGrammar {
            rules: vec![
            GrammarRule {
                name: r#"compilation_unit"#.to_string(),
                body: GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                        GrammarElement::RuleReference { name: r#"decorated_declaration"#.to_string() },
                    ] }) },
                line_number: 9,
            },
            GrammarRule {
                name: r#"decorated_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"attribute_section"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"declaration_body"#.to_string() },
                ] },
                line_number: 11,
            },
            GrammarRule {
                name: r#"declaration_body"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"module_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"namespace_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"open_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"let_binding"#.to_string() },
                    GrammarElement::RuleReference { name: r#"use_binding"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"member_declaration"#.to_string() },
                    GrammarElement::RuleReference { name: r#"do_binding"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 13,
            },
            GrammarRule {
                name: r#"module_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"module"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"module_modifier"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"rec"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                        ] }) },
                ] },
                line_number: 23,
            },
            GrammarRule {
                name: r#"module_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"private"#.to_string() },
                    GrammarElement::Literal { value: r#"internal"#.to_string() },
                ] },
                line_number: 25,
            },
            GrammarRule {
                name: r#"namespace_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"namespace"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"namespace_modifier"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                ] },
                line_number: 28,
            },
            GrammarRule {
                name: r#"namespace_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"private"#.to_string() },
                    GrammarElement::Literal { value: r#"internal"#.to_string() },
                ] },
                line_number: 30,
            },
            GrammarRule {
                name: r#"open_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"open"#.to_string() },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                ] },
                line_number: 33,
            },
            GrammarRule {
                name: r#"let_binding"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"let"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"rec"#.to_string() }) },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"and"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                        ] }) },
                ] },
                line_number: 35,
            },
            GrammarRule {
                name: r#"use_binding"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"use"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"rec"#.to_string() }) },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"and"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                        ] }) },
                ] },
                line_number: 37,
            },
            GrammarRule {
                name: r#"binding_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"inline"#.to_string() },
                    GrammarElement::Literal { value: r#"mutable"#.to_string() },
                ] },
                line_number: 39,
            },
            GrammarRule {
                name: r#"binding_clause"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"pattern"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"type_annotation"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 42,
            },
            GrammarRule {
                name: r#"do_binding"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"do"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 44,
            },
            GrammarRule {
                name: r#"member_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"member_modifier"#.to_string() }) },
                    GrammarElement::Literal { value: r#"member"#.to_string() },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"parameter_list"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"type_annotation"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                ] },
                line_number: 46,
            },
            GrammarRule {
                name: r#"member_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"static"#.to_string() },
                    GrammarElement::Literal { value: r#"override"#.to_string() },
                    GrammarElement::Literal { value: r#"default"#.to_string() },
                    GrammarElement::Literal { value: r#"abstract"#.to_string() },
                    GrammarElement::Literal { value: r#"new"#.to_string() },
                ] },
                line_number: 48,
            },
            GrammarRule {
                name: r#"type_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"type"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"type_modifier"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"generic_parameters"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"parameter_list"#.to_string() }) },
                            GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"type_definition"#.to_string() },
                ] },
                line_number: 54,
            },
            GrammarRule {
                name: r#"type_modifier"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"private"#.to_string() },
                    GrammarElement::Literal { value: r#"public"#.to_string() },
                    GrammarElement::Literal { value: r#"internal"#.to_string() },
                ] },
                line_number: 56,
            },
            GrammarRule {
                name: r#"generic_parameters"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_parameter"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_parameter"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                ] },
                line_number: 60,
            },
            GrammarRule {
                name: r#"type_parameter"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"TYPEVAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                ] },
                line_number: 62,
            },
            GrammarRule {
                name: r#"type_definition"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"class_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"interface_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"struct_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"union_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"alias_type"#.to_string() },
                ] },
                line_number: 65,
            },
            GrammarRule {
                name: r#"class_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"class"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                            GrammarElement::RuleReference { name: r#"declaration_body"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"end"#.to_string() },
                ] },
                line_number: 72,
            },
            GrammarRule {
                name: r#"interface_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"interface"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                            GrammarElement::RuleReference { name: r#"declaration_body"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"end"#.to_string() },
                ] },
                line_number: 74,
            },
            GrammarRule {
                name: r#"struct_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"struct"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                            GrammarElement::RuleReference { name: r#"declaration_body"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"end"#.to_string() },
                ] },
                line_number: 76,
            },
            GrammarRule {
                name: r#"record_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"field_declaration"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"field_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"field_declaration"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 78,
            },
            GrammarRule {
                name: r#"field_declaration"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                ] },
                line_number: 80,
            },
            GrammarRule {
                name: r#"union_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"union_case"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"case_separator"#.to_string() },
                            GrammarElement::RuleReference { name: r#"union_case"#.to_string() },
                        ] }) },
                ] },
                line_number: 82,
            },
            GrammarRule {
                name: r#"union_case"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"of"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                                ] }) },
                        ] }) },
                ] },
                line_number: 84,
            },
            GrammarRule {
                name: r#"alias_type"#.to_string(),
                body: GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                line_number: 86,
            },
            GrammarRule {
                name: r#"field_separator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                ] },
                line_number: 88,
            },
            GrammarRule {
                name: r#"case_separator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"PIPE"#.to_string() }) },
                    ] },
                    GrammarElement::TokenReference { name: r#"PIPE"#.to_string() },
                ] },
                line_number: 91,
            },
            GrammarRule {
                name: r#"attribute_section"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"attribute"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"attribute"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                ] },
                line_number: 94,
            },
            GrammarRule {
                name: r#"attribute"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"attribute_target"#.to_string() },
                            GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"attribute_arguments"#.to_string() }) },
                            GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                        ] }) },
                ] },
                line_number: 96,
            },
            GrammarRule {
                name: r#"attribute_target"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Literal { value: r#"assembly"#.to_string() },
                    GrammarElement::Literal { value: r#"field"#.to_string() },
                    GrammarElement::Literal { value: r#"method"#.to_string() },
                    GrammarElement::Literal { value: r#"module"#.to_string() },
                    GrammarElement::Literal { value: r#"param"#.to_string() },
                    GrammarElement::Literal { value: r#"property"#.to_string() },
                    GrammarElement::Literal { value: r#"return"#.to_string() },
                    GrammarElement::Literal { value: r#"type"#.to_string() },
                    GrammarElement::Literal { value: r#"event"#.to_string() },
                ] },
                line_number: 98,
            },
            GrammarRule {
                name: r#"attribute_arguments"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"attribute_argument"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"attribute_argument"#.to_string() },
                        ] }) },
                ] },
                line_number: 108,
            },
            GrammarRule {
                name: r#"attribute_argument"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 110,
            },
            GrammarRule {
                name: r#"parameter_list"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"pattern"#.to_string() }) },
                ] },
                line_number: 112,
            },
            GrammarRule {
                name: r#"type_annotation"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                ] },
                line_number: 114,
            },
            GrammarRule {
                name: r#"type_expression"#.to_string(),
                body: GrammarElement::RuleReference { name: r#"function_type"#.to_string() },
                line_number: 116,
            },
            GrammarRule {
                name: r#"function_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"type_product"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                            GrammarElement::RuleReference { name: r#"function_type"#.to_string() },
                        ] }) },
                ] },
                line_number: 118,
            },
            GrammarRule {
                name: r#"type_product"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"type_application"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_application"#.to_string() },
                        ] }) },
                ] },
                line_number: 120,
            },
            GrammarRule {
                name: r#"type_application"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"type_atom"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"type_atom"#.to_string() }) },
                ] },
                line_number: 122,
            },
            GrammarRule {
                name: r#"generic_type_arguments"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                ] },
                line_number: 124,
            },
            GrammarRule {
                name: r#"type_atom"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"generic_type_arguments"#.to_string() }) },
                    ] },
                    GrammarElement::TokenReference { name: r#"TYPEVAR"#.to_string() },
                    GrammarElement::RuleReference { name: r#"tuple_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"unit_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_type"#.to_string() },
                    GrammarElement::RuleReference { name: r#"parenthesized_type"#.to_string() },
                ] },
                line_number: 126,
            },
            GrammarRule {
                name: r#"tuple_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 133,
            },
            GrammarRule {
                name: r#"unit_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 135,
            },
            GrammarRule {
                name: r#"parenthesized_type"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 137,
            },
            GrammarRule {
                name: r#"qualified_name"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 139,
            },
            GrammarRule {
                name: r#"expression"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"if_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"match_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"let_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"function_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"fun_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"for_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"while_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"sequence_expression"#.to_string() },
                ] },
                line_number: 141,
            },
            GrammarRule {
                name: r#"sequence_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"infix_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                                GrammarElement::RuleReference { name: r#"infix_expression"#.to_string() },
                            ] },
                            GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                                GrammarElement::RuleReference { name: r#"infix_expression"#.to_string() },
                            ] },
                        ] }) },
                ] },
                line_number: 150,
            },
            GrammarRule {
                name: r#"infix_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"application_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"infix_operator"#.to_string() },
                            GrammarElement::RuleReference { name: r#"application_expression"#.to_string() },
                        ] }) },
                ] },
                line_number: 152,
            },
            GrammarRule {
                name: r#"infix_operator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"PIPE_RIGHT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PIPE_LEFT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMPOSE_RIGHT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMPOSE_LEFT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NOT_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"GREATER_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COLON_EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"DOUBLE_COLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LARROW"#.to_string() },
                    GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                    GrammarElement::TokenReference { name: r#"DOT_DOT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"AND_AND"#.to_string() },
                    GrammarElement::TokenReference { name: r#"OR_OR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"SLASH"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PERCENT"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"GREATER_THAN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"AMPERSAND"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PIPE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CARET"#.to_string() },
                ] },
                line_number: 154,
            },
            GrammarRule {
                name: r#"application_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"prefix_expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"prefix_expression"#.to_string() }) },
                ] },
                line_number: 181,
            },
            GrammarRule {
                name: r#"prefix_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"unary_operator"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"atomic_expression"#.to_string() },
                ] },
                line_number: 183,
            },
            GrammarRule {
                name: r#"unary_operator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                    GrammarElement::TokenReference { name: r#"BANG"#.to_string() },
                ] },
                line_number: 185,
            },
            GrammarRule {
                name: r#"atomic_expression"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"computation_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"if_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"match_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"let_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"function_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"fun_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"for_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"while_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"unit_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"tuple_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"list_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"array_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"parenthesized_expression"#.to_string() },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CHAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"TRUE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"FALSE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NULL"#.to_string() },
                ] },
                line_number: 189,
            },
            GrammarRule {
                name: r#"computation_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"expression"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 211,
            },
            GrammarRule {
                name: r#"unit_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 213,
            },
            GrammarRule {
                name: r#"tuple_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 215,
            },
            GrammarRule {
                name: r#"list_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                    GrammarElement::NegativeLookahead { element: Box::new(GrammarElement::TokenReference { name: r#"LESS_THAN"#.to_string() }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"element_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                ] },
                line_number: 217,
            },
            GrammarRule {
                name: r#"array_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"ARRAY_LBRACKET"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"element_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"ARRAY_RBRACKET"#.to_string() },
                ] },
                line_number: 219,
            },
            GrammarRule {
                name: r#"record_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"field_assignment"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"field_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"field_assignment"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 221,
            },
            GrammarRule {
                name: r#"field_assignment"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 223,
            },
            GrammarRule {
                name: r#"parenthesized_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 225,
            },
            GrammarRule {
                name: r#"element_separator"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                ] },
                line_number: 227,
            },
            GrammarRule {
                name: r#"if_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"if"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Literal { value: r#"then"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"else"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                ] },
                line_number: 230,
            },
            GrammarRule {
                name: r#"match_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"match"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Literal { value: r#"with"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"case_separator"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"match_case"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"case_separator"#.to_string() },
                            GrammarElement::RuleReference { name: r#"match_case"#.to_string() },
                        ] }) },
                ] },
                line_number: 232,
            },
            GrammarRule {
                name: r#"match_case"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"when"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 234,
            },
            GrammarRule {
                name: r#"fun_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"fun"#.to_string() },
                    GrammarElement::RuleReference { name: r#"parameter_list"#.to_string() },
                    GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 236,
            },
            GrammarRule {
                name: r#"function_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"function"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"case_separator"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"match_case"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"case_separator"#.to_string() },
                            GrammarElement::RuleReference { name: r#"match_case"#.to_string() },
                        ] }) },
                ] },
                line_number: 238,
            },
            GrammarRule {
                name: r#"let_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"let"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"rec"#.to_string() }) },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"and"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"binding_modifier"#.to_string() }) },
                            GrammarElement::RuleReference { name: r#"binding_clause"#.to_string() },
                        ] }) },
                    GrammarElement::Literal { value: r#"in"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 240,
            },
            GrammarRule {
                name: r#"for_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"for"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::Literal { value: r#"to"#.to_string() },
                            GrammarElement::Literal { value: r#"downto"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Literal { value: r#"do"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 242,
            },
            GrammarRule {
                name: r#"while_expression"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"while"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::Literal { value: r#"do"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                ] },
                line_number: 244,
            },
            GrammarRule {
                name: r#"pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"pattern_atom"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type_expression"#.to_string() },
                        ] }) },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"as"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] }) },
                ] },
                line_number: 246,
            },
            GrammarRule {
                name: r#"pattern_atom"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::RuleReference { name: r#"wildcard_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"literal_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"tuple_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"list_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"array_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"record_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"unit_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"parenthesized_pattern"#.to_string() },
                    GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                    GrammarElement::TokenReference { name: r#"TYPEVAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                ] },
                line_number: 248,
            },
            GrammarRule {
                name: r#"wildcard_pattern"#.to_string(),
                body: GrammarElement::TokenReference { name: r#"UNDERSCORE"#.to_string() },
                line_number: 260,
            },
            GrammarRule {
                name: r#"literal_pattern"#.to_string(),
                body: GrammarElement::Alternation { choices: vec![
                    GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                    GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                    GrammarElement::TokenReference { name: r#"CHAR"#.to_string() },
                    GrammarElement::TokenReference { name: r#"TRUE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"FALSE"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NULL"#.to_string() },
                ] },
                line_number: 262,
            },
            GrammarRule {
                name: r#"tuple_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                            GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 269,
            },
            GrammarRule {
                name: r#"list_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"element_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                ] },
                line_number: 271,
            },
            GrammarRule {
                name: r#"array_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"ARRAY_LBRACKET"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"element_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"ARRAY_RBRACKET"#.to_string() },
                ] },
                line_number: 273,
            },
            GrammarRule {
                name: r#"record_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"field_pattern"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::RuleReference { name: r#"field_separator"#.to_string() },
                                    GrammarElement::RuleReference { name: r#"field_pattern"#.to_string() },
                                ] }) },
                        ] }) },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
                line_number: 275,
            },
            GrammarRule {
                name: r#"field_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                ] },
                line_number: 277,
            },
            GrammarRule {
                name: r#"unit_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 279,
            },
            GrammarRule {
                name: r#"parenthesized_pattern"#.to_string(),
                body: GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                line_number: 281,
            },
        ],
            version: 1,
        }
    }
}

