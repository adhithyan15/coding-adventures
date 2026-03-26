// AUTO-GENERATED FILE - DO NOT EDIT
package typescriptparser

import (
	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
)

var TypescriptGrammar = &grammartools.ParserGrammar{
	Version: 1,
	Rules: []grammartools.GrammarRule{
		{
			Name: "program",
			LineNumber: 29,
			Body: grammartools.Repetition{Element: grammartools.RuleReference{Name: "statement", IsToken: false}},
		},
		{
			Name: "statement",
			LineNumber: 30,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "var_declaration", IsToken: false}, grammartools.RuleReference{Name: "assignment", IsToken: false}, grammartools.RuleReference{Name: "expression_stmt", IsToken: false}}},
		},
		{
			Name: "var_declaration",
			LineNumber: 31,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "KEYWORD", IsToken: true}, grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.RuleReference{Name: "EQUALS", IsToken: true}, grammartools.RuleReference{Name: "expression", IsToken: false}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}}},
		},
		{
			Name: "assignment",
			LineNumber: 32,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.RuleReference{Name: "EQUALS", IsToken: true}, grammartools.RuleReference{Name: "expression", IsToken: false}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}}},
		},
		{
			Name: "expression_stmt",
			LineNumber: 33,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "expression", IsToken: false}, grammartools.RuleReference{Name: "SEMICOLON", IsToken: true}}},
		},
		{
			Name: "expression",
			LineNumber: 34,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "term", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Group{Element: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "PLUS", IsToken: true}, grammartools.RuleReference{Name: "MINUS", IsToken: true}}}}, grammartools.RuleReference{Name: "term", IsToken: false}}}}}},
		},
		{
			Name: "term",
			LineNumber: 35,
			Body: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "factor", IsToken: false}, grammartools.Repetition{Element: grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.Group{Element: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "STAR", IsToken: true}, grammartools.RuleReference{Name: "SLASH", IsToken: true}}}}, grammartools.RuleReference{Name: "factor", IsToken: false}}}}}},
		},
		{
			Name: "factor",
			LineNumber: 36,
			Body: grammartools.Alternation{Choices: []grammartools.GrammarElement{grammartools.RuleReference{Name: "NUMBER", IsToken: true}, grammartools.RuleReference{Name: "STRING", IsToken: true}, grammartools.RuleReference{Name: "NAME", IsToken: true}, grammartools.RuleReference{Name: "KEYWORD", IsToken: true}, grammartools.Sequence{Elements: []grammartools.GrammarElement{grammartools.RuleReference{Name: "LPAREN", IsToken: true}, grammartools.RuleReference{Name: "expression", IsToken: false}, grammartools.RuleReference{Name: "RPAREN", IsToken: true}}}}},
		},
	},
}
