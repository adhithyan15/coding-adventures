package lexer

// Benchmarks
// ==========
//
// Each workload is measured twice: through the legacy NewGrammarLexer path and
// through Compile/Compiled.Tokenize. The legacy numbers are the "before"; the
// compiled numbers are the "after".
//
//	go test -run xxx -bench Tokenize -benchtime 2s
//
// "PerLine" tokenizes the strace corpus one line at a time, the way a trace
// parser consumes it. "Whole" tokenizes the same text as one newline-joined
// document. Throughput is reported as MB/s by SetBytes.

import (
	"testing"
)

func benchLegacy(b *testing.B, tokens string, inputs []string) {
	g := mustGrammar(b, tokens)
	b.SetBytes(totalBytes(inputs))
	b.ReportAllocs()
	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		for _, s := range inputs {
			NewGrammarLexer(s, g).Tokenize()
		}
	}
}

func benchCompiled(b *testing.B, tokens string, inputs []string) {
	c, err := Compile(mustGrammar(b, tokens))
	if err != nil {
		b.Fatal(err)
	}
	b.SetBytes(totalBytes(inputs))
	b.ReportAllocs()
	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		for _, s := range inputs {
			if _, err := c.Tokenize(s); err != nil {
				b.Fatal(err)
			}
		}
	}
}

func totalBytes(inputs []string) int64 {
	total := 0
	for _, s := range inputs {
		total += len(s)
	}
	return int64(total)
}

func BenchmarkTokenizeLegacyStraceWhole(b *testing.B) {
	benchLegacy(b, straceLikeTokens, []string{straceLikeInput(200)})
}

func BenchmarkTokenizeLegacyStracePerLine(b *testing.B) {
	benchLegacy(b, straceLikeTokens, straceLikeLines)
}

func BenchmarkTokenizeLegacyJSON(b *testing.B) {
	benchLegacy(b, jsonLikeTokens, []string{jsonDocument(200)})
}

func BenchmarkTokenizeCompiledStraceWhole(b *testing.B) {
	benchCompiled(b, straceLikeTokens, []string{straceLikeInput(200)})
}

func BenchmarkTokenizeCompiledStracePerLine(b *testing.B) {
	benchCompiled(b, straceLikeTokens, straceLikeLines)
}

func BenchmarkTokenizeCompiledJSON(b *testing.B) {
	benchCompiled(b, jsonLikeTokens, []string{jsonDocument(200)})
}

// The tiny-input pair measures constructor plus Tokenize on a very short
// source, where fixed per-call cost dominates. NewGrammarLexer reuses a
// cached compilation; Compiled.Tokenize is the reference.
func BenchmarkConstructTinyInputLegacy(b *testing.B) {
	g := mustGrammar(b, jsonLikeTokens)
	b.ReportAllocs()
	for i := 0; i < b.N; i++ {
		NewGrammarLexer(`{"a": 1}`, g).Tokenize()
	}
}

func BenchmarkConstructTinyInputCompiled(b *testing.B) {
	c, err := Compile(mustGrammar(b, jsonLikeTokens))
	if err != nil {
		b.Fatal(err)
	}
	b.ReportAllocs()
	for i := 0; i < b.N; i++ {
		if _, err := c.Tokenize(`{"a": 1}`); err != nil {
			b.Fatal(err)
		}
	}
}

// BenchmarkCompile measures the one-time cost the legacy path paid on every
// construction.
func BenchmarkCompile(b *testing.B) {
	g := mustGrammar(b, straceLikeTokens)
	b.ReportAllocs()
	for i := 0; i < b.N; i++ {
		if _, err := Compile(g); err != nil {
			b.Fatal(err)
		}
	}
}
