package grammartools

// f10_test.go — Tests for F10 declarative lexer mode transitions.
//
// F10 adds two directives to the .tokens grammar format:
//
//   start_mode: MODE   — the name of the initial lexer mode (default "default")
//   transitions:       — section whose indented lines declare mode-switch rules
//
// Each rule has the shape:
//
//   on TOKENS [in MODE] -> ACTION [, ACTION ...]
//
// where TOKENS is a bare name, a parenthesised alternation "(A | B | C)", or a
// keyword-value guard KEYWORD="value"; and each ACTION is one of:
//
//   set-mode MODE  push GROUP  pop  enable-skip  disable-skip
//
// Action kinds are stored with underscores (set_mode / enable_skip) to match
// the Python, Ruby, and TypeScript ports; the DSL spells them with hyphens.
//
// These tests mirror the Python and TypeScript F10 test suites so all language
// ports exercise the same contract.

import (
	"strings"
	"testing"
)

// ---------------------------------------------------------------------------
// Backward-compatibility
// ---------------------------------------------------------------------------

func TestF10NoTransitionsIsBackwardCompatible(t *testing.T) {
	// A grammar with no start_mode or transitions: section should parse cleanly
	// and leave both fields at their zero values.  Existing grammars must
	// continue to work without any changes.
	g, err := ParseTokenGrammar("NUMBER = /[0-9]+/")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if g.StartMode != "" {
		t.Errorf("StartMode: got %q, want empty", g.StartMode)
	}
	if len(g.Transitions) != 0 {
		t.Errorf("Transitions: got %d rules, want 0", len(g.Transitions))
	}
}

// ---------------------------------------------------------------------------
// start_mode: parsing
// ---------------------------------------------------------------------------

func TestF10ParseStartMode(t *testing.T) {
	src := "NAME = /[a-z]+/\nstart_mode: div\ngroup div:\n  SLASH = \"/\"\n"
	g, err := ParseTokenGrammar(src)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if g.StartMode != "div" {
		t.Errorf("StartMode: got %q, want %q", g.StartMode, "div")
	}
}

func TestF10ParseStartModeDefault(t *testing.T) {
	// start_mode: default is valid — the lexer just starts in the normal mode.
	src := "NAME = /[a-z]+/\ngroup div:\n  SLASH = \"/\"\nstart_mode: default\n"
	g, err := ParseTokenGrammar(src)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if g.StartMode != "default" {
		t.Errorf("StartMode: got %q, want %q", g.StartMode, "default")
	}
}

func TestF10StartModeMissingValueErrors(t *testing.T) {
	// A start_mode: line with no value after the colon must return an error.
	_, err := ParseTokenGrammar("NAME = /x/\nstart_mode:\n")
	if err == nil {
		t.Fatal("expected error for bare 'start_mode:', got nil")
	}
	if !strings.Contains(err.Error(), "start_mode") {
		t.Errorf("error %q should mention 'start_mode'", err.Error())
	}
}

// ---------------------------------------------------------------------------
// transitions: section parsing
// ---------------------------------------------------------------------------

func TestF10ParseTransitionAlternationAndValueGuard(t *testing.T) {
	// Exercises two common rule shapes:
	//   1. A parenthesised multi-token alternation → one action.
	//   2. A keyword-value guard with two comma-separated actions.
	src := (
		"NAME = /[a-z]+/\n" +
		"transitions:\n" +
		"  on (NAME | NUMBER | RPAREN) -> set-mode div\n" +
		"  on KEYWORD=\"return\" -> set-mode default, pop\n")
	g, err := ParseTokenGrammar(src)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(g.Transitions) != 2 {
		t.Fatalf("got %d transitions, want 2", len(g.Transitions))
	}

	r0 := g.Transitions[0]
	if len(r0.OnTokens) != 3 || r0.OnTokens[0] != "NAME" || r0.OnTokens[1] != "NUMBER" || r0.OnTokens[2] != "RPAREN" {
		t.Errorf("r0.OnTokens: got %v, want [NAME NUMBER RPAREN]", r0.OnTokens)
	}
	if len(r0.Actions) != 1 || r0.Actions[0].Kind != "set_mode" || r0.Actions[0].Target != "div" {
		t.Errorf("r0.Actions: got %v", r0.Actions)
	}

	r1 := g.Transitions[1]
	if len(r1.OnTokens) != 1 || r1.OnTokens[0] != "KEYWORD" {
		t.Errorf("r1.OnTokens: got %v, want [KEYWORD]", r1.OnTokens)
	}
	if r1.OnValue != "return" {
		t.Errorf("r1.OnValue: got %q, want %q", r1.OnValue, "return")
	}
	if len(r1.Actions) != 2 {
		t.Fatalf("r1.Actions: got %d actions, want 2", len(r1.Actions))
	}
	if r1.Actions[0].Kind != "set_mode" || r1.Actions[0].Target != "default" {
		t.Errorf("r1.Actions[0]: got %+v", r1.Actions[0])
	}
	if r1.Actions[1].Kind != "pop" {
		t.Errorf("r1.Actions[1]: got %+v, want pop", r1.Actions[1])
	}
}

func TestF10ParseInGuard(t *testing.T) {
	// Verifies that an "in MODE" clause after the token list is parsed into InMode
	// and does NOT appear in OnTokens.
	src := (
		"NAME = /[a-z]+/\n" +
		"group template:\n  TAIL = /x/\n" +
		"transitions:\n" +
		"  on TEMPLATE_HEAD in default -> push template, set-mode default\n")
	g, err := ParseTokenGrammar(src)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(g.Transitions) != 1 {
		t.Fatalf("got %d transitions, want 1", len(g.Transitions))
	}
	r := g.Transitions[0]
	if r.InMode != "default" {
		t.Errorf("InMode: got %q, want %q", r.InMode, "default")
	}
	if len(r.OnTokens) != 1 || r.OnTokens[0] != "TEMPLATE_HEAD" {
		t.Errorf("OnTokens: got %v, want [TEMPLATE_HEAD]", r.OnTokens)
	}
	if len(r.Actions) != 2 {
		t.Fatalf("Actions: got %d, want 2", len(r.Actions))
	}
	if r.Actions[0].Kind != "push" || r.Actions[0].Target != "template" {
		t.Errorf("Actions[0]: got %+v", r.Actions[0])
	}
	if r.Actions[1].Kind != "set_mode" || r.Actions[1].Target != "default" {
		t.Errorf("Actions[1]: got %+v", r.Actions[1])
	}
}

func TestF10AllActionKindsParsed(t *testing.T) {
	// Smoke-test all five action kinds in a single grammar.
	src := (
		"NAME = /[a-z]+/\n" +
		"group g:\n  T = /x/\n" +
		"transitions:\n" +
		"  on NAME -> set-mode g, push g, pop, enable-skip, disable-skip\n")
	g, err := ParseTokenGrammar(src)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(g.Transitions) != 1 {
		t.Fatalf("got %d transitions, want 1", len(g.Transitions))
	}
	actions := g.Transitions[0].Actions
	if len(actions) != 5 {
		t.Fatalf("got %d actions, want 5", len(actions))
	}
	want := []TransitionAction{
		{Kind: "set_mode", Target: "g"},
		{Kind: "push", Target: "g"},
		{Kind: "pop"},
		{Kind: "enable_skip"},
		{Kind: "disable_skip"},
	}
	for i, a := range actions {
		if a.Kind != want[i].Kind || a.Target != want[i].Target {
			t.Errorf("Actions[%d]: got %+v, want %+v", i, a, want[i])
		}
	}
}

// ---------------------------------------------------------------------------
// Error cases
// ---------------------------------------------------------------------------

func TestF10TransitionMissingArrowErrors(t *testing.T) {
	_, err := ParseTokenGrammar("NAME = /x/\ntransitions:\n  on NAME set-mode div\n")
	if err == nil {
		t.Fatal("expected error for missing '->', got nil")
	}
	if !strings.Contains(err.Error(), "->") {
		t.Errorf("error %q should mention '->'", err.Error())
	}
}

func TestF10TransitionUnknownActionErrors(t *testing.T) {
	_, err := ParseTokenGrammar("NAME = /x/\ntransitions:\n  on NAME -> teleport\n")
	if err == nil {
		t.Fatal("expected error for unknown action 'teleport', got nil")
	}
	if !strings.Contains(err.Error(), "Unknown transition action") {
		t.Errorf("error %q should mention 'Unknown transition action'", err.Error())
	}
}

func TestF10TransitionMissingOnErrors(t *testing.T) {
	_, err := ParseTokenGrammar("NAME = /x/\ntransitions:\n  NAME -> set-mode div\n")
	if err == nil {
		t.Fatal("expected error for missing 'on', got nil")
	}
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

func TestF10ValidateRejectsUndefinedTargetMode(t *testing.T) {
	g, err := ParseTokenGrammar("NAME = /x/\ntransitions:\n  on NAME -> set-mode div\n")
	if err != nil {
		t.Fatalf("unexpected parse error: %v", err)
	}
	issues := ValidateTokenGrammar(g)
	found := false
	for _, issue := range issues {
		if strings.Contains(issue, "undeclared mode") {
			found = true
			break
		}
	}
	if !found {
		t.Errorf("expected an 'undeclared mode' validation issue, got %v", issues)
	}
}

func TestF10ValidateRejectsUndefinedStartMode(t *testing.T) {
	g, err := ParseTokenGrammar("NAME = /x/\nstart_mode: nonexistent\n")
	if err != nil {
		t.Fatalf("unexpected parse error: %v", err)
	}
	issues := ValidateTokenGrammar(g)
	found := false
	for _, issue := range issues {
		if strings.Contains(issue, "start_mode") {
			found = true
			break
		}
	}
	if !found {
		t.Errorf("expected a 'start_mode' validation issue, got %v", issues)
	}
}

func TestF10ValidateAcceptsDeclaredModes(t *testing.T) {
	// A grammar with start_mode and transitions that only reference declared
	// groups ("default" and "div") must produce no mode-related issues.
	src := (
		"NAME = /[a-z]+/\n" +
		"start_mode: default\n" +
		"group div:\n  SLASH = \"/\"\n" +
		"transitions:\n" +
		"  on NAME -> set-mode div\n" +
		"  on KEYWORD=\"return\" -> set-mode default\n")
	g, err := ParseTokenGrammar(src)
	if err != nil {
		t.Fatalf("unexpected parse error: %v", err)
	}
	for _, issue := range ValidateTokenGrammar(g) {
		if strings.Contains(issue, "mode") {
			t.Errorf("unexpected mode-related validation issue: %q", issue)
		}
	}
}

func TestF10ValidateRejectsUndefinedInGuardMode(t *testing.T) {
	src := (
		"NAME = /x/\n" +
		"group g:\n  T = /x/\n" +
		"transitions:\n" +
		"  on NAME in ghost -> set-mode g\n")
	g, err := ParseTokenGrammar(src)
	if err != nil {
		t.Fatalf("unexpected parse error: %v", err)
	}
	issues := ValidateTokenGrammar(g)
	found := false
	for _, issue := range issues {
		if strings.Contains(issue, "undeclared mode") {
			found = true
			break
		}
	}
	if !found {
		t.Errorf("expected an 'undeclared mode' issue for in-guard, got %v", issues)
	}
}

// ---------------------------------------------------------------------------
// Compiler round-trip
// ---------------------------------------------------------------------------

func TestF10CompilerEmitsStartModeAndTransitions(t *testing.T) {
	// The compiler must include StartMode and Transitions in the generated
	// Go source literal so it can be parsed back to an equivalent grammar.
	src := (
		"NAME = /[a-z]+/\n" +
		"start_mode: default\n" +
		"group div:\n  SLASH = \"/\"\n" +
		"transitions:\n  on (NAME | RPAREN) -> set-mode div\n")
	g, err := ParseTokenGrammar(src)
	if err != nil {
		t.Fatalf("unexpected parse error: %v", err)
	}
	code := CompileTokenGrammar(g, "js.tokens", "generated")

	// goStringLit uses backtick raw-string literals for strings that contain no
	// backtick.  Check for the backtick-quoted forms that the compiler emits.
	for _, want := range []string{
		`StartMode:`,
		"`default`",
		`Transitions:`,
		"`set_mode`",
		"`div`",
	} {
		if !strings.Contains(code, want) {
			t.Errorf("generated code missing %q", want)
		}
	}
}
