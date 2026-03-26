// AUTO-GENERATED FILE - DO NOT EDIT
#![allow(clippy::all)]
use grammar_tools::parser_grammar::{ParserGrammar, GrammarRule, GrammarElement};

pub fn VhdlGrammar() -> ParserGrammar {
    ParserGrammar {
        version: 0,
        rules: vec![
            GrammarRule {
                name: "design_file".to_string(),
                line_number: 64,
                body: GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: "design_unit".to_string() }) },
            },
            GrammarRule {
                name: "design_unit".to_string(),
                line_number: 66,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: "context_item".to_string() }) }, GrammarElement::RuleReference { name: "library_unit".to_string() }] },
            },
            GrammarRule {
                name: "context_item".to_string(),
                line_number: 68,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::RuleReference { name: "library_clause".to_string() }, GrammarElement::RuleReference { name: "use_clause".to_string() }] },
            },
            GrammarRule {
                name: "library_clause".to_string(),
                line_number: 71,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "library".to_string() }, GrammarElement::RuleReference { name: "name_list".to_string() }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }] },
            },
            GrammarRule {
                name: "use_clause".to_string(),
                line_number: 74,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "use".to_string() }, GrammarElement::RuleReference { name: "selected_name".to_string() }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }] },
            },
            GrammarRule {
                name: "selected_name".to_string(),
                line_number: 77,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "DOT".to_string() }, GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::Literal { value: "all".to_string() }] }) }] }) }] },
            },
            GrammarRule {
                name: "name_list".to_string(),
                line_number: 79,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "COMMA".to_string() }, GrammarElement::TokenReference { name: "NAME".to_string() }] }) }] },
            },
            GrammarRule {
                name: "library_unit".to_string(),
                line_number: 81,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::RuleReference { name: "entity_declaration".to_string() }, GrammarElement::RuleReference { name: "architecture_body".to_string() }, GrammarElement::RuleReference { name: "package_declaration".to_string() }, GrammarElement::RuleReference { name: "package_body".to_string() }] },
            },
            GrammarRule {
                name: "entity_declaration".to_string(),
                line_number: 112,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "entity".to_string() }, GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::Literal { value: "is".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: "generic_clause".to_string() }) }, GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: "port_clause".to_string() }) }, GrammarElement::Literal { value: "end".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: "entity".to_string() }) }, GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: "NAME".to_string() }) }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }] },
            },
            GrammarRule {
                name: "generic_clause".to_string(),
                line_number: 117,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "generic".to_string() }, GrammarElement::TokenReference { name: "LPAREN".to_string() }, GrammarElement::RuleReference { name: "interface_list".to_string() }, GrammarElement::TokenReference { name: "RPAREN".to_string() }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }] },
            },
            GrammarRule {
                name: "port_clause".to_string(),
                line_number: 118,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "port".to_string() }, GrammarElement::TokenReference { name: "LPAREN".to_string() }, GrammarElement::RuleReference { name: "interface_list".to_string() }, GrammarElement::TokenReference { name: "RPAREN".to_string() }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }] },
            },
            GrammarRule {
                name: "interface_list".to_string(),
                line_number: 123,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "interface_element".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "SEMICOLON".to_string() }, GrammarElement::RuleReference { name: "interface_element".to_string() }] }) }] },
            },
            GrammarRule {
                name: "interface_element".to_string(),
                line_number: 124,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "name_list".to_string() }, GrammarElement::TokenReference { name: "COLON".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: "mode".to_string() }) }, GrammarElement::RuleReference { name: "subtype_indication".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "VAR_ASSIGN".to_string() }, GrammarElement::RuleReference { name: "expression".to_string() }] }) }] },
            },
            GrammarRule {
                name: "mode".to_string(),
                line_number: 132,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::Literal { value: "in".to_string() }, GrammarElement::Literal { value: "out".to_string() }, GrammarElement::Literal { value: "inout".to_string() }, GrammarElement::Literal { value: "buffer".to_string() }] },
            },
            GrammarRule {
                name: "architecture_body".to_string(),
                line_number: 154,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "architecture".to_string() }, GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::Literal { value: "of".to_string() }, GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::Literal { value: "is".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: "block_declarative_item".to_string() }) }, GrammarElement::Literal { value: "begin".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: "concurrent_statement".to_string() }) }, GrammarElement::Literal { value: "end".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: "architecture".to_string() }) }, GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: "NAME".to_string() }) }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }] },
            },
            GrammarRule {
                name: "block_declarative_item".to_string(),
                line_number: 160,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::RuleReference { name: "signal_declaration".to_string() }, GrammarElement::RuleReference { name: "constant_declaration".to_string() }, GrammarElement::RuleReference { name: "type_declaration".to_string() }, GrammarElement::RuleReference { name: "subtype_declaration".to_string() }, GrammarElement::RuleReference { name: "component_declaration".to_string() }, GrammarElement::RuleReference { name: "function_declaration".to_string() }, GrammarElement::RuleReference { name: "function_body".to_string() }, GrammarElement::RuleReference { name: "procedure_declaration".to_string() }, GrammarElement::RuleReference { name: "procedure_body".to_string() }] },
            },
            GrammarRule {
                name: "signal_declaration".to_string(),
                line_number: 189,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "signal".to_string() }, GrammarElement::RuleReference { name: "name_list".to_string() }, GrammarElement::TokenReference { name: "COLON".to_string() }, GrammarElement::RuleReference { name: "subtype_indication".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "VAR_ASSIGN".to_string() }, GrammarElement::RuleReference { name: "expression".to_string() }] }) }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }] },
            },
            GrammarRule {
                name: "constant_declaration".to_string(),
                line_number: 191,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "constant".to_string() }, GrammarElement::RuleReference { name: "name_list".to_string() }, GrammarElement::TokenReference { name: "COLON".to_string() }, GrammarElement::RuleReference { name: "subtype_indication".to_string() }, GrammarElement::TokenReference { name: "VAR_ASSIGN".to_string() }, GrammarElement::RuleReference { name: "expression".to_string() }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }] },
            },
            GrammarRule {
                name: "variable_declaration".to_string(),
                line_number: 193,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "variable".to_string() }, GrammarElement::RuleReference { name: "name_list".to_string() }, GrammarElement::TokenReference { name: "COLON".to_string() }, GrammarElement::RuleReference { name: "subtype_indication".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "VAR_ASSIGN".to_string() }, GrammarElement::RuleReference { name: "expression".to_string() }] }) }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }] },
            },
            GrammarRule {
                name: "type_declaration".to_string(),
                line_number: 218,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "type".to_string() }, GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::Literal { value: "is".to_string() }, GrammarElement::RuleReference { name: "type_definition".to_string() }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }] },
            },
            GrammarRule {
                name: "subtype_declaration".to_string(),
                line_number: 219,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "subtype".to_string() }, GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::Literal { value: "is".to_string() }, GrammarElement::RuleReference { name: "subtype_indication".to_string() }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }] },
            },
            GrammarRule {
                name: "type_definition".to_string(),
                line_number: 221,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::RuleReference { name: "enumeration_type".to_string() }, GrammarElement::RuleReference { name: "array_type".to_string() }, GrammarElement::RuleReference { name: "record_type".to_string() }] },
            },
            GrammarRule {
                name: "enumeration_type".to_string(),
                line_number: 227,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "LPAREN".to_string() }, GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::TokenReference { name: "CHAR_LITERAL".to_string() }] }) }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "COMMA".to_string() }, GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::TokenReference { name: "CHAR_LITERAL".to_string() }] }) }] }) }, GrammarElement::TokenReference { name: "RPAREN".to_string() }] },
            },
            GrammarRule {
                name: "array_type".to_string(),
                line_number: 232,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "array".to_string() }, GrammarElement::TokenReference { name: "LPAREN".to_string() }, GrammarElement::RuleReference { name: "index_constraint".to_string() }, GrammarElement::TokenReference { name: "RPAREN".to_string() }, GrammarElement::Literal { value: "of".to_string() }, GrammarElement::RuleReference { name: "subtype_indication".to_string() }] },
            },
            GrammarRule {
                name: "index_constraint".to_string(),
                line_number: 234,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "discrete_range".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "COMMA".to_string() }, GrammarElement::RuleReference { name: "discrete_range".to_string() }] }) }] },
            },
            GrammarRule {
                name: "discrete_range".to_string(),
                line_number: 235,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::RuleReference { name: "subtype_indication".to_string() }, GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "expression".to_string() }, GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![GrammarElement::Literal { value: "to".to_string() }, GrammarElement::Literal { value: "downto".to_string() }] }) }, GrammarElement::RuleReference { name: "expression".to_string() }] }] },
            },
            GrammarRule {
                name: "record_type".to_string(),
                line_number: 239,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "record".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::TokenReference { name: "COLON".to_string() }, GrammarElement::RuleReference { name: "subtype_indication".to_string() }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }] }) }, GrammarElement::Literal { value: "end".to_string() }, GrammarElement::Literal { value: "record".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: "NAME".to_string() }) }] },
            },
            GrammarRule {
                name: "subtype_indication".to_string(),
                line_number: 247,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "selected_name".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: "constraint".to_string() }) }] },
            },
            GrammarRule {
                name: "constraint".to_string(),
                line_number: 249,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "LPAREN".to_string() }, GrammarElement::RuleReference { name: "expression".to_string() }, GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![GrammarElement::Literal { value: "to".to_string() }, GrammarElement::Literal { value: "downto".to_string() }] }) }, GrammarElement::RuleReference { name: "expression".to_string() }, GrammarElement::TokenReference { name: "RPAREN".to_string() }] }, GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "range".to_string() }, GrammarElement::RuleReference { name: "expression".to_string() }, GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![GrammarElement::Literal { value: "to".to_string() }, GrammarElement::Literal { value: "downto".to_string() }] }) }, GrammarElement::RuleReference { name: "expression".to_string() }] }] },
            },
            GrammarRule {
                name: "concurrent_statement".to_string(),
                line_number: 264,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::RuleReference { name: "process_statement".to_string() }, GrammarElement::RuleReference { name: "signal_assignment_concurrent".to_string() }, GrammarElement::RuleReference { name: "component_instantiation".to_string() }, GrammarElement::RuleReference { name: "generate_statement".to_string() }] },
            },
            GrammarRule {
                name: "signal_assignment_concurrent".to_string(),
                line_number: 272,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::TokenReference { name: "LESS_EQUALS".to_string() }, GrammarElement::RuleReference { name: "waveform".to_string() }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }] },
            },
            GrammarRule {
                name: "waveform".to_string(),
                line_number: 274,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "waveform_element".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "COMMA".to_string() }, GrammarElement::RuleReference { name: "waveform_element".to_string() }] }) }] },
            },
            GrammarRule {
                name: "waveform_element".to_string(),
                line_number: 275,
                body: GrammarElement::RuleReference { name: "expression".to_string() },
            },
            GrammarRule {
                name: "process_statement".to_string(),
                line_number: 307,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::TokenReference { name: "COLON".to_string() }] }) }, GrammarElement::Literal { value: "process".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "LPAREN".to_string() }, GrammarElement::RuleReference { name: "sensitivity_list".to_string() }, GrammarElement::TokenReference { name: "RPAREN".to_string() }] }) }, GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: "is".to_string() }) }, GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: "process_declarative_item".to_string() }) }, GrammarElement::Literal { value: "begin".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: "sequential_statement".to_string() }) }, GrammarElement::Literal { value: "end".to_string() }, GrammarElement::Literal { value: "process".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: "NAME".to_string() }) }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }] },
            },
            GrammarRule {
                name: "sensitivity_list".to_string(),
                line_number: 315,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "COMMA".to_string() }, GrammarElement::TokenReference { name: "NAME".to_string() }] }) }] },
            },
            GrammarRule {
                name: "process_declarative_item".to_string(),
                line_number: 317,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::RuleReference { name: "variable_declaration".to_string() }, GrammarElement::RuleReference { name: "constant_declaration".to_string() }, GrammarElement::RuleReference { name: "type_declaration".to_string() }, GrammarElement::RuleReference { name: "subtype_declaration".to_string() }] },
            },
            GrammarRule {
                name: "sequential_statement".to_string(),
                line_number: 329,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::RuleReference { name: "signal_assignment_seq".to_string() }, GrammarElement::RuleReference { name: "variable_assignment".to_string() }, GrammarElement::RuleReference { name: "if_statement".to_string() }, GrammarElement::RuleReference { name: "case_statement".to_string() }, GrammarElement::RuleReference { name: "loop_statement".to_string() }, GrammarElement::RuleReference { name: "return_statement".to_string() }, GrammarElement::RuleReference { name: "null_statement".to_string() }] },
            },
            GrammarRule {
                name: "signal_assignment_seq".to_string(),
                line_number: 342,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::TokenReference { name: "LESS_EQUALS".to_string() }, GrammarElement::RuleReference { name: "waveform".to_string() }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }] },
            },
            GrammarRule {
                name: "variable_assignment".to_string(),
                line_number: 346,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::TokenReference { name: "VAR_ASSIGN".to_string() }, GrammarElement::RuleReference { name: "expression".to_string() }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }] },
            },
            GrammarRule {
                name: "if_statement".to_string(),
                line_number: 356,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "if".to_string() }, GrammarElement::RuleReference { name: "expression".to_string() }, GrammarElement::Literal { value: "then".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: "sequential_statement".to_string() }) }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "elsif".to_string() }, GrammarElement::RuleReference { name: "expression".to_string() }, GrammarElement::Literal { value: "then".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: "sequential_statement".to_string() }) }] }) }, GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "else".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: "sequential_statement".to_string() }) }] }) }, GrammarElement::Literal { value: "end".to_string() }, GrammarElement::Literal { value: "if".to_string() }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }] },
            },
            GrammarRule {
                name: "case_statement".to_string(),
                line_number: 372,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "case".to_string() }, GrammarElement::RuleReference { name: "expression".to_string() }, GrammarElement::Literal { value: "is".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "when".to_string() }, GrammarElement::RuleReference { name: "choices".to_string() }, GrammarElement::TokenReference { name: "ARROW".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: "sequential_statement".to_string() }) }] }) }, GrammarElement::Literal { value: "end".to_string() }, GrammarElement::Literal { value: "case".to_string() }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }] },
            },
            GrammarRule {
                name: "choices".to_string(),
                line_number: 376,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "choice".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "PIPE".to_string() }, GrammarElement::RuleReference { name: "choice".to_string() }] }) }] },
            },
            GrammarRule {
                name: "choice".to_string(),
                line_number: 377,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::RuleReference { name: "expression".to_string() }, GrammarElement::RuleReference { name: "discrete_range".to_string() }, GrammarElement::Literal { value: "others".to_string() }] },
            },
            GrammarRule {
                name: "loop_statement".to_string(),
                line_number: 391,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::TokenReference { name: "COLON".to_string() }] }) }, GrammarElement::Optional { element: Box::new(GrammarElement::Alternation { choices: vec![GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "for".to_string() }, GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::Literal { value: "in".to_string() }, GrammarElement::RuleReference { name: "discrete_range".to_string() }] }, GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "while".to_string() }, GrammarElement::RuleReference { name: "expression".to_string() }] }] }) }, GrammarElement::Literal { value: "loop".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: "sequential_statement".to_string() }) }, GrammarElement::Literal { value: "end".to_string() }, GrammarElement::Literal { value: "loop".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: "NAME".to_string() }) }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }] },
            },
            GrammarRule {
                name: "return_statement".to_string(),
                line_number: 398,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "return".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: "expression".to_string() }) }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }] },
            },
            GrammarRule {
                name: "null_statement".to_string(),
                line_number: 399,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "null".to_string() }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }] },
            },
            GrammarRule {
                name: "component_declaration".to_string(),
                line_number: 425,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "component".to_string() }, GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: "is".to_string() }) }, GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: "generic_clause".to_string() }) }, GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: "port_clause".to_string() }) }, GrammarElement::Literal { value: "end".to_string() }, GrammarElement::Literal { value: "component".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: "NAME".to_string() }) }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }] },
            },
            GrammarRule {
                name: "component_instantiation".to_string(),
                line_number: 430,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::TokenReference { name: "COLON".to_string() }, GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "entity".to_string() }, GrammarElement::RuleReference { name: "selected_name".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "LPAREN".to_string() }, GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::TokenReference { name: "RPAREN".to_string() }] }) }] }] }) }, GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "generic".to_string() }, GrammarElement::Literal { value: "map".to_string() }, GrammarElement::TokenReference { name: "LPAREN".to_string() }, GrammarElement::RuleReference { name: "association_list".to_string() }, GrammarElement::TokenReference { name: "RPAREN".to_string() }] }) }, GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "port".to_string() }, GrammarElement::Literal { value: "map".to_string() }, GrammarElement::TokenReference { name: "LPAREN".to_string() }, GrammarElement::RuleReference { name: "association_list".to_string() }, GrammarElement::TokenReference { name: "RPAREN".to_string() }] }) }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }] },
            },
            GrammarRule {
                name: "association_list".to_string(),
                line_number: 437,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "association_element".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "COMMA".to_string() }, GrammarElement::RuleReference { name: "association_element".to_string() }] }) }] },
            },
            GrammarRule {
                name: "association_element".to_string(),
                line_number: 438,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::Sequence { elements: vec![GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::TokenReference { name: "ARROW".to_string() }] }) }, GrammarElement::RuleReference { name: "expression".to_string() }] }, GrammarElement::Sequence { elements: vec![GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::TokenReference { name: "ARROW".to_string() }] }) }, GrammarElement::Literal { value: "open".to_string() }] }] },
            },
            GrammarRule {
                name: "generate_statement".to_string(),
                line_number: 461,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::TokenReference { name: "COLON".to_string() }, GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![GrammarElement::RuleReference { name: "for_generate".to_string() }, GrammarElement::RuleReference { name: "if_generate".to_string() }] }) }] },
            },
            GrammarRule {
                name: "for_generate".to_string(),
                line_number: 463,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "for".to_string() }, GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::Literal { value: "in".to_string() }, GrammarElement::RuleReference { name: "discrete_range".to_string() }, GrammarElement::Literal { value: "generate".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: "concurrent_statement".to_string() }) }, GrammarElement::Literal { value: "end".to_string() }, GrammarElement::Literal { value: "generate".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: "NAME".to_string() }) }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }] },
            },
            GrammarRule {
                name: "if_generate".to_string(),
                line_number: 467,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "if".to_string() }, GrammarElement::RuleReference { name: "expression".to_string() }, GrammarElement::Literal { value: "generate".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: "concurrent_statement".to_string() }) }, GrammarElement::Literal { value: "end".to_string() }, GrammarElement::Literal { value: "generate".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: "NAME".to_string() }) }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }] },
            },
            GrammarRule {
                name: "package_declaration".to_string(),
                line_number: 488,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "package".to_string() }, GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::Literal { value: "is".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: "package_declarative_item".to_string() }) }, GrammarElement::Literal { value: "end".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: "package".to_string() }) }, GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: "NAME".to_string() }) }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }] },
            },
            GrammarRule {
                name: "package_body".to_string(),
                line_number: 492,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "package".to_string() }, GrammarElement::Literal { value: "body".to_string() }, GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::Literal { value: "is".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: "package_body_declarative_item".to_string() }) }, GrammarElement::Literal { value: "end".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "package".to_string() }, GrammarElement::Literal { value: "body".to_string() }] }) }, GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: "NAME".to_string() }) }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }] },
            },
            GrammarRule {
                name: "package_declarative_item".to_string(),
                line_number: 496,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::RuleReference { name: "type_declaration".to_string() }, GrammarElement::RuleReference { name: "subtype_declaration".to_string() }, GrammarElement::RuleReference { name: "constant_declaration".to_string() }, GrammarElement::RuleReference { name: "signal_declaration".to_string() }, GrammarElement::RuleReference { name: "component_declaration".to_string() }, GrammarElement::RuleReference { name: "function_declaration".to_string() }, GrammarElement::RuleReference { name: "procedure_declaration".to_string() }] },
            },
            GrammarRule {
                name: "package_body_declarative_item".to_string(),
                line_number: 504,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::RuleReference { name: "type_declaration".to_string() }, GrammarElement::RuleReference { name: "subtype_declaration".to_string() }, GrammarElement::RuleReference { name: "constant_declaration".to_string() }, GrammarElement::RuleReference { name: "function_body".to_string() }, GrammarElement::RuleReference { name: "procedure_body".to_string() }] },
            },
            GrammarRule {
                name: "function_declaration".to_string(),
                line_number: 520,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Optional { element: Box::new(GrammarElement::Alternation { choices: vec![GrammarElement::Literal { value: "pure".to_string() }, GrammarElement::Literal { value: "impure".to_string() }] }) }, GrammarElement::Literal { value: "function".to_string() }, GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "LPAREN".to_string() }, GrammarElement::RuleReference { name: "interface_list".to_string() }, GrammarElement::TokenReference { name: "RPAREN".to_string() }] }) }, GrammarElement::Literal { value: "return".to_string() }, GrammarElement::RuleReference { name: "subtype_indication".to_string() }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }] },
            },
            GrammarRule {
                name: "function_body".to_string(),
                line_number: 525,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Optional { element: Box::new(GrammarElement::Alternation { choices: vec![GrammarElement::Literal { value: "pure".to_string() }, GrammarElement::Literal { value: "impure".to_string() }] }) }, GrammarElement::Literal { value: "function".to_string() }, GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "LPAREN".to_string() }, GrammarElement::RuleReference { name: "interface_list".to_string() }, GrammarElement::TokenReference { name: "RPAREN".to_string() }] }) }, GrammarElement::Literal { value: "return".to_string() }, GrammarElement::RuleReference { name: "subtype_indication".to_string() }, GrammarElement::Literal { value: "is".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: "process_declarative_item".to_string() }) }, GrammarElement::Literal { value: "begin".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: "sequential_statement".to_string() }) }, GrammarElement::Literal { value: "end".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: "function".to_string() }) }, GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: "NAME".to_string() }) }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }] },
            },
            GrammarRule {
                name: "procedure_declaration".to_string(),
                line_number: 534,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "procedure".to_string() }, GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "LPAREN".to_string() }, GrammarElement::RuleReference { name: "interface_list".to_string() }, GrammarElement::TokenReference { name: "RPAREN".to_string() }] }) }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }] },
            },
            GrammarRule {
                name: "procedure_body".to_string(),
                line_number: 537,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "procedure".to_string() }, GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "LPAREN".to_string() }, GrammarElement::RuleReference { name: "interface_list".to_string() }, GrammarElement::TokenReference { name: "RPAREN".to_string() }] }) }, GrammarElement::Literal { value: "is".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: "process_declarative_item".to_string() }) }, GrammarElement::Literal { value: "begin".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: "sequential_statement".to_string() }) }, GrammarElement::Literal { value: "end".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: "procedure".to_string() }) }, GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: "NAME".to_string() }) }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }] },
            },
            GrammarRule {
                name: "expression".to_string(),
                line_number: 574,
                body: GrammarElement::RuleReference { name: "logical_expr".to_string() },
            },
            GrammarRule {
                name: "logical_expr".to_string(),
                line_number: 581,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "relation".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "logical_op".to_string() }, GrammarElement::RuleReference { name: "relation".to_string() }] }) }] },
            },
            GrammarRule {
                name: "logical_op".to_string(),
                line_number: 582,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::Literal { value: "and".to_string() }, GrammarElement::Literal { value: "or".to_string() }, GrammarElement::Literal { value: "xor".to_string() }, GrammarElement::Literal { value: "nand".to_string() }, GrammarElement::Literal { value: "nor".to_string() }, GrammarElement::Literal { value: "xnor".to_string() }] },
            },
            GrammarRule {
                name: "relation".to_string(),
                line_number: 586,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "shift_expr".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "relational_op".to_string() }, GrammarElement::RuleReference { name: "shift_expr".to_string() }] }) }] },
            },
            GrammarRule {
                name: "relational_op".to_string(),
                line_number: 587,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "EQUALS".to_string() }, GrammarElement::TokenReference { name: "NOT_EQUALS".to_string() }, GrammarElement::TokenReference { name: "LESS_THAN".to_string() }, GrammarElement::TokenReference { name: "LESS_EQUALS".to_string() }, GrammarElement::TokenReference { name: "GREATER_THAN".to_string() }, GrammarElement::TokenReference { name: "GREATER_EQUALS".to_string() }] },
            },
            GrammarRule {
                name: "shift_expr".to_string(),
                line_number: 592,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "adding_expr".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "shift_op".to_string() }, GrammarElement::RuleReference { name: "adding_expr".to_string() }] }) }] },
            },
            GrammarRule {
                name: "shift_op".to_string(),
                line_number: 593,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::Literal { value: "sll".to_string() }, GrammarElement::Literal { value: "srl".to_string() }, GrammarElement::Literal { value: "sla".to_string() }, GrammarElement::Literal { value: "sra".to_string() }, GrammarElement::Literal { value: "rol".to_string() }, GrammarElement::Literal { value: "ror".to_string() }] },
            },
            GrammarRule {
                name: "adding_expr".to_string(),
                line_number: 597,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "multiplying_expr".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "adding_op".to_string() }, GrammarElement::RuleReference { name: "multiplying_expr".to_string() }] }) }] },
            },
            GrammarRule {
                name: "adding_op".to_string(),
                line_number: 598,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "PLUS".to_string() }, GrammarElement::TokenReference { name: "MINUS".to_string() }, GrammarElement::TokenReference { name: "AMPERSAND".to_string() }] },
            },
            GrammarRule {
                name: "multiplying_expr".to_string(),
                line_number: 601,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "unary_expr".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "multiplying_op".to_string() }, GrammarElement::RuleReference { name: "unary_expr".to_string() }] }) }] },
            },
            GrammarRule {
                name: "multiplying_op".to_string(),
                line_number: 602,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "STAR".to_string() }, GrammarElement::TokenReference { name: "SLASH".to_string() }, GrammarElement::Literal { value: "mod".to_string() }, GrammarElement::Literal { value: "rem".to_string() }] },
            },
            GrammarRule {
                name: "unary_expr".to_string(),
                line_number: 605,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "abs".to_string() }, GrammarElement::RuleReference { name: "unary_expr".to_string() }] }, GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "not".to_string() }, GrammarElement::RuleReference { name: "unary_expr".to_string() }] }, GrammarElement::Sequence { elements: vec![GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "PLUS".to_string() }, GrammarElement::TokenReference { name: "MINUS".to_string() }] }) }, GrammarElement::RuleReference { name: "unary_expr".to_string() }] }, GrammarElement::RuleReference { name: "power_expr".to_string() }] },
            },
            GrammarRule {
                name: "power_expr".to_string(),
                line_number: 611,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "primary".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "POWER".to_string() }, GrammarElement::RuleReference { name: "primary".to_string() }] }) }] },
            },
            GrammarRule {
                name: "primary".to_string(),
                line_number: 619,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "NUMBER".to_string() }, GrammarElement::TokenReference { name: "REAL_NUMBER".to_string() }, GrammarElement::TokenReference { name: "BASED_LITERAL".to_string() }, GrammarElement::TokenReference { name: "STRING".to_string() }, GrammarElement::TokenReference { name: "CHAR_LITERAL".to_string() }, GrammarElement::TokenReference { name: "BIT_STRING".to_string() }, GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "TICK".to_string() }, GrammarElement::TokenReference { name: "NAME".to_string() }] }) }] }, GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::TokenReference { name: "LPAREN".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "expression".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "COMMA".to_string() }, GrammarElement::RuleReference { name: "expression".to_string() }] }) }] }) }, GrammarElement::TokenReference { name: "RPAREN".to_string() }] }, GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "LPAREN".to_string() }, GrammarElement::RuleReference { name: "expression".to_string() }, GrammarElement::TokenReference { name: "RPAREN".to_string() }] }, GrammarElement::RuleReference { name: "aggregate".to_string() }, GrammarElement::Literal { value: "null".to_string() }] },
            },
            GrammarRule {
                name: "aggregate".to_string(),
                line_number: 635,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "LPAREN".to_string() }, GrammarElement::RuleReference { name: "element_association".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "COMMA".to_string() }, GrammarElement::RuleReference { name: "element_association".to_string() }] }) }, GrammarElement::TokenReference { name: "RPAREN".to_string() }] },
            },
            GrammarRule {
                name: "element_association".to_string(),
                line_number: 636,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "choices".to_string() }, GrammarElement::TokenReference { name: "ARROW".to_string() }] }) }, GrammarElement::RuleReference { name: "expression".to_string() }] },
            },
        ],
    }
}
