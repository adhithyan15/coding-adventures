// AUTO-GENERATED FILE - DO NOT EDIT
package excelparser

import (
	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
)

var ExcelGrammar = &grammartools.ParserGrammar{
	Version: 1,
	Rules: []grammartools.GrammarRule{
		{
			Name: "formula",
			LineNumber: 15,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "ws", IsToken: false}, grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "EQUALS", IsToken: true}, grammartools.RuleReference{Name: "ws", IsToken: false}}}}, grammartools.RuleReference{Name: "expression", IsToken: false}, grammartools.RuleReference{Name: "ws", IsToken: false}}},
		},
		{
			Name: "ws",
			LineNumber: 17,
			Body: grammartools.Repetition{Element: grammartools.RuleReference{Name: "SPACE", IsToken: true}},
		},
		{
			Name: "req_space",
			LineNumber: 18,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "SPACE", IsToken: true}, grammartools.Repetition{Element: grammartools.RuleReference{Name: "SPACE", IsToken: true}}}},
		},
		{
			Name: "expression",
			LineNumber: 20,
			Body: grammartools.RuleReference{Name: "comparison_expr", IsToken: false},
		},
		{
			Name: "comparison_expr",
			LineNumber: 22,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "concat_expr", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "ws", IsToken: false}, grammartools.RuleReference{Name: "comparison_op", IsToken: false}, grammartools.RuleReference{Name: "ws", IsToken: false}, grammartools.RuleReference{Name: "concat_expr", IsToken: false}}}}}},
		},
		{
			Name: "comparison_op",
			LineNumber: 23,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "EQUALS", IsToken: true}, grammartools.RuleReference{Name: "NOT_EQUALS", IsToken: true}, grammartools.RuleReference{Name: "LESS_THAN", IsToken: true}, grammartools.RuleReference{Name: "LESS_EQUALS", IsToken: true}, grammartools.RuleReference{Name: "GREATER_THAN", IsToken: true}, grammartools.RuleReference{Name: "GREATER_EQUALS", IsToken: true}}},
		},
		{
			Name: "concat_expr",
			LineNumber: 26,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "additive_expr", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "ws", IsToken: false}, grammartools.RuleReference{Name: "AMP", IsToken: true}, grammartools.RuleReference{Name: "ws", IsToken: false}, grammartools.RuleReference{Name: "additive_expr", IsToken: false}}}}}},
		},
		{
			Name: "additive_expr",
			LineNumber: 27,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "multiplicative_expr", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "ws", IsToken: false}, grammartools.Group{Element: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "PLUS", IsToken: true}, grammartools.RuleReference{Name: "MINUS", IsToken: true}}}}, grammartools.RuleReference{Name: "ws", IsToken: false}, grammartools.RuleReference{Name: "multiplicative_expr", IsToken: false}}}}}},
		},
		{
			Name: "multiplicative_expr",
			LineNumber: 28,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "power_expr", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "ws", IsToken: false}, grammartools.Group{Element: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "STAR", IsToken: true}, grammartools.RuleReference{Name: "SLASH", IsToken: true}}}}, grammartools.RuleReference{Name: "ws", IsToken: false}, grammartools.RuleReference{Name: "power_expr", IsToken: false}}}}}},
		},
		{
			Name: "power_expr",
			LineNumber: 29,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "unary_expr", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "ws", IsToken: false}, grammartools.RuleReference{Name: "CARET", IsToken: true}, grammartools.RuleReference{Name: "ws", IsToken: false}, grammartools.RuleReference{Name: "unary_expr", IsToken: false}}}}}},
		},
		{
			Name: "unary_expr",
			LineNumber: 30,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "prefix_op", IsToken: false}, grammartools.RuleReference{Name: "ws", IsToken: false}}}}, grammartools.RuleReference{Name: "postfix_expr", IsToken: false}}},
		},
		{
			Name: "prefix_op",
			LineNumber: 31,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "PLUS", IsToken: true}, grammartools.RuleReference{Name: "MINUS", IsToken: true}}},
		},
		{
			Name: "postfix_expr",
			LineNumber: 32,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "primary", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "ws", IsToken: false}, grammartools.RuleReference{Name: "PERCENT", IsToken: true}}}}}},
		},
		{
			Name: "primary",
			LineNumber: 34,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "parenthesized_expression", IsToken: false}, grammartools.RuleReference{Name: "constant", IsToken: false}, grammartools.RuleReference{Name: "function_call", IsToken: false}, grammartools.RuleReference{Name: "structure_reference", IsToken: false}, grammartools.RuleReference{Name: "reference_expression", IsToken: false}, grammartools.RuleReference{Name: "bang_reference", IsToken: false}, grammartools.RuleReference{Name: "bang_name", IsToken: false}, grammartools.RuleReference{Name: "name_reference", IsToken: false}}},
		},
		{
			Name: "parenthesized_expression",
			LineNumber: 43,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "LPAREN", IsToken: true}, grammartools.RuleReference{Name: "ws", IsToken: false}, grammartools.RuleReference{Name: "expression", IsToken: false}, grammartools.RuleReference{Name: "ws", IsToken: false}, grammartools.RuleReference{Name: "RPAREN", IsToken: true}}},
		},
		{
			Name: "constant",
			LineNumber: 45,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "NUMBER", IsToken: true}, grammartools.RuleReference{Name: "STRING", IsToken: true}, grammartools.RuleReference{Name: "KEYWORD", IsToken: true}, grammartools.RuleReference{Name: "ERROR_CONSTANT", IsToken: true}, grammartools.RuleReference{Name: "array_constant", IsToken: false}}},
		},
		{
			Name: "array_constant",
			LineNumber: 47,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "LBRACE", IsToken: true}, grammartools.RuleReference{Name: "ws", IsToken: false}, grammartools.RuleReference{Name: "array_row", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "ws", IsToken: false}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}, grammartools.RuleReference{Name: "ws", IsToken: false}, grammartools.RuleReference{Name: "array_row", IsToken: false}}}}, grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "ws", IsToken: false}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}}}}, grammartools.RuleReference{Name: "ws", IsToken: false}, grammartools.RuleReference{Name: "RBRACE", IsToken: true}}},
		},
		{
			Name: "array_row",
			LineNumber: 48,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "array_item", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "ws", IsToken: false}, grammartools.RuleReference{Name: "COMMA", IsToken: true}, grammartools.RuleReference{Name: "ws", IsToken: false}, grammartools.RuleReference{Name: "array_item", IsToken: false}}}}, grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "ws", IsToken: false}, grammartools.RuleReference{Name: "COMMA", IsToken: true}}}}}},
		},
		{
			Name: "array_item",
			LineNumber: 49,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "NUMBER", IsToken: true}, grammartools.RuleReference{Name: "STRING", IsToken: true}, grammartools.RuleReference{Name: "KEYWORD", IsToken: true}, grammartools.RuleReference{Name: "ERROR_CONSTANT", IsToken: true}}},
		},
		{
			Name: "function_call",
			LineNumber: 51,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "function_name", IsToken: false}, grammartools.RuleReference{Name: "LPAREN", IsToken: true}, grammartools.RuleReference{Name: "ws", IsToken: false}, grammartools.Optional{Element: grammartools.RuleReference{Name: "function_argument_list", IsToken: false}}, grammartools.RuleReference{Name: "ws", IsToken: false}, grammartools.RuleReference{Name: "RPAREN", IsToken: true}}},
		},
		{
			Name: "function_name",
			LineNumber: 52,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "FUNCTION_NAME", IsToken: true}, grammartools.RuleReference{Name: "NAME", IsToken: true}}},
		},
		{
			Name: "function_argument_list",
			LineNumber: 53,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "function_argument", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "ws", IsToken: false}, grammartools.RuleReference{Name: "COMMA", IsToken: true}, grammartools.RuleReference{Name: "ws", IsToken: false}, grammartools.RuleReference{Name: "function_argument", IsToken: false}}}}, grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "ws", IsToken: false}, grammartools.RuleReference{Name: "COMMA", IsToken: true}}}}}},
		},
		{
			Name: "function_argument",
			LineNumber: 54,
			Body: grammartools.Optional{Element: grammartools.RuleReference{Name: "expression", IsToken: false}},
		},
		{
			Name: "reference_expression",
			LineNumber: 56,
			Body: grammartools.RuleReference{Name: "union_reference", IsToken: false},
		},
		{
			Name: "union_reference",
			LineNumber: 57,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "intersection_reference", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "ws", IsToken: false}, grammartools.RuleReference{Name: "COMMA", IsToken: true}, grammartools.RuleReference{Name: "ws", IsToken: false}, grammartools.RuleReference{Name: "intersection_reference", IsToken: false}}}}}},
		},
		{
			Name: "intersection_reference",
			LineNumber: 58,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "range_reference", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "req_space", IsToken: false}, grammartools.RuleReference{Name: "range_reference", IsToken: false}}}}}},
		},
		{
			Name: "range_reference",
			LineNumber: 59,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "reference_primary", IsToken: false}, grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "ws", IsToken: false}, grammartools.RuleReference{Name: "COLON", IsToken: true}, grammartools.RuleReference{Name: "ws", IsToken: false}, grammartools.RuleReference{Name: "reference_primary", IsToken: false}}}}}},
		},
		{
			Name: "reference_primary",
			LineNumber: 61,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "parenthesized_reference", IsToken: false}, grammartools.RuleReference{Name: "prefixed_reference", IsToken: false}, grammartools.RuleReference{Name: "external_reference", IsToken: false}, grammartools.RuleReference{Name: "structure_reference", IsToken: false}, grammartools.RuleReference{Name: "a1_reference", IsToken: false}, grammartools.RuleReference{Name: "bang_reference", IsToken: false}, grammartools.RuleReference{Name: "bang_name", IsToken: false}, grammartools.RuleReference{Name: "name_reference", IsToken: false}}},
		},
		{
			Name: "parenthesized_reference",
			LineNumber: 70,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "LPAREN", IsToken: true}, grammartools.RuleReference{Name: "ws", IsToken: false}, grammartools.RuleReference{Name: "reference_expression", IsToken: false}, grammartools.RuleReference{Name: "ws", IsToken: false}, grammartools.RuleReference{Name: "RPAREN", IsToken: true}}},
		},
		{
			Name: "prefixed_reference",
			LineNumber: 71,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "REF_PREFIX", IsToken: true}, grammartools.Group{Element: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "a1_reference", IsToken: false}, grammartools.RuleReference{Name: "name_reference", IsToken: false}, grammartools.RuleReference{Name: "structure_reference", IsToken: false}}}}}},
		},
		{
			Name: "external_reference",
			LineNumber: 72,
			Body: grammartools.RuleReference{Name: "REF_PREFIX", IsToken: true},
		},
		{
			Name: "bang_reference",
			LineNumber: 73,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "BANG", IsToken: true}, grammartools.Group{Element: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "CELL", IsToken: true}, grammartools.RuleReference{Name: "COLUMN_REF", IsToken: true}, grammartools.RuleReference{Name: "ROW_REF", IsToken: true}, grammartools.RuleReference{Name: "NUMBER", IsToken: true}}}}}},
		},
		{
			Name: "bang_name",
			LineNumber: 74,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "BANG", IsToken: true}, grammartools.RuleReference{Name: "name_reference", IsToken: false}}},
		},
		{
			Name: "name_reference",
			LineNumber: 75,
			Body: grammartools.RuleReference{Name: "NAME", IsToken: true},
		},
		{
			Name: "column_reference",
			LineNumber: 77,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Optional{Element: grammartools.RuleReference{Name: "DOLLAR", IsToken: true}}, grammartools.Group{Element: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "COLUMN_REF", IsToken: true}, grammartools.RuleReference{Name: "NAME", IsToken: true}}}}}},
		},
		{
			Name: "row_reference",
			LineNumber: 78,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Optional{Element: grammartools.RuleReference{Name: "DOLLAR", IsToken: true}}, grammartools.Group{Element: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "ROW_REF", IsToken: true}, grammartools.RuleReference{Name: "NUMBER", IsToken: true}}}}}},
		},
		{
			Name: "a1_reference",
			LineNumber: 80,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "CELL", IsToken: true}, grammartools.RuleReference{Name: "column_reference", IsToken: false}, grammartools.RuleReference{Name: "row_reference", IsToken: false}, grammartools.RuleReference{Name: "COLUMN_REF", IsToken: true}, grammartools.RuleReference{Name: "ROW_REF", IsToken: true}, grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.RuleReference{Name: "NUMBER", IsToken: true}}},
		},
		{
			Name: "structure_reference",
			LineNumber: 82,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Optional{Element: grammartools.RuleReference{Name: "table_name", IsToken: false}}, grammartools.RuleReference{Name: "intra_table_reference", IsToken: false}}},
		},
		{
			Name: "table_name",
			LineNumber: 83,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "TABLE_NAME", IsToken: true}, grammartools.RuleReference{Name: "NAME", IsToken: true}}},
		},
		{
			Name: "intra_table_reference",
			LineNumber: 84,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "STRUCTURED_KEYWORD", IsToken: true}, grammartools.RuleReference{Name: "structured_column_range", IsToken: false}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "LBRACKET", IsToken: true}, grammartools.RuleReference{Name: "ws", IsToken: false}, grammartools.Optional{Element: grammartools.RuleReference{Name: "inner_structure_reference", IsToken: false}}, grammartools.RuleReference{Name: "ws", IsToken: false}, grammartools.RuleReference{Name: "RBRACKET", IsToken: true}}}}},
		},
		{
			Name: "inner_structure_reference",
			LineNumber: 87,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "structured_keyword_list", IsToken: false}, grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "ws", IsToken: false}, grammartools.RuleReference{Name: "COMMA", IsToken: true}, grammartools.RuleReference{Name: "ws", IsToken: false}, grammartools.RuleReference{Name: "structured_column_range", IsToken: false}}}}}}, grammartools.RuleReference{Name: "structured_column_range", IsToken: false}}},
		},
		{
			Name: "structured_keyword_list",
			LineNumber: 89,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "STRUCTURED_KEYWORD", IsToken: true}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "ws", IsToken: false}, grammartools.RuleReference{Name: "COMMA", IsToken: true}, grammartools.RuleReference{Name: "ws", IsToken: false}, grammartools.RuleReference{Name: "STRUCTURED_KEYWORD", IsToken: true}}}}}},
		},
		{
			Name: "structured_column_range",
			LineNumber: 90,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "structured_column", IsToken: false}, grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "ws", IsToken: false}, grammartools.RuleReference{Name: "COLON", IsToken: true}, grammartools.RuleReference{Name: "ws", IsToken: false}, grammartools.RuleReference{Name: "structured_column", IsToken: false}}}}}},
		},
		{
			Name: "structured_column",
			LineNumber: 91,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "STRUCTURED_COLUMN", IsToken: true}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "AT", IsToken: true}, grammartools.RuleReference{Name: "STRUCTURED_COLUMN", IsToken: true}}}}},
		},
	},
}
