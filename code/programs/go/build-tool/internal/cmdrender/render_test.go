package cmdrender

import (
	"strings"
	"testing"
)

// TestRenderCommand_Simple verifies that a command with no args renders cleanly.
func TestRenderCommand_Simple(t *testing.T) {
	cmd := map[string]interface{}{
		"type":    "cmd",
		"program": "go",
		"args":    []interface{}{"build", "./..."},
	}
	got, err := RenderCommand(cmd)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	want := "go build ./..."
	if got != want {
		t.Errorf("RenderCommand() = %q, want %q", got, want)
	}
}

// TestRenderCommand_NoArgs verifies rendering when args is omitted.
func TestRenderCommand_NoArgs(t *testing.T) {
	cmd := map[string]interface{}{
		"type":    "cmd",
		"program": "make",
	}
	got, err := RenderCommand(cmd)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if got != "make" {
		t.Errorf("RenderCommand() = %q, want %q", got, "make")
	}
}

// TestRenderCommand_ArgsWithSpaces verifies that args containing spaces are quoted.
func TestRenderCommand_ArgsWithSpaces(t *testing.T) {
	cmd := map[string]interface{}{
		"type":    "cmd",
		"program": "python",
		"args":    []interface{}{"-m", "pytest", "--cov-report=term-missing", "tests/my test/"},
	}
	got, err := RenderCommand(cmd)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	// The arg with spaces should be quoted.
	if !strings.Contains(got, `"tests/my test/"`) {
		t.Errorf("RenderCommand() = %q, expected quoted arg for path with space", got)
	}
}

// TestRenderCommand_ArgsWithBrackets verifies that glob patterns are quoted.
func TestRenderCommand_ArgsWithBrackets(t *testing.T) {
	cmd := map[string]interface{}{
		"type":    "cmd",
		"program": "find",
		"args":    []interface{}{".", "-name", "*.go"},
	}
	got, err := RenderCommand(cmd)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	// "*.go" contains *, which is a shell metacharacter and should be quoted.
	if !strings.Contains(got, `"*.go"`) {
		t.Errorf("RenderCommand() = %q, expected quoted glob pattern", got)
	}
}

// TestRenderCommands_NilFiltered verifies that nil entries are skipped.
func TestRenderCommands_NilFiltered(t *testing.T) {
	cmds := []interface{}{
		map[string]interface{}{
			"type":    "cmd",
			"program": "go",
			"args":    []interface{}{"build", "./..."},
		},
		nil, // should be skipped
		map[string]interface{}{
			"type":    "cmd",
			"program": "go",
			"args":    []interface{}{"test", "./..."},
		},
	}
	got, err := RenderCommands(cmds)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(got) != 2 {
		t.Errorf("RenderCommands() returned %d items, want 2: %v", len(got), got)
	}
	if got[0] != "go build ./..." {
		t.Errorf("got[0] = %q, want %q", got[0], "go build ./...")
	}
	if got[1] != "go test ./..." {
		t.Errorf("got[1] = %q, want %q", got[1], "go test ./...")
	}
}

// TestRenderCommand_MissingProgram verifies error on missing 'program' key.
func TestRenderCommand_MissingProgram(t *testing.T) {
	cmd := map[string]interface{}{
		"type": "cmd",
		"args": []interface{}{"build"},
	}
	_, err := RenderCommand(cmd)
	if err == nil {
		t.Error("expected error for missing 'program' key, got nil")
	}
	if !strings.Contains(err.Error(), "program") {
		t.Errorf("error %q should mention 'program'", err.Error())
	}
}

// TestRenderCommands_NonDictEntry verifies error when a command is not a dict.
func TestRenderCommands_NonDictEntry(t *testing.T) {
	cmds := []interface{}{
		"not a dict",
	}
	_, err := RenderCommands(cmds)
	if err == nil {
		t.Error("expected error for non-dict command, got nil")
	}
}

// TestQuoteArg_EmptyString verifies that empty string becomes "".
func TestQuoteArg_EmptyString(t *testing.T) {
	got := quoteArg("")
	if got != `""` {
		t.Errorf("quoteArg(%q) = %q, want %q", "", got, `""`)
	}
}

// TestQuoteArg_PlainArg verifies that plain args are passed through unchanged.
func TestQuoteArg_PlainArg(t *testing.T) {
	cases := []string{"build", "./...", "-v", "--cover", "go"}
	for _, arg := range cases {
		got := quoteArg(arg)
		if got != arg {
			t.Errorf("quoteArg(%q) = %q, want unchanged %q", arg, got, arg)
		}
	}
}
