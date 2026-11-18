package lexer

// TokenizeInto keeps the compiled lexer useful to callers that already own a
// token buffer. These tests compare it to the owning API so the storage
// optimization cannot change the lexer's observable results.

import (
	"reflect"
	"sync"
	"testing"

	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
)

func tokenizeIntoStandardGrammar() *grammartools.TokenGrammar {
	return &grammartools.TokenGrammar{
		CaseSensitive:    true,
		Definitions:      []grammartools.TokenDefinition{def("NAME", `[a-zA-Z_]+`, true), def("NUMBER", `[0-9]+`, true), def("PLUS", "+", false), def("EQ", "=", false)},
		SkipDefinitions:  []grammartools.TokenDefinition{def("WS", `[ \t\r\n]+`, true)},
		Keywords:         []string{"let"},
		ContextKeywords:  []string{"async"},
		ReservedKeywords: []string{"forbidden"},
	}
}

func assertTokenizeIntoEquivalent(t *testing.T, c *Compiled, source string, dst []Token) []Token {
	t.Helper()
	want, wantErr := c.Tokenize(source)
	got, gotErr := c.TokenizeInto(source, dst)
	if !reflect.DeepEqual(got, want) {
		t.Errorf("TokenizeInto(%q) tokens differ\n got: %#v\nwant: %#v", source, got, want)
	}
	if (gotErr == nil) != (wantErr == nil) {
		t.Errorf("TokenizeInto(%q) error = %v, Tokenize error = %v", source, gotErr, wantErr)
	} else if gotErr != nil {
		var gotLex, wantLex *LexError
		if !reflect.DeepEqual(gotErr, wantErr) ||
			(!asLexError(gotErr, &gotLex)) || (!asLexError(wantErr, &wantLex)) ||
			!reflect.DeepEqual(gotLex, wantLex) {
			t.Errorf("TokenizeInto(%q) error = (%T) %v, Tokenize error = (%T) %v", source, gotErr, gotErr, wantErr, wantErr)
		}
	}
	return got
}

func asLexError(err error, target **LexError) bool {
	le, ok := err.(*LexError)
	if ok {
		*target = le
	}
	return ok
}

func TestTokenizeIntoMatchesOwningTokenizerAcrossModes(t *testing.T) {
	standard := tokenizeIntoStandardGrammar()
	grouped := fixtureTransitionGrammar()
	grouped.Definitions = append(grouped.Definitions, def("NAME", `[a-z]+`, true))
	grouped.SkipDefinitions = []grammartools.TokenDefinition{def("WS", `[ ]+`, true)}
	grouped.Keywords = []string{"go"}
	cases := []struct {
		name    string
		grammar *grammartools.TokenGrammar
		inputs  []string
	}{
		{"standard", standard, []string{"let async = 23 + name\n", "forbidden", "x = §", ""}},
		{"grouped", grouped, []string{`a / b / c`, `x "inside" y`, `{ raw text } z`, "go go", "a @"}},
		{"layout", &grammartools.TokenGrammar{CaseSensitive: true, Mode: "layout", Definitions: []grammartools.TokenDefinition{def("NAME", `[a-z]+`, true), def("EQ", "=", false)}, SkipDefinitions: []grammartools.TokenDefinition{def("WS", `[ \t\r]+`, true)}}, []string{"module where\n  a = b\n  c = d\ne", "where\n\n  a\n b"}},
		{"indentation", indentGrammar(), []string{"a:\n    b\n    c\nd\n", "a:\n\tb\n", "a:\n    b\n  c\n", ""}},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			c, err := Compile(tc.grammar)
			if err != nil {
				t.Fatal(err)
			}
			for _, source := range tc.inputs {
				assertTokenizeIntoEquivalent(t, c, source, nil)
				// oracleOutcome runs the frozen pre-compiled lexer, so this check
				// stays independent even if Tokenize and TokenizeInto share an emitter.
				want, panicMessage, ok := oracleOutcome(tc.grammar, source)
				if !ok {
					t.Fatalf("legacy oracle could not build a lexer for %q", source)
				}
				got, gotErr := c.TokenizeInto(source, nil)
				if panicMessage != "" {
					if gotErr == nil || gotErr.Error() != panicMessage {
						t.Errorf("TokenizeInto(%q) error = %v, legacy oracle panic = %q", source, gotErr, panicMessage)
					}
				} else if gotErr != nil || !reflect.DeepEqual(got, want) {
					t.Errorf("TokenizeInto(%q) = (%#v, %v), legacy oracle = %#v", source, got, gotErr, want)
				}
			}
		})
	}
}

func TestTokenizeIntoReusesAndReplacesCallerStorage(t *testing.T) {
	standard, err := Compile(tokenizeIntoStandardGrammar())
	if err != nil {
		t.Fatal(err)
	}
	groupedGrammar := fixtureTransitionGrammar()
	groupedGrammar.SkipDefinitions = []grammartools.TokenDefinition{def("WS", `[ ]+`, true)}
	grouped, err := Compile(groupedGrammar)
	if err != nil {
		t.Fatal(err)
	}
	for _, tc := range []struct {
		name string
		c    *Compiled
	}{
		{"standard", standard},
		{"grouped", grouped},
	} {
		t.Run(tc.name, func(t *testing.T) {
			storage := make([]Token, 0, 32)
			first := assertTokenizeIntoEquivalent(t, tc.c, "alpha beta gamma", storage)
			if len(first) == 0 || cap(storage) == 0 || &first[0] != &storage[:cap(storage)][0] {
				t.Fatal("TokenizeInto did not reuse the supplied backing storage")
			}

			// Reuse the previous result for very different lengths and outcomes.
			for _, source := range []string{"x", "x §", "valid tokens again"} {
				storage = assertTokenizeIntoEquivalent(t, tc.c, source, first)
				first = storage
			}

			short := make([]Token, 0, 1)
			replaced := assertTokenizeIntoEquivalent(t, tc.c, "alpha beta gamma", short)
			if len(replaced) == 0 || cap(replaced) < len(replaced) {
				t.Fatalf("replacement result has invalid length/capacity: len=%d cap=%d", len(replaced), cap(replaced))
			}
		})
	}
}

func TestTokenizeIntoDoesNotMutateOwnedOutput(t *testing.T) {
	c, err := Compile(tokenizeIntoStandardGrammar())
	if err != nil {
		t.Fatal(err)
	}
	owned, err := c.Tokenize("let async = 17")
	if err != nil {
		t.Fatal(err)
	}
	before := append([]Token(nil), owned...)
	scratch := make([]Token, 0, len(owned)+8)
	for _, source := range []string{"different result with more tokens", "x", "bad §", "another valid result"} {
		scratch, _ = c.TokenizeInto(source, scratch)
	}
	if !reflect.DeepEqual(owned, before) {
		t.Fatalf("later TokenizeInto call mutated owning Tokenize result\n got: %#v\nwant: %#v", owned, before)
	}
}

func TestTokenizeIntoClearsCallerStorageOnGrowthThenError(t *testing.T) {
	cases := []struct {
		name    string
		input   string
		grammar *grammartools.TokenGrammar
	}{
		{"standard", "++++++++@", tokenizeIntoStandardGrammar()},
		{"case-folded unicode", "K++++++++@", func() *grammartools.TokenGrammar {
			g := tokenizeIntoStandardGrammar()
			g.CaseSensitive = false
			return g
		}()},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			c, err := Compile(tc.grammar)
			if err != nil {
				t.Fatal(err)
			}
			scratch := make([]Token, 0, 5)
			backing := scratch[:cap(scratch)]
			if _, err := c.TokenizeInto(tc.input, scratch); err == nil {
				t.Fatal("TokenizeInto accepted an unexpected character")
			} else if _, ok := err.(*LexError); !ok {
				t.Fatalf("error type = %T, want *LexError", err)
			}
			if want := make([]Token, cap(backing)); !reflect.DeepEqual(backing, want) {
				t.Fatalf("original scratch backing array retains tokens after partial-error growth:\n got: %#v\nwant: %#v", backing, want)
			}
		})
	}
}

func TestTokenizeIntoClearsOldStorageAfterGrowthOnSuccess(t *testing.T) {
	c, err := Compile(tokenizeIntoStandardGrammar())
	if err != nil {
		t.Fatal(err)
	}
	scratch := make([]Token, 0, 4)
	backing := scratch[:cap(scratch)]
	got, err := c.TokenizeInto("++++++++", scratch)
	if err != nil {
		t.Fatal(err)
	}
	if len(got) != 9 || &got[0] == &backing[0] {
		t.Fatalf("expected output growth from the original backing: len=%d", len(got))
	}
	if want := make([]Token, len(backing)); !reflect.DeepEqual(backing, want) {
		t.Fatalf("unused original scratch retains tokens after successful growth:\n got: %#v\nwant: %#v", backing, want)
	}
}

func TestTokenizeIntoConcurrentWithIndependentBuffers(t *testing.T) {
	grammar := fixtureTransitionGrammar()
	grammar.SkipDefinitions = []grammartools.TokenDefinition{def("WS", `[ ]+`, true)}
	c, err := Compile(grammar)
	if err != nil {
		t.Fatal(err)
	}
	inputs := []string{`x "inside" y`, `{ raw text } z`, "a / b / c", "a @ b"}
	var wg sync.WaitGroup
	errCh := make(chan string, len(inputs)*20)
	for _, source := range inputs {
		source := source
		for worker := 0; worker < 20; worker++ {
			wg.Add(1)
			go func() {
				defer wg.Done()
				var scratch []Token
				for i := 0; i < 20; i++ {
					want, wantErr := c.Tokenize(source)
					got, gotErr := c.TokenizeInto(source, scratch)
					if !reflect.DeepEqual(got, want) || !reflect.DeepEqual(gotErr, wantErr) {
						errCh <- source
						return
					}
					scratch = got
				}
			}()
		}
	}
	wg.Wait()
	close(errCh)
	for source := range errCh {
		t.Errorf("concurrent TokenizeInto mismatch for %q", source)
	}
}

func FuzzTokenizeIntoMatchesLegacyAndOwning(f *testing.F) {
	for _, seed := range []string{"", "name + 12", "++++++++@", "K++++++++@", "a\n+b", "x § y"} {
		f.Add(seed)
	}
	sensitive, err := Compile(tokenizeIntoStandardGrammar())
	if err != nil {
		f.Fatal(err)
	}
	caseFoldedGrammar := tokenizeIntoStandardGrammar()
	caseFoldedGrammar.CaseSensitive = false
	caseFolded, err := Compile(caseFoldedGrammar)
	if err != nil {
		f.Fatal(err)
	}
	f.Fuzz(func(t *testing.T, source string) {
		if len(source) > 4096 {
			t.Skip()
		}
		for _, c := range []*Compiled{sensitive, caseFolded} {
			owned, ownedErr := c.Tokenize(source)
			ownedBefore := append([]Token(nil), owned...)
			want, panicMessage, ok := oracleOutcome(c.grammar, source)
			if !ok {
				t.Fatalf("legacy oracle could not build lexer for %q", source)
			}
			if panicMessage == "" {
				if ownedErr != nil || !reflect.DeepEqual(owned, want) {
					t.Fatalf("owning tokenizer differs from legacy oracle for %q: err=%v", source, ownedErr)
				}
			} else if ownedErr == nil || ownedErr.Error() != panicMessage {
				t.Fatalf("owning tokenizer error for %q = %v, legacy panic = %q", source, ownedErr, panicMessage)
			}

			hint := tokenCapacityHint(len(c.newLexer(source).source))
			caps := []int{0, max(0, hint-1), hint, hint + 1}
			for _, capacity := range caps {
				dst := make([]Token, 0, capacity)
				original := dst[:cap(dst)]
				got, gotErr := c.TokenizeInto(source, dst)
				if !reflect.DeepEqual(got, owned) || !reflect.DeepEqual(gotErr, ownedErr) {
					t.Fatalf("TokenizeInto differs from owned for %q with cap %d: err=%v want=%v", source, capacity, gotErr, ownedErr)
				}
				if panicMessage == "" && !reflect.DeepEqual(got, want) {
					t.Fatalf("TokenizeInto differs from legacy for %q with cap %d", source, capacity)
				}
				if !reflect.DeepEqual(owned, ownedBefore) {
					t.Fatalf("TokenizeInto mutated an owned result for %q", source)
				}
				moved := len(got) > 0 && cap(original) > 0 && &got[0] != &original[0]
				if (gotErr != nil || moved) && !reflect.DeepEqual(original, make([]Token, len(original))) {
					t.Fatalf("TokenizeInto left stale values in original scratch for %q with cap %d", source, capacity)
				}
			}
		}
	})
}
