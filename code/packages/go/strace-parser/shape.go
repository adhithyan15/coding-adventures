package straceparser

import (
	"strings"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
)

// This file has the small helpers a consumer needs to read a parsed line: what
// shape it is, and the raw text of its arguments and of what follows the call.
// They only walk the tree and cut text out of the line by token Column; they
// interpret nothing.

// BodyKind returns the name of the rule that matched the body of the line:
// "call_line", "resumed_line", "exit_line", "signal_line", "dead_line" or
// "dead_resumed_line".
func BodyKind(ast *parser.ASTNode) string {
	if b := Body(ast); b != nil {
		return b.RuleName
	}
	return ""
}

// span returns the first and last token under a node (or token).
func span(n interface{}) (first, last lexer.Token, ok bool) {
	switch v := n.(type) {
	case lexer.Token:
		return v, v, true
	case *parser.ASTNode:
		for _, c := range v.Children {
			if f, l, found := span(c); found {
				if !ok {
					first, ok = f, true
				}
				last = l
			}
		}
	}
	return first, last, ok
}

// RawArgs returns the raw text of each top-level argument of a call_line, cut
// from line by token columns: from the first token of the argument to the last,
// so spaces inside are kept and spaces around are not.  For "f()" it is empty.
// For the interrupted form the arguments printed before the marker are
// returned.  It is nil for any other body.
func RawArgs(line string, ast *parser.ASTNode) []string {
	call := Body(ast)
	if call == nil || call.RuleName != "call_line" {
		return nil
	}
	var out []string
	for _, c := range call.Children {
		part, ok := c.(*parser.ASTNode)
		if !ok {
			continue
		}
		for _, cc := range part.Children {
			args, ok := cc.(*parser.ASTNode)
			if !ok || args.RuleName != "args" {
				continue
			}
			for _, a := range args.Children {
				if arg, ok := a.(*parser.ASTNode); ok && arg.RuleName == "arg" {
					f, l, _ := span(arg)
					out = append(out, line[f.Column-1:l.Column-1+len(l.Value)])
				}
			}
		}
	}
	return out
}

// TextAfterClose returns the raw text after the closing ")" of a call_line,
// or resumed_line (what the consumer's result parser reads), with its
// leading spaces: " = 3</etc/passwd>".  It is "" when the line has no closing
// parenthesis (an interrupted call, an exit, signal or dead line: a dead line
// is a single token).
func TextAfterClose(line string, ast *parser.ASTNode) string {
	b := Body(ast)
	if b == nil {
		return ""
	}
	if t, ok := closeParen(b); ok {
		return line[t.Column-1+len(t.Value):]
	}
	return ""
}

// closeParen finds the RPAREN that closes the call: a direct child of
// resumed_line or dead_line, or of the call_closed under call_line.
func closeParen(n *parser.ASTNode) (lexer.Token, bool) {
	for _, c := range n.Children {
		switch v := c.(type) {
		case lexer.Token:
			if v.TypeName == "RPAREN" {
				return v, true
			}
		case *parser.ASTNode:
			if v.RuleName == "call_closed" {
				if t, ok := closeParen(v); ok {
					return t, true
				}
			}
		}
	}
	return lexer.Token{}, false
}

// PrefixBeforeUnfinished returns, for an interrupted call (a call_line that ends
// in "<unfinished ...>"), the text from the call name to the marker with ONE
// trailing space removed, exactly what the old parser kept as the pending
// prefix: `wait4(-1, ` (one space left) for `wait4(-1,  <unfinished ...>` (strace pads with two
// spaces, one is dropped, and the glue with the resumed half re-adds the rest).
// ok is false for any other line.
func PrefixBeforeUnfinished(line string, ast *parser.ASTNode) (prefix string, ok bool) {
	call := Body(ast)
	if call == nil || call.RuleName != "call_line" {
		return "", false
	}
	var name, marker *lexer.Token
	for _, c := range call.Children {
		switch v := c.(type) {
		case lexer.Token:
			if v.TypeName == "SYSCALL" {
				t := v
				name = &t
			}
		case *parser.ASTNode:
			if v.RuleName == "call_open" {
				for _, cc := range v.Children {
					if t, isTok := cc.(lexer.Token); isTok && t.TypeName == "UNFINISHED" {
						m := t
						marker = &m
					}
				}
			}
		}
	}
	if name == nil || marker == nil {
		return "", false
	}
	return strings.TrimSuffix(line[name.Column-1:marker.Column-1], " "), true
}
