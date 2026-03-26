// AUTO-GENERATED FILE - DO NOT EDIT
package tomlparser

import (
	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
)

var TomlGrammar = &grammartools.ParserGrammar{
	Version: 1,
	Rules: []grammartools.GrammarRule{
		{
			Name: "document",
			LineNumber: 38,
			Body: grammartools.Repetition{Element: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "NEWLINE", IsToken: true}, grammartools.RuleReference{Name: "expression", IsToken: false}}}},
		},
		{
			Name: "expression",
			LineNumber: 49,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "array_table_header", IsToken: false}, grammartools.RuleReference{Name: "table_header", IsToken: false}, grammartools.RuleReference{Name: "keyval", IsToken: false}}},
		},
		{
			Name: "keyval",
			LineNumber: 57,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "key", IsToken: false}, grammartools.RuleReference{Name: "EQUALS", IsToken: true}, grammartools.RuleReference{Name: "value", IsToken: false}}},
		},
		{
			Name: "key",
			LineNumber: 65,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "simple_key", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "DOT", IsToken: true}, grammartools.RuleReference{Name: "simple_key", IsToken: false}}}}}},
		},
		{
			Name: "simple_key",
			LineNumber: 82,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "BARE_KEY", IsToken: true}, grammartools.RuleReference{Name: "BASIC_STRING", IsToken: true}, grammartools.RuleReference{Name: "LITERAL_STRING", IsToken: true}, grammartools.RuleReference{Name: "TRUE", IsToken: true}, grammartools.RuleReference{Name: "FALSE", IsToken: true}, grammartools.RuleReference{Name: "INTEGER", IsToken: true}, grammartools.RuleReference{Name: "FLOAT", IsToken: true}, grammartools.RuleReference{Name: "OFFSET_DATETIME", IsToken: true}, grammartools.RuleReference{Name: "LOCAL_DATETIME", IsToken: true}, grammartools.RuleReference{Name: "LOCAL_DATE", IsToken: true}, grammartools.RuleReference{Name: "LOCAL_TIME", IsToken: true}}},
		},
		{
			Name: "table_header",
			LineNumber: 92,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "LBRACKET", IsToken: true}, grammartools.RuleReference{Name: "key", IsToken: false}, grammartools.RuleReference{Name: "RBRACKET", IsToken: true}}},
		},
		{
			Name: "array_table_header",
			LineNumber: 104,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "LBRACKET", IsToken: true}, grammartools.RuleReference{Name: "LBRACKET", IsToken: true}, grammartools.RuleReference{Name: "key", IsToken: false}, grammartools.RuleReference{Name: "RBRACKET", IsToken: true}, grammartools.RuleReference{Name: "RBRACKET", IsToken: true}}},
		},
		{
			Name: "value",
			LineNumber: 121,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "BASIC_STRING", IsToken: true}, grammartools.RuleReference{Name: "ML_BASIC_STRING", IsToken: true}, grammartools.RuleReference{Name: "LITERAL_STRING", IsToken: true}, grammartools.RuleReference{Name: "ML_LITERAL_STRING", IsToken: true}, grammartools.RuleReference{Name: "INTEGER", IsToken: true}, grammartools.RuleReference{Name: "FLOAT", IsToken: true}, grammartools.RuleReference{Name: "TRUE", IsToken: true}, grammartools.RuleReference{Name: "FALSE", IsToken: true}, grammartools.RuleReference{Name: "OFFSET_DATETIME", IsToken: true}, grammartools.RuleReference{Name: "LOCAL_DATETIME", IsToken: true}, grammartools.RuleReference{Name: "LOCAL_DATE", IsToken: true}, grammartools.RuleReference{Name: "LOCAL_TIME", IsToken: true}, grammartools.RuleReference{Name: "array", IsToken: false}, grammartools.RuleReference{Name: "inline_table", IsToken: false}}},
		},
		{
			Name: "array",
			LineNumber: 140,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "LBRACKET", IsToken: true}, grammartools.RuleReference{Name: "array_values", IsToken: false}, grammartools.RuleReference{Name: "RBRACKET", IsToken: true}}},
		},
		{
			Name: "array_values",
			LineNumber: 142,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Repetition{Element: grammartools.RuleReference{Name: "NEWLINE", IsToken: true}}, grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "value", IsToken: false}, grammartools.Repetition{Element: grammartools.RuleReference{Name: "NEWLINE", IsToken: true}}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "COMMA", IsToken: true}, grammartools.Repetition{Element: grammartools.RuleReference{Name: "NEWLINE", IsToken: true}}, grammartools.RuleReference{Name: "value", IsToken: false}, grammartools.Repetition{Element: grammartools.RuleReference{Name: "NEWLINE", IsToken: true}}}}}, grammartools.Optional{Element: grammartools.RuleReference{Name: "COMMA", IsToken: true}}, grammartools.Repetition{Element: grammartools.RuleReference{Name: "NEWLINE", IsToken: true}}}}}}},
		},
		{
			Name: "inline_table",
			LineNumber: 162,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "LBRACE", IsToken: true}, grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "keyval", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "COMMA", IsToken: true}, grammartools.RuleReference{Name: "keyval", IsToken: false}}}}}}}, grammartools.RuleReference{Name: "RBRACE", IsToken: true}}},
		},
	},
}
