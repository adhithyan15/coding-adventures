package parser

// Adversarial generators for the left-recursion knob differential, adapted
// from the review's generator: grammars that mention NEWLINE, EOF tokens,
// empty literals, undefined rules, tokens named like rules and duplicate rule
// names; token streams with no trailing EOF, EOF mid-stream, empty streams,
// enum-only tokens, tokens whose TypeName disagrees with the enum, and
// real tokens with empty text.

import (
	"fmt"
	"math/rand"

	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

var advVocab = []string{"NUM", "X", "Y", "NEWLINE", "EOF", "NUM", "X"}
var advVals = map[string]string{"NUM": "1", "PLUS": "+", "X": "x", "Y": "y", "Z": "z", "LP": "(", "RP": ")", "NEWLINE": "\n", "EOF": "", "NAME": "n", "COMMA": ","}

func advGrammar(rng *rand.Rand, nrules int, allowNL bool) *grammartools.ParserGrammar {
	rn := func(i int) string { return fmt.Sprintf("r%d", i) }
	tokName := func() string {
		for {
			v := advVocab[rng.Intn(len(advVocab))]
			if v == "NEWLINE" && !allowNL {
				continue
			}
			return v
		}
	}
	var gen func(d int) grammartools.GrammarElement
	gen = func(d int) grammartools.GrammarElement {
		k := 16
		if d <= 0 {
			k = 6
		}
		switch rng.Intn(k) {
		case 0, 1, 2:
			return grammartools.RuleReference{Name: rn(rng.Intn(nrules))}
		case 3, 4:
			return grammartools.RuleReference{Name: tokName(), IsToken: true}
		case 5:
			switch rng.Intn(6) {
			case 0:
				return grammartools.Literal{Value: ""}
			case 1:
				return grammartools.RuleReference{Name: "nope"}
			case 2:
				return grammartools.RuleReference{Name: rn(rng.Intn(nrules)), IsToken: true} // token named like a rule
			case 3:
				return grammartools.Literal{Value: "NUM"}
			}
			return grammartools.Literal{Value: advVals[tokName()]}
		case 6, 7, 8:
			n := 1 + rng.Intn(3)
			els := make([]grammartools.GrammarElement, n)
			for i := range els {
				els[i] = gen(d - 1)
			}
			return grammartools.Sequence{Elements: els}
		case 9, 10:
			n := 2 + rng.Intn(2)
			els := make([]grammartools.GrammarElement, n)
			for i := range els {
				els[i] = gen(d - 1)
			}
			return grammartools.Alternation{Choices: els}
		case 11:
			return grammartools.Optional{Element: gen(d - 1)}
		case 12:
			return grammartools.Repetition{Element: gen(d - 1)}
		case 13:
			if rng.Intn(2) == 0 {
				return grammartools.PositiveLookahead{Element: gen(d - 1)}
			}
			return grammartools.NegativeLookahead{Element: gen(d - 1)}
		case 14:
			if rng.Intn(2) == 0 {
				return grammartools.Group{Element: gen(d - 1)}
			}
			return grammartools.OneOrMoreRepetition{Element: gen(d - 1)}
		default:
			return grammartools.SeparatedRepetition{Element: gen(d - 1), Separator: gen(d - 1), AtLeastOne: rng.Intn(2) == 0}
		}
	}
	pg := &grammartools.ParserGrammar{}
	for i := 0; i < nrules; i++ {
		pg.Rules = append(pg.Rules, grammartools.GrammarRule{Name: rn(i), Body: gen(1 + rng.Intn(4))})
	}
	if rng.Intn(8) == 0 { // duplicate rule name
		pg.Rules = append(pg.Rules, grammartools.GrammarRule{Name: rn(0), Body: gen(2)})
	}
	return pg
}

var advEnum = []lexer.TokenType{lexer.TokenName, lexer.TokenNumber, lexer.TokenPlus, lexer.TokenLParen, lexer.TokenRParen, lexer.TokenComma, lexer.TokenNewline, lexer.TokenEOF}

func advTokens(rng *rand.Rand) []lexer.Token {
	n := rng.Intn(12)
	var toks []lexer.Token
	for i := 0; i < n; i++ {
		v := advVocab[rng.Intn(len(advVocab))]
		if (v == "EOF" || v == "NEWLINE") && rng.Intn(3) != 0 {
			v = "NUM"
		}
		t := lexer.Token{TypeName: v, Value: advVals[v], Line: 1, Column: i + 1}
		switch rng.Intn(40) {
		case 0: // enum-only token
			t = lexer.Token{Type: advEnum[rng.Intn(len(advEnum))], Value: advVals[v], Line: 1, Column: i + 1}
		case 1: // TypeName disagrees with enum
			t.Type = advEnum[rng.Intn(len(advEnum))]
		case 2: // empty value real token
			t.Value = ""
		case 3:
			t.TypeName = fmt.Sprintf("r%d", rng.Intn(4))
		}
		toks = append(toks, t)
	}
	if rng.Intn(4) != 0 {
		toks = append(toks, lexer.Token{TypeName: "EOF", Line: 1, Column: n + 1})
	}
	return toks
}
