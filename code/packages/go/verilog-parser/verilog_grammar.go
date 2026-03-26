// AUTO-GENERATED FILE - DO NOT EDIT
package verilogparser

import (
	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
)

var VerilogGrammar = &grammartools.ParserGrammar{
	Version: 0,
	Rules: []grammartools.GrammarRule{
		{
			Name: "source_text",
			LineNumber: 42,
			Body: grammartools.Repetition{Element: grammartools.RuleReference{Name: "description", IsToken: false}},
		},
		{
			Name: "description",
			LineNumber: 44,
			Body: grammartools.RuleReference{Name: "module_declaration", IsToken: false},
		},
		{
			Name: "module_declaration",
			LineNumber: 73,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "module"}, grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.Optional{Element: grammartools.RuleReference{Name: "parameter_port_list", IsToken: false}}, grammartools.Optional{Element: grammartools.RuleReference{Name: "port_list", IsToken: false}}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}, grammartools.Repetition{Element: grammartools.RuleReference{Name: "module_item", IsToken: false}}, grammartools.Literal{Value: "endmodule"}}},
		},
		{
			Name: "parameter_port_list",
			LineNumber: 91,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "HASH", IsToken: true}, grammartools.RuleReference{Name: "LPAREN", IsToken: true}, grammartools.RuleReference{Name: "parameter_declaration", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "COMMA", IsToken: true}, grammartools.RuleReference{Name: "parameter_declaration", IsToken: false}}}}, grammartools.RuleReference{Name: "RPAREN", IsToken: true}}},
		},
		{
			Name: "parameter_declaration",
			LineNumber: 94,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "parameter"}, grammartools.Optional{Element: grammartools.RuleReference{Name: "range", IsToken: false}}, grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.RuleReference{Name: "EQUALS", IsToken: true}, grammartools.RuleReference{Name: "expression", IsToken: false}}},
		},
		{
			Name: "localparam_declaration",
			LineNumber: 95,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "localparam"}, grammartools.Optional{Element: grammartools.RuleReference{Name: "range", IsToken: false}}, grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.RuleReference{Name: "EQUALS", IsToken: true}, grammartools.RuleReference{Name: "expression", IsToken: false}}},
		},
		{
			Name: "port_list",
			LineNumber: 115,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "LPAREN", IsToken: true}, grammartools.RuleReference{Name: "port", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "COMMA", IsToken: true}, grammartools.RuleReference{Name: "port", IsToken: false}}}}, grammartools.RuleReference{Name: "RPAREN", IsToken: true}}},
		},
		{
			Name: "port",
			LineNumber: 117,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Optional{Element: grammartools.RuleReference{Name: "port_direction", IsToken: false}}, grammartools.Optional{Element: grammartools.RuleReference{Name: "net_type", IsToken: false}}, grammartools.Optional{Element: grammartools.Literal{Value: "signed"}}, grammartools.Optional{Element: grammartools.RuleReference{Name: "range", IsToken: false}}, grammartools.RuleReference{Name: "NAME", IsToken: true}}},
		},
		{
			Name: "port_direction",
			LineNumber: 119,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.Literal{Value: "input"}, grammartools.Literal{Value: "output"}, grammartools.Literal{Value: "inout"}}},
		},
		{
			Name: "net_type",
			LineNumber: 120,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.Literal{Value: "wire"}, grammartools.Literal{Value: "reg"}, grammartools.Literal{Value: "tri"}, grammartools.Literal{Value: "supply0"}, grammartools.Literal{Value: "supply1"}}},
		},
		{
			Name: "range",
			LineNumber: 122,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "LBRACKET", IsToken: true}, grammartools.RuleReference{Name: "expression", IsToken: false}, grammartools.RuleReference{Name: "COLON", IsToken: true}, grammartools.RuleReference{Name: "expression", IsToken: false}, grammartools.RuleReference{Name: "RBRACKET", IsToken: true}}},
		},
		{
			Name: "module_item",
			LineNumber: 139,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "port_declaration", IsToken: false}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}}}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "net_declaration", IsToken: false}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}}}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "reg_declaration", IsToken: false}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}}}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "integer_declaration", IsToken: false}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}}}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "parameter_declaration", IsToken: false}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}}}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "localparam_declaration", IsToken: false}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}}}, grammartools.RuleReference{Name: "continuous_assign", IsToken: false}, grammartools.RuleReference{Name: "always_construct", IsToken: false}, grammartools.RuleReference{Name: "initial_construct", IsToken: false}, grammartools.RuleReference{Name: "module_instantiation", IsToken: false}, grammartools.RuleReference{Name: "generate_region", IsToken: false}, grammartools.RuleReference{Name: "function_declaration", IsToken: false}, grammartools.RuleReference{Name: "task_declaration", IsToken: false}}},
		},
		{
			Name: "port_declaration",
			LineNumber: 174,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "port_direction", IsToken: false}, grammartools.Optional{Element: grammartools.RuleReference{Name: "net_type", IsToken: false}}, grammartools.Optional{Element: grammartools.Literal{Value: "signed"}}, grammartools.Optional{Element: grammartools.RuleReference{Name: "range", IsToken: false}}, grammartools.RuleReference{Name: "name_list", IsToken: false}}},
		},
		{
			Name: "net_declaration",
			LineNumber: 176,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "net_type", IsToken: false}, grammartools.Optional{Element: grammartools.Literal{Value: "signed"}}, grammartools.Optional{Element: grammartools.RuleReference{Name: "range", IsToken: false}}, grammartools.RuleReference{Name: "name_list", IsToken: false}}},
		},
		{
			Name: "reg_declaration",
			LineNumber: 177,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "reg"}, grammartools.Optional{Element: grammartools.Literal{Value: "signed"}}, grammartools.Optional{Element: grammartools.RuleReference{Name: "range", IsToken: false}}, grammartools.RuleReference{Name: "name_list", IsToken: false}}},
		},
		{
			Name: "integer_declaration",
			LineNumber: 178,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "integer"}, grammartools.RuleReference{Name: "name_list", IsToken: false}}},
		},
		{
			Name: "name_list",
			LineNumber: 179,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "COMMA", IsToken: true}, grammartools.RuleReference{Name: "NAME", IsToken: true}}}}}},
		},
		{
			Name: "continuous_assign",
			LineNumber: 198,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "assign"}, grammartools.RuleReference{Name: "assignment", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "COMMA", IsToken: true}, grammartools.RuleReference{Name: "assignment", IsToken: false}}}}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}}},
		},
		{
			Name: "assignment",
			LineNumber: 199,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "lvalue", IsToken: false}, grammartools.RuleReference{Name: "EQUALS", IsToken: true}, grammartools.RuleReference{Name: "expression", IsToken: false}}},
		},
		{
			Name: "lvalue",
			LineNumber: 203,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.Optional{Element: grammartools.RuleReference{Name: "range_select", IsToken: false}}}}, grammartools.RuleReference{Name: "concatenation", IsToken: false}}},
		},
		{
			Name: "range_select",
			LineNumber: 206,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "LBRACKET", IsToken: true}, grammartools.RuleReference{Name: "expression", IsToken: false}, grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "COLON", IsToken: true}, grammartools.RuleReference{Name: "expression", IsToken: false}}}}, grammartools.RuleReference{Name: "RBRACKET", IsToken: true}}},
		},
		{
			Name: "always_construct",
			LineNumber: 243,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "always"}, grammartools.RuleReference{Name: "AT", IsToken: true}, grammartools.RuleReference{Name: "sensitivity_list", IsToken: false}, grammartools.RuleReference{Name: "statement", IsToken: false}}},
		},
		{
			Name: "initial_construct",
			LineNumber: 244,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "initial"}, grammartools.RuleReference{Name: "statement", IsToken: false}}},
		},
		{
			Name: "sensitivity_list",
			LineNumber: 246,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "LPAREN", IsToken: true}, grammartools.RuleReference{Name: "sensitivity_item", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Group{Element: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.Literal{Value: "or"}, grammartools.RuleReference{Name: "COMMA", IsToken: true}}}}, grammartools.RuleReference{Name: "sensitivity_item", IsToken: false}}}}, grammartools.RuleReference{Name: "RPAREN", IsToken: true}}}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "LPAREN", IsToken: true}, grammartools.RuleReference{Name: "STAR", IsToken: true}, grammartools.RuleReference{Name: "RPAREN", IsToken: true}}}}},
		},
		{
			Name: "sensitivity_item",
			LineNumber: 250,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Optional{Element: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.Literal{Value: "posedge"}, grammartools.Literal{Value: "negedge"}}}}, grammartools.RuleReference{Name: "expression", IsToken: false}}},
		},
		{
			Name: "statement",
			LineNumber: 259,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "block_statement", IsToken: false}, grammartools.RuleReference{Name: "if_statement", IsToken: false}, grammartools.RuleReference{Name: "case_statement", IsToken: false}, grammartools.RuleReference{Name: "for_statement", IsToken: false}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "blocking_assignment", IsToken: false}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}}}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "nonblocking_assignment", IsToken: false}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}}}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "task_call", IsToken: false}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}}}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}}},
		},
		{
			Name: "block_statement",
			LineNumber: 275,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "begin"}, grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "COLON", IsToken: true}, grammartools.RuleReference{Name: "NAME", IsToken: true}}}}, grammartools.Repetition{Element: grammartools.RuleReference{Name: "statement", IsToken: false}}, grammartools.Literal{Value: "end"}}},
		},
		{
			Name: "if_statement",
			LineNumber: 286,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "if"}, grammartools.RuleReference{Name: "LPAREN", IsToken: true}, grammartools.RuleReference{Name: "expression", IsToken: false}, grammartools.RuleReference{Name: "RPAREN", IsToken: true}, grammartools.RuleReference{Name: "statement", IsToken: false}, grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "else"}, grammartools.RuleReference{Name: "statement", IsToken: false}}}}}},
		},
		{
			Name: "case_statement",
			LineNumber: 301,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Group{Element: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.Literal{Value: "case"}, grammartools.Literal{Value: "casex"}, grammartools.Literal{Value: "casez"}}}}, grammartools.RuleReference{Name: "LPAREN", IsToken: true}, grammartools.RuleReference{Name: "expression", IsToken: false}, grammartools.RuleReference{Name: "RPAREN", IsToken: true}, grammartools.Repetition{Element: grammartools.RuleReference{Name: "case_item", IsToken: false}}, grammartools.Literal{Value: "endcase"}}},
		},
		{
			Name: "case_item",
			LineNumber: 306,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "expression_list", IsToken: false}, grammartools.RuleReference{Name: "COLON", IsToken: true}, grammartools.RuleReference{Name: "statement", IsToken: false}}}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "default"}, grammartools.Optional{Element: grammartools.RuleReference{Name: "COLON", IsToken: true}}, grammartools.RuleReference{Name: "statement", IsToken: false}}}}},
		},
		{
			Name: "expression_list",
			LineNumber: 309,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "expression", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "COMMA", IsToken: true}, grammartools.RuleReference{Name: "expression", IsToken: false}}}}}},
		},
		{
			Name: "for_statement",
			LineNumber: 313,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "for"}, grammartools.RuleReference{Name: "LPAREN", IsToken: true}, grammartools.RuleReference{Name: "blocking_assignment", IsToken: false}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}, grammartools.RuleReference{Name: "expression", IsToken: false}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}, grammartools.RuleReference{Name: "blocking_assignment", IsToken: false}, grammartools.RuleReference{Name: "RPAREN", IsToken: true}, grammartools.RuleReference{Name: "statement", IsToken: false}}},
		},
		{
			Name: "blocking_assignment",
			LineNumber: 317,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "lvalue", IsToken: false}, grammartools.RuleReference{Name: "EQUALS", IsToken: true}, grammartools.RuleReference{Name: "expression", IsToken: false}}},
		},
		{
			Name: "nonblocking_assignment",
			LineNumber: 318,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "lvalue", IsToken: false}, grammartools.RuleReference{Name: "LESS_EQUALS", IsToken: true}, grammartools.RuleReference{Name: "expression", IsToken: false}}},
		},
		{
			Name: "task_call",
			LineNumber: 321,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.RuleReference{Name: "LPAREN", IsToken: true}, grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "expression", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "COMMA", IsToken: true}, grammartools.RuleReference{Name: "expression", IsToken: false}}}}}}}, grammartools.RuleReference{Name: "RPAREN", IsToken: true}}},
		},
		{
			Name: "module_instantiation",
			LineNumber: 340,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.Optional{Element: grammartools.RuleReference{Name: "parameter_value_assignment", IsToken: false}}, grammartools.RuleReference{Name: "instance", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "COMMA", IsToken: true}, grammartools.RuleReference{Name: "instance", IsToken: false}}}}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}}},
		},
		{
			Name: "parameter_value_assignment",
			LineNumber: 343,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "HASH", IsToken: true}, grammartools.RuleReference{Name: "LPAREN", IsToken: true}, grammartools.RuleReference{Name: "expression", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "COMMA", IsToken: true}, grammartools.RuleReference{Name: "expression", IsToken: false}}}}, grammartools.RuleReference{Name: "RPAREN", IsToken: true}}},
		},
		{
			Name: "instance",
			LineNumber: 345,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.RuleReference{Name: "LPAREN", IsToken: true}, grammartools.RuleReference{Name: "port_connections", IsToken: false}, grammartools.RuleReference{Name: "RPAREN", IsToken: true}}},
		},
		{
			Name: "port_connections",
			LineNumber: 347,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "named_port_connection", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "COMMA", IsToken: true}, grammartools.RuleReference{Name: "named_port_connection", IsToken: false}}}}}}, grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "expression", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "COMMA", IsToken: true}, grammartools.RuleReference{Name: "expression", IsToken: false}}}}}}}}},
		},
		{
			Name: "named_port_connection",
			LineNumber: 350,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "DOT", IsToken: true}, grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.RuleReference{Name: "LPAREN", IsToken: true}, grammartools.Optional{Element: grammartools.RuleReference{Name: "expression", IsToken: false}}, grammartools.RuleReference{Name: "RPAREN", IsToken: true}}},
		},
		{
			Name: "generate_region",
			LineNumber: 377,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "generate"}, grammartools.Repetition{Element: grammartools.RuleReference{Name: "generate_item", IsToken: false}}, grammartools.Literal{Value: "endgenerate"}}},
		},
		{
			Name: "generate_item",
			LineNumber: 379,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "genvar_declaration", IsToken: false}, grammartools.RuleReference{Name: "generate_for", IsToken: false}, grammartools.RuleReference{Name: "generate_if", IsToken: false}, grammartools.RuleReference{Name: "module_item", IsToken: false}}},
		},
		{
			Name: "genvar_declaration",
			LineNumber: 384,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "genvar"}, grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "COMMA", IsToken: true}, grammartools.RuleReference{Name: "NAME", IsToken: true}}}}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}}},
		},
		{
			Name: "generate_for",
			LineNumber: 386,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "for"}, grammartools.RuleReference{Name: "LPAREN", IsToken: true}, grammartools.RuleReference{Name: "genvar_assignment", IsToken: false}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}, grammartools.RuleReference{Name: "expression", IsToken: false}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}, grammartools.RuleReference{Name: "genvar_assignment", IsToken: false}, grammartools.RuleReference{Name: "RPAREN", IsToken: true}, grammartools.RuleReference{Name: "generate_block", IsToken: false}}},
		},
		{
			Name: "generate_if",
			LineNumber: 390,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "if"}, grammartools.RuleReference{Name: "LPAREN", IsToken: true}, grammartools.RuleReference{Name: "expression", IsToken: false}, grammartools.RuleReference{Name: "RPAREN", IsToken: true}, grammartools.RuleReference{Name: "generate_block", IsToken: false}, grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "else"}, grammartools.RuleReference{Name: "generate_block", IsToken: false}}}}}},
		},
		{
			Name: "generate_block",
			LineNumber: 393,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "begin"}, grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "COLON", IsToken: true}, grammartools.RuleReference{Name: "NAME", IsToken: true}}}}, grammartools.Repetition{Element: grammartools.RuleReference{Name: "generate_item", IsToken: false}}, grammartools.Literal{Value: "end"}}}, grammartools.RuleReference{Name: "generate_item", IsToken: false}}},
		},
		{
			Name: "genvar_assignment",
			LineNumber: 396,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.RuleReference{Name: "EQUALS", IsToken: true}, grammartools.RuleReference{Name: "expression", IsToken: false}}},
		},
		{
			Name: "function_declaration",
			LineNumber: 415,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "function"}, grammartools.Optional{Element: grammartools.RuleReference{Name: "range", IsToken: false}}, grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}, grammartools.Repetition{Element: grammartools.RuleReference{Name: "function_item", IsToken: false}}, grammartools.RuleReference{Name: "statement", IsToken: false}, grammartools.Literal{Value: "endfunction"}}},
		},
		{
			Name: "function_item",
			LineNumber: 420,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "port_declaration", IsToken: false}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}}}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "reg_declaration", IsToken: false}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}}}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "integer_declaration", IsToken: false}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}}}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "parameter_declaration", IsToken: false}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}}}}},
		},
		{
			Name: "task_declaration",
			LineNumber: 425,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "task"}, grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}, grammartools.Repetition{Element: grammartools.RuleReference{Name: "task_item", IsToken: false}}, grammartools.RuleReference{Name: "statement", IsToken: false}, grammartools.Literal{Value: "endtask"}}},
		},
		{
			Name: "task_item",
			LineNumber: 430,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "port_declaration", IsToken: false}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}}}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "reg_declaration", IsToken: false}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}}}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "integer_declaration", IsToken: false}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}}}}},
		},
		{
			Name: "expression",
			LineNumber: 458,
			Body: grammartools.RuleReference{Name: "ternary_expr", IsToken: false},
		},
		{
			Name: "ternary_expr",
			LineNumber: 464,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "or_expr", IsToken: false}, grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "QUESTION", IsToken: true}, grammartools.RuleReference{Name: "expression", IsToken: false}, grammartools.RuleReference{Name: "COLON", IsToken: true}, grammartools.RuleReference{Name: "ternary_expr", IsToken: false}}}}}},
		},
		{
			Name: "or_expr",
			LineNumber: 467,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "and_expr", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "LOGIC_OR", IsToken: true}, grammartools.RuleReference{Name: "and_expr", IsToken: false}}}}}},
		},
		{
			Name: "and_expr",
			LineNumber: 468,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "bit_or_expr", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "LOGIC_AND", IsToken: true}, grammartools.RuleReference{Name: "bit_or_expr", IsToken: false}}}}}},
		},
		{
			Name: "bit_or_expr",
			LineNumber: 471,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "bit_xor_expr", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "PIPE", IsToken: true}, grammartools.RuleReference{Name: "bit_xor_expr", IsToken: false}}}}}},
		},
		{
			Name: "bit_xor_expr",
			LineNumber: 472,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "bit_and_expr", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "CARET", IsToken: true}, grammartools.RuleReference{Name: "bit_and_expr", IsToken: false}}}}}},
		},
		{
			Name: "bit_and_expr",
			LineNumber: 473,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "equality_expr", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "AMP", IsToken: true}, grammartools.RuleReference{Name: "equality_expr", IsToken: false}}}}}},
		},
		{
			Name: "equality_expr",
			LineNumber: 477,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "relational_expr", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Group{Element: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "EQUALS_EQUALS", IsToken: true}, grammartools.RuleReference{Name: "NOT_EQUALS", IsToken: true}, grammartools.RuleReference{Name: "CASE_EQ", IsToken: true}, grammartools.RuleReference{Name: "CASE_NEQ", IsToken: true}}}}, grammartools.RuleReference{Name: "relational_expr", IsToken: false}}}}}},
		},
		{
			Name: "relational_expr",
			LineNumber: 484,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "shift_expr", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Group{Element: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "LESS_THAN", IsToken: true}, grammartools.RuleReference{Name: "LESS_EQUALS", IsToken: true}, grammartools.RuleReference{Name: "GREATER_THAN", IsToken: true}, grammartools.RuleReference{Name: "GREATER_EQUALS", IsToken: true}}}}, grammartools.RuleReference{Name: "shift_expr", IsToken: false}}}}}},
		},
		{
			Name: "shift_expr",
			LineNumber: 489,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "additive_expr", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Group{Element: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "LEFT_SHIFT", IsToken: true}, grammartools.RuleReference{Name: "RIGHT_SHIFT", IsToken: true}, grammartools.RuleReference{Name: "ARITH_LEFT_SHIFT", IsToken: true}, grammartools.RuleReference{Name: "ARITH_RIGHT_SHIFT", IsToken: true}}}}, grammartools.RuleReference{Name: "additive_expr", IsToken: false}}}}}},
		},
		{
			Name: "additive_expr",
			LineNumber: 494,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "multiplicative_expr", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Group{Element: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "PLUS", IsToken: true}, grammartools.RuleReference{Name: "MINUS", IsToken: true}}}}, grammartools.RuleReference{Name: "multiplicative_expr", IsToken: false}}}}}},
		},
		{
			Name: "multiplicative_expr",
			LineNumber: 495,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "power_expr", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Group{Element: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "STAR", IsToken: true}, grammartools.RuleReference{Name: "SLASH", IsToken: true}, grammartools.RuleReference{Name: "PERCENT", IsToken: true}}}}, grammartools.RuleReference{Name: "power_expr", IsToken: false}}}}}},
		},
		{
			Name: "power_expr",
			LineNumber: 496,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "unary_expr", IsToken: false}, grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "POWER", IsToken: true}, grammartools.RuleReference{Name: "unary_expr", IsToken: false}}}}}},
		},
		{
			Name: "unary_expr",
			LineNumber: 508,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Group{Element: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "PLUS", IsToken: true}, grammartools.RuleReference{Name: "MINUS", IsToken: true}, grammartools.RuleReference{Name: "BANG", IsToken: true}, grammartools.RuleReference{Name: "TILDE", IsToken: true}, grammartools.RuleReference{Name: "AMP", IsToken: true}, grammartools.RuleReference{Name: "PIPE", IsToken: true}, grammartools.RuleReference{Name: "CARET", IsToken: true}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "TILDE", IsToken: true}, grammartools.RuleReference{Name: "AMP", IsToken: true}}}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "TILDE", IsToken: true}, grammartools.RuleReference{Name: "PIPE", IsToken: true}}}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "TILDE", IsToken: true}, grammartools.RuleReference{Name: "CARET", IsToken: true}}}}}}, grammartools.RuleReference{Name: "unary_expr", IsToken: false}}}, grammartools.RuleReference{Name: "primary", IsToken: false}}},
		},
		{
			Name: "primary",
			LineNumber: 518,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "NUMBER", IsToken: true}, grammartools.RuleReference{Name: "SIZED_NUMBER", IsToken: true}, grammartools.RuleReference{Name: "REAL_NUMBER", IsToken: true}, grammartools.RuleReference{Name: "STRING", IsToken: true}, grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.RuleReference{Name: "SYSTEM_ID", IsToken: true}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "LPAREN", IsToken: true}, grammartools.RuleReference{Name: "expression", IsToken: false}, grammartools.RuleReference{Name: "RPAREN", IsToken: true}}}, grammartools.RuleReference{Name: "concatenation", IsToken: false}, grammartools.RuleReference{Name: "replication", IsToken: false}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "primary", IsToken: false}, grammartools.RuleReference{Name: "LBRACKET", IsToken: true}, grammartools.RuleReference{Name: "expression", IsToken: false}, grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "COLON", IsToken: true}, grammartools.RuleReference{Name: "expression", IsToken: false}}}}, grammartools.RuleReference{Name: "RBRACKET", IsToken: true}}}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.RuleReference{Name: "LPAREN", IsToken: true}, grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "expression", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "COMMA", IsToken: true}, grammartools.RuleReference{Name: "expression", IsToken: false}}}}}}}, grammartools.RuleReference{Name: "RPAREN", IsToken: true}}}}},
		},
		{
			Name: "concatenation",
			LineNumber: 534,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "LBRACE", IsToken: true}, grammartools.RuleReference{Name: "expression", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "COMMA", IsToken: true}, grammartools.RuleReference{Name: "expression", IsToken: false}}}}, grammartools.RuleReference{Name: "RBRACE", IsToken: true}}},
		},
		{
			Name: "replication",
			LineNumber: 540,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "LBRACE", IsToken: true}, grammartools.RuleReference{Name: "expression", IsToken: false}, grammartools.RuleReference{Name: "concatenation", IsToken: false}, grammartools.RuleReference{Name: "RBRACE", IsToken: true}}},
		},
	},
}
