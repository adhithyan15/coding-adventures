package trace

import (
	"strings"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
	straceparser "github.com/adhithyan15/coding-adventures/code/packages/go/strace-parser"
)

// handleLineG classifies one line, exactly as handleLine does, reading it with
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
func handleLineG(text string, lineNo int, st *lineState) (rec *record, e *Error) {
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
	bt, ok := bodyToken(ast)
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
		if p := pend[pid]; p != nil {
			// A process killed or exiting inside a call that has no effect on
			// paths is normal (wait4 interrupted by SIGKILL).  A path call that
			// never returned is not: its result is unknown.
			if c := lookup(p.name); c.class != classIgnored && !p.dead {
				return nil, incomplete(p.line, reasonUnresumed)
			}
			delete(pend, pid)
		}
		return &record{line: lineNo, pid: pid, exit: ex}, nil

	case "signal_line":
		if p := pend[pid]; p != nil && p.dead {
			return nil, incomplete(lineNo, reasonInterleaved)
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
		pend[pid] = &pending{name: "???", line: lineNo, dead: true, resumed: bt.TypeName == "DEAD_WHOLE"}
		return nil, nil

	case "resumed_line":
		// "<... NAME resumed>": the lexer matched exactly "<... " NAME " resumed>".
		name := bt.Value[len("<... ") : len(bt.Value)-len(" resumed>")]
		p := pend[pid]
		if p == nil || p.name != name || p.dead {
			return nil, incomplete(lineNo, reasonUnmatched)
		}
		delete(pend, pid)
		// Glue the half before "<unfinished ...>" to the text after the marker
		// and read the whole call again, as one line.  Each half was already
		// read on its own (the parse above), so no construct can span the seam:
		// strace never splits a token (KILN04 section 4.9, table).
		glued := p.prefix + text[bt.Column-1+len(bt.Value):]
		call, unfinished, _, r := parseCallG(glued, maxLine)
		if r != "" {
			return nil, incomplete(lineNo, r)
		}
		if unfinished {
			return nil, incomplete(lineNo, reasonUnparsable)
		}
		if isExitName(call.name) && !call.ret.unknown {
			// The call began (the marker was taken at its start line) but
			// returned: it failed, and the group is not dying.
			return &record{line: p.line, pid: pid, exitCall: true, rawExit: call.name == "exit", retract: true}, nil
		}
		return finishCall(pid, lineNo, call, true, st.helper)

	case "call_line":
		call, unfinished, prefix, r := callFromTree(text, ast, bt)
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
			pend[pid] = &pending{name: call.name, prefix: prefix, line: lineNo}
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
		return finishCall(pid, lineNo, call, false, st.helper)
	}
	return nil, incomplete(lineNo, reasonUnparsable)
}

// parseCallG reads a glued call ("name(args) = result") as a line of its own,
// under a dummy pid, and returns what parseCall returns.
func parseCallG(glued string, maxLine int) (call rawCall, unfinished bool, prefix string, reason string) {
	line := "1  " + glued
	ast, _, err := straceparser.ParseLineTokens(line, grammarOptions(maxLine))
	if err != nil {
		return call, false, "", grammarReason(err)
	}
	bt, ok := bodyToken(ast)
	if !ok || straceparser.BodyKind(ast) != "call_line" {
		return call, false, "", reasonUnparsable
	}
	return callFromTree(line, ast, bt)
}

// callFromTree builds a rawCall from a parsed call_line: the name from the
// SYSCALL token, the arguments from the raw spans, and the result from the
// strict result parser run on the text after the closing parenthesis.  For an
// interrupted call it returns the pending prefix instead.
func callFromTree(line string, ast *parser.ASTNode, name lexer.Token) (call rawCall, unfinished bool, prefix string, reason string) {
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
	if open {
		return call, true, pre, ""
	}
	call.args = args
	// The grammar's tail is free-form on purpose: the strict result parser
	// decides what may follow the ")", for EVERY closed call.
	ret, r := parseResult(straceparser.TextAfterClose(line, ast))
	if r != "" {
		return call, false, "", r
	}
	call.ret = ret
	return call, false, "", ""
}
