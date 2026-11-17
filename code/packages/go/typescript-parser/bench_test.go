package typescriptparser

// BenchmarkParseSmallInputLargeGrammar measures the per-parse overhead of the
// grammar engine on a tiny input against a real 180+ rule grammar (ts5.8).
// Tokenising happens once, outside the loop, so only the parse (including
// whatever per-parser setup NewGrammarParser does) is timed.

import (
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
	typescriptlexer "github.com/adhithyan15/coding-adventures/code/packages/go/typescript-lexer"
)

func BenchmarkParseSmallInputLargeGrammar(b *testing.B) {
	tokens, err := typescriptlexer.TokenizeTypescript("let x = 1 + 2;", "ts5.8")
	if err != nil {
		b.Fatal(err)
	}
	grammar := VersionedParserGrammars["ts5.8"]
	b.ReportAllocs()
	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		if _, err := parser.NewGrammarParser(tokens, grammar).Parse(); err != nil {
			b.Fatal(err)
		}
	}
}
