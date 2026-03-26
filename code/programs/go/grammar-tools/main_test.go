package main

import (
	"os"
	"path/filepath"
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
	if code := dispatch("unknown", []string{}); code != 2 {
		t.Errorf("expected exit 2, got %d", code)
	}
}

func TestDispatchValidateWrongCountReturns2(t *testing.T) {
	if code := dispatch("validate", []string{"only-one.tokens"}); code != 2 {
		t.Errorf("expected exit 2, got %d", code)
	}
}

func TestDispatchValidateTokensNoFilesReturns2(t *testing.T) {
	if code := dispatch("validate-tokens", []string{}); code != 2 {
		t.Errorf("expected exit 2, got %d", code)
	}
}

func TestDispatchValidateGrammarNoFilesReturns2(t *testing.T) {
	if code := dispatch("validate-grammar", []string{}); code != 2 {
		t.Errorf("expected exit 2, got %d", code)
	}
}

func TestDispatchValidateDispatches(t *testing.T) {
	tokens := grammarFile(t, "json.tokens")
	grammar := grammarFile(t, "json.grammar")
	if code := dispatch("validate", []string{tokens, grammar}); code != 0 {
		t.Errorf("expected exit 0, got %d", code)
	}
}
