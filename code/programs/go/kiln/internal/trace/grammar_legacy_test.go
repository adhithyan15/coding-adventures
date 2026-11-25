package trace

// This file is a TEST-ONLY copy of the ASTNode-based handleLineG as it was
// before P2 PR 5 moved production onto the compact straceparser.LineParser.
// TestCompactEqualsLegacy* compare the two on every corpus and generator: the
// move must change no record and no refusal.  Do not "improve" this file.

import (
	"strings"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
	straceparser "github.com/adhithyan15/coding-adventures/code/packages/go/strace-parser"
)

// handleLineGLegacy classifies one line, exactly as handleLine does, reading it with
// the grammar (see the head of grammar_line.go).  It returns the completed
// record (nil when the line completes nothing, e.g. an unfinished call or a
// signal line).
//
// The steps, in order, are the old ones:
//
//  1. byte pre-checks (empty line, control characters, a personality line):
//     validation, not lexing, so they stay plain code;
//  2. lex and parse the line; a failure is a refusal whose reason grammarReason
//     gives;
//  3. switch on the body rule, and run the old state machine for that shape.
func handleLineGLegacy(text string, lineNo int, st *lineState) (rec *record, e *Error) {
	// A panic in the grammar path must never become a partial trace: refuse the
	// whole trace.  (The lexer is total and the parser is depth-limited, so this
	// is a guard against a bug, counted so the tests can insist it never fires.)
	defer func() {
		if p := recover(); p != nil {
			grammarPanics.Add(1)
			rec, e = nil, incomplete(lineNo, reasonUnparsable)
		}
	}()
	pend := st.pend
	if text == "" {
		return nil, incomplete(lineNo, reasonUnparsable)
	}
	for i := 0; i < len(text); i++ {
		if c := text[i]; c < 0x20 || c == 0x7f {
			// strace escapes every control character; a raw one means the line
			// is not strace's own output.
			return nil, incomplete(lineNo, reasonUnparsable)
		}
	}
	if strings.HasPrefix(text, "[ Process PID=") {
		return nil, incomplete(lineNo, reasonPersonality)
	}
	maxLine := st.maxLine
	if maxLine <= 0 {
		maxLine = DefaultLimits().MaxLineBytes
	}
	ast, toks, err := straceparser.ParseLineTokens(text, grammarOptions(maxLine))
	if err != nil {
		return nil, incomplete(lineNo, grammarReason(err))
	}
	if len(toks) == 0 {
		return nil, incomplete(lineNo, reasonUnparsable)
	}
	pid, ok := pidOfToken(toks[0])
	if !ok {
		return nil, incomplete(lineNo, reasonUnparsable)
	}
	bt, ok := bodyTokenLegacy(ast)
	if !ok {
		return nil, incomplete(lineNo, reasonUnparsable)
	}

	switch straceparser.BodyKind(ast) {
	case "exit_line":
		// The token is the whole "+++ ... +++"; the old exit parser reads what is
		// inside, so the number is still read by the same strict code.
		v := bt.Value
		if len(v) < 8 {
			return nil, incomplete(lineNo, reasonUnparsable)
		}
		ex, ok := parseExit(v[4:len(v)-4], pid)
		if !ok {
			return nil, incomplete(lineNo, reasonUnparsable)
		}
		dropped, e := exitInsideCall(st, pid, ex)
		if e != nil {
			return nil, e
		}
		return &record{line: lineNo, pid: pid, exit: ex, spawnDropped: dropped}, nil

	case "signal_line":
		if p := pend[pid]; p != nil {
			if p.dead {
				return nil, incomplete(lineNo, reasonInterleaved)
			}
			p.sawSignal = true
		}
		return nil, nil

	case "dead_resumed_line":
		// "<... ??? resumed>)  = ?", exactly (the lexer accepts no other
		// spelling).  Only a pending killed-at-entry call may be resumed by it,
		// and only once.
		p := pend[pid]
		if p == nil || !p.dead || p.resumed {
			return nil, incomplete(lineNo, reasonUnmatched)
		}
		p.resumed = true
		return nil, nil

	case "dead_line":
		// See "A thread killed at syscall entry" in line.go.  Something already
		// pending for the pid is the usual interleaving error.
		if pend[pid] != nil {
			return nil, incomplete(lineNo, reasonInterleaved)
		}
		st.setPending(pid, &pending{name: "???", line: lineNo, dead: true, resumed: bt.TypeName == "DEAD_WHOLE"})
		return nil, nil

	case "resumed_line":
		// "<... NAME resumed>": the lexer matched exactly "<... " NAME " resumed>".
		name := bt.Value[len("<... ") : len(bt.Value)-len(" resumed>")]
		p := pend[pid]
		if p == nil || p.name != name || p.dead {
			return nil, incomplete(lineNo, reasonUnmatched)
		}
		st.clearPending(pid)
		// Glue the half before "<unfinished ...>" to the text after the marker
		// and read the whole call again, as one line.  Each half was already
		// read on its own (the parse above), so no construct can span the seam:
		// strace never splits a token (KILN04 section 4.9, table).
		after := text[bt.Column-1+len(bt.Value):]
		if dyingResumeTail(after) {
			diedInCall(st, pid, p.line, p.name, p.prefix, p.args)
			return nil, nil
		}
		glued := p.prefix + after
		call, unfinished, _, r := parseCallGLegacy(glued, maxLine)
		if r != "" {
			return nil, incomplete(lineNo, r)
		}
		if unfinished {
			return nil, incomplete(lineNo, reasonUnparsable)
		}
		if call.ret.unavailable {
			return diedWithResult(st, pid, lineNo, p.line, call, true, st.helper)
		}
		if isExitName(call.name) && !call.ret.unknown {
			// The call began (the marker was taken at its start line) but
			// returned: it failed, and the group is not dying.
			return &record{line: p.line, pid: pid, exitCall: true, rawExit: call.name == "exit", retract: true}, nil
		}
		rec, e := finishCall(pid, lineNo, call, true, st.helper)
		if rec != nil {
			rec.startLine = p.line
		}
		return withGhosts(p, call, rec, e)

	case "call_line":
		call, unfinished, prefix, r := callFromTreeLegacy(text, ast, bt)
		if r != "" {
			return nil, incomplete(lineNo, r)
		}
		if pend[pid] != nil {
			return nil, incomplete(lineNo, reasonInterleaved)
		}
		if unfinished {
			if !st.helper {
				if e := classifyName(call.name, lineNo); e != nil {
					return nil, e
				}
			}
			st.setPending(pid, &pending{name: call.name, prefix: prefix, line: lineNo, args: call.args})
			if isExitName(call.name) {
				return &record{line: lineNo, pid: pid, exitCall: true, rawExit: call.name == "exit"}, nil
			}
			if lookup(call.name).class == classHandled {
				// The working directory a split call runs in is the one at its
				// START; pass 2 samples it when it meets this marker.
				return &record{line: lineNo, pid: pid, start: true}, nil
			}
			return nil, nil
		}
		if call.ret.unavailable {
			return diedWithResult(st, pid, lineNo, lineNo, call, false, st.helper)
		}
		return finishCall(pid, lineNo, call, false, st.helper)
	}
	return nil, incomplete(lineNo, reasonUnparsable)
}

// parseCallGLegacy reads a glued call ("name(args) = result") as a line of its own,
// under a dummy pid, and returns what parseCall returns.
func parseCallGLegacy(glued string, maxLine int) (call rawCall, unfinished bool, prefix string, reason string) {
	line := "1  " + glued
	ast, _, err := straceparser.ParseLineTokens(line, grammarOptions(maxLine))
	if err != nil {
		return call, false, "", grammarReason(err)
	}
	bt, ok := bodyTokenLegacy(ast)
	if !ok || straceparser.BodyKind(ast) != "call_line" {
		return call, false, "", reasonUnparsable
	}
	return callFromTreeLegacy(line, ast, bt)
}

// callFromTreeLegacy builds a rawCall from a parsed call_line: the name from the
// SYSCALL token, the arguments from the raw spans, and the result from the
// strict result parser run on the text after the closing parenthesis.  For an
// interrupted call it returns the pending prefix instead.
func callFromTreeLegacy(line string, ast *parser.ASTNode, name lexer.Token) (call rawCall, unfinished bool, prefix string, reason string) {
	n := name.Value[:len(name.Value)-1] // drop the "("
	if !validName(n) {
		return call, false, "", reasonUnparsable
	}
	call.name = n
	// A handled call has a known arity; anything else gets a generous global
	// cap.
	maxItems := maxUnknownItems
	if ent := lookup(n); ent.class == classHandled {
		maxItems = ent.max
	}
	pre, open := straceparser.PrefixBeforeUnfinished(line, ast)
	commaAfter := open && strings.HasSuffix(strings.TrimRight(pre, " "), ",")
	args, r := trimArgs(straceparser.RawArgs(line, ast), open, commaAfter)
	if r != "" {
		return call, false, "", r
	}
	if len(args) > maxItems {
		return call, false, "", reasonArity
	}
	call.args = args
	if open {
		return call, true, pre, ""
	}
	// The grammar's tail is free-form on purpose: the strict result parser
	// decides what may follow the ")", for EVERY closed call.
	ret, r := parseResult(straceparser.TextAfterClose(line, ast))
	if r != "" {
		return call, false, "", r
	}
	call.ret = ret
	return call, false, "", ""
}

// bodyTokenLegacy returns the first token of the body node: the SYSCALL of a call,
// the RESUMED marker, the exit token, SIG_OPEN, or a killed-at-entry token.
func bodyTokenLegacy(ast *parser.ASTNode) (lexer.Token, bool) {
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
