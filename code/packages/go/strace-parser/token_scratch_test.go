package straceparser

import (
	"reflect"
	"strings"
	"sync"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
)

func TestLineParserScratchReuseKeepsOwnedResultsAndSourceBorrows(t *testing.T) {
	line := `1  openat(AT_FDCWD</w>, "/e", O_RDONLY) = 3</e>`
	ownedTree, ownedTokens, err := ParseLineTokens(line, DefaultOptions())
	if err != nil {
		t.Fatal(err)
	}
	ownedTokensBefore := append([]lexer.Token(nil), ownedTokens...)
	if ownedTree == nil {
		t.Fatal("ParseLineTokens returned no owning tree")
	}

	var lp LineParser
	if err := lp.ParseLineInto(line, DefaultOptions()); err != nil {
		t.Fatal(err)
	}
	if !reflect.DeepEqual(lp.Tokens(), ownedTokensBefore) {
		t.Fatalf("LineParser tokens differ from owning path:\n got: %#v\nwant: %#v", lp.Tokens(), ownedTokensBefore)
	}
	if len(lp.scratch) == 0 {
		t.Fatal("successful hook-free parse did not retain its token scratch")
	}
	backing := lp.scratch[:cap(lp.scratch)]
	firstSlot := &backing[0]
	args, rest := lp.RawArgs(), lp.TextAfterClose()

	short := `1  close(3</e>) = 0`
	if err := lp.ParseLineInto(short, DefaultOptions()); err != nil {
		t.Fatal(err)
	}
	if len(lp.Tokens()) == 0 || &lp.scratch[:cap(lp.scratch)][0] != firstSlot {
		t.Fatal("LineParser did not reuse its token backing array for the next line")
	}
	if !reflect.DeepEqual(ownedTokens, ownedTokensBefore) {
		t.Fatal("a LineParser call mutated ParseLineTokens' owning token slice")
	}
	if !reflect.DeepEqual(args, []string{"AT_FDCWD</w>", `"/e"`, "O_RDONLY"}) || rest != " = 3</e>" {
		t.Fatalf("source-backed helper strings changed after reuse: args=%q rest=%q", args, rest)
	}
}

func TestLineParserConcurrentIndependentScratch(t *testing.T) {
	lines := []string{
		`1  openat(AT_FDCWD</w>, "/e", O_RDONLY) = 3</e>`,
		`2  close(3</e>) = 0`,
		`3  +++ exited with 0 +++`,
	}
	wants := make([][]lexer.Token, len(lines))
	for i, line := range lines {
		_, toks, err := ParseLineTokens(line, DefaultOptions())
		if err != nil {
			t.Fatal(err)
		}
		wants[i] = toks
	}
	var wg sync.WaitGroup
	errCh := make(chan string, 8)
	for worker := 0; worker < 8; worker++ {
		wg.Add(1)
		go func(worker int) {
			defer wg.Done()
			var lp LineParser
			for repeat := 0; repeat < 100; repeat++ {
				i := (worker + repeat) % len(lines)
				if err := lp.ParseLineInto(lines[i], DefaultOptions()); err != nil {
					errCh <- err.Error()
					return
				}
				if !reflect.DeepEqual(lp.Tokens(), wants[i]) {
					errCh <- "token mismatch"
					return
				}
			}
		}(worker)
	}
	wg.Wait()
	close(errCh)
	for err := range errCh {
		t.Error(err)
	}
}

func TestLineParserPreParseRetainedTokensStayOwned(t *testing.T) {
	var lp LineParser
	var retained []lexer.Token
	opts := DefaultOptions()
	opts.PreParse = []parser.PreParseHook{func(tokens []lexer.Token) []lexer.Token {
		retained = tokens
		return tokens
	}}
	line := `1  openat(AT_FDCWD</w>, "/e", O_RDONLY) = 3</e>`
	if err := lp.ParseLineInto(line, opts); err != nil {
		t.Fatal(err)
	}
	if len(retained) == 0 {
		t.Fatal("PreParse hook did not receive tokens")
	}
	before := append([]lexer.Token(nil), retained...)
	if !reflect.DeepEqual(retained, lp.Tokens()) {
		t.Fatal("PreParse hook and LineParser did not observe the same input tokens")
	}
	for _, next := range []string{`1  close(3</e>) = 0`, `1  +++ exited with 0 +++`} {
		if err := lp.ParseLineInto(next, DefaultOptions()); err != nil {
			t.Fatalf("ParseLineInto(%q): %v", next, err)
		}
	}
	if !reflect.DeepEqual(retained, before) {
		t.Fatalf("later parser calls changed input retained by PreParse:\n got: %#v\nwant: %#v", retained, before)
	}
}

func TestLineParserPreParseReplacementStaysOwned(t *testing.T) {
	var lp LineParser
	var original, replacement []lexer.Token
	opts := DefaultOptions()
	opts.PreParse = []parser.PreParseHook{func(tokens []lexer.Token) []lexer.Token {
		original = tokens
		replacement = append([]lexer.Token(nil), tokens...)
		return replacement
	}}
	line := `1  openat(AT_FDCWD</w>, "/e", O_RDONLY) = 3</e>`
	if err := lp.ParseLineInto(line, opts); err != nil {
		t.Fatal(err)
	}
	if len(original) == 0 || len(replacement) == 0 {
		t.Fatal("PreParse hook did not retain both token slices")
	}
	if &original[0] == &replacement[0] {
		t.Fatal("PreParse replacement unexpectedly shares the input backing array")
	}
	originalBefore := append([]lexer.Token(nil), original...)
	replacementBefore := append([]lexer.Token(nil), replacement...)

	lp.Reset()
	if !reflect.DeepEqual(original, originalBefore) || !reflect.DeepEqual(replacement, replacementBefore) {
		t.Fatal("Reset mutated token slices retained by PreParse")
	}
	if err := lp.ParseLineInto(`1  close(3</e>) = 0`, DefaultOptions()); err != nil {
		t.Fatalf("hook-free parse after Reset: %v", err)
	}
	if !reflect.DeepEqual(original, originalBefore) || !reflect.DeepEqual(replacement, replacementBefore) {
		t.Fatal("later hook-free parse mutated token slices retained by PreParse")
	}
}

func TestLineParserReusesScratchAfterError(t *testing.T) {
	var lp LineParser
	bad := `1  openat(AT_FDCWD</w>, "/e", O_RDONLY`
	if err := lp.ParseLineInto(bad, DefaultOptions()); err == nil {
		t.Fatal("malformed line parsed successfully")
	}
	if len(lp.scratch) == 0 {
		t.Fatal("error case did not retain lexed tokens for reuse")
	}
	good := `1  close(3</e>) = 0`
	wantTree, wantTokens, wantErr := ParseLineTokens(good, DefaultOptions())
	if wantErr != nil {
		t.Fatal(wantErr)
	}
	if err := lp.ParseLineInto(good, DefaultOptions()); err != nil {
		t.Fatalf("valid line after error: %v", err)
	}
	if wantTree == nil || !reflect.DeepEqual(lp.Tokens(), wantTokens) || lp.BodyKind() != "call_line" {
		t.Fatalf("valid line after error differs from owning parse: tokens=%#v body=%q", lp.Tokens(), lp.BodyKind())
	}
}

func TestLineParserResetClearsLiveTokenStrings(t *testing.T) {
	var lp LineParser
	if err := lp.ParseLineInto(`1  openat(AT_FDCWD</w>, "/e", O_RDONLY) = 3</e>`, DefaultOptions()); err != nil {
		t.Fatal(err)
	}
	backing := lp.scratch[:cap(lp.scratch)]
	if len(lp.scratch) == 0 {
		t.Fatal("successful parse had no occupied scratch slots")
	}
	lp.Reset()
	if lp.scratch == nil || len(lp.scratch) != 0 {
		t.Fatalf("small scratch should be retained empty after Reset: %#v", lp.scratch)
	}
	if want := make([]lexer.Token, len(backing)); !reflect.DeepEqual(backing, want) {
		t.Fatalf("Reset left token values or source strings in scratch:\n got: %#v\nwant: %#v", backing, want)
	}
}

func TestLineParserTinyInputsDoNotClearUnusedScratchCapacity(t *testing.T) {
	var lp LineParser
	long := `1  write(1, "` + strings.Repeat("x", 32*1024) + `", 32768) = 32768`
	if err := lp.ParseLineInto(long, DefaultOptions()); err != nil {
		t.Fatalf("long low-token line: %v", err)
	}
	if len(lp.scratch) > 32 {
		t.Fatalf("long line unexpectedly produced %d tokens", len(lp.scratch))
	}

	// Seed a generous retained capacity to make a capacity-wide clear visible
	// without timing assertions. The far tail is outside every parse's length.
	lp.Reset()
	lp.scratch = make([]lexer.Token, 0, 1024)
	backing := lp.scratch[:cap(lp.scratch)]
	marker := lexer.Token{Type: lexer.TokenName, Value: "untouched capacity tail"}
	backing[len(backing)-1] = marker
	for _, line := range []string{`1  close(3</e>) = 0`, `1  +++ exited with 0 +++`, `1  close(4</e>) = 0`} {
		if err := lp.ParseLineInto(line, DefaultOptions()); err != nil {
			t.Fatalf("ParseLineInto(%q): %v", line, err)
		}
		if backing[len(backing)-1] != marker {
			t.Fatal("a tiny parse cleared token slots outside the occupied scratch length")
		}
	}
}

func TestLineParserDropsOversizedScratchAfterClearingReferences(t *testing.T) {
	var lp LineParser
	line := `1  openat(AT_FDCWD</w>, "/e", ` + strings.Repeat("O_RDONLY|", 4000) + `O_CLOEXEC) = 3</e>`
	if err := lp.ParseLineInto(line, DefaultOptions()); err != nil {
		t.Fatalf("large token line: %v", err)
	}
	if cap(lp.scratch) <= maxRetainedTokenCapacity {
		t.Fatalf("fixture scratch capacity = %d, want greater than %d", cap(lp.scratch), maxRetainedTokenCapacity)
	}
	backing := lp.scratch[:cap(lp.scratch)]
	lp.Reset()
	if lp.scratch != nil {
		t.Fatalf("Reset retained oversized scratch capacity %d", cap(lp.scratch))
	}
	if want := make([]lexer.Token, len(backing)); !reflect.DeepEqual(backing, want) {
		t.Fatalf("Reset dropped oversized scratch with stale token/source references:\n got len=%d\nwant zeroed length=%d", len(backing), len(want))
	}
}
