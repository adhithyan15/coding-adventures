// AUTO-GENERATED FILE - DO NOT EDIT
package rubyparser

import (
	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
)

var RubyGrammar = &grammartools.ParserGrammar{
	Version: 1,
	Rules: []grammartools.GrammarRule{
		{
			Name: "program",
			LineNumber: 22,
			Body: grammartools.Repetition{Element: grammartools.RuleReference{Name: "statement", IsToken: false}},
		},
		{
			Name: "statement",
			LineNumber: 23,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "assignment", IsToken: false}, grammartools.RuleReference{Name: "method_call", IsToken: false}, grammartools.RuleReference{Name: "expression_stmt", IsToken: false}}},
		},
		{
			Name: "assignment",
			LineNumber: 24,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.RuleReference{Name: "EQUALS", IsToken: true}, grammartools.RuleReference{Name: "expression", IsToken: false}}},
		},
		{
			Name: "method_call",
			LineNumber: 25,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Group{Element: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.RuleReference{Name: "KEYWORD", IsToken: true}}}}, grammartools.RuleReference{Name: "LPAREN", IsToken: true}, grammartools.Optional{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "expression", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "COMMA", IsToken: true}, grammartools.RuleReference{Name: "expression", IsToken: false}}}}}}}, grammartools.RuleReference{Name: "RPAREN", IsToken: true}}},
		},
		{
			Name: "expression_stmt",
			LineNumber: 26,
			Body: grammartools.RuleReference{Name: "expression", IsToken: false},
		},
		{
			Name: "expression",
			LineNumber: 27,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "term", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Group{Element: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "PLUS", IsToken: true}, grammartools.RuleReference{Name: "MINUS", IsToken: true}}}}, grammartools.RuleReference{Name: "term", IsToken: false}}}}}},
		},
		{
			Name: "term",
			LineNumber: 28,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "factor", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Group{Element: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "STAR", IsToken: true}, grammartools.RuleReference{Name: "SLASH", IsToken: true}}}}, grammartools.RuleReference{Name: "factor", IsToken: false}}}}}},
		},
		{
			Name: "factor",
			LineNumber: 29,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "NUMBER", IsToken: true}, grammartools.RuleReference{Name: "STRING", IsToken: true}, grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.RuleReference{Name: "KEYWORD", IsToken: true}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "LPAREN", IsToken: true}, grammartools.RuleReference{Name: "expression", IsToken: false}, grammartools.RuleReference{Name: "RPAREN", IsToken: true}}}}},
		},
	},
}
