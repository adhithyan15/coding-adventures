package parser

// Benchmarks on REAL grammars (code/grammars/json and typescript/ts5.8) over
// real source text, so the numbers are not a synthetic grammar's. Lexing is
// done once, outside the timed loop: only Parse is measured. SetBytes is the
// source size, so `MB/s` is source megabytes parsed per second.

import (
	"fmt"
	"strings"
	"testing"
)

func benchReal(b *testing.B, rg *realGrammar, src string) {
	toks := rg.tok(src)
	if _, err := rg.c.Parse(toks, Options{}); err != nil {
		b.Fatal(err)
	}
	b.SetBytes(int64(len(src)))
	b.ReportAllocs()
	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		if _, err := rg.c.Parse(toks, Options{}); err != nil {
			b.Fatal(err)
		}
	}
}

func BenchmarkParseJSONReal(b *testing.B) {
	var sb strings.Builder
	sb.WriteString("[")
	for i := 0; i < 400; i++ {
		if i > 0 {
			sb.WriteString(", ")
		}
		fmt.Fprintf(&sb, `{"id": %d, "name": "item-%d", "tags": ["a", "b", "c"], "pos": {"x": %d.5, "y": -%d}, "ok": true, "n": null}`, i, i, i, i)
	}
	sb.WriteString("]")
	benchReal(b, loadReal(b, "json", "json"), sb.String())
}

func BenchmarkParseTypeScriptReal(b *testing.B) {
	names, srcs := goldenSources(b, "ts_0")
	var sb strings.Builder
	for i, n := range names {
		if strings.Contains(n, "error") || strings.Contains(n, "unclosed") || strings.Contains(n, "garbage") {
			continue
		}
		for k := 0; k < 4; k++ {
			sb.WriteString(srcs[i])
			sb.WriteString("\n")
		}
	}
	benchReal(b, loadReal(b, "typescript", "ts5.8"), sb.String())
}
