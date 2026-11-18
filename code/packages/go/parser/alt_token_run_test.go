package parser

import (
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"strconv"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
	old "github.com/adhithyan15/coding-adventures/code/packages/go/parser/internal/oldengine"
)

const altTokenRunObservationPrefix = "KILN_G1_SAT_OBS_JSON: "

type altTokenRunObservation struct {
	ID       int      `json:"id"`
	TypeName string   `json:"type_name"`
	Enum     string   `json:"enum"`
	Newline  bool     `json:"newline"`
	Matched  bool     `json:"matched"`
	Position int      `json:"position"`
	Failed   []string `json:"failed"`
}

func altTokenRunDomain(t testing.TB, rowID int) ([]altTokenRunObservation, *Compiled) {
	t.Helper()
	grammar := `p = NUMBER | STRING | NAME ;`
	c := mustCompile(t, grammar)
	if got := c.rules[c.byName["p"]].body.leadingTokens; got != 3 {
		t.Fatalf("bounded observation grammar optimized %d leading tokens, want 3", got)
	}
	names := []string{"", "NUMBER", "STRING", "NAME"}
	enums := []lexer.TokenType{lexer.TokenNumber, lexer.TokenString, lexer.TokenName, lexer.TokenPlus}
	var rows []altTokenRunObservation
	for ni, name := range names {
		for ei, enum := range enums {
			for newline := 0; newline < 2; newline++ {
				id := (ni*len(enums)+ei)*2 + newline
				if rowID >= 0 && rowID != id {
					continue
				}
				toks := make([]lexer.Token, 0, 3)
				if newline != 0 {
					toks = append(toks, lexer.Token{Type: lexer.TokenNewline, TypeName: "NEWLINE", Value: "\n", Line: 1, Column: 1})
				}
				toks = append(toks,
					lexer.Token{Type: enum, TypeName: name, Value: "x", Line: 1, Column: len(toks) + 1},
					lexer.Token{Type: lexer.TokenEOF, TypeName: "EOF", Line: 1, Column: len(toks) + 2},
				)
				s := &parseState{c: c}
				s.reset(toks, Options{})
				matched := s.match(c.rules[c.byName["p"]].body)
				failed := make([]string, len(s.furthestExpected))
				for i, expected := range s.furthestExpected {
					failed[i] = c.expNames[expected]
				}
				rows = append(rows, altTokenRunObservation{
					ID: id, TypeName: name, Enum: enum.String(), Newline: newline != 0,
					Matched: matched, Position: s.pos, Failed: failed,
				})
			}
		}
	}
	return rows, c
}

func TestLeadingTokenRunCompileEligibility(t *testing.T) {
	tok := func(skipNL bool) *node { return &node{kind: kTok, skipNL: skipNL} }
	if got := leadingTokenRunEnd([]*node{{kind: kRule}, tok(true), tok(true)}); got != 0 {
		t.Fatalf("non-token first alternative produced run length %d", got)
	}
	if got := leadingTokenRunEnd([]*node{tok(true), tok(false), tok(true)}); got != 0 {
		t.Fatalf("mixed newline policy at second choice produced run length %d", got)
	}
	if got := leadingTokenRunEnd([]*node{tok(true), tok(true), tok(false), tok(true)}); got != 2 {
		t.Fatalf("run did not stop at newline-policy boundary: %d", got)
	}
}

func TestLeadingTokenRunCapsPrefixAndFallsBackForTail(t *testing.T) {
	const choices = 256
	var grammar strings.Builder
	grammar.WriteString("p = ")
	for i := 0; i < choices; i++ {
		if i > 0 {
			grammar.WriteString(" | ")
		}
		fmt.Fprintf(&grammar, "TOKEN_%03d", i)
	}
	grammar.WriteString(" ;")
	src := grammar.String()
	c := mustCompile(t, src)
	if got := c.rules[c.byName["p"]].body.leadingTokens; got != 255 {
		t.Fatalf("cached run length %d, want cap 255", got)
	}
	tokens := []lexer.Token{
		{Type: lexer.TokenName, TypeName: "TOKEN_255", Value: "x", Line: 1, Column: 1},
		{Type: lexer.TokenEOF, TypeName: "EOF", Line: 1, Column: 2},
	}
	compareTokenRunWithOld(t, src, tokens)
}

func compareTokenRunWithOld(t *testing.T, grammar string, tokens []lexer.Token) {
	t.Helper()
	g := mustGrammar(t, grammar)
	c := mustCompile(t, grammar)
	oldAST, oldErr := old.NewGrammarParser(append([]lexer.Token(nil), tokens...), g).Parse()
	newAST, newErr := c.Parse(tokens, Options{})
	if (oldErr == nil) != (newErr == nil) {
		t.Fatalf("old error %v, new error %v", oldErr, newErr)
	}
	if oldErr != nil {
		if oldErr.Error() != newErr.Error() {
			t.Fatalf("old error %q, new error %q", oldErr, newErr)
		}
		return
	}
	var oldDump, newDump strings.Builder
	dumpOld(oldAST, &oldDump)
	dumpNew(newAST, &newDump)
	if oldDump.String() != newDump.String() {
		t.Fatalf("old tree %s, new tree %s", oldDump.String(), newDump.String())
	}
}

func TestLeadingTokenRunMatchesFrozenEngine(t *testing.T) {
	tests := []struct {
		name    string
		grammar string
		tokens  []lexer.Token
	}{
		{
			name:    "type name and legacy enum both match first alternative",
			grammar: `p = NUMBER | NAME ;`,
			tokens: []lexer.Token{
				{Type: lexer.TokenNumber, TypeName: "NAME", Value: "n", Line: 1, Column: 1},
				{Type: lexer.TokenEOF, TypeName: "EOF", Line: 1, Column: 2},
			},
		},
		{
			name:    "legacy enum only",
			grammar: `p = NUMBER | NAME ;`,
			tokens: []lexer.Token{
				{Type: lexer.TokenNumber, Value: "3", Line: 1, Column: 1},
				{Type: lexer.TokenEOF, Value: "", Line: 1, Column: 2},
			},
		},
		{
			name:    "explicit EOF token",
			grammar: `p = EOF | NAME | NUMBER ;`,
			tokens: []lexer.Token{
				{Type: lexer.TokenEOF, TypeName: "EOF", Value: "", Line: 1, Column: 1},
			},
		},
		{
			name:    "skip newline and roll back before later rule alternative",
			grammar: `p = NUMBER | NAME | item ; item = PUNCT ASSIGN ;`,
			tokens: []lexer.Token{
				{Type: lexer.TokenNewline, TypeName: "NEWLINE", Value: "\n", Line: 1, Column: 1},
				{Type: lexer.TokenName, TypeName: "PUNCT", Value: "!", Line: 2, Column: 1},
				{Type: lexer.TokenEquals, TypeName: "ASSIGN", Value: "=", Line: 2, Column: 2},
				{Type: lexer.TokenEOF, TypeName: "EOF", Value: "", Line: 2, Column: 3},
			},
		},
		{
			name:    "whole run miss reports failure at post-skip token",
			grammar: `p = NUMBER | STRING | NAME ;`,
			tokens: []lexer.Token{
				{Type: lexer.TokenNewline, TypeName: "NEWLINE", Value: "\n", Line: 1, Column: 1},
				{Type: lexer.TokenComma, TypeName: "PUNCT", Value: "?", Line: 2, Column: 1},
				{Type: lexer.TokenEOF, TypeName: "EOF", Value: "", Line: 2, Column: 2},
			},
		},
		{
			name:    "expected prefix preserves alternative order",
			grammar: `p = WORD | ASSIGN | ANNOTATED | PUNCT ;`,
			tokens: []lexer.Token{
				{Type: lexer.TokenComma, TypeName: "QUOTED", Value: "?", Line: 1, Column: 1},
				{Type: lexer.TokenEOF, TypeName: "EOF", Value: "", Line: 1, Column: 2},
			},
		},
	}
	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) { compareTokenRunWithOld(t, tc.grammar, tc.tokens) })
	}

	// Every row of the bounded type-name/enum/newline domain is compared to the
	// frozen independent parser, including both successes and ordered failures.
	rows, _ := altTokenRunDomain(t, -1)
	for _, row := range rows {
		name := fmt.Sprintf("row-%02d-%s-%s-newline=%v", row.ID, row.TypeName, row.Enum, row.Newline)
		t.Run(name, func(t *testing.T) {
			tokens := make([]lexer.Token, 0, 3)
			column := 1
			if row.Newline {
				tokens = append(tokens, lexer.Token{Type: lexer.TokenNewline, TypeName: "NEWLINE", Value: "\n", Line: 1, Column: column})
				column++
			}
			var enum lexer.TokenType
			for _, candidate := range []lexer.TokenType{lexer.TokenNumber, lexer.TokenString, lexer.TokenName, lexer.TokenPlus} {
				if candidate.String() == row.Enum {
					enum = candidate
					break
				}
			}
			tokens = append(tokens,
				lexer.Token{Type: enum, TypeName: row.TypeName, Value: "x", Line: 1, Column: column},
				lexer.Token{Type: lexer.TokenEOF, TypeName: "EOF", Line: 1, Column: column + 1},
			)
			compareTokenRunWithOld(t, `p = NUMBER | STRING | NAME ;`, tokens)
		})
	}
}

// TestLeadingTokenRunBoundedDomain emits test-only observations for the
// optional SAT miter. Ordinary `go test ./...` skips this opt-in exporter and
// never starts Node or any external tool.
func TestLeadingTokenRunBoundedDomain(t *testing.T) {
	mode := os.Getenv("KILN_G1_SAT_MODE")
	if mode == "" {
		t.Skip("opt-in bounded SAT observations")
	}
	rowID := -1
	if mode == "replay" {
		parsed, err := strconv.Atoi(os.Getenv("KILN_G1_SAT_ROW"))
		if err != nil || parsed < 0 || parsed >= 32 {
			t.Fatalf("KILN_G1_SAT_ROW must be an integer in [0,32), got %q", os.Getenv("KILN_G1_SAT_ROW"))
		}
		rowID = parsed
	} else if mode != "all" {
		t.Fatalf("unknown KILN_G1_SAT_MODE %q", mode)
	}
	rows, _ := altTokenRunDomain(t, rowID)
	payload, err := json.Marshal(struct {
		Version int                      `json:"version"`
		Rows    []altTokenRunObservation `json:"rows"`
	}{Version: 1, Rows: rows})
	if err != nil {
		t.Fatal(err)
	}
	fmt.Fprintf(os.Stdout, "%s%s\n", altTokenRunObservationPrefix, payload)
}

func TestLeadingTokenRunPreservesStickyResourceRefusal(t *testing.T) {
	grammar := `p = NUMBER | NAME | nested | STRING | QUOTED ; nested = a ; a = b ; b = c ; c = EOF ;`
	c := mustCompile(t, grammar)
	tokens := []lexer.Token{
		{Type: lexer.TokenComma, TypeName: "PUNCT", Value: "?", Line: 1, Column: 1},
		{Type: lexer.TokenEOF, TypeName: "EOF", Line: 1, Column: 2},
	}
	start := c.byName["p"]
	for _, tc := range []struct {
		name string
		opts Options
		want error
	}{
		{name: "depth", opts: Options{MaxDepth: 1}, want: ErrMaxDepth},
		{name: "memo slots", opts: Options{MaxMemoSlots: 1}, want: ErrMaxMemoSlots},
	} {
		t.Run(tc.name, func(t *testing.T) {
			s := &parseState{c: c}
			s.reset(tokens, tc.opts)
			_, err := s.run(start)
			if !errors.Is(err, tc.want) {
				t.Fatalf("error %v, want %v", err, tc.want)
			}
			got := s.expectedText()
			if got != "NUMBER or NAME or p" {
				t.Fatalf("expected prefix after resource refusal %q", got)
			}
			if strings.Contains(got, "STRING") || strings.Contains(got, "QUOTED") {
				t.Fatalf("recorded alternatives after resource refusal: %q", got)
			}
		})
	}
}

func TestLeadingTokenRunHandlesSyntheticEOF(t *testing.T) {
	c := mustCompile(t, `p = EOF | NAME | NUMBER ;`)
	ast, err := c.Parse(nil, Options{})
	if err != nil {
		t.Fatalf("synthetic EOF should match first alternative: %v", err)
	}
	if ast == nil || len(ast.Children) != 1 {
		t.Fatalf("synthetic EOF tree: %#v", ast)
	}
	got, ok := ast.Children[0].(lexer.Token)
	if !ok || got.Type != lexer.TokenEOF || got.TypeName != "EOF" {
		t.Fatalf("synthetic EOF leaf: %#v", ast.Children[0])
	}

	failed, err := mustCompile(t, `p = NUMBER | STRING | NAME ;`).Parse(nil, Options{})
	if err == nil || failed != nil || err.Error() != `Parse error at 0:0: Expected NUMBER or STRING or NAME or p, got ""` {
		t.Fatalf("synthetic EOF failure tree=%#v err=%v", failed, err)
	}
}
