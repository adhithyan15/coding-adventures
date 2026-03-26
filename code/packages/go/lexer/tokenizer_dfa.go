// Tokenizer DFA -- formal model of the hand-written lexer's dispatch logic.
//
// The hand-written tokenizer in lexer.go has an *implicit* DFA in its main
// loop: it looks at the current character, classifies it, and dispatches to
// the appropriate sub-routine. This file makes that implicit DFA *explicit*
// by defining it as a formal DFA object using the state-machine library.
//
// # States
//
//   - "start"        -- idle, examining the next character
//   - "in_number"    -- reading a sequence of digits
//   - "in_name"      -- reading an identifier
//   - "in_string"    -- reading a string literal
//   - "in_operator"  -- emitting a single-character operator/delimiter
//   - "in_equals"    -- handling = with lookahead for ==
//   - "at_newline"   -- emitting a NEWLINE token
//   - "at_whitespace"-- skipping whitespace
//   - "done"         -- end of input
//   - "error"        -- unexpected character
//
// # How the DFA is used
//
// The DFA does NOT replace the tokenizer's logic. The sub-routines like
// readNumber() and readString() still do the actual work. What the DFA
// provides is a formal, verifiable model of the dispatch decision.
package lexer

import (
	"unicode"

	statemachine "github.com/adhithyan15/coding-adventures/code/packages/go/state-machine"
)

// ClassifyChar maps a rune to its character class for the tokenizer DFA.
//
// Character class table:
//
//	Class           Characters       Triggers
//	"eof"           end of input     EOF token
//	"whitespace"    space/tab/CR     skip whitespace
//	"newline"       \n               NEWLINE token
//	"digit"         0-9              read number
//	"alpha"         a-zA-Z           read name/keyword
//	"underscore"    _                read name/keyword
//	"quote"         "                read string literal
//	"equals"        =                lookahead for = vs ==
//	"operator"      +-*/             simple operator token
//	"open_paren"    (                LPAREN
//	"close_paren"   )                RPAREN
//	"comma"         ,                COMMA
//	"colon"         :                COLON
//	"semicolon"     ;                SEMICOLON
//	"open_brace"    {                LBRACE
//	"close_brace"   }                RBRACE
//	"open_bracket"  [                LBRACKET
//	"close_bracket" ]                RBRACKET
//	"dot"           .                DOT
//	"bang"          !                BANG
//	"other"         everything else  error
func ClassifyChar(ch rune, ok bool) string {
	if !ok {
		return "eof"
	}
	switch {
	case ch == ' ' || ch == '\t' || ch == '\r':
		return "whitespace"
	case ch == '\n':
		return "newline"
	case unicode.IsDigit(ch):
		return "digit"
	case unicode.IsLetter(ch):
		return "alpha"
	case ch == '_':
		return "underscore"
	case ch == '"':
		return "quote"
	case ch == '=':
		return "equals"
	case ch == '+' || ch == '-' || ch == '*' || ch == '/':
		return "operator"
	case ch == '(':
		return "open_paren"
	case ch == ')':
		return "close_paren"
	case ch == ',':
		return "comma"
	case ch == ':':
		return "colon"
	case ch == ';':
		return "semicolon"
	case ch == '{':
		return "open_brace"
	case ch == '}':
		return "close_brace"
	case ch == '[':
		return "open_bracket"
	case ch == ']':
		return "close_bracket"
	case ch == '.':
		return "dot"
	case ch == '!':
		return "bang"
	default:
		return "other"
	}
}

// tokenizerDFAStates is the set of states for the tokenizer DFA.
var tokenizerDFAStates = []string{
	"start", "in_number", "in_name", "in_string",
	"in_operator", "in_equals", "at_newline", "at_whitespace",
	"done", "error",
}

// tokenizerDFAAlphabet is the set of character classes the DFA operates on.
var tokenizerDFAAlphabet = []string{
	"digit", "alpha", "underscore", "quote", "newline", "whitespace",
	"operator", "equals", "open_paren", "close_paren", "comma", "colon",
	"semicolon", "open_brace", "close_brace", "open_bracket",
	"close_bracket", "dot", "bang", "eof", "other",
}

// startDispatch maps each character class to its target state from "start".
var startDispatch = map[string]string{
	"digit":         "in_number",
	"alpha":         "in_name",
	"underscore":    "in_name",
	"quote":         "in_string",
	"newline":       "at_newline",
	"whitespace":    "at_whitespace",
	"operator":      "in_operator",
	"equals":        "in_equals",
	"open_paren":    "in_operator",
	"close_paren":   "in_operator",
	"comma":         "in_operator",
	"colon":         "in_operator",
	"semicolon":     "in_operator",
	"open_brace":    "in_operator",
	"close_brace":   "in_operator",
	"open_bracket":  "in_operator",
	"close_bracket": "in_operator",
	"dot":           "in_operator",
	"bang":          "in_operator",
	"eof":           "done",
	"other":         "error",
}

// buildTokenizerDFATransitions constructs the full transition map.
func buildTokenizerDFATransitions() map[[2]string]string {
	transitions := make(map[[2]string]string)

	// From "start", dispatch based on character class.
	for charClass, target := range startDispatch {
		transitions[[2]string{"start", charClass}] = target
	}

	// All handler states return to "start" on every symbol.
	handlers := []string{
		"in_number", "in_name", "in_string", "in_operator",
		"in_equals", "at_newline", "at_whitespace",
	}
	for _, handler := range handlers {
		for _, symbol := range tokenizerDFAAlphabet {
			transitions[[2]string{handler, symbol}] = "start"
		}
	}

	// "done" loops on itself for every symbol.
	for _, symbol := range tokenizerDFAAlphabet {
		transitions[[2]string{"done", symbol}] = "done"
	}

	// "error" loops on itself for every symbol.
	for _, symbol := range tokenizerDFAAlphabet {
		transitions[[2]string{"error", symbol}] = "error"
	}

	return transitions
}

// NewTokenizerDFA creates a new instance of the tokenizer dispatch DFA.
//
// Each call returns a fresh DFA so callers can process independently.
// The DFA models the top-level character classification dispatch of the
// hand-written tokenizer.
func NewTokenizerDFA() *statemachine.DFA {
	return statemachine.NewDFA(
		tokenizerDFAStates,
		tokenizerDFAAlphabet,
		buildTokenizerDFATransitions(),
		"start",
		[]string{"done"},
		nil,
	)
}
