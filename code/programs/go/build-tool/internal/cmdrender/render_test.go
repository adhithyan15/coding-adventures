// render_test.go -- Tests for the command renderer.
//
// ============================================================================
// TEST ORGANIZATION
// ============================================================================
//
//   1. quoteArg       -- Argument quoting edge cases
//   2. RenderCommand  -- Single command dict to shell string
//   3. RenderCommands -- List of command dicts, including nil filtering
//   4. Error cases    -- Missing keys, wrong types
//
package cmdrender

import (
	"testing"
)

// ============================================================================
// 1. QUOTEARG TESTS
// ============================================================================

func TestQuoteArgSimple(t *testing.T) {
	// Simple arguments without metacharacters pass through unchanged.
	cases := []struct {
		input, expected string
	}{
		{"hello", "hello"},
		{"python", "python"},
		{"-m", "-m"},
		{"pytest", "pytest"},
		{"--cov", "--cov"},
		{"./...", "./..."},
		{"--cov-report=term-missing", "--cov-report=term-missing"},
	}
	for _, c := range cases {
		got := quoteArg(c.input)
		if got != c.expected {
			t.Errorf("quoteArg(%q) = %q, want %q", c.input, got, c.expected)
		}
	}
}

func TestQuoteArgEmpty(t *testing.T) {
	got := quoteArg("")
	if got != `""` {
		t.Errorf("quoteArg(\"\") = %q, want %q", got, `""`)
	}
}

func TestQuoteArgWithSpaces(t *testing.T) {
	got := quoteArg("hello world")
	if got != `"hello world"` {
		t.Errorf("quoteArg(\"hello world\") = %q, want %q", got, `"hello world"`)
	}
}

func TestQuoteArgWithSquareBrackets(t *testing.T) {
	// .[dev] contains [ and ] which are shell metacharacters.
	got := quoteArg(".[dev]")
	if got != `".[dev]"` {
		t.Errorf("quoteArg(\".[dev]\") = %q, want %q", got, `".[dev]"`)
	}
}

func TestQuoteArgWithDoubleQuotes(t *testing.T) {
	got := quoteArg(`say "hi"`)
	if got != `"say \"hi\""` {
		t.Errorf("quoteArg(%q) = %q, want %q", `say "hi"`, got, `"say \"hi\""`)
	}
}

func TestQuoteArgWithBackslash(t *testing.T) {
	got := quoteArg(`path\to\file`)
	if got != `"path\\to\\file"` {
		t.Errorf("quoteArg(%q) = %q, want %q", `path\to\file`, got, `"path\\to\\file"`)
	}
}

func TestQuoteArgWithRedirect(t *testing.T) {
	got := quoteArg("2>/dev/null")
	if got != `"2>/dev/null"` {
		t.Errorf("quoteArg(%q) = %q, want %q", "2>/dev/null", got, `"2>/dev/null"`)
	}
}

// ============================================================================
// 2. RENDERCOMMAND TESTS
// ============================================================================

func TestRenderCommandSimple(t *testing.T) {
	cmd := map[string]interface{}{
		"type":    "cmd",
		"program": "python",
		"args":    []interface{}{"-m", "pytest"},
	}
	got, err := RenderCommand(cmd)
	if err != nil {
		t.Fatal(err)
	}
	if got != "python -m pytest" {
		t.Errorf("RenderCommand = %q, want %q", got, "python -m pytest")
	}
}

func TestRenderCommandNoArgs(t *testing.T) {
	cmd := map[string]interface{}{
		"type":    "cmd",
		"program": "cargo",
	}
	got, err := RenderCommand(cmd)
	if err != nil {
		t.Fatal(err)
	}
	if got != "cargo" {
		t.Errorf("RenderCommand = %q, want %q", got, "cargo")
	}
}

func TestRenderCommandEmptyArgs(t *testing.T) {
	cmd := map[string]interface{}{
		"type":    "cmd",
		"program": "go",
		"args":    []interface{}{},
	}
	got, err := RenderCommand(cmd)
	if err != nil {
		t.Fatal(err)
	}
	if got != "go" {
		t.Errorf("RenderCommand = %q, want %q", got, "go")
	}
}

func TestRenderCommandWithQuotedArgs(t *testing.T) {
	cmd := map[string]interface{}{
		"type":    "cmd",
		"program": "uv",
		"args":    []interface{}{"pip", "install", "--system", "-e", ".[dev]"},
	}
	got, err := RenderCommand(cmd)
	if err != nil {
		t.Fatal(err)
	}
	expected := `uv pip install --system -e ".[dev]"`
	if got != expected {
		t.Errorf("RenderCommand = %q, want %q", got, expected)
	}
}

func TestRenderCommandPytest(t *testing.T) {
	cmd := map[string]interface{}{
		"type":    "cmd",
		"program": "python",
		"args":    []interface{}{"-m", "pytest", "--cov", "--cov-report=term-missing"},
	}
	got, err := RenderCommand(cmd)
	if err != nil {
		t.Fatal(err)
	}
	expected := "python -m pytest --cov --cov-report=term-missing"
	if got != expected {
		t.Errorf("RenderCommand = %q, want %q", got, expected)
	}
}

func TestRenderCommandGoVet(t *testing.T) {
	cmd := map[string]interface{}{
		"type":    "cmd",
		"program": "go",
		"args":    []interface{}{"vet", "./..."},
	}
	got, err := RenderCommand(cmd)
	if err != nil {
		t.Fatal(err)
	}
	if got != "go vet ./..." {
		t.Errorf("RenderCommand = %q, want %q", got, "go vet ./...")
	}
}

func TestRenderCommandCargo(t *testing.T) {
	cmd := map[string]interface{}{
		"type":    "cmd",
		"program": "cargo",
		"args":    []interface{}{"build", "--workspace"},
	}
	got, err := RenderCommand(cmd)
	if err != nil {
		t.Fatal(err)
	}
	if got != "cargo build --workspace" {
		t.Errorf("RenderCommand = %q, want %q", got, "cargo build --workspace")
	}
}

func TestRenderCommandNpmInstall(t *testing.T) {
	cmd := map[string]interface{}{
		"type":    "cmd",
		"program": "npm",
		"args":    []interface{}{"install", "--silent"},
	}
	got, err := RenderCommand(cmd)
	if err != nil {
		t.Fatal(err)
	}
	if got != "npm install --silent" {
		t.Errorf("RenderCommand = %q, want %q", got, "npm install --silent")
	}
}

// ============================================================================
// 3. RENDERCOMMANDS TESTS
// ============================================================================

func TestRenderCommandsList(t *testing.T) {
	cmds := []interface{}{
		map[string]interface{}{
			"type":    "cmd",
			"program": "cargo",
			"args":    []interface{}{"build"},
		},
		map[string]interface{}{
			"type":    "cmd",
			"program": "cargo",
			"args":    []interface{}{"test"},
		},
	}
	got, err := RenderCommands(cmds)
	if err != nil {
		t.Fatal(err)
	}
	if len(got) != 2 {
		t.Fatalf("expected 2 commands, got %d", len(got))
	}
	if got[0] != "cargo build" {
		t.Errorf("commands[0] = %q, want %q", got[0], "cargo build")
	}
	if got[1] != "cargo test" {
		t.Errorf("commands[1] = %q, want %q", got[1], "cargo test")
	}
}

func TestRenderCommandsSkipsNil(t *testing.T) {
	// nil entries (from platform-filtered commands) should be skipped.
	cmds := []interface{}{
		map[string]interface{}{
			"type":    "cmd",
			"program": "cargo",
			"args":    []interface{}{"build"},
		},
		nil, // cmd_linux returned None on macOS
		map[string]interface{}{
			"type":    "cmd",
			"program": "cargo",
			"args":    []interface{}{"test"},
		},
		nil, // another filtered command
	}
	got, err := RenderCommands(cmds)
	if err != nil {
		t.Fatal(err)
	}
	if len(got) != 2 {
		t.Fatalf("expected 2 commands (2 nil skipped), got %d", len(got))
	}
	if got[0] != "cargo build" {
		t.Errorf("commands[0] = %q, want %q", got[0], "cargo build")
	}
	if got[1] != "cargo test" {
		t.Errorf("commands[1] = %q, want %q", got[1], "cargo test")
	}
}

func TestRenderCommandsEmpty(t *testing.T) {
	got, err := RenderCommands([]interface{}{})
	if err != nil {
		t.Fatal(err)
	}
	if got != nil {
		t.Errorf("expected nil for empty list, got %v", got)
	}
}

func TestRenderCommandsAllNil(t *testing.T) {
	got, err := RenderCommands([]interface{}{nil, nil, nil})
	if err != nil {
		t.Fatal(err)
	}
	if got != nil {
		t.Errorf("expected nil for all-nil list, got %v", got)
	}
}

// ============================================================================
// 4. ERROR CASES
// ============================================================================

func TestRenderCommandMissingProgram(t *testing.T) {
	cmd := map[string]interface{}{
		"type": "cmd",
		"args": []interface{}{"test"},
	}
	_, err := RenderCommand(cmd)
	if err == nil {
		t.Error("expected error for missing program")
	}
}

func TestRenderCommandProgramWrongType(t *testing.T) {
	cmd := map[string]interface{}{
		"type":    "cmd",
		"program": 42, // should be string
	}
	_, err := RenderCommand(cmd)
	if err == nil {
		t.Error("expected error for non-string program")
	}
}

func TestRenderCommandArgsWrongType(t *testing.T) {
	cmd := map[string]interface{}{
		"type":    "cmd",
		"program": "go",
		"args":    "not a list", // should be []interface{}
	}
	_, err := RenderCommand(cmd)
	if err == nil {
		t.Error("expected error for non-list args")
	}
}

func TestRenderCommandsNonDictEntry(t *testing.T) {
	cmds := []interface{}{
		"not a dict",
	}
	_, err := RenderCommands(cmds)
	if err == nil {
		t.Error("expected error for non-dict command entry")
	}
}
