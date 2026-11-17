package lexer

// Shared grammars and inputs for the benchmarks and differential tests.
// They live in one file so that the benchmark, the differential test and
// the fuzz seeds all exercise exactly the same material.

import (
	"strings"
	"testing"

	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
)

// straceLikeTokens is a trace-line grammar shaped like the one Kiln uses to
// parse strace output. It deliberately contains the three things that are
// expensive for a regexp-per-rule lexer: a quoted string with escapes, a fused
// `name<annotation>` token, and a long tail of single-character punctuation
// rules that all have to be rejected before the final catch-all.
const straceLikeTokens = `
UNFINISHED = /<unfinished \.\.\.>/
RESUMED    = /<\.\.\. [a-z_0-9]+ resumed>/
ANNOTATED  = /[A-Za-z_0-9]+<[^>\n]*>/
QUOTED     = /"([^"\\]|\\.)*"(\.\.\.)?/
HEX        = /0x[0-9a-fA-F]+/
NUM        = /-?[0-9]+/
IDENT      = /[A-Za-z_][A-Za-z0-9_]*/
ARROW      = "=>"
EQ         = "="
PIPE       = "|"
COMMA      = ","
LPAREN     = "("
RPAREN     = ")"
LBRACKET   = "["
RBRACKET   = "]"
LBRACE     = "{"
RBRACE     = "}"
TILDE      = "~"
STAR       = "*"
AMP        = "&"
PLUS       = "+"
MINUS      = "-"
SLASH      = "/"
DOT        = "."
COLON      = ":"
QMARK      = "?"
BANG       = "!"
ERR_CHAR   = /./

skip:
  WS = /[ \t\r]+/
`

// straceLikeLines is a corpus of strace-shaped lines (no trailing newline).
var straceLikeLines = []string{
	`openat(AT_FDCWD, "/etc/ld.so.cache", O_RDONLY|O_CLOEXEC) = 3</etc/ld.so.cache>`,
	`read(3</etc/passwd>, "root:x:0:0:root:/root:/bin/bash\n", 4096) = 1024`,
	`write(1</dev/pts/0>, "say \"hi\" to \\ the world\n", 23) = 23`,
	`execve("/usr/bin/ls", ["ls", "-la", "/tmp"], {"PATH": "/usr/bin", "HOME": "/root"}) = 0`,
	`epoll_wait(5<anon_inode:[eventpoll]>, [{events=EPOLLIN, data={u32=7, u64=7}}], 64, -1) = 1`,
	`mmap(NULL, 8192, PROT_READ|PROT_WRITE, MAP_PRIVATE|MAP_ANONYMOUS, -1, 0) = 0x7f3a1c000000`,
	`fstat(3</lib/x86_64-linux-gnu/libc.so.6>, {st_mode=S_IFREG|0755, st_size=2216304, ...}) = 0`,
	`futex(0x7ffd1c2f8a20, FUTEX_WAIT_PRIVATE, 2, NULL <unfinished ...>`,
	`<... futex resumed>) = 0`,
	`rt_sigaction(SIGINT, {sa_handler=0x55d4c8a0, sa_mask=[], sa_flags=SA_RESTORER, sa_restorer=0x7f3a}, NULL, 8) = 0`,
	`connect(4<socket:[12345]>, {sa_family=AF_INET, sin_port=htons(443), sin_addr=inet_addr("10.0.0.1")}, 16) = -1 EINPROGRESS`,
	`clone3({flags=CLONE_VM|CLONE_FS|CLONE_FILES, exit_signal=SIGCHLD, stack=NULL, stack_size=0}, 88) = 4242`,
	`unlink("/tmp/some/long/path/that/keeps/going/and/going/file.txt") = -1 ENOENT (No such file or directory)`,
	`getdents64(3</proc/self/fd>, [{d_ino=1, d_off=2, d_reclen=24, d_type=DT_DIR, d_name="."}], 32768) = 24`,
}

// straceLikeInput is the whole corpus repeated, joined by newlines.
func straceLikeInput(repeat int) string {
	var sb strings.Builder
	for i := 0; i < repeat; i++ {
		for _, l := range straceLikeLines {
			sb.WriteString(l)
			sb.WriteByte('\n')
		}
	}
	return sb.String()
}

// jsonDocument is a realistic JSON payload for the benchmark.
func jsonDocument(repeat int) string {
	item := `{"id": 12345, "name": "Widget \"Pro\"", "price": -12.50e+3, "tags": ["a", "b", "c"], "ok": true, "none": null, "nested": {"x": [1, 2, [3, 4, {"deep": false}]]}}`
	var sb strings.Builder
	sb.WriteString("[")
	for i := 0; i < repeat; i++ {
		if i > 0 {
			sb.WriteString(",\n")
		}
		sb.WriteString(item)
	}
	sb.WriteString("]")
	return sb.String()
}

const jsonLikeTokens = `
escapes: none

STRING   = /"([^"\\]|\\["\\\x2fbfnrt]|\\u[0-9a-fA-F]{4})*"/
NUMBER   = /-?[0-9]+\.?[0-9]*[eE]?[-+]?[0-9]*/
TRUE     = "true"
FALSE    = "false"
NULL     = "null"
LBRACE   = "{"
RBRACE   = "}"
LBRACKET = "["
RBRACKET = "]"
COLON    = ":"
COMMA    = ","

skip:
  WHITESPACE = /[ \t\r\n]+/
`

func mustGrammar(tb testing.TB, src string) *grammartools.TokenGrammar {
	tb.Helper()
	g, err := grammartools.ParseTokenGrammar(src)
	if err != nil {
		tb.Fatalf("grammar parse: %v", err)
	}
	return g
}
