package trace

import (
	"errors"
	"strings"
	"sync/atomic"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
	straceparser "github.com/adhithyan15/coding-adventures/code/packages/go/strace-parser"
)

// This file is the grammar-driven twin of handleLine (D24, KILN04 section 4.9).
//
// handleLine reads a line with the hand-written scanner in lex.go.  handleLineG
// reads the SAME line with the shared grammar toolchain (code/grammars/strace,
// through the strace-parser package) and builds the SAME []record.  Phase A of
// D24 changes only HOW THE TEXT IS READ:
//
//   - the line's shape (call, resumed half, exit, signal, killed-at-entry) comes
//     from the grammar's body rule, not from prefix tests;
//   - the arguments of a call are the raw spans the grammar found, not the
//     items scanItems cut;
//   - everything that is POLICY stays exactly as it was: the pending map, the
//     D18 rules for a call killed at entry, classifyName, the strict result
//     parser (parseResult), and every per-argument interpreter (pathArg,
//     parseFD, parseArgv, sockaddr, clone3), which still read raw text.
//
// # Which parser is in charge
//
// The old scanner stays the DEFAULT.  Nothing in production calls handleLineG
// except the cross-check (crosscheck.go), which runs both on every trace and
// refuses to go on when they disagree in a way the spec does not allow.  The
// flip to the grammar path is a later PR, gated on a measured throughput.
//
// # Spaces are invisible to the grammar
//
// The lexer skips spaces between tokens, so the grammar sees `"a""b"`,
// `3</a>x</b>` and `O_RDONLY O_CLOEXEC` each as one argument made of several
// pieces.  That is fine, because RawArgs hands over the raw text of the whole
// argument and the old interpreters are strict about it: pathArg wants exactly
// one string, parseFD wants a number and at most one annotation, and so on.
// strace-parser's tests and ours (grammar_line_test.go) pin that.

// maxLineTokens is the most tokens a line may have: KILN04 section 4.9 (Q2).
const maxLineTokens = 65536

// grammarPanics counts the panics handleLineG recovered from.  There must never
// be one: the lexer is total and the parser has a depth limit.  A recovered
// panic refuses the WHOLE trace (never a partial one), and the tests assert the
// count stays zero over every differential run.
var grammarPanics atomic.Int64

// grammarOptions is the explicit limit set the grammar path runs with.  Nothing
// is left to a default: a zero limit in the shared parser means "unlimited".
func grammarOptions(maxLine int) straceparser.Options {
	o := straceparser.DefaultOptions() // memo bound: rules * (tokens+1), the most the packrat table can use
	o.MaxLineBytes = maxLine
	o.MaxDepth = straceparser.DefaultMaxDepth // 64 bracket levels
	o.MaxTokens = maxLineTokens
	o.LenientQuoted = false // every quoted string must decode strictly
	return o
}

// grammarReason maps a refusal of the shared parser to the reason the old
// scanner gave (the table of KILN04 section 4.9).
func grammarReason(err error) string {
	var le *straceparser.LineError
	if !errors.As(err, &le) {
		return reasonUnparsable
	}
	switch le.Kind {
	case straceparser.KindLimit:
		return reasonSize
	case straceparser.KindBadAnnotation, straceparser.KindBadQuoted:
		return reasonAmbiguousPath
	case straceparser.KindErrToken:
		if le.Ambiguous() {
			return reasonAmbiguousPath
		}
	}
	return reasonUnparsable
}

// pidOfToken reads the pid out of the PID token of a line: "1234  " or
// "[pid 1234] ".  The lexer already restricted it to 1 to 9 digits with no
// leading zero; atoiPid checks the same again, so a grammar change cannot
// loosen it unnoticed.
func pidOfToken(t lexer.Token) (int, bool) {
	s := t.Value
	s = strings.TrimPrefix(s, "[pid ")
	s = strings.TrimRight(s, "] ")
	return atoiPid(s)
}

// bodyToken returns the first token of the body node: the SYSCALL of a call,
// the RESUMED marker, the exit token, SIG_OPEN, or a killed-at-entry token.
func bodyToken(ast *parser.ASTNode) (lexer.Token, bool) {
	b := straceparser.Body(ast)
	if b == nil {
		return lexer.Token{}, false
	}
	for _, c := range b.Children {
		if t, ok := c.(lexer.Token); ok {
			return t, true
		}
	}
	return lexer.Token{}, false
}

// trimArgs turns the raw argument spans of RawArgs into the items scanItems
// would have produced.  scanItems trims each item with strings.TrimSpace, which
// also removes Unicode spaces (U+00A0 and friends) that the grammar sees as
// ordinary punctuation; trimming here keeps the spans byte-identical.  An item
// that is empty after trimming is an error, except where scanItems let it go: a
// call whose only item is empty has no arguments, and the item right before
// "<unfinished ...>" is dropped.
func trimArgs(raw []string, open bool) ([]string, string) {
	var out []string
	for i, a := range raw {
		t := strings.TrimSpace(a)
		if t != "" {
			out = append(out, t)
			continue
		}
		last := i == len(raw)-1
		switch {
		case last && open:
			// the item before the marker: dropped
		case last && !open && len(raw) == 1:
			// f(<spaces>): no arguments
		default:
			return nil, reasonUnparsable
		}
	}
	return out, ""
}
