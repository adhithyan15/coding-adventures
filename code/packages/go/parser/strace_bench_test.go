package parser

// BenchmarkParseStraceLike measures the parser on the real strace grammar
// (testdata/strace.grammar, a committed copy of code/grammars/strace/strace.grammar;
// the strace-parser package imports this one, so the benchmark cannot import
// it back). The token streams are hand-built lines in the shapes strace
// produces: a call with a nested struct and a quoted string, a short call, an
// exit line. It is the "lex once, parse many short lines" workload, and it is
// dominated by tokens, rule nodes and the failure-recording that every
// alternative makes.

import (
	"os"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

func straceLikeLines() ([][]lexer.Token, int64) {
	t := benchTok
	lines := [][]lexer.Token{
		{ // 100  openat(AT_FDCWD</work/repo>, "/etc/ld.so.cache", O_RDONLY|O_CLOEXEC) = 3</etc/ld.so.cache>
			t("PID", "100"), t("SYSCALL", "openat("), t("ANNOTATED", "AT_FDCWD</work/repo>"), t("COMMA", ","),
			t("QUOTED", `"/etc/ld.so.cache"`), t("COMMA", ","), t("WORD", "O_RDONLY|O_CLOEXEC"),
			t("RPAREN", ")"), t("ASSIGN", "="), t("ANNOTATED", "3</etc/ld.so.cache>"), t("EOF", ""),
		},
		{ // 100  newfstatat(AT_FDCWD</work/repo>, "go.mod", {st_mode=S_IFREG|0644, st_size=40, ...}, 0) = 0
			t("PID", "100"), t("SYSCALL", "newfstatat("), t("ANNOTATED", "AT_FDCWD</work/repo>"), t("COMMA", ","),
			t("QUOTED", `"go.mod"`), t("COMMA", ","),
			t("LBRACE", "{"), t("ASSIGN", "st_mode=S_IFREG|0644"), t("COMMA", ","), t("ASSIGN", "st_size=40"),
			t("COMMA", ","), t("PUNCT", "..."), t("RBRACE", "}"), t("COMMA", ","), t("WORD", "0"),
			t("RPAREN", ")"), t("ASSIGN", "="), t("WORD", "0"), t("EOF", ""),
		},
		{ // 100  execve("/usr/bin/sh", ["sh", "-c", "make"], 0x7ffd1c2e3a40 /* 41 vars */) = 0
			t("PID", "100"), t("SYSCALL", "execve("), t("QUOTED", `"/usr/bin/sh"`), t("COMMA", ","),
			t("LBRACKET", "["), t("QUOTED", `"sh"`), t("COMMA", ","), t("QUOTED", `"-c"`), t("COMMA", ","),
			t("QUOTED", `"make"`), t("RBRACKET", "]"), t("COMMA", ","), t("WORD", "0x7ffd1c2e3a40"),
			t("ANNOTATED", "/* 41 vars */"), t("RPAREN", ")"), t("ASSIGN", "="), t("WORD", "0"), t("EOF", ""),
		},
		{ // 100  stat("missing.h", 0x7ffd1c2e3500) = -1 ENOENT (No such file or directory)
			t("PID", "100"), t("SYSCALL", "stat("), t("QUOTED", `"missing.h"`), t("COMMA", ","),
			t("WORD", "0x7ffd1c2e3500"), t("RPAREN", ")"), t("ASSIGN", "="), t("WORD", "-1"),
			t("WORD", "ENOENT"), t("LPAREN", "("), t("WORD", "No"), t("WORD", "such"), t("WORD", "file"),
			t("WORD", "or"), t("WORD", "directory"), t("RPAREN", ")"), t("EOF", ""),
		},
		{ // 100  +++ exited with 0 +++
			t("PID", "100"), t("EXITED_LINE", "+++ exited with 0 +++"), t("EOF", ""),
		},
	}
	var total int64
	for _, l := range lines {
		total += benchBytes(l)
	}
	return lines, total
}

func BenchmarkParseStraceLike(b *testing.B) {
	src, err := os.ReadFile("testdata/strace.grammar")
	if err != nil {
		b.Fatal(err)
	}
	c, err := CompileGrammar(mustGrammar(b, string(src)))
	if err != nil {
		b.Fatal(err)
	}
	lines, total := straceLikeLines()
	for _, l := range lines { // every line must parse, or the numbers mean nothing
		if _, err := c.Parse(l, Options{}); err != nil {
			b.Fatalf("%v: %v", l, err)
		}
	}
	b.SetBytes(total)
	b.ReportAllocs()
	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		for _, l := range lines {
			if _, err := c.Parse(l, Options{}); err != nil {
				b.Fatal(err)
			}
		}
	}
}
