package clibuilder

import "testing"

// =========================================================================
// Tests for the error utilities (levenshtein, fuzzyMatch) and error types.
// =========================================================================

func TestLevenshtein_SameString(t *testing.T) {
	if d := levenshtein("hello", "hello"); d != 0 {
		t.Errorf("expected 0, got %d", d)
	}
}

func TestLevenshtein_EmptyStrings(t *testing.T) {
	if d := levenshtein("", ""); d != 0 {
		t.Errorf("expected 0, got %d", d)
	}
	if d := levenshtein("abc", ""); d != 3 {
		t.Errorf("expected 3, got %d", d)
	}
	if d := levenshtein("", "abc"); d != 3 {
		t.Errorf("expected 3, got %d", d)
	}
}

func TestLevenshtein_OneDeletion(t *testing.T) {
	// "commit" vs "comit" — one deletion
	if d := levenshtein("commit", "comit"); d != 1 {
		t.Errorf("expected 1, got %d", d)
	}
}

func TestLevenshtein_OneInsertion(t *testing.T) {
	if d := levenshtein("comit", "commit"); d != 1 {
		t.Errorf("expected 1, got %d", d)
	}
}

func TestLevenshtein_OneSubstitution(t *testing.T) {
	if d := levenshtein("kitten", "sitten"); d != 1 {
		t.Errorf("expected 1, got %d", d)
	}
}

func TestLevenshtein_ComplexEdit(t *testing.T) {
	// "kitten" → "sitting": k→s, e→i, insert g = 3 edits
	if d := levenshtein("kitten", "sitting"); d != 3 {
		t.Errorf("expected 3, got %d", d)
	}
}

func TestFuzzyMatch_CloseMatch(t *testing.T) {
	candidates := []string{"commit", "checkout", "branch", "remote"}
	match, ok := fuzzyMatch("comit", candidates)
	if !ok {
		t.Fatal("expected a fuzzy match")
	}
	if match != "commit" {
		t.Errorf("expected 'commit', got %q", match)
	}
}

func TestFuzzyMatch_NoCloseMatch(t *testing.T) {
	candidates := []string{"commit", "checkout", "branch"}
	_, ok := fuzzyMatch("zzzzz", candidates)
	if ok {
		t.Error("expected no fuzzy match for completely different string")
	}
}

func TestFuzzyMatch_ExactMatch(t *testing.T) {
	candidates := []string{"--verbose", "--version", "--help"}
	match, ok := fuzzyMatch("--verbose", candidates)
	if !ok {
		t.Fatal("expected match")
	}
	if match != "--verbose" {
		t.Errorf("expected '--verbose', got %q", match)
	}
}

func TestFuzzyMatch_EmptyCandidates(t *testing.T) {
	_, ok := fuzzyMatch("anything", []string{})
	if ok {
		t.Error("expected no match for empty candidates")
	}
}
