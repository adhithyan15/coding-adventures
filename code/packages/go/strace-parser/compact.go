package straceparser

// This file is the COMPACT path: the same line parser, with no *ASTNode tree.
//
// # Why a second path
//
// ParseLineTokens builds one *parser.ASTNode per rule match, plus a Children
// slice for each.  For a consumer that reads one line, asks "which shape is it,
// and where are the arguments?" and throws the tree away, that is most of the
// cost of the line.  The shared parser's compact API (parser.Tree, filled by
// Compiled.ParseInto) keeps the same result in two flat arrays that are reused,
// so a LineParser that is kept between lines parses without building a tree:
//
//	var lp straceparser.LineParser        // once, e.g. one per goroutine
//	for each line {
//	    if err := lp.ParseLineInto(line, opts); err != nil { ... }
//	    switch lp.BodyKind() {
//	    case "call_line":
//	        args := lp.AppendRawArgs(buf[:0])
//	        rest := lp.TextAfterClose()
//	    }
//	}
//
// # Same answers, by construction
//
// Nothing here re-implements a check.  ParseLineInto runs checkLine (the byte
// limit, newline, spaces, lexing, ERR tokens, strict decoding: shared with
// ParseLineTokens) and then the shared parser's ParseInto, which is documented to
// accept and refuse exactly what Parse does (except Options.PostParse hooks,
// which are refused: see ParseLineInto).  A refusal is the same *LineError,
// with the same text.  The helpers below are the ASTNode helpers of shape.go
// rewritten over NodeIDs, and a test checks every one against its original on
// the corpora, on random lines and under a fuzzer.
//
// # Ownership and concurrency
//
// A LineParser is NOT safe for concurrent use: give each goroutine its own.  It
// must not be copied (it embeds a parser.Tree, which go vet's copylocks flags).
//
// Everything a LineParser returns about the last line (Tree, Tokens, the NodeIDs
// from Body) is valid only until the NEXT call to ParseLineInto or Reset.  The
// strings the helpers return (RawArgs, TextAfterClose, PrefixBeforeUnfinished)
// are different: they are substrings of the line you passed in, so they stay
// valid for as long as that string does and need no copying.
//
// # Memory bound
//
// The Tree reuses its flat arrays from line to line, but (inherited from the
// parser package) drops an array larger than its pooled cap at the next parse,
// so one huge line does not pin memory forever. For hook-free parses, the
// parser-owned token slice is also reused; a token slice exposed to PreParse
// hooks stays on the owning path and is never reused or cleared by this parser.
//
// What a LineParser pins between calls: the last line string, its token slice
// and the Tree.  That is bounded by Options.MaxLineBytes (up to 1 MiB for a
// default line, plus its tokens and tree) and is released by the next
// ParseLineInto or by Reset.  Call Reset after a very large line if the
// LineParser will then sit idle.  Do not mutate Tokens(): the tree and the
// helpers index into it.
//
// # What is not yet allocation-free
//
// The standard compiled lexer has a caller-buffer entry point. LineParser uses
// its own token buffer as scratch only when no PreParse hooks are present.

import (
	"errors"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
)

// ruleIDs are the ids, in the compiled grammar, of the rules the compact
// helpers look for.  Resolved once; ids are only meaningful with `compiled`.
var ruleIDs = func() (r struct{ body, callLine, callOpen, callClosed, args, arg int32 }) {
	if compileErr != nil {
		return
	}
	get := func(name string) int32 {
		id, ok := compiled.RuleID(name)
		if !ok {
			return -1
		}
		return id
	}
	r.body, r.callLine, r.callOpen = get("body"), get("call_line"), get("call_open")
	r.callClosed, r.args, r.arg = get("call_closed"), get("args"), get("arg")
	return
}()

var errNilLineParser = errors.New("strace-parser: ParseLineInto needs a non-nil *LineParser")

// LineParser parses lines into a reusable compact tree.  The zero value is
// ready to use.  Not safe for concurrent use; do not copy.  See the file
// comment for what stays valid until when.
type LineParser struct {
	tree    parser.Tree
	line    string
	toks    []lexer.Token
	scratch []lexer.Token
	cursor  lexer.TokenCursor
	body    parser.NodeID
	valid   bool
}

// maxRetainedTokenCapacity keeps a large token slice from pinning memory when
// a LineParser moves back to short lines. Larger buffers are cleared only over
// their occupied length, then dropped on reset.
const maxRetainedTokenCapacity = 4096

// ParseLineInto tokenizes and parses one line into lp, discarding the previous
// result.  It accepts and refuses exactly what ParseLineTokens does, with the
// same *LineError, and Tokens returns what ParseLineTokens returns as its token
// slice (including alongside an error).  Every zero limit in opts is filled
// from DefaultOptions.
//
// The one exception is Options.PostParse: those hooks transform *ASTNode
// trees, which this path never builds, so a non-empty PostParse is REFUSED
// (KindSyntax, "ParseInto does not support Options.PostParse"), where
// ParseLineTokens would run the hooks.  Apply hooks yourself to
// lp.Tree().ToAST() if you need them.
func (lp *LineParser) ParseLineInto(line string, opts Options) error {
	if lp == nil {
		return errNilLineParser
	}
	opts = opts.withDefaults()
	lp.Reset()
	lp.line = line
	usesPreParse := len(opts.PreParse) > 0
	var cursor *lexer.TokenCursor
	if !usesPreParse {
		cursor = &lp.cursor
	}
	toks, scratch, err := checkLineWithCursorScratch(line, opts, lp.scratch, cursor)
	lp.scratch = scratch
	lp.toks = toks
	if err != nil {
		return err
	}
	if perr := compiled.ParseInto(&lp.tree, toks, opts.Options); perr != nil {
		return classifyParseError(perr)
	}
	lp.valid = true
	lp.body = lp.findBody()
	return nil
}

// Reset forgets the last line and releases what the parser holds that is
// larger than its pooled cap.  ParseLineInto does this itself.
// A nil *LineParser is a no-op.
func (lp *LineParser) Reset() {
	if lp == nil {
		return
	}
	lp.tree.Reset()
	clear(lp.scratch)
	if cap(lp.scratch) > maxRetainedTokenCapacity {
		lp.scratch = nil
	} else {
		lp.scratch = lp.scratch[:0]
	}
	lp.line, lp.toks, lp.body, lp.valid = "", nil, parser.NoNode, false
}

// Tree returns the compact tree of the last successful line (empty after a
// failed one; nil for a nil *LineParser).  Valid until the next ParseLineInto
// or Reset.  It is READ-ONLY: it is the LineParser's own tree, so never call
// Reset or ParseInto on it (the helpers then see an empty tree and return
// empty results, but the next ParseLineInto is the only supported way to
// refill it).
func (lp *LineParser) Tree() *parser.Tree {
	if lp == nil {
		return nil
	}
	return &lp.tree
}

// Tokens returns the tokens of the last line, as ParseLineTokens returns them.
// Valid until the next ParseLineInto or Reset.  Read-only: the tree and the
// helpers index into this slice, so mutating it (or its length) can make them
// panic or return wrong text.
func (lp *LineParser) Tokens() []lexer.Token {
	if lp == nil {
		return nil
	}
	return lp.toks
}

// Body returns the NodeID of the body node of the last line (the child of the
// `line` rule after the pid), or parser.NoNode after a failed parse.  The
// compact Body.
func (lp *LineParser) Body() parser.NodeID {
	// Root() is checked too: if a caller reset the tree behind our back
	// (lp.Tree().Reset()), the cached body id no longer means anything.
	if lp == nil || !lp.valid || lp.tree.Root() == parser.NoNode {
		return parser.NoNode
	}
	return lp.body
}

// findBody: the first child node of `line` that is a `body`, then that node's
// first child node (as Body in parser.go does).
func (lp *LineParser) findBody() parser.NodeID {
	t := &lp.tree
	root := t.Root()
	if root == parser.NoNode {
		return parser.NoNode
	}
	// No check that root is `line`: `line` is the only rule that has a `body`
	// child, so a tree rooted at any other rule simply finds none.
	for i, n := 0, t.NumChildren(root); i < n; i++ {
		if c := t.Child(root, i); !c.IsToken() && t.RuleID(c.Node()) == ruleIDs.body {
			b := c.Node()
			for j, m := 0, t.NumChildren(b); j < m; j++ {
				if cc := t.Child(b, j); !cc.IsToken() {
					return cc.Node()
				}
			}
		}
	}
	return parser.NoNode
}

// BodyKind is the compact BodyKind: the name of the rule that matched the body
// of the last line, or "" after a failed parse.
func (lp *LineParser) BodyKind() string {
	if b := lp.Body(); b != parser.NoNode {
		return lp.tree.Rule(b)
	}
	return ""
}

// callBody returns the body if it is a call_line.
func (lp *LineParser) callBody() (parser.NodeID, bool) {
	b := lp.Body()
	if b == parser.NoNode || lp.tree.RuleID(b) != ruleIDs.callLine {
		return parser.NoNode, false
	}
	return b, true
}

// AppendRawArgs appends to dst the raw text of each top-level argument of a
// call_line (see RawArgs) and returns the extended slice.  Pass dst[:0] of a
// kept buffer to avoid allocating.  Other bodies append nothing.  The strings
// are substrings of the line.
func (lp *LineParser) AppendRawArgs(dst []string) []string {
	call, ok := lp.callBody()
	if !ok {
		return dst
	}
	t := &lp.tree
	for i, n := 0, t.NumChildren(call); i < n; i++ {
		part := t.Child(call, i)
		if part.IsToken() {
			continue
		}
		p := part.Node()
		for j, m := 0, t.NumChildren(p); j < m; j++ {
			ac := t.Child(p, j)
			if ac.IsToken() || t.RuleID(ac.Node()) != ruleIDs.args {
				continue
			}
			args := ac.Node()
			for k, q := 0, t.NumChildren(args); k < q; k++ {
				a := t.Child(args, k)
				if a.IsToken() || t.RuleID(a.Node()) != ruleIDs.arg {
					continue
				}
				first, last := t.Span(a.Node())
				f, l := t.Token(first), t.Token(last)
				dst = append(dst, lp.line[f.Column-1:l.Column-1+len(l.Value)])
			}
		}
	}
	return dst
}

// RawArgs is the compact RawArgs: the same strings, nil when there are none or
// the body is not a call_line.  It allocates the result slice; use
// AppendRawArgs to reuse one.
func (lp *LineParser) RawArgs() []string { return lp.AppendRawArgs(nil) }

// TextAfterClose is the compact TextAfterClose: the raw text after the closing
// ")" of a call_line or resumed_line, "" when there is none.  (A dead_line is a
// single token with no separate ")", so it always gives "".)
func (lp *LineParser) TextAfterClose() string {
	b := lp.Body()
	if b == parser.NoNode {
		return ""
	}
	if i, ok := lp.closeParen(b); ok {
		t := lp.tree.Token(i)
		return lp.line[t.Column-1+len(t.Value):]
	}
	return ""
}

// closeParen is shape.go's closeParen: the RPAREN child of n, or of a
// call_closed child (recursively).  It returns the token index.
func (lp *LineParser) closeParen(n parser.NodeID) (int, bool) {
	t := &lp.tree
	for i, m := 0, t.NumChildren(n); i < m; i++ {
		c := t.Child(n, i)
		if c.IsToken() {
			if t.Token(c.Token()).TypeName == "RPAREN" {
				return c.Token(), true
			}
			continue
		}
		if v := c.Node(); t.RuleID(v) == ruleIDs.callClosed {
			if ti, ok := lp.closeParen(v); ok {
				return ti, true
			}
		}
	}
	return 0, false
}

// PrefixBeforeUnfinished is the compact PrefixBeforeUnfinished: for an
// interrupted call, the text from the call name to the marker with one
// trailing space removed; ok is false for any other line.
func (lp *LineParser) PrefixBeforeUnfinished() (prefix string, ok bool) {
	call, isCall := lp.callBody()
	if !isCall {
		return "", false
	}
	t := &lp.tree
	name, marker := -1, -1
	for i, n := 0, t.NumChildren(call); i < n; i++ {
		c := t.Child(call, i)
		if c.IsToken() {
			if t.Token(c.Token()).TypeName == "SYSCALL" {
				name = c.Token()
			}
			continue
		}
		v := c.Node()
		if t.RuleID(v) != ruleIDs.callOpen {
			continue
		}
		for j, m := 0, t.NumChildren(v); j < m; j++ {
			if cc := t.Child(v, j); cc.IsToken() && t.Token(cc.Token()).TypeName == "UNFINISHED" {
				marker = cc.Token()
			}
		}
	}
	if name < 0 || marker < 0 {
		return "", false
	}
	s := lp.line[t.Token(name).Column-1 : t.Token(marker).Column-1]
	if len(s) > 0 && s[len(s)-1] == ' ' {
		s = s[:len(s)-1]
	}
	return s, true
}
