package pythonlexer

import (
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// ---------------------------------------------------------------------------
// Helper: collect token types as strings for readable assertions.
// ---------------------------------------------------------------------------

func tokenTypes(tokens []lexer.Token) []string {
	types := make([]string, len(tokens))
	for i, t := range tokens {
		types[i] = t.EffectiveTypeName()
	}
	return types
}

func tokenValues(tokens []lexer.Token) []string {
	values := make([]string, len(tokens))
	for i, t := range tokens {
		values[i] = t.Value
	}
	return values
}

// ---------------------------------------------------------------------------
// Basic tokenization — should work on all versions.
// ---------------------------------------------------------------------------

func TestTokenizeSimpleAssignment(t *testing.T) {
	source := "x = 1\n"
	tokens, err := TokenizePython(source, "3.12")
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}
	if len(tokens) < 3 {
		t.Fatalf("Expected at least 3 tokens (NAME, EQUALS, INT), got %d", len(tokens))
	}
}

func TestTokenizeDefaultVersion(t *testing.T) {
	// Empty string should default to 3.12.
	tokens, err := TokenizePython("x = 1\n", "")
	if err != nil {
		t.Fatalf("Failed to tokenize with default version: %v", err)
	}
	if len(tokens) < 3 {
		t.Fatalf("Expected at least 3 tokens, got %d", len(tokens))
	}
}

// ---------------------------------------------------------------------------
// Version-specific features.
// ---------------------------------------------------------------------------

func TestTokenizePython312Keywords(t *testing.T) {
	// 'if' and 'True' should be keywords in 3.12.
	source := `print("hello") if True else False` + "\n"
	tokens, err := TokenizePython(source, "3.12")
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	// Find 'if' token — should be a keyword (TypeName == "KEYWORD").
	found := false
	for _, tok := range tokens {
		if tok.Value == "if" && tok.EffectiveTypeName() == "KEYWORD" {
			found = true
			break
		}
	}
	if !found {
		// Debug: print all tokens.
		for i, tok := range tokens {
			t.Logf("Token %d: Type=%v TypeName=%q Value=%q", i, tok.Type, tok.TypeName, tok.Value)
		}
		t.Errorf("Expected 'if' to be a KEYWORD token in Python 3.12")
	}
}

func TestTokenizePython27PrintKeyword(t *testing.T) {
	// In Python 2.7, 'print' is a keyword (statement), not a NAME.
	source := "print x\n"
	tokens, err := TokenizePython(source, "2.7")
	if err != nil {
		t.Fatalf("Failed to tokenize Python 2.7: %v", err)
	}

	if len(tokens) < 2 {
		t.Fatalf("Expected at least 2 tokens, got %d", len(tokens))
	}

	// 'print' should be a KEYWORD in 2.7.
	if tokens[0].Value != "print" || tokens[0].Type != lexer.TokenKeyword {
		t.Errorf("Expected 'print' to be KEYWORD in Python 2.7, got type=%s value=%s",
			tokens[0].Type, tokens[0].Value)
	}
}

// ---------------------------------------------------------------------------
// Supported versions.
// ---------------------------------------------------------------------------

func TestAllSupportedVersions(t *testing.T) {
	// Verify that every supported version can tokenize a simple program.
	source := "x = 1\n"
	for _, v := range SupportedVersions {
		t.Run("Python_"+v, func(t *testing.T) {
			tokens, err := TokenizePython(source, v)
			if err != nil {
				t.Fatalf("Failed to tokenize with Python %s: %v", v, err)
			}
			if len(tokens) < 3 {
				t.Fatalf("Expected at least 3 tokens for Python %s, got %d", v, len(tokens))
			}
		})
	}
}

func TestInvalidVersion(t *testing.T) {
	_, err := TokenizePython("x = 1\n", "4.0")
	if err == nil {
		t.Errorf("Expected error for unsupported version 4.0, got nil")
	}
}

// ---------------------------------------------------------------------------
// Grammar caching — second call should be fast (no file re-read).
// ---------------------------------------------------------------------------

func TestGrammarCaching(t *testing.T) {
	// First call loads from disk.
	_, err := TokenizePython("a = 1\n", "3.12")
	if err != nil {
		t.Fatalf("First call failed: %v", err)
	}
	// Second call should use cache.
	_, err = TokenizePython("b = 2\n", "3.12")
	if err != nil {
		t.Fatalf("Second call (cached) failed: %v", err)
	}
}

// ---------------------------------------------------------------------------
// CreatePythonLexer factory.
// ---------------------------------------------------------------------------

func TestCreatePythonLexer(t *testing.T) {
	l, err := CreatePythonLexer("x = 1\n", "3.12")
	if err != nil {
		t.Fatalf("CreatePythonLexer failed: %v", err)
	}
	tokens := l.Tokenize()
	if len(tokens) < 3 {
		t.Fatalf("Expected at least 3 tokens, got %d", len(tokens))
	}
}
