package parser

// Benchmarks for the grammar-driven parser.
//
// Three workloads are measured, because they stress different parts of the
// engine:
//
//   - BenchmarkParseJSONLike: a nested structure (objects, arrays, scalars).
//     Few rules per token, but many tokens: dominated by sequence/repetition
//     bookkeeping and AST allocation.
//
//   - BenchmarkParseExpression: an arithmetic expression grammar with a
//     six-level precedence chain. Every atom is reached through six rules at
//     the same token position, which is exactly the shape packrat memoization
//     is for. Dominated by memo lookups and inserts.
//
//   - BenchmarkParseSmallLine: the same grammar on a tiny token list, the
//     "parse one log line, thousands of times" shape. Dominated by per-parse
//     setup cost.
//
// SetBytes uses the total length of the token text (plus one separator per
// token), a stand-in for source size, so MB/s is comparable across runs.

import (
	"strings"
	"testing"

	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

const jsonLikeGrammar = `
value = object | array | STRING | NUMBER | "true" | "false" | "null" ;
object = LBRACE [ pair { COMMA pair } ] RBRACE ;
pair = STRING COLON value ;
array = LBRACKET [ value { COMMA value } ] RBRACKET ;
`

const exprGrammar = `
expression = comparison ;
comparison = additive { ( "<" | ">" ) additive } ;
additive = term { ( PLUS | MINUS ) term } ;
term = unary { ( STAR | SLASH ) unary } ;
unary = [ MINUS ] power ;
power = atom [ "^" unary ] ;
atom = NUMBER | NAME | LPAREN expression RPAREN ;
`

func benchTok(typeName, value string) lexer.Token {
	return lexer.Token{TypeName: typeName, Value: value, Line: 1, Column: 1}
}

// jsonLikeTokens builds `[ {"k0": 1, "k1": [1, 2, 3], "k2": "s"}, ... ]`.
func jsonLikeTokens(records int) []lexer.Token {
	toks := []lexer.Token{benchTok("LBRACKET", "[")}
	for i := 0; i < records; i++ {
		if i > 0 {
			toks = append(toks, benchTok("COMMA", ","))
		}
		toks = append(toks,
			benchTok("LBRACE", "{"),
			benchTok("STRING", "k0"), benchTok("COLON", ":"), benchTok("NUMBER", "1"),
			benchTok("COMMA", ","),
			benchTok("STRING", "k1"), benchTok("COLON", ":"),
			benchTok("LBRACKET", "["), benchTok("NUMBER", "1"), benchTok("COMMA", ","),
			benchTok("NUMBER", "2"), benchTok("COMMA", ","), benchTok("NUMBER", "3"),
			benchTok("RBRACKET", "]"),
			benchTok("COMMA", ","),
			benchTok("STRING", "k2"), benchTok("COLON", ":"), benchTok("STRING", "s"),
			benchTok("RBRACE", "}"),
		)
	}
	return append(toks, benchTok("RBRACKET", "]"), benchTok("EOF", ""))
}

// exprTokens builds `(a + 1) * b - c / (2 + d) + ...` repeated.
func exprTokens(repeats int) []lexer.Token {
	var toks []lexer.Token
	for i := 0; i < repeats; i++ {
		if i > 0 {
			toks = append(toks, benchTok("PLUS", "+"))
		}
		toks = append(toks,
			benchTok("LPAREN", "("), benchTok("NAME", "a"), benchTok("PLUS", "+"),
			benchTok("NUMBER", "1"), benchTok("RPAREN", ")"),
			benchTok("STAR", "*"), benchTok("NAME", "b"), benchTok("MINUS", "-"),
			benchTok("NAME", "c"), benchTok("SLASH", "/"),
			benchTok("LPAREN", "("), benchTok("NUMBER", "2"), benchTok("PLUS", "+"),
			benchTok("NAME", "d"), benchTok("RPAREN", ")"),
		)
	}
	return append(toks, benchTok("EOF", ""))
}

func benchBytes(toks []lexer.Token) int64 {
	n := 0
	for _, t := range toks {
		n += len(t.Value) + 1
	}
	return int64(n)
}

func mustGrammar(tb testing.TB, src string) *grammartools.ParserGrammar {
	tb.Helper()
	g, err := grammartools.ParseParserGrammar(strings.TrimSpace(src))
	if err != nil {
		tb.Fatalf("grammar: %v", err)
	}
	return g
}

func benchmarkParse(b *testing.B, src string, toks []lexer.Token) {
	g := mustGrammar(b, src)
	b.SetBytes(benchBytes(toks))
	b.ReportAllocs()
	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		if _, err := NewGrammarParser(toks, g).Parse(); err != nil {
			b.Fatal(err)
		}
	}
}

func BenchmarkParseJSONLike(b *testing.B) { benchmarkParse(b, jsonLikeGrammar, jsonLikeTokens(200)) }

func BenchmarkParseExpression(b *testing.B) { benchmarkParse(b, exprGrammar, exprTokens(200)) }

func BenchmarkParseSmallLine(b *testing.B) { benchmarkParse(b, exprGrammar, exprTokens(2)) }
