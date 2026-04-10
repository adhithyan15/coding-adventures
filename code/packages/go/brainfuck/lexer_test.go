// Brainfuck lexer tests.
//
// These tests verify that TokenizeBrainfuck correctly converts raw Brainfuck
// source text into a flat stream of tokens. The key behaviors we test:
//
//  1. All eight command characters produce the correct token types
//  2. Comments (non-command characters) are silently consumed
//  3. Whitespace is silently consumed
//  4. Line and column numbers are tracked accurately
//  5. The empty string produces only an EOF token
//  6. Complex programs tokenize into the expected exact sequence
//
// Every test follows the same pattern:
//   1. Call TokenizeBrainfuck with a known source string
//   2. Assert no error
//   3. Assert the token stream matches expected type names, values, and positions
//
// We use TypeName (e.g., "INC", "RIGHT") rather than the numeric TokenType
// constant because Brainfuck tokens are grammar-driven (they don't map to
// any of the hand-coded TokenType enum values in the lexer package).
package brainfuck

import (
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// =============================================================================
// TestTokenizeBrainfuck_AllEightCommands
// =============================================================================
//
// Verifies that each of the eight Brainfuck commands produces the correct
// token type when they all appear together in a single string. After all eight
// commands, the stream must end with an EOF token.
//
// The eight commands and their expected TypeNames:
//   >   RIGHT
//   <   LEFT
//   +   INC
//   -   DEC
//   .   OUTPUT
//   ,   INPUT
//   [   LOOP_START
//   ]   LOOP_END
func TestTokenizeBrainfuck_AllEightCommands(t *testing.T) {
	// All eight commands in one string, no whitespace or comments
	source := "><+-.,[]"
	tokens, err := TokenizeBrainfuck(source)
	if err != nil {
		t.Fatalf("Unexpected error tokenizing %q: %v", source, err)
	}

	// We expect exactly 9 tokens: one per command plus EOF
	expectedTypeNames := []string{
		"RIGHT",
		"LEFT",
		"INC",
		"DEC",
		"OUTPUT",
		"INPUT",
		"LOOP_START",
		"LOOP_END",
		"EOF",
	}

	if len(tokens) != len(expectedTypeNames) {
		t.Fatalf("Expected %d tokens, got %d: %v", len(expectedTypeNames), len(tokens), tokens)
	}

	// Verify each token has the correct TypeName and the correct character value
	expectedValues := []string{">", "<", "+", "-", ".", ",", "[", "]", ""}
	for i, want := range expectedTypeNames {
		tok := tokens[i]
		if tok.TypeName != want {
			t.Errorf("Token[%d]: expected TypeName %q, got %q (value=%q)", i, want, tok.TypeName, tok.Value)
		}
		if tok.Value != expectedValues[i] {
			t.Errorf("Token[%d]: expected Value %q, got %q", i, expectedValues[i], tok.Value)
		}
	}
}

// =============================================================================
// TestTokenizeBrainfuck_CommentsSkipped
// =============================================================================
//
// Verifies that non-command characters (comments) are silently consumed and
// never appear in the token stream.
//
// In Brainfuck there is no dedicated comment syntax. Any character that is not
// one of the eight commands is a comment. The text "increment" after "++" in
// the source below is a comment; it should produce no tokens.
func TestTokenizeBrainfuck_CommentsSkipped(t *testing.T) {
	// Two INC commands followed by a plain-English comment
	source := "++ increment"
	tokens, err := TokenizeBrainfuck(source)
	if err != nil {
		t.Fatalf("Unexpected error tokenizing %q: %v", source, err)
	}

	// Expected: INC INC EOF — "increment" is a COMMENT skip and is discarded
	if len(tokens) != 3 {
		t.Fatalf("Expected 3 tokens (INC INC EOF), got %d: %v", len(tokens), tokens)
	}

	if tokens[0].TypeName != "INC" {
		t.Errorf("Token[0]: expected INC, got %q", tokens[0].TypeName)
	}
	if tokens[1].TypeName != "INC" {
		t.Errorf("Token[1]: expected INC, got %q", tokens[1].TypeName)
	}
	if tokens[2].TypeName != "EOF" {
		t.Errorf("Token[2]: expected EOF, got %q", tokens[2].TypeName)
	}

	// Extra: verify the COMMENT token type is absent entirely
	for _, tok := range tokens {
		if tok.TypeName == "COMMENT" {
			t.Errorf("COMMENT token should be skipped, but found: %v", tok)
		}
	}
}

// =============================================================================
// TestTokenizeBrainfuck_LineColumnTracking
// =============================================================================
//
// Verifies that the lexer correctly tracks line and column numbers across
// newlines. The WHITESPACE skip pattern consumes newlines while the lexer
// increments its internal line counter — this is what lets us report accurate
// positions even when comments and whitespace appear before commands.
//
// Input layout (1-indexed lines and columns):
//
//	Line 1: "++"      positions: col 1, col 2
//	Line 2: ">>"      positions: col 1, col 2
func TestTokenizeBrainfuck_LineColumnTracking(t *testing.T) {
	source := "++\n>>"
	tokens, err := TokenizeBrainfuck(source)
	if err != nil {
		t.Fatalf("Unexpected error tokenizing %q: %v", source, err)
	}

	// Expected token stream: INC INC RIGHT RIGHT EOF
	// Token positions:
	//   tokens[0] INC    at line 1, col 1
	//   tokens[1] INC    at line 1, col 2
	//   tokens[2] RIGHT  at line 2, col 1
	//   tokens[3] RIGHT  at line 2, col 2
	if len(tokens) < 5 {
		t.Fatalf("Expected at least 5 tokens, got %d: %v", len(tokens), tokens)
	}

	type posCheck struct {
		typeName string
		line     int
		col      int
	}
	checks := []posCheck{
		{"INC", 1, 1},
		{"INC", 1, 2},
		{"RIGHT", 2, 1},
		{"RIGHT", 2, 2},
	}

	for i, check := range checks {
		tok := tokens[i]
		if tok.TypeName != check.typeName {
			t.Errorf("Token[%d]: expected TypeName %q, got %q", i, check.typeName, tok.TypeName)
		}
		if tok.Line != check.line {
			t.Errorf("Token[%d] (%s): expected Line %d, got %d", i, tok.TypeName, check.line, tok.Line)
		}
		if tok.Column != check.col {
			t.Errorf("Token[%d] (%s): expected Column %d, got %d", i, tok.TypeName, check.col, tok.Column)
		}
	}
}

// =============================================================================
// TestTokenizeBrainfuck_EmptySource
// =============================================================================
//
// Verifies that an empty input string produces exactly one token: the EOF
// sentinel at line 1, column 1. An empty Brainfuck program is valid — it
// corresponds to a program that does nothing.
func TestTokenizeBrainfuck_EmptySource(t *testing.T) {
	source := ""
	tokens, err := TokenizeBrainfuck(source)
	if err != nil {
		t.Fatalf("Unexpected error tokenizing empty source: %v", err)
	}

	// Should produce exactly one token: EOF
	if len(tokens) != 1 {
		t.Fatalf("Expected 1 token (EOF) for empty source, got %d: %v", len(tokens), tokens)
	}

	eofTok := tokens[0]

	// The EOF token has a special lexer.TokenEOF type
	if eofTok.Type != lexer.TokenEOF {
		t.Errorf("Expected TokenEOF type, got %v", eofTok.Type)
	}

	// EOF should be at the start of the file since there is no content
	if eofTok.Line != 1 {
		t.Errorf("Expected EOF at Line 1, got %d", eofTok.Line)
	}
	if eofTok.Column != 1 {
		t.Errorf("Expected EOF at Column 1, got %d", eofTok.Column)
	}
}

// =============================================================================
// TestTokenizeBrainfuck_OnlyComments
// =============================================================================
//
// Verifies that a source string consisting entirely of comment characters
// (no Brainfuck commands) produces only an EOF token. This exercises the
// comment skip path exclusively.
//
// "hello world" contains no Brainfuck command characters. Both words are
// consumed by the COMMENT skip pattern (non-command non-whitespace chars)
// and the space between them is consumed by the WHITESPACE skip pattern.
func TestTokenizeBrainfuck_OnlyComments(t *testing.T) {
	source := "hello world"
	tokens, err := TokenizeBrainfuck(source)
	if err != nil {
		t.Fatalf("Unexpected error tokenizing comment-only source: %v", err)
	}

	// The entire source is consumed as skip tokens, leaving only EOF
	if len(tokens) != 1 {
		t.Fatalf("Expected 1 token (EOF) for comment-only source, got %d: %v", len(tokens), tokens)
	}

	if tokens[0].Type != lexer.TokenEOF {
		t.Errorf("Expected EOF token, got TypeName=%q Value=%q", tokens[0].TypeName, tokens[0].Value)
	}
}

// =============================================================================
// TestTokenizeBrainfuck_CanonicalProgram
// =============================================================================
//
// Verifies the exact token stream for the canonical "increment and loop"
// program: "++[>+<-]"
//
// This program:
//   - Sets cell 0 to 2 (two INC commands)
//   - Enters a loop (LOOP_START)
//   - Moves right (RIGHT), increments cell 1 (INC)
//   - Moves left (LEFT), decrements cell 0 (DEC)
//   - Exits loop when cell 0 reaches zero (LOOP_END)
//
// After the loop, cell 0 = 0 and cell 1 = 2. This is the basic "copy cell"
// building block of Brainfuck programming.
func TestTokenizeBrainfuck_CanonicalProgram(t *testing.T) {
	source := "++[>+<-]"
	tokens, err := TokenizeBrainfuck(source)
	if err != nil {
		t.Fatalf("Unexpected error tokenizing canonical program: %v", err)
	}

	// Expected token stream with exact positions:
	//   Position: 1234567 8  9(EOF)
	//   Source:   ++[>+<-]
	type tokenCheck struct {
		typeName string
		value    string
		line     int
		col      int
	}
	expected := []tokenCheck{
		{"INC", "+", 1, 1},
		{"INC", "+", 1, 2},
		{"LOOP_START", "[", 1, 3},
		{"RIGHT", ">", 1, 4},
		{"INC", "+", 1, 5},
		{"LEFT", "<", 1, 6},
		{"DEC", "-", 1, 7},
		{"LOOP_END", "]", 1, 8},
		{"EOF", "", 1, 9},
	}

	if len(tokens) != len(expected) {
		t.Fatalf("Expected %d tokens, got %d: %v", len(expected), len(tokens), tokens)
	}

	for i, exp := range expected {
		tok := tokens[i]
		if tok.TypeName != exp.typeName {
			t.Errorf("Token[%d]: expected TypeName %q, got %q", i, exp.typeName, tok.TypeName)
		}
		if tok.Value != exp.value {
			t.Errorf("Token[%d] (%s): expected Value %q, got %q", i, exp.typeName, exp.value, tok.Value)
		}
		if tok.Line != exp.line {
			t.Errorf("Token[%d] (%s): expected Line %d, got %d", i, exp.typeName, exp.line, tok.Line)
		}
		if tok.Column != exp.col {
			t.Errorf("Token[%d] (%s): expected Column %d, got %d", i, exp.typeName, exp.col, tok.Column)
		}
	}
}

// =============================================================================
// TestTokenizeBrainfuck_MultilineWithComments
// =============================================================================
//
// Verifies that a realistic Brainfuck program with embedded comments on
// multiple lines tokenizes correctly, with accurate line/column tracking
// even after comments cause gaps in the command stream.
//
// The program is a typical "clear cell" idiom with annotations:
//
//	Set cell to 5        ← comment on line 1
//	+++++                ← five INC commands on line 2
//	Loop to clear        ← comment on line 3
//	[-]                  ← loop with DEC on line 4
func TestTokenizeBrainfuck_MultilineWithComments(t *testing.T) {
	source := "Set cell to 5\n+++++\nLoop to clear\n[-]"
	tokens, err := TokenizeBrainfuck(source)
	if err != nil {
		t.Fatalf("Unexpected error tokenizing multiline program: %v", err)
	}

	// The comments on lines 1 and 3 are skipped entirely.
	// Commands are: 5×INC on line 2, then LOOP_START DEC LOOP_END on line 4.
	// Expected: INC INC INC INC INC LOOP_START DEC LOOP_END EOF
	if len(tokens) != 9 {
		t.Fatalf("Expected 9 tokens, got %d: %v", len(tokens), tokens)
	}

	// Verify the five INC commands are all on line 2
	for i := 0; i < 5; i++ {
		tok := tokens[i]
		if tok.TypeName != "INC" {
			t.Errorf("Token[%d]: expected INC, got %q", i, tok.TypeName)
		}
		if tok.Line != 2 {
			t.Errorf("Token[%d] (INC): expected Line 2, got %d", i, tok.Line)
		}
		if tok.Column != i+1 {
			t.Errorf("Token[%d] (INC): expected Column %d, got %d", i, i+1, tok.Column)
		}
	}

	// Verify the loop commands are on line 4
	loopStart := tokens[5]
	if loopStart.TypeName != "LOOP_START" {
		t.Errorf("Token[5]: expected LOOP_START, got %q", loopStart.TypeName)
	}
	if loopStart.Line != 4 {
		t.Errorf("LOOP_START: expected Line 4, got %d", loopStart.Line)
	}

	dec := tokens[6]
	if dec.TypeName != "DEC" {
		t.Errorf("Token[6]: expected DEC, got %q", dec.TypeName)
	}
	if dec.Line != 4 {
		t.Errorf("DEC: expected Line 4, got %d", dec.Line)
	}

	loopEnd := tokens[7]
	if loopEnd.TypeName != "LOOP_END" {
		t.Errorf("Token[7]: expected LOOP_END, got %q", loopEnd.TypeName)
	}
	if loopEnd.Line != 4 {
		t.Errorf("LOOP_END: expected Line 4, got %d", loopEnd.Line)
	}

	// Verify EOF is the last token
	eof := tokens[8]
	if eof.Type != lexer.TokenEOF {
		t.Errorf("Token[8]: expected EOF, got TypeName=%q", eof.TypeName)
	}
}

// =============================================================================
// TestCreateBrainfuckLexer_ReturnsLexer
// =============================================================================
//
// Verifies that CreateBrainfuckLexer returns a non-nil GrammarLexer and that
// calling Tokenize() on it produces a valid token stream. This tests the
// two-step API (create then tokenize) as opposed to the one-shot
// TokenizeBrainfuck convenience function.
func TestCreateBrainfuckLexer_ReturnsLexer(t *testing.T) {
	source := "+-"
	bfLexer, err := CreateBrainfuckLexer(source)
	if err != nil {
		t.Fatalf("Unexpected error from CreateBrainfuckLexer: %v", err)
	}

	// The returned lexer must not be nil
	if bfLexer == nil {
		t.Fatal("CreateBrainfuckLexer returned nil lexer")
	}

	// Tokenizing should produce INC DEC EOF
	tokens := bfLexer.Tokenize()
	if len(tokens) != 3 {
		t.Fatalf("Expected 3 tokens (INC DEC EOF), got %d: %v", len(tokens), tokens)
	}

	if tokens[0].TypeName != "INC" {
		t.Errorf("Token[0]: expected INC, got %q", tokens[0].TypeName)
	}
	if tokens[1].TypeName != "DEC" {
		t.Errorf("Token[1]: expected DEC, got %q", tokens[1].TypeName)
	}
	if tokens[2].Type != lexer.TokenEOF {
		t.Errorf("Token[2]: expected EOF, got %q", tokens[2].TypeName)
	}
}

// =============================================================================
// TestCapabilityViolation_BadPath
// =============================================================================
//
// Verifies that attempting to read a file path that was not declared in
// required_capabilities.json returns a capability violation error. This
// exercises the _FileCapabilities.ReadFile error path and the
// _capabilityViolationError.Error() string formatting.
//
// The capability system anchors allowed paths at startup. Any other path —
// even a valid file on disk — must be rejected with a descriptive error.
func TestCapabilityViolation_BadPath(t *testing.T) {
	// Create a file capabilities instance and try to read an undeclared path.
	// The _FileCapabilities type is unexported but accessible within the package.
	caps := &_FileCapabilities{}

	_, err := caps.ReadFile("/some/undeclared/path.txt")
	if err == nil {
		t.Fatal("Expected capability violation error for undeclared path, got nil")
	}

	// Verify the error message mentions the key elements of a capability violation
	errMsg := err.Error()
	if errMsg == "" {
		t.Error("Expected non-empty error message from capability violation")
	}

	// The error should reference the category ("fs"), action ("read"), and path
	// to help the developer understand what was blocked and why.
	capViolErr, ok := err.(*_capabilityViolationError)
	if !ok {
		t.Fatalf("Expected *_capabilityViolationError, got %T: %v", err, err)
	}
	if capViolErr.category != "fs" {
		t.Errorf("Expected category 'fs', got %q", capViolErr.category)
	}
	if capViolErr.action != "read" {
		t.Errorf("Expected action 'read', got %q", capViolErr.action)
	}
}
