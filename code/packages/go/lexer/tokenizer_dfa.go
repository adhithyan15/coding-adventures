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
	result, _ := StartNew[string]("lexer.ClassifyChar", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			op.AddProperty("ok", ok)
			if !ok {
				return rf.Generate(true, false, "eof")
			}
			var class string
			switch {
			case ch == ' ' || ch == '\t' || ch == '\r':
				class = "whitespace"
			case ch == '\n':
				class = "newline"
			case unicode.IsDigit(ch):
				class = "digit"
			case unicode.IsLetter(ch):
				class = "alpha"
			case ch == '_':
				class = "underscore"
			case ch == '"':
				class = "quote"
			case ch == '=':
				class = "equals"
			case ch == '+' || ch == '-' || ch == '*' || ch == '/':
				class = "operator"
			case ch == '(':
				class = "open_paren"
			case ch == ')':
				class = "close_paren"
			case ch == ',':
				class = "comma"
			case ch == ':':
				class = "colon"
			case ch == ';':
				class = "semicolon"
			case ch == '{':
				class = "open_brace"
			case ch == '}':
				class = "close_brace"
			case ch == '[':
				class = "open_bracket"
			case ch == ']':
				class = "close_bracket"
			case ch == '.':
				class = "dot"
			case ch == '!':
				class = "bang"
			default:
				class = "other"
			}
			return rf.Generate(true, false, class)
		}).GetResult()
	return result
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
	result, _ := StartNew[*statemachine.DFA]("lexer.NewTokenizerDFA", nil,
		func(op *Operation[*statemachine.DFA], rf *ResultFactory[*statemachine.DFA]) *OperationResult[*statemachine.DFA] {
			dfa := statemachine.NewDFA(
				tokenizerDFAStates,
				tokenizerDFAAlphabet,
				buildTokenizerDFATransitions(),
				"start",
				[]string{"done"},
				nil,
			)
			return rf.Generate(true, false, dfa)
		}).GetResult()
	return result
}
