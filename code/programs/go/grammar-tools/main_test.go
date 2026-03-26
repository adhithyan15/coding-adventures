package main

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// grammarsDir returns the path to the repo's code/grammars directory.
func grammarsDir(t *testing.T) string {
	t.Helper()
	root := findRoot()
	return filepath.Join(root, "code", "grammars")
}

// grammarFile returns the path to a file in code/grammars, skipping the test
// if the file does not exist (it may not be present in all CI environments).
func grammarFile(t *testing.T, name string) string {
	t.Helper()
	path := filepath.Join(grammarsDir(t), name)
	if _, err := os.Stat(path); os.IsNotExist(err) {
		t.Skipf("grammar file not found: %s", path)
	}
	return path
}

// ----------------------------------------------------------------------------
// validateCommand
// ----------------------------------------------------------------------------

func TestValidateCommandSucceedsOnJSON(t *testing.T) {
	tokens := grammarFile(t, "json.tokens")
	grammar := grammarFile(t, "json.grammar")
	if code := validateCommand(tokens, grammar); code != 0 {
		t.Errorf("expected exit 0, got %d", code)
	}
}

func TestValidateCommandSucceedsOnLisp(t *testing.T) {
	tokens := grammarFile(t, "lisp.tokens")
	grammar := grammarFile(t, "lisp.grammar")
	if code := validateCommand(tokens, grammar); code != 0 {
		t.Errorf("expected exit 0, got %d", code)
	}
}

func TestValidateCommandMissingTokens(t *testing.T) {
	if code := validateCommand("/nonexistent/x.tokens", "any.grammar"); code != 1 {
		t.Errorf("expected exit 1, got %d", code)
	}
}

func TestValidateCommandMissingGrammar(t *testing.T) {
	tokens := grammarFile(t, "json.tokens")
	if code := validateCommand(tokens, "/nonexistent/x.grammar"); code != 1 {
		t.Errorf("expected exit 1, got %d", code)
	}
}

// ----------------------------------------------------------------------------
// validateTokensOnly
// ----------------------------------------------------------------------------

func TestValidateTokensOnlySucceeds(t *testing.T) {
	tokens := grammarFile(t, "json.tokens")
	if code := validateTokensOnly(tokens); code != 0 {
		t.Errorf("expected exit 0, got %d", code)
	}
}

func TestValidateTokensOnlyMissingFile(t *testing.T) {
	if code := validateTokensOnly("/nonexistent/x.tokens"); code != 1 {
		t.Errorf("expected exit 1, got %d", code)
	}
}

// ----------------------------------------------------------------------------
// validateGrammarOnly
// ----------------------------------------------------------------------------

func TestValidateGrammarOnlySucceeds(t *testing.T) {
	grammar := grammarFile(t, "json.grammar")
	if code := validateGrammarOnly(grammar); code != 0 {
		t.Errorf("expected exit 0, got %d", code)
	}
}

func TestValidateGrammarOnlyMissingFile(t *testing.T) {
	if code := validateGrammarOnly("/nonexistent/x.grammar"); code != 1 {
		t.Errorf("expected exit 1, got %d", code)
	}
}

// ----------------------------------------------------------------------------
// dispatch
// ----------------------------------------------------------------------------

func TestDispatchUnknownCommandReturns2(t *testing.T) {
	if code := dispatch("unknown", []string{}, ""); code != 2 {
		t.Errorf("expected exit 2, got %d", code)
	}
}

func TestDispatchValidateWrongCountReturns2(t *testing.T) {
	if code := dispatch("validate", []string{"only-one.tokens"}, ""); code != 2 {
		t.Errorf("expected exit 2, got %d", code)
	}
}

func TestDispatchValidateTokensNoFilesReturns2(t *testing.T) {
	if code := dispatch("validate-tokens", []string{}, ""); code != 2 {
		t.Errorf("expected exit 2, got %d", code)
	}
}

func TestDispatchValidateGrammarNoFilesReturns2(t *testing.T) {
	if code := dispatch("validate-grammar", []string{}, ""); code != 2 {
		t.Errorf("expected exit 2, got %d", code)
	}
}

func TestDispatchValidateDispatches(t *testing.T) {
	tokens := grammarFile(t, "json.tokens")
	grammar := grammarFile(t, "json.grammar")
	if code := dispatch("validate", []string{tokens, grammar}, ""); code != 0 {
		t.Errorf("expected exit 0, got %d", code)
	}
}

// ----------------------------------------------------------------------------
// compileTokensCommand
// ----------------------------------------------------------------------------

func TestCompileTokensCommandSucceeds(t *testing.T) {
	tokens := grammarFile(t, "json.tokens")
	if code := compileTokensCommand(tokens, ""); code != 0 {
		t.Errorf("expected exit 0, got %d", code)
	}
}

func TestCompileTokensCommandMissingFile(t *testing.T) {
	if code := compileTokensCommand("/nonexistent/x.tokens", ""); code != 1 {
		t.Errorf("expected exit 1, got %d", code)
	}
}

func TestCompileTokensCommandWritesFile(t *testing.T) {
	tokens := grammarFile(t, "json.tokens")
	out := filepath.Join(t.TempDir(), "json_tokens.go")
	if code := compileTokensCommand(tokens, out); code != 0 {
		t.Errorf("expected exit 0, got %d", code)
	}
	data, err := os.ReadFile(out)
	if err != nil {
		t.Fatalf("output file not written: %v", err)
	}
	content := string(data)
	if !strings.Contains(content, "TokenGrammarData") {
		t.Error("expected 'TokenGrammarData' in generated code")
	}
	if !strings.Contains(content, "DO NOT EDIT") {
		t.Error("expected 'DO NOT EDIT' header in generated code")
	}
}

// ----------------------------------------------------------------------------
// compileGrammarCommand
// ----------------------------------------------------------------------------

func TestCompileGrammarCommandSucceeds(t *testing.T) {
	grammar := grammarFile(t, "json.grammar")
	if code := compileGrammarCommand(grammar, ""); code != 0 {
		t.Errorf("expected exit 0, got %d", code)
	}
}

func TestCompileGrammarCommandMissingFile(t *testing.T) {
	if code := compileGrammarCommand("/nonexistent/x.grammar", ""); code != 1 {
		t.Errorf("expected exit 1, got %d", code)
	}
}

func TestCompileGrammarCommandWritesFile(t *testing.T) {
	grammar := grammarFile(t, "json.grammar")
	out := filepath.Join(t.TempDir(), "json_parser.go")
	if code := compileGrammarCommand(grammar, out); code != 0 {
		t.Errorf("expected exit 0, got %d", code)
	}
	data, err := os.ReadFile(out)
	if err != nil {
		t.Fatalf("output file not written: %v", err)
	}
	content := string(data)
	if !strings.Contains(content, "ParserGrammarData") {
		t.Error("expected 'ParserGrammarData' in generated code")
	}
	if !strings.Contains(content, "DO NOT EDIT") {
		t.Error("expected 'DO NOT EDIT' header in generated code")
	}
}

// ----------------------------------------------------------------------------
// dispatch — compile commands
// ----------------------------------------------------------------------------

func TestDispatchCompileTokensNoFilesReturns2(t *testing.T) {
	if code := dispatch("compile-tokens", []string{}, ""); code != 2 {
		t.Errorf("expected exit 2, got %d", code)
	}
}

func TestDispatchCompileGrammarNoFilesReturns2(t *testing.T) {
	if code := dispatch("compile-grammar", []string{}, ""); code != 2 {
		t.Errorf("expected exit 2, got %d", code)
	}
}

func TestDispatchCompileTokensDispatches(t *testing.T) {
	tokens := grammarFile(t, "json.tokens")
	if code := dispatch("compile-tokens", []string{tokens}, ""); code != 0 {
		t.Errorf("expected exit 0, got %d", code)
	}
}

func TestDispatchCompileGrammarDispatches(t *testing.T) {
	grammar := grammarFile(t, "json.grammar")
	if code := dispatch("compile-grammar", []string{grammar}, ""); code != 0 {
		t.Errorf("expected exit 0, got %d", code)
	}
}

