package clibuilder

import (
	"strings"
	"testing"
)

// =========================================================================
// Help generator tests
// =========================================================================

// rootHelpSpec is used for root-level help tests.
const rootHelpSpec = `{
  "cli_builder_spec_version": "1.0",
  "name": "myapp",
  "display_name": "My Application",
  "description": "A demonstration application",
  "version": "1.2.3",
  "global_flags": [
    {
      "id": "verbose",
      "short": "v",
      "long": "verbose",
      "description": "Enable verbose output",
      "type": "boolean"
    }
  ],
  "flags": [
    {
      "id": "output",
      "short": "o",
      "long": "output",
      "description": "Output file",
      "type": "string",
      "value_name": "FILE"
    }
  ],
  "commands": [
    {
      "id": "cmd-run",
      "name": "run",
      "description": "Run the application",
      "arguments": [
        {
          "id": "script",
          "name": "SCRIPT",
          "description": "Script to run",
          "type": "path",
          "required": true
        }
      ]
    },
    {
      "id": "cmd-build",
      "name": "build",
      "description": "Build the application"
    }
  ]
}`

func parseTestSpec(t *testing.T, raw string) map[string]any {
	t.Helper()
	spec, err := LoadSpecFromBytes([]byte(raw))
	if err != nil {
		t.Fatalf("failed to load spec: %v", err)
	}
	return spec
}

func TestHelpGenerator_Root(t *testing.T) {
	spec := parseTestSpec(t, rootHelpSpec)
	hg := NewHelpGenerator(spec, []string{"myapp"})
	text := hg.Generate()

	// Should contain USAGE, DESCRIPTION, COMMANDS, OPTIONS, GLOBAL OPTIONS sections
	sections := []string{"USAGE", "DESCRIPTION", "COMMANDS", "OPTIONS", "GLOBAL OPTIONS"}
	for _, s := range sections {
		if !strings.Contains(text, s) {
			t.Errorf("expected help text to contain section %q\nGot:\n%s", s, text)
		}
	}

	// Should mention the program name
	if !strings.Contains(text, "myapp") {
		t.Errorf("expected help text to mention 'myapp'\nGot:\n%s", text)
	}

	// Should list commands
	if !strings.Contains(text, "run") {
		t.Errorf("expected help text to list 'run' command\nGot:\n%s", text)
	}
	if !strings.Contains(text, "build") {
		t.Errorf("expected help text to list 'build' command\nGot:\n%s", text)
	}

	// Should mention --output flag
	if !strings.Contains(text, "output") {
		t.Errorf("expected help text to mention 'output' flag\nGot:\n%s", text)
	}

	// Should mention --verbose global flag
	if !strings.Contains(text, "verbose") {
		t.Errorf("expected help text to mention 'verbose' global flag\nGot:\n%s", text)
	}

	// Should include builtin --help
	if !strings.Contains(text, "help") {
		t.Errorf("expected help text to mention 'help' builtin\nGot:\n%s", text)
	}
}

func TestHelpGenerator_Subcommand(t *testing.T) {
	spec := parseTestSpec(t, rootHelpSpec)
	hg := NewHelpGenerator(spec, []string{"myapp", "run"})
	text := hg.Generate()

	// Should contain USAGE and ARGUMENTS sections for the subcommand
	if !strings.Contains(text, "USAGE") {
		t.Errorf("expected USAGE section\nGot:\n%s", text)
	}

	// Should mention the SCRIPT argument
	if !strings.Contains(text, "SCRIPT") {
		t.Errorf("expected SCRIPT argument in help\nGot:\n%s", text)
	}

	// Should include both program and subcommand in usage
	if !strings.Contains(text, "myapp") {
		t.Errorf("expected 'myapp' in usage\nGot:\n%s", text)
	}
	if !strings.Contains(text, "run") {
		t.Errorf("expected 'run' in usage\nGot:\n%s", text)
	}
}

func TestHelpGenerator_NoVersion_NoVersionBuiltin(t *testing.T) {
	// Spec without version field — --version should not appear
	raw := `{
    "cli_builder_spec_version": "1.0",
    "name": "app",
    "description": "test app"
  }`
	spec := parseTestSpec(t, raw)
	hg := NewHelpGenerator(spec, []string{"app"})
	text := hg.Generate()

	if strings.Contains(text, "--version") {
		t.Errorf("expected no --version in help when version not set\nGot:\n%s", text)
	}
}

func TestHelpGenerator_BooleanFlag_NoValuePlaceholder(t *testing.T) {
	spec := parseTestSpec(t, rootHelpSpec)
	hg := NewHelpGenerator(spec, []string{"myapp"})
	text := hg.Generate()

	// --verbose is boolean — should NOT have a <VALUE> placeholder
	// We look for -v, --verbose followed by description, not <BOOLEAN>
	if strings.Contains(text, "--verbose <") {
		t.Errorf("boolean flag should not have value placeholder\nGot:\n%s", text)
	}
}

func TestHelpGenerator_NonBooleanFlag_HasValuePlaceholder(t *testing.T) {
	spec := parseTestSpec(t, rootHelpSpec)
	hg := NewHelpGenerator(spec, []string{"myapp"})
	text := hg.Generate()

	// --output is string — should have a <FILE> placeholder (value_name is "FILE")
	if !strings.Contains(text, "<FILE>") {
		t.Errorf("string flag should have <FILE> value placeholder\nGot:\n%s", text)
	}
}

func TestHelpGenerator_RequiredArgument_AngleBrackets(t *testing.T) {
	spec := parseTestSpec(t, rootHelpSpec)
	hg := NewHelpGenerator(spec, []string{"myapp", "run"})
	text := hg.Generate()

	// Required SCRIPT argument → <SCRIPT>
	if !strings.Contains(text, "<SCRIPT>") {
		t.Errorf("required argument should use <NAME> format\nGot:\n%s", text)
	}
}

func TestHelpGenerator_CommandPath_Unknown(t *testing.T) {
	// If command path leads to unknown subcommand, fallback to root
	spec := parseTestSpec(t, rootHelpSpec)
	hg := NewHelpGenerator(spec, []string{"myapp", "nonexistent"})
	text := hg.Generate()
	// Should still generate something (root help as fallback)
	if text == "" {
		t.Error("expected non-empty help text even for unknown subcommand path")
	}
}

func TestArgUsageToken(t *testing.T) {
	tests := []struct {
		arg      map[string]any
		expected string
	}{
		{map[string]any{"name": "FILE", "required": true, "variadic": false}, "<FILE>"},
		{map[string]any{"name": "FILE", "required": false, "variadic": false}, "[FILE]"},
		{map[string]any{"name": "FILE", "required": true, "variadic": true}, "<FILE>..."},
		{map[string]any{"name": "FILE", "required": false, "variadic": true}, "[FILE...]"},
	}

	for _, tt := range tests {
		got := argUsageToken(tt.arg)
		if got != tt.expected {
			t.Errorf("argUsageToken(%v): expected %q, got %q", tt.arg, tt.expected, got)
		}
	}
}
