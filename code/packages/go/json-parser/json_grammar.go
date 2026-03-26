// AUTO-GENERATED FILE - DO NOT EDIT
package jsonparser

import (
	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
)

var JsonGrammar = &grammartools.ParserGrammar{
	Version: 1,
	Rules: []grammartools.GrammarRule{
		{
			Name: "value",
			LineNumber: 28,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "object", IsToken: false}, grammartools.RuleReference{Name: "array", IsToken: false}, grammartools.RuleReference{Name: "STRING", IsToken: true}, grammartools.RuleReference{Name: "NUMBER", IsToken: true}, grammartools.RuleReference{Name: "TRUE", IsToken: true}, grammartools.RuleReference{Name: "FALSE", IsToken: true}, grammartools.RuleReference{Name: "NULL", IsToken: true}}},
		},
		{
			Name: "object",
			LineNumber: 34,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "LBRACE", IsToken: true}, grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "pair", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "COMMA", IsToken: true}, grammartools.RuleReference{Name: "pair", IsToken: false}}}}}}}, grammartools.RuleReference{Name: "RBRACE", IsToken: true}}},
		},
		{
			Name: "pair",
			LineNumber: 38,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "STRING", IsToken: true}, grammartools.RuleReference{Name: "COLON", IsToken: true}, grammartools.RuleReference{Name: "value", IsToken: false}}},
		},
		{
			Name: "array",
			LineNumber: 42,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "LBRACKET", IsToken: true}, grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "value", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "COMMA", IsToken: true}, grammartools.RuleReference{Name: "value", IsToken: false}}}}}}}, grammartools.RuleReference{Name: "RBRACKET", IsToken: true}}},
		},
	},
}
