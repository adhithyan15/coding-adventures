// AUTO-GENERATED FILE - DO NOT EDIT
package pythonparser

import (
	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
)

var PythonGrammar = &grammartools.ParserGrammar{
	Version: 1,
	Rules: []grammartools.GrammarRule{
		{
			Name: "program",
			LineNumber: 17,
			Body: grammartools.Repetition{Element: grammartools.RuleReference{Name: "statement", IsToken: false}},
		},
		{
			Name: "statement",
			LineNumber: 18,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "assignment", IsToken: false}, grammartools.RuleReference{Name: "expression_stmt", IsToken: false}}},
		},
		{
			Name: "assignment",
			LineNumber: 19,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.RuleReference{Name: "EQUALS", IsToken: true}, grammartools.RuleReference{Name: "expression", IsToken: false}}},
		},
		{
			Name: "expression_stmt",
			LineNumber: 20,
			Body: grammartools.RuleReference{Name: "expression", IsToken: false},
		},
		{
			Name: "expression",
			LineNumber: 21,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "term", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Group{Element: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "PLUS", IsToken: true}, grammartools.RuleReference{Name: "MINUS", IsToken: true}}}}, grammartools.RuleReference{Name: "term", IsToken: false}}}}}},
		},
		{
			Name: "term",
			LineNumber: 22,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "factor", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Group{Element: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "STAR", IsToken: true}, grammartools.RuleReference{Name: "SLASH", IsToken: true}}}}, grammartools.RuleReference{Name: "factor", IsToken: false}}}}}},
		},
		{
			Name: "factor",
			LineNumber: 23,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "NUMBER", IsToken: true}, grammartools.RuleReference{Name: "STRING", IsToken: true}, grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "LPAREN", IsToken: true}, grammartools.RuleReference{Name: "expression", IsToken: false}, grammartools.RuleReference{Name: "RPAREN", IsToken: true}}}}},
		},
	},
}
