// AUTO-GENERATED FILE - DO NOT EDIT
package vhdlparser

import (
	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
)

var VhdlGrammar = &grammartools.ParserGrammar{
	Version: 0,
	Rules: []grammartools.GrammarRule{
		{
			Name: "design_file",
			LineNumber: 64,
			Body: grammartools.Repetition{Element: grammartools.RuleReference{Name: "design_unit", IsToken: false}},
		},
		{
			Name: "design_unit",
			LineNumber: 66,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Repetition{Element: grammartools.RuleReference{Name: "context_item", IsToken: false}}, grammartools.RuleReference{Name: "library_unit", IsToken: false}}},
		},
		{
			Name: "context_item",
			LineNumber: 68,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "library_clause", IsToken: false}, grammartools.RuleReference{Name: "use_clause", IsToken: false}}},
		},
		{
			Name: "library_clause",
			LineNumber: 71,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "library"}, grammartools.RuleReference{Name: "name_list", IsToken: false}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}}},
		},
		{
			Name: "use_clause",
			LineNumber: 74,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "use"}, grammartools.RuleReference{Name: "selected_name", IsToken: false}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}}},
		},
		{
			Name: "selected_name",
			LineNumber: 77,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "DOT", IsToken: true}, grammartools.Group{Element: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.Literal{Value: "all"}}}}}}}}},
		},
		{
			Name: "name_list",
			LineNumber: 79,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "COMMA", IsToken: true}, grammartools.RuleReference{Name: "NAME", IsToken: true}}}}}},
		},
		{
			Name: "library_unit",
			LineNumber: 81,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "entity_declaration", IsToken: false}, grammartools.RuleReference{Name: "architecture_body", IsToken: false}, grammartools.RuleReference{Name: "package_declaration", IsToken: false}, grammartools.RuleReference{Name: "package_body", IsToken: false}}},
		},
		{
			Name: "entity_declaration",
			LineNumber: 112,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "entity"}, grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.Literal{Value: "is"}, grammartools.Optional{Element: grammartools.RuleReference{Name: "generic_clause", IsToken: false}}, grammartools.Optional{Element: grammartools.RuleReference{Name: "port_clause", IsToken: false}}, grammartools.Literal{Value: "end"}, grammartools.Optional{Element: grammartools.Literal{Value: "entity"}}, grammartools.Optional{Element: grammartools.RuleReference{Name: "NAME", IsToken: true}}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}}},
		},
		{
			Name: "generic_clause",
			LineNumber: 117,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "generic"}, grammartools.RuleReference{Name: "LPAREN", IsToken: true}, grammartools.RuleReference{Name: "interface_list", IsToken: false}, grammartools.RuleReference{Name: "RPAREN", IsToken: true}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}}},
		},
		{
			Name: "port_clause",
			LineNumber: 118,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "port"}, grammartools.RuleReference{Name: "LPAREN", IsToken: true}, grammartools.RuleReference{Name: "interface_list", IsToken: false}, grammartools.RuleReference{Name: "RPAREN", IsToken: true}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}}},
		},
		{
			Name: "interface_list",
			LineNumber: 123,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "interface_element", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}, grammartools.RuleReference{Name: "interface_element", IsToken: false}}}}}},
		},
		{
			Name: "interface_element",
			LineNumber: 124,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "name_list", IsToken: false}, grammartools.RuleReference{Name: "COLON", IsToken: true}, grammartools.Optional{Element: grammartools.RuleReference{Name: "mode", IsToken: false}}, grammartools.RuleReference{Name: "subtype_indication", IsToken: false}, grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "VAR_ASSIGN", IsToken: true}, grammartools.RuleReference{Name: "expression", IsToken: false}}}}}},
		},
		{
			Name: "mode",
			LineNumber: 132,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.Literal{Value: "in"}, grammartools.Literal{Value: "out"}, grammartools.Literal{Value: "inout"}, grammartools.Literal{Value: "buffer"}}},
		},
		{
			Name: "architecture_body",
			LineNumber: 154,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "architecture"}, grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.Literal{Value: "of"}, grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.Literal{Value: "is"}, grammartools.Repetition{Element: grammartools.RuleReference{Name: "block_declarative_item", IsToken: false}}, grammartools.Literal{Value: "begin"}, grammartools.Repetition{Element: grammartools.RuleReference{Name: "concurrent_statement", IsToken: false}}, grammartools.Literal{Value: "end"}, grammartools.Optional{Element: grammartools.Literal{Value: "architecture"}}, grammartools.Optional{Element: grammartools.RuleReference{Name: "NAME", IsToken: true}}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}}},
		},
		{
			Name: "block_declarative_item",
			LineNumber: 160,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "signal_declaration", IsToken: false}, grammartools.RuleReference{Name: "constant_declaration", IsToken: false}, grammartools.RuleReference{Name: "type_declaration", IsToken: false}, grammartools.RuleReference{Name: "subtype_declaration", IsToken: false}, grammartools.RuleReference{Name: "component_declaration", IsToken: false}, grammartools.RuleReference{Name: "function_declaration", IsToken: false}, grammartools.RuleReference{Name: "function_body", IsToken: false}, grammartools.RuleReference{Name: "procedure_declaration", IsToken: false}, grammartools.RuleReference{Name: "procedure_body", IsToken: false}}},
		},
		{
			Name: "signal_declaration",
			LineNumber: 189,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "signal"}, grammartools.RuleReference{Name: "name_list", IsToken: false}, grammartools.RuleReference{Name: "COLON", IsToken: true}, grammartools.RuleReference{Name: "subtype_indication", IsToken: false}, grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "VAR_ASSIGN", IsToken: true}, grammartools.RuleReference{Name: "expression", IsToken: false}}}}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}}},
		},
		{
			Name: "constant_declaration",
			LineNumber: 191,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "constant"}, grammartools.RuleReference{Name: "name_list", IsToken: false}, grammartools.RuleReference{Name: "COLON", IsToken: true}, grammartools.RuleReference{Name: "subtype_indication", IsToken: false}, grammartools.RuleReference{Name: "VAR_ASSIGN", IsToken: true}, grammartools.RuleReference{Name: "expression", IsToken: false}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}}},
		},
		{
			Name: "variable_declaration",
			LineNumber: 193,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "variable"}, grammartools.RuleReference{Name: "name_list", IsToken: false}, grammartools.RuleReference{Name: "COLON", IsToken: true}, grammartools.RuleReference{Name: "subtype_indication", IsToken: false}, grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "VAR_ASSIGN", IsToken: true}, grammartools.RuleReference{Name: "expression", IsToken: false}}}}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}}},
		},
		{
			Name: "type_declaration",
			LineNumber: 218,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "type"}, grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.Literal{Value: "is"}, grammartools.RuleReference{Name: "type_definition", IsToken: false}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}}},
		},
		{
			Name: "subtype_declaration",
			LineNumber: 219,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "subtype"}, grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.Literal{Value: "is"}, grammartools.RuleReference{Name: "subtype_indication", IsToken: false}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}}},
		},
		{
			Name: "type_definition",
			LineNumber: 221,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "enumeration_type", IsToken: false}, grammartools.RuleReference{Name: "array_type", IsToken: false}, grammartools.RuleReference{Name: "record_type", IsToken: false}}},
		},
		{
			Name: "enumeration_type",
			LineNumber: 227,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "LPAREN", IsToken: true}, grammartools.Group{Element: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.RuleReference{Name: "CHAR_LITERAL", IsToken: true}}}}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "COMMA", IsToken: true}, grammartools.Group{Element: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.RuleReference{Name: "CHAR_LITERAL", IsToken: true}}}}}}}, grammartools.RuleReference{Name: "RPAREN", IsToken: true}}},
		},
		{
			Name: "array_type",
			LineNumber: 232,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "array"}, grammartools.RuleReference{Name: "LPAREN", IsToken: true}, grammartools.RuleReference{Name: "index_constraint", IsToken: false}, grammartools.RuleReference{Name: "RPAREN", IsToken: true}, grammartools.Literal{Value: "of"}, grammartools.RuleReference{Name: "subtype_indication", IsToken: false}}},
		},
		{
			Name: "index_constraint",
			LineNumber: 234,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "discrete_range", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "COMMA", IsToken: true}, grammartools.RuleReference{Name: "discrete_range", IsToken: false}}}}}},
		},
		{
			Name: "discrete_range",
			LineNumber: 235,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "subtype_indication", IsToken: false}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "expression", IsToken: false}, grammartools.Group{Element: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.Literal{Value: "to"}, grammartools.Literal{Value: "downto"}}}}, grammartools.RuleReference{Name: "expression", IsToken: false}}}}},
		},
		{
			Name: "record_type",
			LineNumber: 239,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "record"}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.RuleReference{Name: "COLON", IsToken: true}, grammartools.RuleReference{Name: "subtype_indication", IsToken: false}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}}}}, grammartools.Literal{Value: "end"}, grammartools.Literal{Value: "record"}, grammartools.Optional{Element: grammartools.RuleReference{Name: "NAME", IsToken: true}}}},
		},
		{
			Name: "subtype_indication",
			LineNumber: 247,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "selected_name", IsToken: false}, grammartools.Optional{Element: grammartools.RuleReference{Name: "constraint", IsToken: false}}}},
		},
		{
			Name: "constraint",
			LineNumber: 249,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "LPAREN", IsToken: true}, grammartools.RuleReference{Name: "expression", IsToken: false}, grammartools.Group{Element: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.Literal{Value: "to"}, grammartools.Literal{Value: "downto"}}}}, grammartools.RuleReference{Name: "expression", IsToken: false}, grammartools.RuleReference{Name: "RPAREN", IsToken: true}}}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "range"}, grammartools.RuleReference{Name: "expression", IsToken: false}, grammartools.Group{Element: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.Literal{Value: "to"}, grammartools.Literal{Value: "downto"}}}}, grammartools.RuleReference{Name: "expression", IsToken: false}}}}},
		},
		{
			Name: "concurrent_statement",
			LineNumber: 264,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "process_statement", IsToken: false}, grammartools.RuleReference{Name: "signal_assignment_concurrent", IsToken: false}, grammartools.RuleReference{Name: "component_instantiation", IsToken: false}, grammartools.RuleReference{Name: "generate_statement", IsToken: false}}},
		},
		{
			Name: "signal_assignment_concurrent",
			LineNumber: 272,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.RuleReference{Name: "LESS_EQUALS", IsToken: true}, grammartools.RuleReference{Name: "waveform", IsToken: false}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}}},
		},
		{
			Name: "waveform",
			LineNumber: 274,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "waveform_element", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "COMMA", IsToken: true}, grammartools.RuleReference{Name: "waveform_element", IsToken: false}}}}}},
		},
		{
			Name: "waveform_element",
			LineNumber: 275,
			Body: grammartools.RuleReference{Name: "expression", IsToken: false},
		},
		{
			Name: "process_statement",
			LineNumber: 307,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.RuleReference{Name: "COLON", IsToken: true}}}}, grammartools.Literal{Value: "process"}, grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "LPAREN", IsToken: true}, grammartools.RuleReference{Name: "sensitivity_list", IsToken: false}, grammartools.RuleReference{Name: "RPAREN", IsToken: true}}}}, grammartools.Optional{Element: grammartools.Literal{Value: "is"}}, grammartools.Repetition{Element: grammartools.RuleReference{Name: "process_declarative_item", IsToken: false}}, grammartools.Literal{Value: "begin"}, grammartools.Repetition{Element: grammartools.RuleReference{Name: "sequential_statement", IsToken: false}}, grammartools.Literal{Value: "end"}, grammartools.Literal{Value: "process"}, grammartools.Optional{Element: grammartools.RuleReference{Name: "NAME", IsToken: true}}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}}},
		},
		{
			Name: "sensitivity_list",
			LineNumber: 315,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "COMMA", IsToken: true}, grammartools.RuleReference{Name: "NAME", IsToken: true}}}}}},
		},
		{
			Name: "process_declarative_item",
			LineNumber: 317,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "variable_declaration", IsToken: false}, grammartools.RuleReference{Name: "constant_declaration", IsToken: false}, grammartools.RuleReference{Name: "type_declaration", IsToken: false}, grammartools.RuleReference{Name: "subtype_declaration", IsToken: false}}},
		},
		{
			Name: "sequential_statement",
			LineNumber: 329,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "signal_assignment_seq", IsToken: false}, grammartools.RuleReference{Name: "variable_assignment", IsToken: false}, grammartools.RuleReference{Name: "if_statement", IsToken: false}, grammartools.RuleReference{Name: "case_statement", IsToken: false}, grammartools.RuleReference{Name: "loop_statement", IsToken: false}, grammartools.RuleReference{Name: "return_statement", IsToken: false}, grammartools.RuleReference{Name: "null_statement", IsToken: false}}},
		},
		{
			Name: "signal_assignment_seq",
			LineNumber: 342,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.RuleReference{Name: "LESS_EQUALS", IsToken: true}, grammartools.RuleReference{Name: "waveform", IsToken: false}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}}},
		},
		{
			Name: "variable_assignment",
			LineNumber: 346,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.RuleReference{Name: "VAR_ASSIGN", IsToken: true}, grammartools.RuleReference{Name: "expression", IsToken: false}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}}},
		},
		{
			Name: "if_statement",
			LineNumber: 356,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "if"}, grammartools.RuleReference{Name: "expression", IsToken: false}, grammartools.Literal{Value: "then"}, grammartools.Repetition{Element: grammartools.RuleReference{Name: "sequential_statement", IsToken: false}}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "elsif"}, grammartools.RuleReference{Name: "expression", IsToken: false}, grammartools.Literal{Value: "then"}, grammartools.Repetition{Element: grammartools.RuleReference{Name: "sequential_statement", IsToken: false}}}}}, grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "else"}, grammartools.Repetition{Element: grammartools.RuleReference{Name: "sequential_statement", IsToken: false}}}}}, grammartools.Literal{Value: "end"}, grammartools.Literal{Value: "if"}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}}},
		},
		{
			Name: "case_statement",
			LineNumber: 372,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "case"}, grammartools.RuleReference{Name: "expression", IsToken: false}, grammartools.Literal{Value: "is"}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "when"}, grammartools.RuleReference{Name: "choices", IsToken: false}, grammartools.RuleReference{Name: "ARROW", IsToken: true}, grammartools.Repetition{Element: grammartools.RuleReference{Name: "sequential_statement", IsToken: false}}}}}, grammartools.Literal{Value: "end"}, grammartools.Literal{Value: "case"}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}}},
		},
		{
			Name: "choices",
			LineNumber: 376,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "choice", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "PIPE", IsToken: true}, grammartools.RuleReference{Name: "choice", IsToken: false}}}}}},
		},
		{
			Name: "choice",
			LineNumber: 377,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "expression", IsToken: false}, grammartools.RuleReference{Name: "discrete_range", IsToken: false}, grammartools.Literal{Value: "others"}}},
		},
		{
			Name: "loop_statement",
			LineNumber: 391,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.RuleReference{Name: "COLON", IsToken: true}}}}, grammartools.Optional{Element: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "for"}, grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.Literal{Value: "in"}, grammartools.RuleReference{Name: "discrete_range", IsToken: false}}}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "while"}, grammartools.RuleReference{Name: "expression", IsToken: false}}}}}}, grammartools.Literal{Value: "loop"}, grammartools.Repetition{Element: grammartools.RuleReference{Name: "sequential_statement", IsToken: false}}, grammartools.Literal{Value: "end"}, grammartools.Literal{Value: "loop"}, grammartools.Optional{Element: grammartools.RuleReference{Name: "NAME", IsToken: true}}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}}},
		},
		{
			Name: "return_statement",
			LineNumber: 398,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "return"}, grammartools.Optional{Element: grammartools.RuleReference{Name: "expression", IsToken: false}}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}}},
		},
		{
			Name: "null_statement",
			LineNumber: 399,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "null"}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}}},
		},
		{
			Name: "component_declaration",
			LineNumber: 425,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "component"}, grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.Optional{Element: grammartools.Literal{Value: "is"}}, grammartools.Optional{Element: grammartools.RuleReference{Name: "generic_clause", IsToken: false}}, grammartools.Optional{Element: grammartools.RuleReference{Name: "port_clause", IsToken: false}}, grammartools.Literal{Value: "end"}, grammartools.Literal{Value: "component"}, grammartools.Optional{Element: grammartools.RuleReference{Name: "NAME", IsToken: true}}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}}},
		},
		{
			Name: "component_instantiation",
			LineNumber: 430,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.RuleReference{Name: "COLON", IsToken: true}, grammartools.Group{Element: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "entity"}, grammartools.RuleReference{Name: "selected_name", IsToken: false}, grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "LPAREN", IsToken: true}, grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.RuleReference{Name: "RPAREN", IsToken: true}}}}}}}}}, grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "generic"}, grammartools.Literal{Value: "map"}, grammartools.RuleReference{Name: "LPAREN", IsToken: true}, grammartools.RuleReference{Name: "association_list", IsToken: false}, grammartools.RuleReference{Name: "RPAREN", IsToken: true}}}}, grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "port"}, grammartools.Literal{Value: "map"}, grammartools.RuleReference{Name: "LPAREN", IsToken: true}, grammartools.RuleReference{Name: "association_list", IsToken: false}, grammartools.RuleReference{Name: "RPAREN", IsToken: true}}}}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}}},
		},
		{
			Name: "association_list",
			LineNumber: 437,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "association_element", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "COMMA", IsToken: true}, grammartools.RuleReference{Name: "association_element", IsToken: false}}}}}},
		},
		{
			Name: "association_element",
			LineNumber: 438,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.RuleReference{Name: "ARROW", IsToken: true}}}}, grammartools.RuleReference{Name: "expression", IsToken: false}}}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.RuleReference{Name: "ARROW", IsToken: true}}}}, grammartools.Literal{Value: "open"}}}}},
		},
		{
			Name: "generate_statement",
			LineNumber: 461,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.RuleReference{Name: "COLON", IsToken: true}, grammartools.Group{Element: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "for_generate", IsToken: false}, grammartools.RuleReference{Name: "if_generate", IsToken: false}}}}}},
		},
		{
			Name: "for_generate",
			LineNumber: 463,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "for"}, grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.Literal{Value: "in"}, grammartools.RuleReference{Name: "discrete_range", IsToken: false}, grammartools.Literal{Value: "generate"}, grammartools.Repetition{Element: grammartools.RuleReference{Name: "concurrent_statement", IsToken: false}}, grammartools.Literal{Value: "end"}, grammartools.Literal{Value: "generate"}, grammartools.Optional{Element: grammartools.RuleReference{Name: "NAME", IsToken: true}}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}}},
		},
		{
			Name: "if_generate",
			LineNumber: 467,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "if"}, grammartools.RuleReference{Name: "expression", IsToken: false}, grammartools.Literal{Value: "generate"}, grammartools.Repetition{Element: grammartools.RuleReference{Name: "concurrent_statement", IsToken: false}}, grammartools.Literal{Value: "end"}, grammartools.Literal{Value: "generate"}, grammartools.Optional{Element: grammartools.RuleReference{Name: "NAME", IsToken: true}}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}}},
		},
		{
			Name: "package_declaration",
			LineNumber: 488,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "package"}, grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.Literal{Value: "is"}, grammartools.Repetition{Element: grammartools.RuleReference{Name: "package_declarative_item", IsToken: false}}, grammartools.Literal{Value: "end"}, grammartools.Optional{Element: grammartools.Literal{Value: "package"}}, grammartools.Optional{Element: grammartools.RuleReference{Name: "NAME", IsToken: true}}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}}},
		},
		{
			Name: "package_body",
			LineNumber: 492,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "package"}, grammartools.Literal{Value: "body"}, grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.Literal{Value: "is"}, grammartools.Repetition{Element: grammartools.RuleReference{Name: "package_body_declarative_item", IsToken: false}}, grammartools.Literal{Value: "end"}, grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "package"}, grammartools.Literal{Value: "body"}}}}, grammartools.Optional{Element: grammartools.RuleReference{Name: "NAME", IsToken: true}}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}}},
		},
		{
			Name: "package_declarative_item",
			LineNumber: 496,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "type_declaration", IsToken: false}, grammartools.RuleReference{Name: "subtype_declaration", IsToken: false}, grammartools.RuleReference{Name: "constant_declaration", IsToken: false}, grammartools.RuleReference{Name: "signal_declaration", IsToken: false}, grammartools.RuleReference{Name: "component_declaration", IsToken: false}, grammartools.RuleReference{Name: "function_declaration", IsToken: false}, grammartools.RuleReference{Name: "procedure_declaration", IsToken: false}}},
		},
		{
			Name: "package_body_declarative_item",
			LineNumber: 504,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "type_declaration", IsToken: false}, grammartools.RuleReference{Name: "subtype_declaration", IsToken: false}, grammartools.RuleReference{Name: "constant_declaration", IsToken: false}, grammartools.RuleReference{Name: "function_body", IsToken: false}, grammartools.RuleReference{Name: "procedure_body", IsToken: false}}},
		},
		{
			Name: "function_declaration",
			LineNumber: 520,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Optional{Element: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.Literal{Value: "pure"}, grammartools.Literal{Value: "impure"}}}}, grammartools.Literal{Value: "function"}, grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "LPAREN", IsToken: true}, grammartools.RuleReference{Name: "interface_list", IsToken: false}, grammartools.RuleReference{Name: "RPAREN", IsToken: true}}}}, grammartools.Literal{Value: "return"}, grammartools.RuleReference{Name: "subtype_indication", IsToken: false}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}}},
		},
		{
			Name: "function_body",
			LineNumber: 525,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Optional{Element: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.Literal{Value: "pure"}, grammartools.Literal{Value: "impure"}}}}, grammartools.Literal{Value: "function"}, grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "LPAREN", IsToken: true}, grammartools.RuleReference{Name: "interface_list", IsToken: false}, grammartools.RuleReference{Name: "RPAREN", IsToken: true}}}}, grammartools.Literal{Value: "return"}, grammartools.RuleReference{Name: "subtype_indication", IsToken: false}, grammartools.Literal{Value: "is"}, grammartools.Repetition{Element: grammartools.RuleReference{Name: "process_declarative_item", IsToken: false}}, grammartools.Literal{Value: "begin"}, grammartools.Repetition{Element: grammartools.RuleReference{Name: "sequential_statement", IsToken: false}}, grammartools.Literal{Value: "end"}, grammartools.Optional{Element: grammartools.Literal{Value: "function"}}, grammartools.Optional{Element: grammartools.RuleReference{Name: "NAME", IsToken: true}}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}}},
		},
		{
			Name: "procedure_declaration",
			LineNumber: 534,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "procedure"}, grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "LPAREN", IsToken: true}, grammartools.RuleReference{Name: "interface_list", IsToken: false}, grammartools.RuleReference{Name: "RPAREN", IsToken: true}}}}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}}},
		},
		{
			Name: "procedure_body",
			LineNumber: 537,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "procedure"}, grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "LPAREN", IsToken: true}, grammartools.RuleReference{Name: "interface_list", IsToken: false}, grammartools.RuleReference{Name: "RPAREN", IsToken: true}}}}, grammartools.Literal{Value: "is"}, grammartools.Repetition{Element: grammartools.RuleReference{Name: "process_declarative_item", IsToken: false}}, grammartools.Literal{Value: "begin"}, grammartools.Repetition{Element: grammartools.RuleReference{Name: "sequential_statement", IsToken: false}}, grammartools.Literal{Value: "end"}, grammartools.Optional{Element: grammartools.Literal{Value: "procedure"}}, grammartools.Optional{Element: grammartools.RuleReference{Name: "NAME", IsToken: true}}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}}},
		},
		{
			Name: "expression",
			LineNumber: 574,
			Body: grammartools.RuleReference{Name: "logical_expr", IsToken: false},
		},
		{
			Name: "logical_expr",
			LineNumber: 581,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "relation", IsToken: false}, grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "logical_op", IsToken: false}, grammartools.RuleReference{Name: "relation", IsToken: false}}}}}},
		},
		{
			Name: "logical_op",
			LineNumber: 582,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.Literal{Value: "and"}, grammartools.Literal{Value: "or"}, grammartools.Literal{Value: "xor"}, grammartools.Literal{Value: "nand"}, grammartools.Literal{Value: "nor"}, grammartools.Literal{Value: "xnor"}}},
		},
		{
			Name: "relation",
			LineNumber: 586,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "shift_expr", IsToken: false}, grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "relational_op", IsToken: false}, grammartools.RuleReference{Name: "shift_expr", IsToken: false}}}}}},
		},
		{
			Name: "relational_op",
			LineNumber: 587,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "EQUALS", IsToken: true}, grammartools.RuleReference{Name: "NOT_EQUALS", IsToken: true}, grammartools.RuleReference{Name: "LESS_THAN", IsToken: true}, grammartools.RuleReference{Name: "LESS_EQUALS", IsToken: true}, grammartools.RuleReference{Name: "GREATER_THAN", IsToken: true}, grammartools.RuleReference{Name: "GREATER_EQUALS", IsToken: true}}},
		},
		{
			Name: "shift_expr",
			LineNumber: 592,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "adding_expr", IsToken: false}, grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "shift_op", IsToken: false}, grammartools.RuleReference{Name: "adding_expr", IsToken: false}}}}}},
		},
		{
			Name: "shift_op",
			LineNumber: 593,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.Literal{Value: "sll"}, grammartools.Literal{Value: "srl"}, grammartools.Literal{Value: "sla"}, grammartools.Literal{Value: "sra"}, grammartools.Literal{Value: "rol"}, grammartools.Literal{Value: "ror"}}},
		},
		{
			Name: "adding_expr",
			LineNumber: 597,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "multiplying_expr", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "adding_op", IsToken: false}, grammartools.RuleReference{Name: "multiplying_expr", IsToken: false}}}}}},
		},
		{
			Name: "adding_op",
			LineNumber: 598,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "PLUS", IsToken: true}, grammartools.RuleReference{Name: "MINUS", IsToken: true}, grammartools.RuleReference{Name: "AMPERSAND", IsToken: true}}},
		},
		{
			Name: "multiplying_expr",
			LineNumber: 601,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "unary_expr", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "multiplying_op", IsToken: false}, grammartools.RuleReference{Name: "unary_expr", IsToken: false}}}}}},
		},
		{
			Name: "multiplying_op",
			LineNumber: 602,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "STAR", IsToken: true}, grammartools.RuleReference{Name: "SLASH", IsToken: true}, grammartools.Literal{Value: "mod"}, grammartools.Literal{Value: "rem"}}},
		},
		{
			Name: "unary_expr",
			LineNumber: 605,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "abs"}, grammartools.RuleReference{Name: "unary_expr", IsToken: false}}}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "not"}, grammartools.RuleReference{Name: "unary_expr", IsToken: false}}}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Group{Element: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "PLUS", IsToken: true}, grammartools.RuleReference{Name: "MINUS", IsToken: true}}}}, grammartools.RuleReference{Name: "unary_expr", IsToken: false}}}, grammartools.RuleReference{Name: "power_expr", IsToken: false}}},
		},
		{
			Name: "power_expr",
			LineNumber: 611,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "primary", IsToken: false}, grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "POWER", IsToken: true}, grammartools.RuleReference{Name: "primary", IsToken: false}}}}}},
		},
		{
			Name: "primary",
			LineNumber: 619,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "NUMBER", IsToken: true}, grammartools.RuleReference{Name: "REAL_NUMBER", IsToken: true}, grammartools.RuleReference{Name: "BASED_LITERAL", IsToken: true}, grammartools.RuleReference{Name: "STRING", IsToken: true}, grammartools.RuleReference{Name: "CHAR_LITERAL", IsToken: true}, grammartools.RuleReference{Name: "BIT_STRING", IsToken: true}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "TICK", IsToken: true}, grammartools.RuleReference{Name: "NAME", IsToken: true}}}}}}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.RuleReference{Name: "LPAREN", IsToken: true}, grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "expression", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "COMMA", IsToken: true}, grammartools.RuleReference{Name: "expression", IsToken: false}}}}}}}, grammartools.RuleReference{Name: "RPAREN", IsToken: true}}}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "LPAREN", IsToken: true}, grammartools.RuleReference{Name: "expression", IsToken: false}, grammartools.RuleReference{Name: "RPAREN", IsToken: true}}}, grammartools.RuleReference{Name: "aggregate", IsToken: false}, grammartools.Literal{Value: "null"}}},
		},
		{
			Name: "aggregate",
			LineNumber: 635,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "LPAREN", IsToken: true}, grammartools.RuleReference{Name: "element_association", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "COMMA", IsToken: true}, grammartools.RuleReference{Name: "element_association", IsToken: false}}}}, grammartools.RuleReference{Name: "RPAREN", IsToken: true}}},
		},
		{
			Name: "element_association",
			LineNumber: 636,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "choices", IsToken: false}, grammartools.RuleReference{Name: "ARROW", IsToken: true}}}}, grammartools.RuleReference{Name: "expression", IsToken: false}}},
		},
	},
}
