// AUTO-GENERATED FILE - DO NOT EDIT
#![allow(clippy::all)]
use grammar_tools::parser_grammar::{ParserGrammar, GrammarRule, GrammarElement};

pub fn VerilogGrammar() -> ParserGrammar {
    ParserGrammar {
        version: 0,
        rules: vec![
            GrammarRule {
                name: "source_text".to_string(),
                line_number: 42,
                body: GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: "description".to_string() }) },
            },
            GrammarRule {
                name: "description".to_string(),
                line_number: 44,
                body: GrammarElement::RuleReference { name: "module_declaration".to_string() },
            },
            GrammarRule {
                name: "module_declaration".to_string(),
                line_number: 73,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "module".to_string() }, GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: "parameter_port_list".to_string() }) }, GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: "port_list".to_string() }) }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: "module_item".to_string() }) }, GrammarElement::Literal { value: "endmodule".to_string() }] },
            },
            GrammarRule {
                name: "parameter_port_list".to_string(),
                line_number: 91,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "HASH".to_string() }, GrammarElement::TokenReference { name: "LPAREN".to_string() }, GrammarElement::RuleReference { name: "parameter_declaration".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "COMMA".to_string() }, GrammarElement::RuleReference { name: "parameter_declaration".to_string() }] }) }, GrammarElement::TokenReference { name: "RPAREN".to_string() }] },
            },
            GrammarRule {
                name: "parameter_declaration".to_string(),
                line_number: 94,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "parameter".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: "range".to_string() }) }, GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::TokenReference { name: "EQUALS".to_string() }, GrammarElement::RuleReference { name: "expression".to_string() }] },
            },
            GrammarRule {
                name: "localparam_declaration".to_string(),
                line_number: 95,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "localparam".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: "range".to_string() }) }, GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::TokenReference { name: "EQUALS".to_string() }, GrammarElement::RuleReference { name: "expression".to_string() }] },
            },
            GrammarRule {
                name: "port_list".to_string(),
                line_number: 115,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "LPAREN".to_string() }, GrammarElement::RuleReference { name: "port".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "COMMA".to_string() }, GrammarElement::RuleReference { name: "port".to_string() }] }) }, GrammarElement::TokenReference { name: "RPAREN".to_string() }] },
            },
            GrammarRule {
                name: "port".to_string(),
                line_number: 117,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: "port_direction".to_string() }) }, GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: "net_type".to_string() }) }, GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: "signed".to_string() }) }, GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: "range".to_string() }) }, GrammarElement::TokenReference { name: "NAME".to_string() }] },
            },
            GrammarRule {
                name: "port_direction".to_string(),
                line_number: 119,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::Literal { value: "input".to_string() }, GrammarElement::Literal { value: "output".to_string() }, GrammarElement::Literal { value: "inout".to_string() }] },
            },
            GrammarRule {
                name: "net_type".to_string(),
                line_number: 120,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::Literal { value: "wire".to_string() }, GrammarElement::Literal { value: "reg".to_string() }, GrammarElement::Literal { value: "tri".to_string() }, GrammarElement::Literal { value: "supply0".to_string() }, GrammarElement::Literal { value: "supply1".to_string() }] },
            },
            GrammarRule {
                name: "range".to_string(),
                line_number: 122,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "LBRACKET".to_string() }, GrammarElement::RuleReference { name: "expression".to_string() }, GrammarElement::TokenReference { name: "COLON".to_string() }, GrammarElement::RuleReference { name: "expression".to_string() }, GrammarElement::TokenReference { name: "RBRACKET".to_string() }] },
            },
            GrammarRule {
                name: "module_item".to_string(),
                line_number: 139,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "port_declaration".to_string() }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }] }, GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "net_declaration".to_string() }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }] }, GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "reg_declaration".to_string() }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }] }, GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "integer_declaration".to_string() }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }] }, GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "parameter_declaration".to_string() }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }] }, GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "localparam_declaration".to_string() }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }] }, GrammarElement::RuleReference { name: "continuous_assign".to_string() }, GrammarElement::RuleReference { name: "always_construct".to_string() }, GrammarElement::RuleReference { name: "initial_construct".to_string() }, GrammarElement::RuleReference { name: "module_instantiation".to_string() }, GrammarElement::RuleReference { name: "generate_region".to_string() }, GrammarElement::RuleReference { name: "function_declaration".to_string() }, GrammarElement::RuleReference { name: "task_declaration".to_string() }] },
            },
            GrammarRule {
                name: "port_declaration".to_string(),
                line_number: 174,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "port_direction".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: "net_type".to_string() }) }, GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: "signed".to_string() }) }, GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: "range".to_string() }) }, GrammarElement::RuleReference { name: "name_list".to_string() }] },
            },
            GrammarRule {
                name: "net_declaration".to_string(),
                line_number: 176,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "net_type".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: "signed".to_string() }) }, GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: "range".to_string() }) }, GrammarElement::RuleReference { name: "name_list".to_string() }] },
            },
            GrammarRule {
                name: "reg_declaration".to_string(),
                line_number: 177,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "reg".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: "signed".to_string() }) }, GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: "range".to_string() }) }, GrammarElement::RuleReference { name: "name_list".to_string() }] },
            },
            GrammarRule {
                name: "integer_declaration".to_string(),
                line_number: 178,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "integer".to_string() }, GrammarElement::RuleReference { name: "name_list".to_string() }] },
            },
            GrammarRule {
                name: "name_list".to_string(),
                line_number: 179,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "COMMA".to_string() }, GrammarElement::TokenReference { name: "NAME".to_string() }] }) }] },
            },
            GrammarRule {
                name: "continuous_assign".to_string(),
                line_number: 198,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "assign".to_string() }, GrammarElement::RuleReference { name: "assignment".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "COMMA".to_string() }, GrammarElement::RuleReference { name: "assignment".to_string() }] }) }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }] },
            },
            GrammarRule {
                name: "assignment".to_string(),
                line_number: 199,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "lvalue".to_string() }, GrammarElement::TokenReference { name: "EQUALS".to_string() }, GrammarElement::RuleReference { name: "expression".to_string() }] },
            },
            GrammarRule {
                name: "lvalue".to_string(),
                line_number: 203,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: "range_select".to_string() }) }] }, GrammarElement::RuleReference { name: "concatenation".to_string() }] },
            },
            GrammarRule {
                name: "range_select".to_string(),
                line_number: 206,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "LBRACKET".to_string() }, GrammarElement::RuleReference { name: "expression".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "COLON".to_string() }, GrammarElement::RuleReference { name: "expression".to_string() }] }) }, GrammarElement::TokenReference { name: "RBRACKET".to_string() }] },
            },
            GrammarRule {
                name: "always_construct".to_string(),
                line_number: 243,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "always".to_string() }, GrammarElement::TokenReference { name: "AT".to_string() }, GrammarElement::RuleReference { name: "sensitivity_list".to_string() }, GrammarElement::RuleReference { name: "statement".to_string() }] },
            },
            GrammarRule {
                name: "initial_construct".to_string(),
                line_number: 244,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "initial".to_string() }, GrammarElement::RuleReference { name: "statement".to_string() }] },
            },
            GrammarRule {
                name: "sensitivity_list".to_string(),
                line_number: 246,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "LPAREN".to_string() }, GrammarElement::RuleReference { name: "sensitivity_item".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![GrammarElement::Literal { value: "or".to_string() }, GrammarElement::TokenReference { name: "COMMA".to_string() }] }) }, GrammarElement::RuleReference { name: "sensitivity_item".to_string() }] }) }, GrammarElement::TokenReference { name: "RPAREN".to_string() }] }, GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "LPAREN".to_string() }, GrammarElement::TokenReference { name: "STAR".to_string() }, GrammarElement::TokenReference { name: "RPAREN".to_string() }] }] },
            },
            GrammarRule {
                name: "sensitivity_item".to_string(),
                line_number: 250,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Optional { element: Box::new(GrammarElement::Alternation { choices: vec![GrammarElement::Literal { value: "posedge".to_string() }, GrammarElement::Literal { value: "negedge".to_string() }] }) }, GrammarElement::RuleReference { name: "expression".to_string() }] },
            },
            GrammarRule {
                name: "statement".to_string(),
                line_number: 259,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::RuleReference { name: "block_statement".to_string() }, GrammarElement::RuleReference { name: "if_statement".to_string() }, GrammarElement::RuleReference { name: "case_statement".to_string() }, GrammarElement::RuleReference { name: "for_statement".to_string() }, GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "blocking_assignment".to_string() }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }] }, GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "nonblocking_assignment".to_string() }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }] }, GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "task_call".to_string() }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }] }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }] },
            },
            GrammarRule {
                name: "block_statement".to_string(),
                line_number: 275,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "begin".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "COLON".to_string() }, GrammarElement::TokenReference { name: "NAME".to_string() }] }) }, GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: "statement".to_string() }) }, GrammarElement::Literal { value: "end".to_string() }] },
            },
            GrammarRule {
                name: "if_statement".to_string(),
                line_number: 286,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "if".to_string() }, GrammarElement::TokenReference { name: "LPAREN".to_string() }, GrammarElement::RuleReference { name: "expression".to_string() }, GrammarElement::TokenReference { name: "RPAREN".to_string() }, GrammarElement::RuleReference { name: "statement".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "else".to_string() }, GrammarElement::RuleReference { name: "statement".to_string() }] }) }] },
            },
            GrammarRule {
                name: "case_statement".to_string(),
                line_number: 301,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![GrammarElement::Literal { value: "case".to_string() }, GrammarElement::Literal { value: "casex".to_string() }, GrammarElement::Literal { value: "casez".to_string() }] }) }, GrammarElement::TokenReference { name: "LPAREN".to_string() }, GrammarElement::RuleReference { name: "expression".to_string() }, GrammarElement::TokenReference { name: "RPAREN".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: "case_item".to_string() }) }, GrammarElement::Literal { value: "endcase".to_string() }] },
            },
            GrammarRule {
                name: "case_item".to_string(),
                line_number: 306,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "expression_list".to_string() }, GrammarElement::TokenReference { name: "COLON".to_string() }, GrammarElement::RuleReference { name: "statement".to_string() }] }, GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "default".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: "COLON".to_string() }) }, GrammarElement::RuleReference { name: "statement".to_string() }] }] },
            },
            GrammarRule {
                name: "expression_list".to_string(),
                line_number: 309,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "expression".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "COMMA".to_string() }, GrammarElement::RuleReference { name: "expression".to_string() }] }) }] },
            },
            GrammarRule {
                name: "for_statement".to_string(),
                line_number: 313,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "for".to_string() }, GrammarElement::TokenReference { name: "LPAREN".to_string() }, GrammarElement::RuleReference { name: "blocking_assignment".to_string() }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }, GrammarElement::RuleReference { name: "expression".to_string() }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }, GrammarElement::RuleReference { name: "blocking_assignment".to_string() }, GrammarElement::TokenReference { name: "RPAREN".to_string() }, GrammarElement::RuleReference { name: "statement".to_string() }] },
            },
            GrammarRule {
                name: "blocking_assignment".to_string(),
                line_number: 317,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "lvalue".to_string() }, GrammarElement::TokenReference { name: "EQUALS".to_string() }, GrammarElement::RuleReference { name: "expression".to_string() }] },
            },
            GrammarRule {
                name: "nonblocking_assignment".to_string(),
                line_number: 318,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "lvalue".to_string() }, GrammarElement::TokenReference { name: "LESS_EQUALS".to_string() }, GrammarElement::RuleReference { name: "expression".to_string() }] },
            },
            GrammarRule {
                name: "task_call".to_string(),
                line_number: 321,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::TokenReference { name: "LPAREN".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "expression".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "COMMA".to_string() }, GrammarElement::RuleReference { name: "expression".to_string() }] }) }] }) }, GrammarElement::TokenReference { name: "RPAREN".to_string() }] },
            },
            GrammarRule {
                name: "module_instantiation".to_string(),
                line_number: 340,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: "parameter_value_assignment".to_string() }) }, GrammarElement::RuleReference { name: "instance".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "COMMA".to_string() }, GrammarElement::RuleReference { name: "instance".to_string() }] }) }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }] },
            },
            GrammarRule {
                name: "parameter_value_assignment".to_string(),
                line_number: 343,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "HASH".to_string() }, GrammarElement::TokenReference { name: "LPAREN".to_string() }, GrammarElement::RuleReference { name: "expression".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "COMMA".to_string() }, GrammarElement::RuleReference { name: "expression".to_string() }] }) }, GrammarElement::TokenReference { name: "RPAREN".to_string() }] },
            },
            GrammarRule {
                name: "instance".to_string(),
                line_number: 345,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::TokenReference { name: "LPAREN".to_string() }, GrammarElement::RuleReference { name: "port_connections".to_string() }, GrammarElement::TokenReference { name: "RPAREN".to_string() }] },
            },
            GrammarRule {
                name: "port_connections".to_string(),
                line_number: 347,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "named_port_connection".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "COMMA".to_string() }, GrammarElement::RuleReference { name: "named_port_connection".to_string() }] }) }] }, GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "expression".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "COMMA".to_string() }, GrammarElement::RuleReference { name: "expression".to_string() }] }) }] }) }] },
            },
            GrammarRule {
                name: "named_port_connection".to_string(),
                line_number: 350,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "DOT".to_string() }, GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::TokenReference { name: "LPAREN".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: "expression".to_string() }) }, GrammarElement::TokenReference { name: "RPAREN".to_string() }] },
            },
            GrammarRule {
                name: "generate_region".to_string(),
                line_number: 377,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "generate".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: "generate_item".to_string() }) }, GrammarElement::Literal { value: "endgenerate".to_string() }] },
            },
            GrammarRule {
                name: "generate_item".to_string(),
                line_number: 379,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::RuleReference { name: "genvar_declaration".to_string() }, GrammarElement::RuleReference { name: "generate_for".to_string() }, GrammarElement::RuleReference { name: "generate_if".to_string() }, GrammarElement::RuleReference { name: "module_item".to_string() }] },
            },
            GrammarRule {
                name: "genvar_declaration".to_string(),
                line_number: 384,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "genvar".to_string() }, GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "COMMA".to_string() }, GrammarElement::TokenReference { name: "NAME".to_string() }] }) }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }] },
            },
            GrammarRule {
                name: "generate_for".to_string(),
                line_number: 386,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "for".to_string() }, GrammarElement::TokenReference { name: "LPAREN".to_string() }, GrammarElement::RuleReference { name: "genvar_assignment".to_string() }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }, GrammarElement::RuleReference { name: "expression".to_string() }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }, GrammarElement::RuleReference { name: "genvar_assignment".to_string() }, GrammarElement::TokenReference { name: "RPAREN".to_string() }, GrammarElement::RuleReference { name: "generate_block".to_string() }] },
            },
            GrammarRule {
                name: "generate_if".to_string(),
                line_number: 390,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "if".to_string() }, GrammarElement::TokenReference { name: "LPAREN".to_string() }, GrammarElement::RuleReference { name: "expression".to_string() }, GrammarElement::TokenReference { name: "RPAREN".to_string() }, GrammarElement::RuleReference { name: "generate_block".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "else".to_string() }, GrammarElement::RuleReference { name: "generate_block".to_string() }] }) }] },
            },
            GrammarRule {
                name: "generate_block".to_string(),
                line_number: 393,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "begin".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "COLON".to_string() }, GrammarElement::TokenReference { name: "NAME".to_string() }] }) }, GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: "generate_item".to_string() }) }, GrammarElement::Literal { value: "end".to_string() }] }, GrammarElement::RuleReference { name: "generate_item".to_string() }] },
            },
            GrammarRule {
                name: "genvar_assignment".to_string(),
                line_number: 396,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::TokenReference { name: "EQUALS".to_string() }, GrammarElement::RuleReference { name: "expression".to_string() }] },
            },
            GrammarRule {
                name: "function_declaration".to_string(),
                line_number: 415,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "function".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: "range".to_string() }) }, GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: "function_item".to_string() }) }, GrammarElement::RuleReference { name: "statement".to_string() }, GrammarElement::Literal { value: "endfunction".to_string() }] },
            },
            GrammarRule {
                name: "function_item".to_string(),
                line_number: 420,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "port_declaration".to_string() }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }] }, GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "reg_declaration".to_string() }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }] }, GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "integer_declaration".to_string() }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }] }, GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "parameter_declaration".to_string() }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }] }] },
            },
            GrammarRule {
                name: "task_declaration".to_string(),
                line_number: 425,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "task".to_string() }, GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: "task_item".to_string() }) }, GrammarElement::RuleReference { name: "statement".to_string() }, GrammarElement::Literal { value: "endtask".to_string() }] },
            },
            GrammarRule {
                name: "task_item".to_string(),
                line_number: 430,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "port_declaration".to_string() }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }] }, GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "reg_declaration".to_string() }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }] }, GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "integer_declaration".to_string() }, GrammarElement::TokenReference { name: "SEMICOLON".to_string() }] }] },
            },
            GrammarRule {
                name: "expression".to_string(),
                line_number: 458,
                body: GrammarElement::RuleReference { name: "ternary_expr".to_string() },
            },
            GrammarRule {
                name: "ternary_expr".to_string(),
                line_number: 464,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "or_expr".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "QUESTION".to_string() }, GrammarElement::RuleReference { name: "expression".to_string() }, GrammarElement::TokenReference { name: "COLON".to_string() }, GrammarElement::RuleReference { name: "ternary_expr".to_string() }] }) }] },
            },
            GrammarRule {
                name: "or_expr".to_string(),
                line_number: 467,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "and_expr".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "LOGIC_OR".to_string() }, GrammarElement::RuleReference { name: "and_expr".to_string() }] }) }] },
            },
            GrammarRule {
                name: "and_expr".to_string(),
                line_number: 468,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "bit_or_expr".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "LOGIC_AND".to_string() }, GrammarElement::RuleReference { name: "bit_or_expr".to_string() }] }) }] },
            },
            GrammarRule {
                name: "bit_or_expr".to_string(),
                line_number: 471,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "bit_xor_expr".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "PIPE".to_string() }, GrammarElement::RuleReference { name: "bit_xor_expr".to_string() }] }) }] },
            },
            GrammarRule {
                name: "bit_xor_expr".to_string(),
                line_number: 472,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "bit_and_expr".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "CARET".to_string() }, GrammarElement::RuleReference { name: "bit_and_expr".to_string() }] }) }] },
            },
            GrammarRule {
                name: "bit_and_expr".to_string(),
                line_number: 473,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "equality_expr".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "AMP".to_string() }, GrammarElement::RuleReference { name: "equality_expr".to_string() }] }) }] },
            },
            GrammarRule {
                name: "equality_expr".to_string(),
                line_number: 477,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "relational_expr".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "EQUALS_EQUALS".to_string() }, GrammarElement::TokenReference { name: "NOT_EQUALS".to_string() }, GrammarElement::TokenReference { name: "CASE_EQ".to_string() }, GrammarElement::TokenReference { name: "CASE_NEQ".to_string() }] }) }, GrammarElement::RuleReference { name: "relational_expr".to_string() }] }) }] },
            },
            GrammarRule {
                name: "relational_expr".to_string(),
                line_number: 484,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "shift_expr".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "LESS_THAN".to_string() }, GrammarElement::TokenReference { name: "LESS_EQUALS".to_string() }, GrammarElement::TokenReference { name: "GREATER_THAN".to_string() }, GrammarElement::TokenReference { name: "GREATER_EQUALS".to_string() }] }) }, GrammarElement::RuleReference { name: "shift_expr".to_string() }] }) }] },
            },
            GrammarRule {
                name: "shift_expr".to_string(),
                line_number: 489,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "additive_expr".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "LEFT_SHIFT".to_string() }, GrammarElement::TokenReference { name: "RIGHT_SHIFT".to_string() }, GrammarElement::TokenReference { name: "ARITH_LEFT_SHIFT".to_string() }, GrammarElement::TokenReference { name: "ARITH_RIGHT_SHIFT".to_string() }] }) }, GrammarElement::RuleReference { name: "additive_expr".to_string() }] }) }] },
            },
            GrammarRule {
                name: "additive_expr".to_string(),
                line_number: 494,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "multiplicative_expr".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "PLUS".to_string() }, GrammarElement::TokenReference { name: "MINUS".to_string() }] }) }, GrammarElement::RuleReference { name: "multiplicative_expr".to_string() }] }) }] },
            },
            GrammarRule {
                name: "multiplicative_expr".to_string(),
                line_number: 495,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "power_expr".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "STAR".to_string() }, GrammarElement::TokenReference { name: "SLASH".to_string() }, GrammarElement::TokenReference { name: "PERCENT".to_string() }] }) }, GrammarElement::RuleReference { name: "power_expr".to_string() }] }) }] },
            },
            GrammarRule {
                name: "power_expr".to_string(),
                line_number: 496,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "unary_expr".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "POWER".to_string() }, GrammarElement::RuleReference { name: "unary_expr".to_string() }] }) }] },
            },
            GrammarRule {
                name: "unary_expr".to_string(),
                line_number: 508,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::Sequence { elements: vec![GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "PLUS".to_string() }, GrammarElement::TokenReference { name: "MINUS".to_string() }, GrammarElement::TokenReference { name: "BANG".to_string() }, GrammarElement::TokenReference { name: "TILDE".to_string() }, GrammarElement::TokenReference { name: "AMP".to_string() }, GrammarElement::TokenReference { name: "PIPE".to_string() }, GrammarElement::TokenReference { name: "CARET".to_string() }, GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "TILDE".to_string() }, GrammarElement::TokenReference { name: "AMP".to_string() }] }, GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "TILDE".to_string() }, GrammarElement::TokenReference { name: "PIPE".to_string() }] }, GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "TILDE".to_string() }, GrammarElement::TokenReference { name: "CARET".to_string() }] }] }) }, GrammarElement::RuleReference { name: "unary_expr".to_string() }] }, GrammarElement::RuleReference { name: "primary".to_string() }] },
            },
            GrammarRule {
                name: "primary".to_string(),
                line_number: 518,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "NUMBER".to_string() }, GrammarElement::TokenReference { name: "SIZED_NUMBER".to_string() }, GrammarElement::TokenReference { name: "REAL_NUMBER".to_string() }, GrammarElement::TokenReference { name: "STRING".to_string() }, GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::TokenReference { name: "SYSTEM_ID".to_string() }, GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "LPAREN".to_string() }, GrammarElement::RuleReference { name: "expression".to_string() }, GrammarElement::TokenReference { name: "RPAREN".to_string() }] }, GrammarElement::RuleReference { name: "concatenation".to_string() }, GrammarElement::RuleReference { name: "replication".to_string() }, GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "primary".to_string() }, GrammarElement::TokenReference { name: "LBRACKET".to_string() }, GrammarElement::RuleReference { name: "expression".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "COLON".to_string() }, GrammarElement::RuleReference { name: "expression".to_string() }] }) }, GrammarElement::TokenReference { name: "RBRACKET".to_string() }] }, GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::TokenReference { name: "LPAREN".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "expression".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "COMMA".to_string() }, GrammarElement::RuleReference { name: "expression".to_string() }] }) }] }) }, GrammarElement::TokenReference { name: "RPAREN".to_string() }] }] },
            },
            GrammarRule {
                name: "concatenation".to_string(),
                line_number: 534,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "LBRACE".to_string() }, GrammarElement::RuleReference { name: "expression".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "COMMA".to_string() }, GrammarElement::RuleReference { name: "expression".to_string() }] }) }, GrammarElement::TokenReference { name: "RBRACE".to_string() }] },
            },
            GrammarRule {
                name: "replication".to_string(),
                line_number: 540,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "LBRACE".to_string() }, GrammarElement::RuleReference { name: "expression".to_string() }, GrammarElement::RuleReference { name: "concatenation".to_string() }, GrammarElement::TokenReference { name: "RBRACE".to_string() }] },
            },
        ],
    }
}
