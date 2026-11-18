package lexer

import (
	"math/rand"
	"reflect"
	"testing"

	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
)

func transitionPair(rules []grammartools.ModeTransition, stack []string, skip bool) (*GrammarLexer, *oracleLexer) {
	compiled := &Compiled{transitions: rules}
	actual := &GrammarLexer{c: compiled, groupStack: append([]string(nil), stack...), skipEnabled: skip}
	want := &oracleLexer{transitions: rules, groupStack: append([]string(nil), stack...), skipEnabled: skip}
	return actual, want
}

func assertTransitionPair(t *testing.T, rules []grammartools.ModeTransition, stack []string, skip bool, tok Token) {
	t.Helper()
	actual, want := transitionPair(rules, stack, skip)
	actual.applyTransitions(tok)
	want.applyTransitions(tok)
	if !reflect.DeepEqual(actual.groupStack, want.groupStack) || actual.skipEnabled != want.skipEnabled {
		t.Fatalf("transition mismatch for token %#v: got stack=%v skip=%v; oracle stack=%v skip=%v", tok, actual.groupStack, actual.skipEnabled, want.groupStack, want.skipEnabled)
	}
}

func TestApplyTransitionsGuardPriorityAndActionOrder(t *testing.T) {
	// A wrong-mode and a wrong-value overlap must fall through. The next
	// matching wildcard rule wins even with no actions, so the later specific
	// rule must not push a group.
	rules := []grammartools.ModeTransition{
		{OnTokens: []string{"MARK"}, InMode: "quote", Actions: []grammartools.TransitionAction{{Kind: "push", Target: "wrong-mode"}}},
		{OnTokens: []string{"MARK"}, OnValue: "other", Actions: []grammartools.TransitionAction{{Kind: "push", Target: "wrong-value"}}},
		{OnTokens: []string{"MARK"}},
		{OnTokens: []string{"MARK"}, InMode: "default", Actions: []grammartools.TransitionAction{{Kind: "push", Target: "too-late"}}},
	}
	assertTransitionPair(t, rules, []string{"default"}, true, Token{TypeName: "MARK", Value: "hit"})
	actual, _ := transitionPair(rules, []string{"default"}, true)
	actual.applyTransitions(Token{TypeName: "MARK", Value: "hit"})
	if !reflect.DeepEqual(actual.groupStack, []string{"default"}) {
		t.Fatalf("empty-action first match did not stop later rules: stack=%v", actual.groupStack)
	}

	// Duplicate token types and overlapping rules preserve source priority;
	// all actions from the winner run in order, including pop clamping and
	// skip toggles.
	ordered := []grammartools.ModeTransition{
		{OnTokens: []string{"MARK", "MARK"}, InMode: "default", OnValue: "hit", Actions: []grammartools.TransitionAction{
			{Kind: "push", Target: "inner"}, {Kind: "disable_skip"}, {Kind: "pop"}, {Kind: "pop"}, {Kind: "enable_skip"},
		}},
		{OnTokens: []string{"MARK"}, Actions: []grammartools.TransitionAction{{Kind: "set_mode", Target: "late"}}},
	}
	assertTransitionPair(t, ordered, []string{"default"}, false, Token{TypeName: "MARK", Value: "hit"})
	actual, _ = transitionPair(ordered, []string{"default"}, false)
	actual.applyTransitions(Token{TypeName: "MARK", Value: "hit"})
	if !reflect.DeepEqual(actual.groupStack, []string{"default"}) || !actual.skipEnabled {
		t.Fatalf("ordered actions produced stack=%v skip=%v", actual.groupStack, actual.skipEnabled)
	}
}

func TestApplyTransitionsRunsAfterCallbackStackChanges(t *testing.T) {
	grammar := grammarWithTransitions(
		[]grammartools.TokenDefinition{
			{Name: "OPEN", Pattern: `a`, IsRegex: false},
			{Name: "DEFAULT_B", Pattern: `b`, IsRegex: false},
		},
		"after", []grammartools.TokenDefinition{{Name: "AFTER_B", Pattern: `b`, IsRegex: false}},
		[]grammartools.ModeTransition{{
			OnTokens: []string{"OPEN"}, InMode: "callback",
			Actions: []grammartools.TransitionAction{{Kind: "set_mode", Target: "after"}},
		}}, "",
	)
	grammar.Groups["callback"] = &grammartools.PatternGroup{Name: "callback"}
	lexer := NewGrammarLexer("ab", grammar)
	depthAtB := 0
	lexer.SetOnToken(func(tok Token, ctx *LexerContext) {
		if tok.TypeName == "OPEN" {
			ctx.PushGroup("callback")
		}
		if tok.TypeName == "AFTER_B" || tok.TypeName == "DEFAULT_B" {
			depthAtB = ctx.GroupStackDepth()
		}
	})
	tokens := lexer.Tokenize()
	if len(tokens) < 3 || tokens[0].TypeName != "OPEN" || tokens[1].TypeName != "AFTER_B" {
		t.Fatalf("callback stack must be visible to the later transition: tokens=%v", tokens)
	}
	if depthAtB != 2 {
		t.Fatalf("the transition should replace the callback-pushed top before the next callback; depth=%d", depthAtB)
	}
}

func TestApplyTransitionsMatchesFrozenOracleBoundedCases(t *testing.T) {
	rng := rand.New(rand.NewSource(0x4731))
	types := []string{"WORD", "MARK", "OPEN", "CLOSE"}
	modes := []string{"", "default", "quote", "comment"}
	values := []string{"", "x", "\"", "other"}
	actions := []grammartools.TransitionAction{
		{Kind: "push", Target: "quote"}, {Kind: "set_mode", Target: "default"},
		{Kind: "pop"}, {Kind: "enable_skip"}, {Kind: "disable_skip"},
	}
	const cases = 2048
	for n := 0; n < cases; n++ {
		ruleCount := 1 + rng.Intn(7)
		rules := make([]grammartools.ModeTransition, ruleCount)
		for i := range rules {
			rule := grammartools.ModeTransition{InMode: modes[rng.Intn(len(modes))], OnValue: values[rng.Intn(len(values))]}
			for j, count := 0, rng.Intn(4); j < count; j++ {
				rule.OnTokens = append(rule.OnTokens, types[rng.Intn(len(types))])
			}
			for j, count := 0, rng.Intn(5); j < count; j++ {
				rule.Actions = append(rule.Actions, actions[rng.Intn(len(actions))])
			}
			rules[i] = rule
		}
		stack := []string{"default"}
		for i, count := 0, rng.Intn(4); i < count; i++ {
			stack = append(stack, modes[1+rng.Intn(len(modes)-1)])
		}
		tok := Token{TypeName: types[rng.Intn(len(types))], Value: values[rng.Intn(len(values))]}
		assertTransitionPair(t, rules, stack, rng.Intn(2) == 0, tok)
	}
}

func FuzzApplyTransitionsMatchesFrozenOracle(f *testing.F) {
	for _, seed := range [][]byte{{0}, {1, 2, 3, 4, 5}, {255, 7, 0, 9, 2, 1, 44}, []byte("strace mode callback")} {
		f.Add(seed)
	}
	f.Fuzz(func(t *testing.T, data []byte) {
		// The grammar shape is deliberately bounded; Go's fuzzer can explore
		// arbitrary bytes without turning a single input into unbounded work.
		if len(data) > 128 {
			data = data[:128]
		}
		byteAt := func(i int) byte {
			if len(data) == 0 {
				return 0
			}
			return data[i%len(data)]
		}
		types := []string{"WORD", "MARK", "OPEN", "CLOSE"}
		modes := []string{"", "default", "quote", "comment"}
		values := []string{"", "x", "\"", "other"}
		actionSet := []grammartools.TransitionAction{{Kind: "push", Target: "quote"}, {Kind: "set_mode", Target: "default"}, {Kind: "pop"}, {Kind: "enable_skip"}, {Kind: "disable_skip"}}
		count := 1 + int(byteAt(0)%6)
		rules := make([]grammartools.ModeTransition, count)
		for i := range rules {
			b := byteAt(i*11 + 1)
			rule := grammartools.ModeTransition{InMode: modes[int(b)%len(modes)], OnValue: values[int(byteAt(i*11+2))%len(values)]}
			for j := 0; j < int(byteAt(i*11+3)%4); j++ {
				rule.OnTokens = append(rule.OnTokens, types[int(byteAt(i*11+4+j))%len(types)])
			}
			for j := 0; j < int(byteAt(i*11+8)%5); j++ {
				rule.Actions = append(rule.Actions, actionSet[int(byteAt(i*11+9+j))%len(actionSet)])
			}
			rules[i] = rule
		}
		stack := []string{"default"}
		if len(data) > 1 && data[1]&1 != 0 {
			stack = append(stack, modes[1+int(byteAt(2))%3])
		}
		tok := Token{TypeName: types[int(byteAt(3))%len(types)], Value: values[int(byteAt(4))%len(values)]}
		assertTransitionPair(t, rules, stack, byteAt(5)&1 == 0, tok)
	})
}
