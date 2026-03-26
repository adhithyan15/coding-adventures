// AUTO-GENERATED FILE - DO NOT EDIT
package latticeparser

import (
	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
)

var LatticeGrammar = &grammartools.ParserGrammar{
	Version: 1,
	Rules: []grammartools.GrammarRule{
		{
			Name: "stylesheet",
			LineNumber: 37,
			Body: grammartools.Repetition{Element: grammartools.RuleReference{Name: "rule", IsToken: false}},
		},
		{
			Name: "rule",
			LineNumber: 39,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "lattice_rule", IsToken: false}, grammartools.RuleReference{Name: "at_rule", IsToken: false}, grammartools.RuleReference{Name: "qualified_rule", IsToken: false}}},
		},
		{
			Name: "lattice_rule",
			LineNumber: 51,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "variable_declaration", IsToken: false}, grammartools.RuleReference{Name: "mixin_definition", IsToken: false}, grammartools.RuleReference{Name: "function_definition", IsToken: false}, grammartools.RuleReference{Name: "use_directive", IsToken: false}, grammartools.RuleReference{Name: "lattice_control", IsToken: false}}},
		},
		{
			Name: "variable_declaration",
			LineNumber: 69,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "VARIABLE", IsToken: true}, grammartools.RuleReference{Name: "COLON", IsToken: true}, grammartools.RuleReference{Name: "value_list", IsToken: false}, grammartools.Optional{Element: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "BANG_DEFAULT", IsToken: true}, grammartools.RuleReference{Name: "BANG_GLOBAL", IsToken: true}}}}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}}},
		},
		{
			Name: "mixin_definition",
			LineNumber: 102,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "@mixin"}, grammartools.RuleReference{Name: "FUNCTION", IsToken: true}, grammartools.Optional{Element: grammartools.RuleReference{Name: "mixin_params", IsToken: false}}, grammartools.RuleReference{Name: "RPAREN", IsToken: true}, grammartools.RuleReference{Name: "block", IsToken: false}}}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "@mixin"}, grammartools.RuleReference{Name: "IDENT", IsToken: true}, grammartools.RuleReference{Name: "block", IsToken: false}}}}},
		},
		{
			Name: "mixin_params",
			LineNumber: 105,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "mixin_param", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "COMMA", IsToken: true}, grammartools.RuleReference{Name: "mixin_param", IsToken: false}}}}}},
		},
		{
			Name: "mixin_param",
			LineNumber: 112,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "VARIABLE", IsToken: true}, grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "COLON", IsToken: true}, grammartools.RuleReference{Name: "mixin_value_list", IsToken: false}}}}}},
		},
		{
			Name: "mixin_value_list",
			LineNumber: 117,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "mixin_value", IsToken: false}, grammartools.Repetition{Element: grammartools.RuleReference{Name: "mixin_value", IsToken: false}}}},
		},
		{
			Name: "mixin_value",
			LineNumber: 119,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "DIMENSION", IsToken: true}, grammartools.RuleReference{Name: "PERCENTAGE", IsToken: true}, grammartools.RuleReference{Name: "NUMBER", IsToken: true}, grammartools.RuleReference{Name: "STRING", IsToken: true}, grammartools.RuleReference{Name: "IDENT", IsToken: true}, grammartools.RuleReference{Name: "HASH", IsToken: true}, grammartools.RuleReference{Name: "CUSTOM_PROPERTY", IsToken: true}, grammartools.RuleReference{Name: "UNICODE_RANGE", IsToken: true}, grammartools.RuleReference{Name: "function_call", IsToken: false}, grammartools.RuleReference{Name: "VARIABLE", IsToken: true}, grammartools.RuleReference{Name: "SLASH", IsToken: true}, grammartools.RuleReference{Name: "PLUS", IsToken: true}, grammartools.RuleReference{Name: "MINUS", IsToken: true}}},
		},
		{
			Name: "include_directive",
			LineNumber: 130,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "@include"}, grammartools.RuleReference{Name: "FUNCTION", IsToken: true}, grammartools.Optional{Element: grammartools.RuleReference{Name: "include_args", IsToken: false}}, grammartools.RuleReference{Name: "RPAREN", IsToken: true}, grammartools.Group{Element: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}, grammartools.RuleReference{Name: "block", IsToken: false}}}}}}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "@include"}, grammartools.RuleReference{Name: "IDENT", IsToken: true}, grammartools.Group{Element: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}, grammartools.RuleReference{Name: "block", IsToken: false}}}}}}}},
		},
		{
			Name: "include_args",
			LineNumber: 133,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "include_arg", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "COMMA", IsToken: true}, grammartools.RuleReference{Name: "include_arg", IsToken: false}}}}}},
		},
		{
			Name: "include_arg",
			LineNumber: 137,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "VARIABLE", IsToken: true}, grammartools.RuleReference{Name: "COLON", IsToken: true}, grammartools.RuleReference{Name: "value_list", IsToken: false}}}, grammartools.RuleReference{Name: "value_list", IsToken: false}}},
		},
		{
			Name: "lattice_control",
			LineNumber: 160,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "if_directive", IsToken: false}, grammartools.RuleReference{Name: "for_directive", IsToken: false}, grammartools.RuleReference{Name: "each_directive", IsToken: false}, grammartools.RuleReference{Name: "while_directive", IsToken: false}}},
		},
		{
			Name: "if_directive",
			LineNumber: 164,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "@if"}, grammartools.RuleReference{Name: "lattice_expression", IsToken: false}, grammartools.RuleReference{Name: "block", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "@else"}, grammartools.Literal{Value: "if"}, grammartools.RuleReference{Name: "lattice_expression", IsToken: false}, grammartools.RuleReference{Name: "block", IsToken: false}}}}, grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "@else"}, grammartools.RuleReference{Name: "block", IsToken: false}}}}}},
		},
		{
			Name: "for_directive",
			LineNumber: 171,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "@for"}, grammartools.RuleReference{Name: "VARIABLE", IsToken: true}, grammartools.Literal{Value: "from"}, grammartools.RuleReference{Name: "lattice_expression", IsToken: false}, grammartools.Group{Element: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.Literal{Value: "through"}, grammartools.Literal{Value: "to"}}}}, grammartools.RuleReference{Name: "lattice_expression", IsToken: false}, grammartools.RuleReference{Name: "block", IsToken: false}}},
		},
		{
			Name: "each_directive",
			LineNumber: 176,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "@each"}, grammartools.RuleReference{Name: "VARIABLE", IsToken: true}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "COMMA", IsToken: true}, grammartools.RuleReference{Name: "VARIABLE", IsToken: true}}}}, grammartools.Literal{Value: "in"}, grammartools.RuleReference{Name: "each_list", IsToken: false}, grammartools.RuleReference{Name: "block", IsToken: false}}},
		},
		{
			Name: "each_list",
			LineNumber: 179,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "value", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "COMMA", IsToken: true}, grammartools.RuleReference{Name: "value", IsToken: false}}}}}},
		},
		{
			Name: "while_directive",
			LineNumber: 184,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "@while"}, grammartools.RuleReference{Name: "lattice_expression", IsToken: false}, grammartools.RuleReference{Name: "block", IsToken: false}}},
		},
		{
			Name: "lattice_expression",
			LineNumber: 203,
			Body: grammartools.RuleReference{Name: "lattice_or_expr", IsToken: false},
		},
		{
			Name: "lattice_or_expr",
			LineNumber: 205,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "lattice_and_expr", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "or"}, grammartools.RuleReference{Name: "lattice_and_expr", IsToken: false}}}}}},
		},
		{
			Name: "lattice_and_expr",
			LineNumber: 207,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "lattice_comparison", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "and"}, grammartools.RuleReference{Name: "lattice_comparison", IsToken: false}}}}}},
		},
		{
			Name: "lattice_comparison",
			LineNumber: 209,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "lattice_additive", IsToken: false}, grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "comparison_op", IsToken: false}, grammartools.RuleReference{Name: "lattice_additive", IsToken: false}}}}}},
		},
		{
			Name: "comparison_op",
			LineNumber: 211,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "EQUALS_EQUALS", IsToken: true}, grammartools.RuleReference{Name: "NOT_EQUALS", IsToken: true}, grammartools.RuleReference{Name: "GREATER", IsToken: true}, grammartools.RuleReference{Name: "GREATER_EQUALS", IsToken: true}, grammartools.RuleReference{Name: "LESS", IsToken: true}, grammartools.RuleReference{Name: "LESS_EQUALS", IsToken: true}}},
		},
		{
			Name: "lattice_additive",
			LineNumber: 214,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "lattice_multiplicative", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Group{Element: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "PLUS", IsToken: true}, grammartools.RuleReference{Name: "MINUS", IsToken: true}}}}, grammartools.RuleReference{Name: "lattice_multiplicative", IsToken: false}}}}}},
		},
		{
			Name: "lattice_multiplicative",
			LineNumber: 219,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "lattice_unary", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Group{Element: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "STAR", IsToken: true}, grammartools.RuleReference{Name: "SLASH", IsToken: true}}}}, grammartools.RuleReference{Name: "lattice_unary", IsToken: false}}}}}},
		},
		{
			Name: "lattice_unary",
			LineNumber: 221,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "MINUS", IsToken: true}, grammartools.RuleReference{Name: "lattice_unary", IsToken: false}}}, grammartools.RuleReference{Name: "lattice_primary", IsToken: false}}},
		},
		{
			Name: "lattice_primary",
			LineNumber: 224,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "VARIABLE", IsToken: true}, grammartools.RuleReference{Name: "NUMBER", IsToken: true}, grammartools.RuleReference{Name: "DIMENSION", IsToken: true}, grammartools.RuleReference{Name: "PERCENTAGE", IsToken: true}, grammartools.RuleReference{Name: "STRING", IsToken: true}, grammartools.RuleReference{Name: "IDENT", IsToken: true}, grammartools.RuleReference{Name: "HASH", IsToken: true}, grammartools.Literal{Value: "true"}, grammartools.Literal{Value: "false"}, grammartools.Literal{Value: "null"}, grammartools.RuleReference{Name: "function_call", IsToken: false}, grammartools.RuleReference{Name: "map_literal", IsToken: false}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "LPAREN", IsToken: true}, grammartools.RuleReference{Name: "lattice_expression", IsToken: false}, grammartools.RuleReference{Name: "RPAREN", IsToken: true}}}}},
		},
		{
			Name: "map_literal",
			LineNumber: 235,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "LPAREN", IsToken: true}, grammartools.RuleReference{Name: "map_entry", IsToken: false}, grammartools.RuleReference{Name: "COMMA", IsToken: true}, grammartools.RuleReference{Name: "map_entry", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "COMMA", IsToken: true}, grammartools.RuleReference{Name: "map_entry", IsToken: false}}}}, grammartools.RuleReference{Name: "RPAREN", IsToken: true}}},
		},
		{
			Name: "map_entry",
			LineNumber: 237,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Group{Element: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "IDENT", IsToken: true}, grammartools.RuleReference{Name: "STRING", IsToken: true}}}}, grammartools.RuleReference{Name: "COLON", IsToken: true}, grammartools.RuleReference{Name: "lattice_expression", IsToken: false}}},
		},
		{
			Name: "function_definition",
			LineNumber: 261,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "@function"}, grammartools.RuleReference{Name: "FUNCTION", IsToken: true}, grammartools.Optional{Element: grammartools.RuleReference{Name: "mixin_params", IsToken: false}}, grammartools.RuleReference{Name: "RPAREN", IsToken: true}, grammartools.RuleReference{Name: "function_body", IsToken: false}}}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "@function"}, grammartools.RuleReference{Name: "IDENT", IsToken: true}, grammartools.RuleReference{Name: "function_body", IsToken: false}}}}},
		},
		{
			Name: "function_body",
			LineNumber: 264,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "LBRACE", IsToken: true}, grammartools.Repetition{Element: grammartools.RuleReference{Name: "function_body_item", IsToken: false}}, grammartools.RuleReference{Name: "RBRACE", IsToken: true}}},
		},
		{
			Name: "function_body_item",
			LineNumber: 266,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "variable_declaration", IsToken: false}, grammartools.RuleReference{Name: "return_directive", IsToken: false}, grammartools.RuleReference{Name: "lattice_control", IsToken: false}}},
		},
		{
			Name: "return_directive",
			LineNumber: 268,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "@return"}, grammartools.RuleReference{Name: "lattice_expression", IsToken: false}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}}},
		},
		{
			Name: "use_directive",
			LineNumber: 281,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "@use"}, grammartools.RuleReference{Name: "STRING", IsToken: true}, grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "as"}, grammartools.RuleReference{Name: "IDENT", IsToken: true}}}}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}}},
		},
		{
			Name: "at_rule",
			LineNumber: 294,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "AT_KEYWORD", IsToken: true}, grammartools.RuleReference{Name: "at_prelude", IsToken: false}, grammartools.Group{Element: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}, grammartools.RuleReference{Name: "block", IsToken: false}}}}}},
		},
		{
			Name: "at_prelude",
			LineNumber: 296,
			Body: grammartools.Repetition{Element: grammartools.RuleReference{Name: "at_prelude_token", IsToken: false}},
		},
		{
			Name: "at_prelude_token",
			LineNumber: 298,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "IDENT", IsToken: true}, grammartools.RuleReference{Name: "STRING", IsToken: true}, grammartools.RuleReference{Name: "NUMBER", IsToken: true}, grammartools.RuleReference{Name: "DIMENSION", IsToken: true}, grammartools.RuleReference{Name: "PERCENTAGE", IsToken: true}, grammartools.RuleReference{Name: "HASH", IsToken: true}, grammartools.RuleReference{Name: "CUSTOM_PROPERTY", IsToken: true}, grammartools.RuleReference{Name: "UNICODE_RANGE", IsToken: true}, grammartools.RuleReference{Name: "VARIABLE", IsToken: true}, grammartools.RuleReference{Name: "function_in_prelude", IsToken: false}, grammartools.RuleReference{Name: "paren_block", IsToken: false}, grammartools.RuleReference{Name: "COLON", IsToken: true}, grammartools.RuleReference{Name: "COMMA", IsToken: true}, grammartools.RuleReference{Name: "SLASH", IsToken: true}, grammartools.RuleReference{Name: "DOT", IsToken: true}, grammartools.RuleReference{Name: "STAR", IsToken: true}, grammartools.RuleReference{Name: "PLUS", IsToken: true}, grammartools.RuleReference{Name: "MINUS", IsToken: true}, grammartools.RuleReference{Name: "GREATER", IsToken: true}, grammartools.RuleReference{Name: "TILDE", IsToken: true}, grammartools.RuleReference{Name: "PIPE", IsToken: true}, grammartools.RuleReference{Name: "EQUALS", IsToken: true}, grammartools.RuleReference{Name: "AMPERSAND", IsToken: true}, grammartools.RuleReference{Name: "CDO", IsToken: true}, grammartools.RuleReference{Name: "CDC", IsToken: true}}},
		},
		{
			Name: "function_in_prelude",
			LineNumber: 306,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "FUNCTION", IsToken: true}, grammartools.RuleReference{Name: "at_prelude_tokens", IsToken: false}, grammartools.RuleReference{Name: "RPAREN", IsToken: true}}},
		},
		{
			Name: "paren_block",
			LineNumber: 307,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "LPAREN", IsToken: true}, grammartools.RuleReference{Name: "at_prelude_tokens", IsToken: false}, grammartools.RuleReference{Name: "RPAREN", IsToken: true}}},
		},
		{
			Name: "at_prelude_tokens",
			LineNumber: 308,
			Body: grammartools.Repetition{Element: grammartools.RuleReference{Name: "at_prelude_token", IsToken: false}},
		},
		{
			Name: "qualified_rule",
			LineNumber: 314,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "selector_list", IsToken: false}, grammartools.RuleReference{Name: "block", IsToken: false}}},
		},
		{
			Name: "selector_list",
			LineNumber: 320,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "complex_selector", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "COMMA", IsToken: true}, grammartools.RuleReference{Name: "complex_selector", IsToken: false}}}}}},
		},
		{
			Name: "complex_selector",
			LineNumber: 322,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "compound_selector", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Optional{Element: grammartools.RuleReference{Name: "combinator", IsToken: false}}, grammartools.RuleReference{Name: "compound_selector", IsToken: false}}}}}},
		},
		{
			Name: "combinator",
			LineNumber: 324,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "GREATER", IsToken: true}, grammartools.RuleReference{Name: "PLUS", IsToken: true}, grammartools.RuleReference{Name: "TILDE", IsToken: true}}},
		},
		{
			Name: "compound_selector",
			LineNumber: 326,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "simple_selector", IsToken: false}, grammartools.Repetition{Element: grammartools.RuleReference{Name: "subclass_selector", IsToken: false}}}}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "subclass_selector", IsToken: false}, grammartools.Repetition{Element: grammartools.RuleReference{Name: "subclass_selector", IsToken: false}}}}}},
		},
		{
			Name: "simple_selector",
			LineNumber: 330,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "IDENT", IsToken: true}, grammartools.RuleReference{Name: "STAR", IsToken: true}, grammartools.RuleReference{Name: "AMPERSAND", IsToken: true}, grammartools.RuleReference{Name: "VARIABLE", IsToken: true}}},
		},
		{
			Name: "subclass_selector",
			LineNumber: 333,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "class_selector", IsToken: false}, grammartools.RuleReference{Name: "id_selector", IsToken: false}, grammartools.RuleReference{Name: "placeholder_selector", IsToken: false}, grammartools.RuleReference{Name: "attribute_selector", IsToken: false}, grammartools.RuleReference{Name: "pseudo_class", IsToken: false}, grammartools.RuleReference{Name: "pseudo_element", IsToken: false}}},
		},
		{
			Name: "placeholder_selector",
			LineNumber: 337,
			Body: grammartools.RuleReference{Name: "PLACEHOLDER", IsToken: true},
		},
		{
			Name: "class_selector",
			LineNumber: 339,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "DOT", IsToken: true}, grammartools.RuleReference{Name: "IDENT", IsToken: true}}},
		},
		{
			Name: "id_selector",
			LineNumber: 341,
			Body: grammartools.RuleReference{Name: "HASH", IsToken: true},
		},
		{
			Name: "attribute_selector",
			LineNumber: 343,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "LBRACKET", IsToken: true}, grammartools.RuleReference{Name: "IDENT", IsToken: true}, grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "attr_matcher", IsToken: false}, grammartools.RuleReference{Name: "attr_value", IsToken: false}, grammartools.Optional{Element: grammartools.RuleReference{Name: "IDENT", IsToken: true}}}}}, grammartools.RuleReference{Name: "RBRACKET", IsToken: true}}},
		},
		{
			Name: "attr_matcher",
			LineNumber: 345,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "EQUALS", IsToken: true}, grammartools.RuleReference{Name: "TILDE_EQUALS", IsToken: true}, grammartools.RuleReference{Name: "PIPE_EQUALS", IsToken: true}, grammartools.RuleReference{Name: "CARET_EQUALS", IsToken: true}, grammartools.RuleReference{Name: "DOLLAR_EQUALS", IsToken: true}, grammartools.RuleReference{Name: "STAR_EQUALS", IsToken: true}}},
		},
		{
			Name: "attr_value",
			LineNumber: 348,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "IDENT", IsToken: true}, grammartools.RuleReference{Name: "STRING", IsToken: true}}},
		},
		{
			Name: "pseudo_class",
			LineNumber: 350,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "COLON", IsToken: true}, grammartools.RuleReference{Name: "FUNCTION", IsToken: true}, grammartools.RuleReference{Name: "pseudo_class_args", IsToken: false}, grammartools.RuleReference{Name: "RPAREN", IsToken: true}}}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "COLON", IsToken: true}, grammartools.RuleReference{Name: "IDENT", IsToken: true}}}}},
		},
		{
			Name: "pseudo_class_args",
			LineNumber: 353,
			Body: grammartools.Repetition{Element: grammartools.RuleReference{Name: "pseudo_class_arg", IsToken: false}},
		},
		{
			Name: "pseudo_class_arg",
			LineNumber: 355,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "IDENT", IsToken: true}, grammartools.RuleReference{Name: "NUMBER", IsToken: true}, grammartools.RuleReference{Name: "DIMENSION", IsToken: true}, grammartools.RuleReference{Name: "STRING", IsToken: true}, grammartools.RuleReference{Name: "HASH", IsToken: true}, grammartools.RuleReference{Name: "PLUS", IsToken: true}, grammartools.RuleReference{Name: "COMMA", IsToken: true}, grammartools.RuleReference{Name: "DOT", IsToken: true}, grammartools.RuleReference{Name: "STAR", IsToken: true}, grammartools.RuleReference{Name: "COLON", IsToken: true}, grammartools.RuleReference{Name: "AMPERSAND", IsToken: true}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "FUNCTION", IsToken: true}, grammartools.RuleReference{Name: "pseudo_class_args", IsToken: false}, grammartools.RuleReference{Name: "RPAREN", IsToken: true}}}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "LBRACKET", IsToken: true}, grammartools.RuleReference{Name: "pseudo_class_args", IsToken: false}, grammartools.RuleReference{Name: "RBRACKET", IsToken: true}}}}},
		},
		{
			Name: "pseudo_element",
			LineNumber: 360,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "COLON_COLON", IsToken: true}, grammartools.RuleReference{Name: "IDENT", IsToken: true}}},
		},
		{
			Name: "block",
			LineNumber: 370,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "LBRACE", IsToken: true}, grammartools.RuleReference{Name: "block_contents", IsToken: false}, grammartools.RuleReference{Name: "RBRACE", IsToken: true}}},
		},
		{
			Name: "block_contents",
			LineNumber: 372,
			Body: grammartools.Repetition{Element: grammartools.RuleReference{Name: "block_item", IsToken: false}},
		},
		{
			Name: "block_item",
			LineNumber: 374,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "lattice_block_item", IsToken: false}, grammartools.RuleReference{Name: "at_rule", IsToken: false}, grammartools.RuleReference{Name: "declaration_or_nested", IsToken: false}}},
		},
		{
			Name: "lattice_block_item",
			LineNumber: 380,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "variable_declaration", IsToken: false}, grammartools.RuleReference{Name: "include_directive", IsToken: false}, grammartools.RuleReference{Name: "lattice_control", IsToken: false}, grammartools.RuleReference{Name: "content_directive", IsToken: false}, grammartools.RuleReference{Name: "extend_directive", IsToken: false}, grammartools.RuleReference{Name: "at_root_directive", IsToken: false}}},
		},
		{
			Name: "content_directive",
			LineNumber: 390,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "@content"}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}}},
		},
		{
			Name: "extend_directive",
			LineNumber: 398,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "@extend"}, grammartools.RuleReference{Name: "selector_list", IsToken: false}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}}},
		},
		{
			Name: "at_root_directive",
			LineNumber: 403,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "@at-root"}, grammartools.Group{Element: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "selector_list", IsToken: false}, grammartools.RuleReference{Name: "block", IsToken: false}}}, grammartools.RuleReference{Name: "block", IsToken: false}}}}}},
		},
		{
			Name: "declaration_or_nested",
			LineNumber: 405,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "declaration", IsToken: false}, grammartools.RuleReference{Name: "qualified_rule", IsToken: false}}},
		},
		{
			Name: "declaration",
			LineNumber: 414,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "property", IsToken: false}, grammartools.RuleReference{Name: "COLON", IsToken: true}, grammartools.RuleReference{Name: "value_list", IsToken: false}, grammartools.Optional{Element: grammartools.RuleReference{Name: "priority", IsToken: false}}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}}}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "property", IsToken: false}, grammartools.RuleReference{Name: "COLON", IsToken: true}, grammartools.RuleReference{Name: "block", IsToken: false}}}}},
		},
		{
			Name: "property",
			LineNumber: 417,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "IDENT", IsToken: true}, grammartools.RuleReference{Name: "CUSTOM_PROPERTY", IsToken: true}}},
		},
		{
			Name: "priority",
			LineNumber: 419,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "BANG", IsToken: true}, grammartools.Literal{Value: "important"}}},
		},
		{
			Name: "value_list",
			LineNumber: 430,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "value", IsToken: false}, grammartools.Repetition{Element: grammartools.RuleReference{Name: "value", IsToken: false}}}},
		},
		{
			Name: "value",
			LineNumber: 432,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "DIMENSION", IsToken: true}, grammartools.RuleReference{Name: "PERCENTAGE", IsToken: true}, grammartools.RuleReference{Name: "NUMBER", IsToken: true}, grammartools.RuleReference{Name: "STRING", IsToken: true}, grammartools.RuleReference{Name: "IDENT", IsToken: true}, grammartools.RuleReference{Name: "HASH", IsToken: true}, grammartools.RuleReference{Name: "CUSTOM_PROPERTY", IsToken: true}, grammartools.RuleReference{Name: "UNICODE_RANGE", IsToken: true}, grammartools.RuleReference{Name: "function_call", IsToken: false}, grammartools.RuleReference{Name: "VARIABLE", IsToken: true}, grammartools.RuleReference{Name: "SLASH", IsToken: true}, grammartools.RuleReference{Name: "COMMA", IsToken: true}, grammartools.RuleReference{Name: "PLUS", IsToken: true}, grammartools.RuleReference{Name: "MINUS", IsToken: true}, grammartools.RuleReference{Name: "map_literal", IsToken: false}}},
		},
		{
			Name: "function_call",
			LineNumber: 438,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "FUNCTION", IsToken: true}, grammartools.RuleReference{Name: "function_args", IsToken: false}, grammartools.RuleReference{Name: "RPAREN", IsToken: true}}}, grammartools.RuleReference{Name: "URL_TOKEN", IsToken: true}}},
		},
		{
			Name: "function_args",
			LineNumber: 441,
			Body: grammartools.Repetition{Element: grammartools.RuleReference{Name: "function_arg", IsToken: false}},
		},
		{
			Name: "function_arg",
			LineNumber: 443,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "DIMENSION", IsToken: true}, grammartools.RuleReference{Name: "PERCENTAGE", IsToken: true}, grammartools.RuleReference{Name: "NUMBER", IsToken: true}, grammartools.RuleReference{Name: "STRING", IsToken: true}, grammartools.RuleReference{Name: "IDENT", IsToken: true}, grammartools.RuleReference{Name: "HASH", IsToken: true}, grammartools.RuleReference{Name: "CUSTOM_PROPERTY", IsToken: true}, grammartools.RuleReference{Name: "COMMA", IsToken: true}, grammartools.RuleReference{Name: "SLASH", IsToken: true}, grammartools.RuleReference{Name: "PLUS", IsToken: true}, grammartools.RuleReference{Name: "MINUS", IsToken: true}, grammartools.RuleReference{Name: "STAR", IsToken: true}, grammartools.RuleReference{Name: "VARIABLE", IsToken: true}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "FUNCTION", IsToken: true}, grammartools.RuleReference{Name: "function_args", IsToken: false}, grammartools.RuleReference{Name: "RPAREN", IsToken: true}}}}},
		},
	},
}
