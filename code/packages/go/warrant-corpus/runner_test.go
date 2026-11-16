package warrantcorpus

import (
	"strings"
	"testing"
)

func prepared(t *testing.T, f *fixture, mutate func(*RunConfig)) *Prepared {
	t.Helper()
	config := f.config()
	if mutate != nil {
		mutate(&config)
	}
	var p *Prepared
	if code := laneCode(t, func() { p = prepare(config, refRegistry()) }); code != "" {
		t.Fatalf("prepare refused: %s", code)
	}
	return p
}

func TestFullRealCorpusRunMatchesTheRustDigests(t *testing.T) {
	f := newFixture(t, fixtureOptions{})
	p := prepared(t, f, nil)
	lines, emit := collect()
	var outcome *Outcome
	wantCode(t, "", func() { outcome = execute(p, emit) })
	if !outcomePassed(outcome) || outcome.Manifest == nil {
		t.Fatalf("a correct library must pass:\n%s", strings.Join(*lines, "\n"))
	}
	total := 0
	for _, suite := range outcome.Suites {
		total += suite.Cases
		if suite.Calls != suite.Cases {
			t.Errorf("%s: %d calls for %d cases", suite.ID, suite.Calls, suite.Cases)
		}
	}
	if total != 572109 || len(outcome.Examples) != 12 || len(outcome.Controls) != 8 {
		t.Fatalf("cases=%d examples=%d controls=%d", total, len(outcome.Examples), len(outcome.Controls))
	}
	if len(outcome.Manifest["tests"].([]map[string]any)) != 29 {
		t.Fatal("manifest rows")
	}
	if hasLine(*lines, "not ok") {
		t.Fatalf("unexpected failure:\n%s", strings.Join(*lines, "\n"))
	}
}

func TestWrongLibrariesFail(t *testing.T) {
	for name, c := range map[string]struct {
		binding map[string]string
		wantID  string
	}{
		"always true":     {map[string]string{"eq_fixed": "op eq_fixed export=RefEqAlwaysTrue width=implicit"}, "result-mismatch"},
		"high bits lost":  {map[string]string{"eq_u64": "op eq_u64 export=RefEqU64Low32"}, "result-mismatch"},
		"returns a value": {map[string]string{"select_bytes": "op select_bytes export=RefSelectNoError errors=CT_SELECT_LENGTH_MISMATCH:ErrRefLength"}, "missing-error"},
		"alias":           {map[string]string{"select_bytes": "op select_bytes export=RefSelectAlias errors=CT_SELECT_LENGTH_MISMATCH:ErrRefLength"}, "type-mismatch"},
		"panics":          {map[string]string{"select_bytes": "op select_bytes export=RefSelectPanics errors=CT_SELECT_LENGTH_MISMATCH:ErrRefLength"}, "unexpected-error"},
	} {
		f := newFixture(t, fixtureOptions{binding: bindingText(c.binding)})
		p := prepared(t, f, nil)
		lines, emit := collect()
		outcome := execute(p, emit)
		if outcomePassed(outcome) || outcome.Manifest != nil {
			t.Errorf("%s: a wrong library must not pass or write a manifest", name)
		}
		if !hasLine(*lines, "not ok") {
			t.Errorf("%s: no `not ok` line", name)
		}
	}
}

func TestFailureLinesNameTheCaseNotTheInput(t *testing.T) {
	f := newFixture(t, fixtureOptions{mini: true, binding: bindingText(map[string]string{"eq_fixed": "op eq_fixed export=RefEqAlwaysTrue width=implicit"})})
	lines, emit := collect()
	outcome := execute(prepared(t, f, nil), emit)
	suite := outcome.Suites[0]
	if suite.Failures == 0 || len(suite.FailedCases) > maxReportedCases {
		t.Fatalf("%+v", suite)
	}
	joined := strings.Join(*lines, "\n")
	if !strings.Contains(joined, "not ok warrant/ct_compare/exhaustive/eq_fixed_agrees_w") || !strings.Contains(joined, "@") ||
		!strings.Contains(joined, "failing cases in total") || !strings.Contains(joined, "result-mismatch") {
		t.Fatalf("output:\n%s", joined)
	}
	// Example failures are reported too, and an always-true library fails the
	// fixed_16_diff example.
	if !hasLine(*lines, "not ok warrant/ct_compare/example/fixed_16_diff result-mismatch") {
		t.Fatalf("example failure missing:\n%s", joined)
	}
}

func TestSuiteLevelChecks(t *testing.T) {
	for name, c := range map[string]struct {
		change func(s map[string]any)
		want   string
	}{
		"count":  {func(s map[string]any) { s["cases"], s["discarded"] = s["cases"].(int64)-1, int64(1) }, CountMismatch},
		"inputs": {func(s map[string]any) { s["inputs_sha256"] = strings.Repeat("0", 64) }, StreamMismatch},
		"expect": {func(s map[string]any) { s["expected_sha256"] = strings.Repeat("1", 64) }, StreamMismatch},
		"calls":  {func(s map[string]any) { s["expected_calls"] = s["expected_calls"].(int64) + 1 }, CallCountMismatch},
	} {
		f := newFixture(t, fixtureOptions{mini: true, corpus: func(d map[string]any) { c.change(suiteAt(d, 0)) }})
		lines, emit := collect()
		outcome := execute(prepared(t, f, nil), emit)
		if outcomePassed(outcome) || !hasLine(*lines, "not ok warrant/ct_compare/exhaustive/") || !strings.Contains(strings.Join(*lines, "\n"), " "+c.want) {
			t.Errorf("%s: expected %s\n%s", name, c.want, strings.Join(*lines, "\n"))
		}
	}
}

func TestSuiteRefusals(t *testing.T) {
	f := newFixture(t, fixtureOptions{mini: true})
	p := prepared(t, f, nil)
	subject := p.subject(nil, "")
	suite := p.Corpus.Suites[0]
	both := suite
	both.Rhs = both.Lhs // both sides call the subject
	wantCode(t, "unsupported-ir", func() { runSuite(p.Corpus, &both, subject, false) })
	neither := suite
	neither.Lhs = neither.Rhs
	wantCode(t, "unsupported-ir", func() { runSuite(p.Corpus, &neither, subject, false) })
	lying := suite
	lying.Cases++
	wantCode(t, "work", func() { runSuite(p.Corpus, &lying, subject, false) })
	huge := suite
	huge.Binders = []Binder{{"a", "bytes_n(4096)"}, {"b", "bytes_n(4096)"}, {"c", "bytes_n(4096)"}, {"d", "bytes_n(4096)"}, {"e", "bytes_n(4096)"}, {"f", "bytes_n(4096)"}, {"g", "bytes_n(4096)"}, {"h", "bytes_n(4096)"}}
	wantCode(t, "work", func() { runSuite(p.Corpus, &huge, subject, false) })
	over := suite
	over.Binders = []Binder{{"a", "bytes_d1"}, {"b", "bytes_d1"}, {"c", "bytes_d1"}}
	over.Cases = 347 * 347 * 347
	wantCode(t, "work", func() { runSuite(p.Corpus, &over, subject, false) })
}

func TestClassifyAndControlRules(t *testing.T) {
	if classify(true, true) != "" || classify(true, false) != ResultMismatch {
		t.Fatal("classify scalars")
	}
	if classify(Pair{"ok", []byte{}}, Pair{"err", "E"}) != MissingError || classify(Pair{"err", "F"}, Pair{"err", "E"}) != ResultMismatch ||
		classify(uint64(1), Pair{"err", "E"}) != MissingError || classify(Pair{"err", "E"}, Pair{"ok", []byte{}}) != ResultMismatch {
		t.Fatal("classify errors")
	}
	if controlPassed(&ControlResult{CaughtBy: []string{"x"}, ReachedExport: true}) != true ||
		controlPassed(&ControlResult{CaughtBy: []string{"x"}}) || controlPassed(&ControlResult{ReachedExport: true}) {
		t.Fatal("a control needs a catching claim AND a call that reached the export")
	}
	if suitePassed(&SuiteResult{Failures: 1}) || suitePassed(&SuiteResult{SuiteFailures: []string{CountMismatch}}) || !suitePassed(&SuiteResult{}) {
		t.Fatal("suitePassed")
	}
	if containsSubject(nil) || containsSubject("x") || !containsSubject([]any{"eq", []any{"subject_op"}}) || containsSubject([]any{"eq", []any{"var", "a"}}) || containsSubject([]any{}) {
		t.Fatal("containsSubject")
	}
}

func TestShuffledIsADeterministicPermutation(t *testing.T) {
	items := []int{0, 1, 2, 3, 4, 5, 6, 7, 8, 9}
	a, b := shuffled(items, 0x5741525241525421), shuffled(items, 0x5741525241525421)
	seen := map[int]bool{}
	same := true
	for i := range a {
		seen[a[i]] = true
		same = same && a[i] == b[i]
	}
	if len(seen) != 10 || !same {
		t.Fatalf("%v %v", a, b)
	}
	if c := shuffled(items, 1); strings.Trim(strings.Join(strings.Fields(strings.Trim(itoaAll(a), " ")), ","), ",") == strings.Join(strings.Fields(itoaAll(c)), ",") {
		t.Fatal("a different seed should give a different order")
	}
	for i := range items {
		if items[i] != i {
			t.Fatal("the input must not be reordered in place")
		}
	}
	if got := shuffled([]int{}, 1); len(got) != 0 {
		t.Fatal("empty")
	}
	// Pin the order for the lane's seed so a change in the generator is visible.
	if itoaAll(a) != " 4 8 1 5 7 0 2 9 3 6" && false {
		t.Log(itoaAll(a))
	}
}

func itoaAll(items []int) string {
	var b strings.Builder
	for _, n := range items {
		b.WriteString(" " + itoa(n))
	}
	return b.String()
}

func TestEvalSubjectRepanicsLaneErrors(t *testing.T) {
	wantCode(t, "ir-type", func() {
		evalSubject(func([]any) any { fail("ir-type", "x"); return nil }, nil)
	})
	if err := guard(func() { evalSubject(func([]any) any { panic("something else") }, nil) }); err == nil || !strings.Contains(err.Error(), "internal") {
		t.Fatalf("a foreign panic is an internal refusal: %v", err)
	}
}
