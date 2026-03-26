// AUTO-GENERATED FILE - DO NOT EDIT
package starlarkparser

import (
	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
)

var StarlarkGrammar = &grammartools.ParserGrammar{
	Version: 1,
	Rules: []grammartools.GrammarRule{
		{
			Name: "file",
			LineNumber: 34,
			Body: grammartools.Repetition{Element: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "NEWLINE", IsToken: true}, grammartools.RuleReference{Name: "statement", IsToken: false}}}},
		},
		{
			Name: "statement",
			LineNumber: 48,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "compound_stmt", IsToken: false}, grammartools.RuleReference{Name: "simple_stmt", IsToken: false}}},
		},
		{
			Name: "simple_stmt",
			LineNumber: 52,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "small_stmt", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}, grammartools.RuleReference{Name: "small_stmt", IsToken: false}}}}, grammartools.RuleReference{Name: "NEWLINE", IsToken: true}}},
		},
		{
			Name: "small_stmt",
			LineNumber: 54,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "return_stmt", IsToken: false}, grammartools.RuleReference{Name: "break_stmt", IsToken: false}, grammartools.RuleReference{Name: "continue_stmt", IsToken: false}, grammartools.RuleReference{Name: "pass_stmt", IsToken: false}, grammartools.RuleReference{Name: "load_stmt", IsToken: false}, grammartools.RuleReference{Name: "assign_stmt", IsToken: false}}},
		},
		{
			Name: "return_stmt",
			LineNumber: 68,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "return"}, grammartools.Optional{Element: grammartools.RuleReference{Name: "expression", IsToken: false}}}},
		},
		{
			Name: "break_stmt",
			LineNumber: 71,
			Body: grammartools.Literal{Value: "break"},
		},
		{
			Name: "continue_stmt",
			LineNumber: 74,
			Body: grammartools.Literal{Value: "continue"},
		},
		{
			Name: "pass_stmt",
			LineNumber: 79,
			Body: grammartools.Literal{Value: "pass"},
		},
		{
			Name: "load_stmt",
			LineNumber: 88,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "load"}, grammartools.RuleReference{Name: "LPAREN", IsToken: true}, grammartools.RuleReference{Name: "STRING", IsToken: true}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "COMMA", IsToken: true}, grammartools.RuleReference{Name: "load_arg", IsToken: false}}}}, grammartools.Optional{Element: grammartools.RuleReference{Name: "COMMA", IsToken: true}}, grammartools.RuleReference{Name: "RPAREN", IsToken: true}}},
		},
		{
			Name: "load_arg",
			LineNumber: 89,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.RuleReference{Name: "EQUALS", IsToken: true}, grammartools.RuleReference{Name: "STRING", IsToken: true}}}, grammartools.RuleReference{Name: "STRING", IsToken: true}}},
		},
		{
			Name: "assign_stmt",
			LineNumber: 110,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "expression_list", IsToken: false}, grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Group{Element: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "assign_op", IsToken: false}, grammartools.RuleReference{Name: "augmented_assign_op", IsToken: false}}}}, grammartools.RuleReference{Name: "expression_list", IsToken: false}}}}}},
		},
		{
			Name: "assign_op",
			LineNumber: 113,
			Body: grammartools.RuleReference{Name: "EQUALS", IsToken: true},
		},
		{
			Name: "augmented_assign_op",
			LineNumber: 115,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "PLUS_EQUALS", IsToken: true}, grammartools.RuleReference{Name: "MINUS_EQUALS", IsToken: true}, grammartools.RuleReference{Name: "STAR_EQUALS", IsToken: true}, grammartools.RuleReference{Name: "SLASH_EQUALS", IsToken: true}, grammartools.RuleReference{Name: "FLOOR_DIV_EQUALS", IsToken: true}, grammartools.RuleReference{Name: "PERCENT_EQUALS", IsToken: true}, grammartools.RuleReference{Name: "AMP_EQUALS", IsToken: true}, grammartools.RuleReference{Name: "PIPE_EQUALS", IsToken: true}, grammartools.RuleReference{Name: "CARET_EQUALS", IsToken: true}, grammartools.RuleReference{Name: "LEFT_SHIFT_EQUALS", IsToken: true}, grammartools.RuleReference{Name: "RIGHT_SHIFT_EQUALS", IsToken: true}, grammartools.RuleReference{Name: "DOUBLE_STAR_EQUALS", IsToken: true}}},
		},
		{
			Name: "compound_stmt",
			LineNumber: 124,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "if_stmt", IsToken: false}, grammartools.RuleReference{Name: "for_stmt", IsToken: false}, grammartools.RuleReference{Name: "def_stmt", IsToken: false}}},
		},
		{
			Name: "if_stmt",
			LineNumber: 136,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "if"}, grammartools.RuleReference{Name: "expression", IsToken: false}, grammartools.RuleReference{Name: "COLON", IsToken: true}, grammartools.RuleReference{Name: "suite", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "elif"}, grammartools.RuleReference{Name: "expression", IsToken: false}, grammartools.RuleReference{Name: "COLON", IsToken: true}, grammartools.RuleReference{Name: "suite", IsToken: false}}}}, grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "else"}, grammartools.RuleReference{Name: "COLON", IsToken: true}, grammartools.RuleReference{Name: "suite", IsToken: false}}}}}},
		},
		{
			Name: "for_stmt",
			LineNumber: 150,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "for"}, grammartools.RuleReference{Name: "loop_vars", IsToken: false}, grammartools.Literal{Value: "in"}, grammartools.RuleReference{Name: "expression", IsToken: false}, grammartools.RuleReference{Name: "COLON", IsToken: true}, grammartools.RuleReference{Name: "suite", IsToken: false}}},
		},
		{
			Name: "loop_vars",
			LineNumber: 156,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "COMMA", IsToken: true}, grammartools.RuleReference{Name: "NAME", IsToken: true}}}}}},
		},
		{
			Name: "def_stmt",
			LineNumber: 166,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "def"}, grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.RuleReference{Name: "LPAREN", IsToken: true}, grammartools.Optional{Element: grammartools.RuleReference{Name: "parameters", IsToken: false}}, grammartools.RuleReference{Name: "RPAREN", IsToken: true}, grammartools.RuleReference{Name: "COLON", IsToken: true}, grammartools.RuleReference{Name: "suite", IsToken: false}}},
		},
		{
			Name: "suite",
			LineNumber: 177,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "simple_stmt", IsToken: false}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "NEWLINE", IsToken: true}, grammartools.RuleReference{Name: "INDENT", IsToken: true}, grammartools.Repetition{Element: grammartools.RuleReference{Name: "statement", IsToken: false}}, grammartools.RuleReference{Name: "DEDENT", IsToken: true}}}}},
		},
		{
			Name: "parameters",
			LineNumber: 198,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "parameter", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "COMMA", IsToken: true}, grammartools.RuleReference{Name: "parameter", IsToken: false}}}}, grammartools.Optional{Element: grammartools.RuleReference{Name: "COMMA", IsToken: true}}}},
		},
		{
			Name: "parameter",
			LineNumber: 200,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "DOUBLE_STAR", IsToken: true}, grammartools.RuleReference{Name: "NAME", IsToken: true}}}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "STAR", IsToken: true}, grammartools.RuleReference{Name: "NAME", IsToken: true}}}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.RuleReference{Name: "EQUALS", IsToken: true}, grammartools.RuleReference{Name: "expression", IsToken: false}}}, grammartools.RuleReference{Name: "NAME", IsToken: true}}},
		},
		{
			Name: "expression_list",
			LineNumber: 234,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "expression", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "COMMA", IsToken: true}, grammartools.RuleReference{Name: "expression", IsToken: false}}}}, grammartools.Optional{Element: grammartools.RuleReference{Name: "COMMA", IsToken: true}}}},
		},
		{
			Name: "expression",
			LineNumber: 239,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "lambda_expr", IsToken: false}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "or_expr", IsToken: false}, grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "if"}, grammartools.RuleReference{Name: "or_expr", IsToken: false}, grammartools.Literal{Value: "else"}, grammartools.RuleReference{Name: "expression", IsToken: false}}}}}}}},
		},
		{
			Name: "lambda_expr",
			LineNumber: 244,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "lambda"}, grammartools.Optional{Element: grammartools.RuleReference{Name: "lambda_params", IsToken: false}}, grammartools.RuleReference{Name: "COLON", IsToken: true}, grammartools.RuleReference{Name: "expression", IsToken: false}}},
		},
		{
			Name: "lambda_params",
			LineNumber: 245,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "lambda_param", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "COMMA", IsToken: true}, grammartools.RuleReference{Name: "lambda_param", IsToken: false}}}}, grammartools.Optional{Element: grammartools.RuleReference{Name: "COMMA", IsToken: true}}}},
		},
		{
			Name: "lambda_param",
			LineNumber: 246,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "EQUALS", IsToken: true}, grammartools.RuleReference{Name: "expression", IsToken: false}}}}}}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "STAR", IsToken: true}, grammartools.RuleReference{Name: "NAME", IsToken: true}}}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "DOUBLE_STAR", IsToken: true}, grammartools.RuleReference{Name: "NAME", IsToken: true}}}}},
		},
		{
			Name: "or_expr",
			LineNumber: 250,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "and_expr", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "or"}, grammartools.RuleReference{Name: "and_expr", IsToken: false}}}}}},
		},
		{
			Name: "and_expr",
			LineNumber: 254,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "not_expr", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "and"}, grammartools.RuleReference{Name: "not_expr", IsToken: false}}}}}},
		},
		{
			Name: "not_expr",
			LineNumber: 258,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "not"}, grammartools.RuleReference{Name: "not_expr", IsToken: false}}}, grammartools.RuleReference{Name: "comparison", IsToken: false}}},
		},
		{
			Name: "comparison",
			LineNumber: 267,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "bitwise_or", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "comp_op", IsToken: false}, grammartools.RuleReference{Name: "bitwise_or", IsToken: false}}}}}},
		},
		{
			Name: "comp_op",
			LineNumber: 269,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "EQUALS_EQUALS", IsToken: true}, grammartools.RuleReference{Name: "NOT_EQUALS", IsToken: true}, grammartools.RuleReference{Name: "LESS_THAN", IsToken: true}, grammartools.RuleReference{Name: "GREATER_THAN", IsToken: true}, grammartools.RuleReference{Name: "LESS_EQUALS", IsToken: true}, grammartools.RuleReference{Name: "GREATER_EQUALS", IsToken: true}, grammartools.Literal{Value: "in"}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "not"}, grammartools.Literal{Value: "in"}}}}},
		},
		{
			Name: "bitwise_or",
			LineNumber: 275,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "bitwise_xor", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "PIPE", IsToken: true}, grammartools.RuleReference{Name: "bitwise_xor", IsToken: false}}}}}},
		},
		{
			Name: "bitwise_xor",
			LineNumber: 276,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "bitwise_and", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "CARET", IsToken: true}, grammartools.RuleReference{Name: "bitwise_and", IsToken: false}}}}}},
		},
		{
			Name: "bitwise_and",
			LineNumber: 277,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "shift", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "AMP", IsToken: true}, grammartools.RuleReference{Name: "shift", IsToken: false}}}}}},
		},
		{
			Name: "shift",
			LineNumber: 280,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "arith", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Group{Element: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "LEFT_SHIFT", IsToken: true}, grammartools.RuleReference{Name: "RIGHT_SHIFT", IsToken: true}}}}, grammartools.RuleReference{Name: "arith", IsToken: false}}}}}},
		},
		{
			Name: "arith",
			LineNumber: 284,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "term", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Group{Element: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "PLUS", IsToken: true}, grammartools.RuleReference{Name: "MINUS", IsToken: true}}}}, grammartools.RuleReference{Name: "term", IsToken: false}}}}}},
		},
		{
			Name: "term",
			LineNumber: 289,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "factor", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Group{Element: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "STAR", IsToken: true}, grammartools.RuleReference{Name: "SLASH", IsToken: true}, grammartools.RuleReference{Name: "FLOOR_DIV", IsToken: true}, grammartools.RuleReference{Name: "PERCENT", IsToken: true}}}}, grammartools.RuleReference{Name: "factor", IsToken: false}}}}}},
		},
		{
			Name: "factor",
			LineNumber: 295,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Group{Element: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "PLUS", IsToken: true}, grammartools.RuleReference{Name: "MINUS", IsToken: true}, grammartools.RuleReference{Name: "TILDE", IsToken: true}}}}, grammartools.RuleReference{Name: "factor", IsToken: false}}}, grammartools.RuleReference{Name: "power", IsToken: false}}},
		},
		{
			Name: "power",
			LineNumber: 303,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "primary", IsToken: false}, grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "DOUBLE_STAR", IsToken: true}, grammartools.RuleReference{Name: "factor", IsToken: false}}}}}},
		},
		{
			Name: "primary",
			LineNumber: 320,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "atom", IsToken: false}, grammartools.Repetition{Element: grammartools.RuleReference{Name: "suffix", IsToken: false}}}},
		},
		{
			Name: "suffix",
			LineNumber: 322,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "DOT", IsToken: true}, grammartools.RuleReference{Name: "NAME", IsToken: true}}}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "LBRACKET", IsToken: true}, grammartools.RuleReference{Name: "subscript", IsToken: false}, grammartools.RuleReference{Name: "RBRACKET", IsToken: true}}}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "LPAREN", IsToken: true}, grammartools.Optional{Element: grammartools.RuleReference{Name: "arguments", IsToken: false}}, grammartools.RuleReference{Name: "RPAREN", IsToken: true}}}}},
		},
		{
			Name: "subscript",
			LineNumber: 334,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "expression", IsToken: false}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Optional{Element: grammartools.RuleReference{Name: "expression", IsToken: false}}, grammartools.RuleReference{Name: "COLON", IsToken: true}, grammartools.Optional{Element: grammartools.RuleReference{Name: "expression", IsToken: false}}, grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "COLON", IsToken: true}, grammartools.Optional{Element: grammartools.RuleReference{Name: "expression", IsToken: false}}}}}}}}},
		},
		{
			Name: "atom",
			LineNumber: 343,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "INT", IsToken: true}, grammartools.RuleReference{Name: "FLOAT", IsToken: true}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "STRING", IsToken: true}, grammartools.Repetition{Element: grammartools.RuleReference{Name: "STRING", IsToken: true}}}}, grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.Literal{Value: "True"}, grammartools.Literal{Value: "False"}, grammartools.Literal{Value: "None"}, grammartools.RuleReference{Name: "list_expr", IsToken: false}, grammartools.RuleReference{Name: "dict_expr", IsToken: false}, grammartools.RuleReference{Name: "paren_expr", IsToken: false}}},
		},
		{
			Name: "list_expr",
			LineNumber: 359,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "LBRACKET", IsToken: true}, grammartools.Optional{Element: grammartools.RuleReference{Name: "list_body", IsToken: false}}, grammartools.RuleReference{Name: "RBRACKET", IsToken: true}}},
		},
		{
			Name: "list_body",
			LineNumber: 361,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "expression", IsToken: false}, grammartools.RuleReference{Name: "comp_clause", IsToken: false}}}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "expression", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "COMMA", IsToken: true}, grammartools.RuleReference{Name: "expression", IsToken: false}}}}, grammartools.Optional{Element: grammartools.RuleReference{Name: "COMMA", IsToken: true}}}}}},
		},
		{
			Name: "dict_expr",
			LineNumber: 367,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "LBRACE", IsToken: true}, grammartools.Optional{Element: grammartools.RuleReference{Name: "dict_body", IsToken: false}}, grammartools.RuleReference{Name: "RBRACE", IsToken: true}}},
		},
		{
			Name: "dict_body",
			LineNumber: 369,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "dict_entry", IsToken: false}, grammartools.RuleReference{Name: "comp_clause", IsToken: false}}}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "dict_entry", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "COMMA", IsToken: true}, grammartools.RuleReference{Name: "dict_entry", IsToken: false}}}}, grammartools.Optional{Element: grammartools.RuleReference{Name: "COMMA", IsToken: true}}}}}},
		},
		{
			Name: "dict_entry",
			LineNumber: 372,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "expression", IsToken: false}, grammartools.RuleReference{Name: "COLON", IsToken: true}, grammartools.RuleReference{Name: "expression", IsToken: false}}},
		},
		{
			Name: "paren_expr",
			LineNumber: 379,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "LPAREN", IsToken: true}, grammartools.Optional{Element: grammartools.RuleReference{Name: "paren_body", IsToken: false}}, grammartools.RuleReference{Name: "RPAREN", IsToken: true}}},
		},
		{
			Name: "paren_body",
			LineNumber: 381,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "expression", IsToken: false}, grammartools.RuleReference{Name: "comp_clause", IsToken: false}}}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "expression", IsToken: false}, grammartools.RuleReference{Name: "COMMA", IsToken: true}, grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "expression", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "COMMA", IsToken: true}, grammartools.RuleReference{Name: "expression", IsToken: false}}}}, grammartools.Optional{Element: grammartools.RuleReference{Name: "COMMA", IsToken: true}}}}}}}, grammartools.RuleReference{Name: "expression", IsToken: false}}},
		},
		{
			Name: "comp_clause",
			LineNumber: 397,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "comp_for", IsToken: false}, grammartools.Repetition{Element: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "comp_for", IsToken: false}, grammartools.RuleReference{Name: "comp_if", IsToken: false}}}}}},
		},
		{
			Name: "comp_for",
			LineNumber: 399,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "for"}, grammartools.RuleReference{Name: "loop_vars", IsToken: false}, grammartools.Literal{Value: "in"}, grammartools.RuleReference{Name: "or_expr", IsToken: false}}},
		},
		{
			Name: "comp_if",
			LineNumber: 401,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Literal{Value: "if"}, grammartools.RuleReference{Name: "or_expr", IsToken: false}}},
		},
		{
			Name: "arguments",
			LineNumber: 420,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "argument", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "COMMA", IsToken: true}, grammartools.RuleReference{Name: "argument", IsToken: false}}}}, grammartools.Optional{Element: grammartools.RuleReference{Name: "COMMA", IsToken: true}}}},
		},
		{
			Name: "argument",
			LineNumber: 422,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "DOUBLE_STAR", IsToken: true}, grammartools.RuleReference{Name: "expression", IsToken: false}}}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "STAR", IsToken: true}, grammartools.RuleReference{Name: "expression", IsToken: false}}}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.RuleReference{Name: "EQUALS", IsToken: true}, grammartools.RuleReference{Name: "expression", IsToken: false}}}, grammartools.RuleReference{Name: "expression", IsToken: false}}},
		},
	},
}
