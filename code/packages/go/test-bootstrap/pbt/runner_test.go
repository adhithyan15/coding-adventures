package pbt

import (
	"errors"
	"fmt"
	"math/big"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/test-bootstrap/prng"
)

const secretSeed = uint64(987654321987)

func localOpts() *Options { return &Options{NumTests: 100, MaxShrinkSteps: 1000, CI: CIOff} }

func ciOpts() *Options { return &Options{NumTests: 100, MaxShrinkSteps: 1000, CI: CIOn} }

func mustCheck[T any](t *testing.T, seed uint64, g Gen[T], prop func(T) bool, o *Options, sh ShrinkFunc[T]) Verdict {
	t.Helper()
	v, err := CheckPropertyShrink(seed, g, prop, o, sh)
	if err != nil {
		t.Fatal(err)
	}
	return v
}

func TestAHoldingPropertyPasses(t *testing.T) {
	v, err := CheckProperty(1, Int64(0, 100), func(x int64) bool { return x >= 0 && x <= 100 }, nil)
	if err != nil || !v.Passed || v.Failed() || v.CasesRun != 100 || v.Err() != nil || v.FailureID != "" {
		t.Fatalf("holding property: %+v %v", v, err)
	}
}

func TestAFalsifiableProgertyIsNeverAPass(t *testing.T) {
	// x < 90 is false for roughly one case in ten; 100 cases must find it.
	for seed := uint64(0); seed < 50; seed++ {
		v := mustCheck(t, seed, Int64(0, 99), func(x int64) bool { return x < 90 }, nil, nil)
		if v.Passed || v.FailureID != FailureCounterexample {
			t.Fatalf("seed %d: falsifiable property gave %+v", seed, v)
		}
		if v.Err() == nil {
			t.Fatal("a failed verdict must have an error form")
		}
	}
}

func TestZeroCasesIsNotAPass(t *testing.T) {
	v := mustCheck(t, 1, Int64(0, 1), func(int64) bool { return true }, &Options{NumTests: 0, MaxShrinkSteps: 10}, nil)
	if v.Passed || v.FailureID != FailureExhausted || v.CasesRun != 0 {
		t.Fatalf("zero cases: %+v", v)
	}
	var f *Failure
	if err := v.Err(); !errors.As(err, &f) || f.ID != FailureExhausted || !strings.Contains(err.Error(), "[property-exhausted]") {
		t.Fatalf("Err() = %v", err)
	}
}

func TestInvalidOptionsAreErrorsAndNeverPass(t *testing.T) {
	g := Int64(0, 1)
	ok := func(int64) bool { return true }
	for name, o := range map[string]*Options{
		"negative tests": {NumTests: -1},
		"too many tests": {NumTests: MaxCases + 1},
		"negative steps": {NumTests: 1, MaxShrinkSteps: -1},
		"too many steps": {NumTests: 1, MaxShrinkSteps: MaxShrinkSteps + 1},
	} {
		v, err := CheckProperty(1, g, ok, o)
		if !errors.Is(err, ErrInvalidArgument) || v.Passed {
			t.Errorf("%s: %+v %v", name, v, err)
		}
	}
	if v, err := CheckProperty[int64](1, g, nil, nil); err == nil || v.Passed {
		t.Error("nil property must be an error")
	}
}

func TestPropertyPanicIsAPanicVerdict(t *testing.T) {
	boom := func(x int64) bool { panic("secret-detail-" + fmt.Sprint(x)) }
	v := mustCheck(t, secretSeed, Int64(0, 9), boom, localOpts(), nil)
	if v.Passed || v.FailureID != FailurePanic || v.Seed == nil || *v.Seed != secretSeed || !strings.Contains(v.Message, "secret-detail-") {
		t.Fatalf("local panic verdict: %+v", v)
	}
	v = mustCheck(t, secretSeed, Int64(0, 9), boom, ciOpts(), nil)
	if v.FailureID != FailurePanic || v.Seed != nil || strings.Contains(v.Message, "secret-detail") || !strings.Contains(v.Message, "string") {
		t.Fatalf("redacted panic verdict leaked or lost the type: %+v", v)
	}
}

func TestGeneratorErrorAndPanicAreExhausted(t *testing.T) {
	never := FilterN(Int64(0, 9), func(int64) bool { return false }, 5)
	v := mustCheck(t, 1, never, func(int64) bool { return true }, ciOpts(), nil)
	if v.FailureID != FailureExhausted || v.Passed || !strings.Contains(v.Message, "filter precondition") {
		t.Fatalf("exhausted filter: %+v", v)
	}
	broken := NewGen(func(*prng.SplitMix64) (int, error) { return 0, errors.New("disk-secret-detail") }, nil)
	local := mustCheck(t, 1, broken, func(int) bool { return true }, localOpts(), nil)
	if local.FailureID != FailureExhausted || !strings.Contains(local.Message, "disk-secret-detail") {
		t.Fatalf("local generator error: %+v", local)
	}
	red := mustCheck(t, 1, broken, func(int) bool { return true }, ciOpts(), nil)
	if strings.Contains(red.Message, "disk-secret-detail") || !strings.Contains(red.Message, "*errors.errorString") {
		t.Fatalf("redacted generator error: %+v", red)
	}
	panicking := NewGen(func(*prng.SplitMix64) (int, error) { panic("gen-secret") }, nil)
	pv := mustCheck(t, 1, panicking, func(int) bool { return true }, localOpts(), nil)
	if pv.FailureID != FailureExhausted || !strings.Contains(pv.Message, "gen-secret") {
		t.Fatalf("generator panic: %+v", pv)
	}
	pr := mustCheck(t, 1, panicking, func(int) bool { return true }, ciOpts(), nil)
	if strings.Contains(pr.Message, "gen-secret") || pr.FailureID != FailureExhausted {
		t.Fatalf("redacted generator panic: %+v", pr)
	}
	// A generator that fails mid-run reports how many cases ran first.
	calls := 0
	flaky := NewGen(func(*prng.SplitMix64) (int, error) {
		calls++
		if calls == 4 {
			return 0, ErrExhausted
		}
		return calls, nil
	}, nil)
	fv := mustCheck(t, 1, flaky, func(int) bool { return true }, nil, nil)
	if fv.CasesRun != 3 || fv.Iteration != 4 {
		t.Fatalf("flaky generator: %+v", fv)
	}
}

func TestShrinkingFindsTheMinimalWitness(t *testing.T) {
	gen := Int(bi(0), bi(100000))
	v := mustCheck(t, 7, gen, func(x *big_Int) bool { return x.Cmp(bi(100)) <= 0 }, localOpts(), nil)
	if v.FailureID != FailureCounterexample {
		t.Fatalf("%+v", v)
	}
	if min := v.Minimal.(*big_Int); min.Int64() != 101 {
		t.Fatalf("minimal %v, want 101 (message: %s)", min, v.Message)
	}
	if v.Seed == nil || *v.Seed != 7 || v.Counterexample == nil || v.ShrinkSteps == 0 {
		t.Fatalf("local verdict fields: %+v", v)
	}
	if !strings.Contains(v.Message, "seed=7") || !strings.Contains(v.Message, "minimal=101") {
		t.Fatalf("local message: %s", v.Message)
	}
}

// big_Int is an alias so the test above can name the type while a local
// variable called big shadows the package.
type big_Int = big.Int

func TestALyingShrinkerCannotChangeTheVerdict(t *testing.T) {
	gen := Int64(0, 1000)
	prop := func(x int64) bool { return x < 500 }
	base := mustCheck(t, 3, gen, prop, localOpts(), nil)
	if base.Passed {
		t.Fatalf("setup: expected a failure: %+v", base)
	}
	original := base.Counterexample.(int64)

	liars := map[string]ShrinkFunc[int64]{
		"returns a passing value": func(v int64, _ func(int64) bool, _ int) (int64, int) { return 0, 5 },
		"panics":                  func(int64, func(int64) bool, int) (int64, int) { panic("shrinker bug") },
		"negative steps":          func(v int64, _ func(int64) bool, _ int) (int64, int) { return v, -3 },
		"absurd steps":            func(v int64, _ func(int64) bool, _ int) (int64, int) { return v, MaxShrinkSteps + 1 },
	}
	for name, liar := range liars {
		for _, opts := range []*Options{localOpts(), ciOpts()} {
			v := mustCheck(t, 3, gen, prop, opts, liar)
			if v.Passed || v.FailureID != FailureCounterexample || v.ShrinkSteps != 0 {
				t.Errorf("%s: %+v", name, v)
			}
			if opts.CI == CIOff && v.Minimal.(int64) != original {
				t.Errorf("%s: reported %v, want the original %d", name, v.Minimal, original)
			}
		}
	}
	// An honest shrinker that returns another still-failing value is accepted.
	honest := func(v int64, falsifies func(int64) bool, _ int) (int64, int) {
		if falsifies(500) {
			return 500, 1
		}
		return v, 0
	}
	v := mustCheck(t, 3, gen, prop, localOpts(), honest)
	if v.Minimal.(int64) != 500 || v.ShrinkSteps != 1 {
		t.Fatalf("honest shrinker: %+v", v)
	}
}

func TestAPropertyThatPanicsOnTheShrunkValueReportsTheOriginal(t *testing.T) {
	gen := Int64(0, 1000)
	// Fails (returns false) above 500, but panics on exactly 400.
	prop := func(x int64) bool {
		if x == 400 {
			panic("only on the shrunk value")
		}
		return x < 500
	}
	v := mustCheck(t, 3, gen, prop, localOpts(), func(v int64, _ func(int64) bool, _ int) (int64, int) { return 400, 1 })
	if v.FailureID != FailureCounterexample || v.ShrinkSteps != 0 || v.Minimal.(int64) != v.Counterexample.(int64) {
		t.Fatalf("%+v", v)
	}
	// During honest shrinking, a candidate on which the property panics is "not
	// falsifying", so the walk steps around it.
	v = mustCheck(t, 3, gen, prop, localOpts(), nil)
	if v.FailureID != FailureCounterexample || v.Minimal.(int64) < 500 {
		t.Fatalf("%+v", v)
	}
}

func TestCIMessagesCarryNoWitnessAndNoSeed(t *testing.T) {
	witnessGen := Int(bi(1_000_000_000_000), bi(2_000_000_000_000))
	prop := func(x *big_Int) bool { return x.Cmp(bi(1_500_000_000_000)) < 0 }
	// The local run tells us the real witness, so the test knows what must not leak.
	local := mustCheck(t, secretSeed, witnessGen, prop, localOpts(), nil)
	if local.FailureID != FailureCounterexample {
		t.Fatalf("setup: %+v", local)
	}
	minimal := local.Minimal.(*big_Int).String()
	initial := local.Counterexample.(*big_Int).String()
	if !strings.Contains(local.Message, fmt.Sprint(secretSeed)) || !strings.Contains(local.Message, minimal) || local.Digest != "" {
		t.Fatalf("local mode must show seed and witness: %+v", local)
	}
	for _, opts := range []*Options{nil, ciOpts(), {NumTests: 100, MaxShrinkSteps: 1000}} {
		v := mustCheck(t, secretSeed, witnessGen, prop, opts, nil)
		if v.FailureID != FailureCounterexample {
			t.Fatalf("setup: %+v", v)
		}
		if v.Seed != nil || v.Counterexample != nil || v.Minimal != nil {
			t.Fatalf("redacted verdict exposes fields: %+v", v)
		}
		for _, secret := range []string{fmt.Sprint(secretSeed), minimal, initial, "seed", "initial=", "minimal="} {
			if strings.Contains(v.Message, secret) {
				t.Errorf("redacted message contains %q: %s", secret, v.Message)
			}
		}
		if len(v.Digest) != 16 || !strings.Contains(v.Message, "repro-fnv: "+v.Digest) {
			t.Fatalf("digest missing: %+v", v)
		}
		// The digest is exactly the FNV of the documented preimage.
		pre := fmt.Sprintf("iter:%d;steps:%d;shrunk:%s", v.Iteration, v.ShrinkSteps, minimal)
		if want := fmt.Sprintf("%016x", prng.FNV1a64String(pre)); want != v.Digest {
			t.Fatalf("digest %s != fnv(%q) = %s", v.Digest, pre, want)
		}
	}
}

func TestCIFromLookup(t *testing.T) {
	cases := map[string]bool{"true": true, "1": true, "yes": true, " TRUE ": true, "": false, "0": false, "false": false, "False": false, " 0 ": false}
	for value, want := range cases {
		got := CIFromLookup(func(key string) (string, bool) {
			if key != "CI" {
				t.Fatalf("looked up %q, want CI", key)
			}
			return value, true
		})
		if got != want {
			t.Errorf("CI=%q -> %v, want %v", value, got, want)
		}
	}
	if CIFromLookup(func(string) (string, bool) { return "", false }) {
		t.Error("an unset CI is not CI")
	}
}

func TestFailureVocabularyIsClosed(t *testing.T) {
	for _, id := range []string{"property-counterexample", "property-exhausted", "property-timeout", "property-panic"} {
		if !IsPBTFailureID(id) {
			t.Errorf("%s must be registered", id)
		}
	}
	for _, id := range []string{"", "unregistered-id", "Property-Panic", "property-panic ", "pass"} {
		if IsPBTFailureID(id) {
			t.Errorf("%q must not be registered", id)
		}
	}
	if len(FailureIDs) != 4 {
		t.Fatal("exactly four failure ids")
	}
}

func TestVerdictIsDeterministicForASeed(t *testing.T) {
	gen := List(Int64(-50, 50), 0, 12)
	prop := func(l []int64) bool {
		var s int64
		for _, v := range l {
			s += v
		}
		return s < 60
	}
	a := mustCheck(t, 11, gen, prop, localOpts(), nil)
	b := mustCheck(t, 11, gen, prop, localOpts(), nil)
	if a.Message != b.Message || a.Iteration != b.Iteration {
		t.Fatalf("same seed, different verdicts:\n%s\n%s", a.Message, b.Message)
	}
}

// -----------------------------------------------------------------------------
// Canon: total and bounded for hostile witnesses
// -----------------------------------------------------------------------------

type panickyString struct{}

func (panickyString) String() string { return panic0() }
func (panickyString) Error() string  { return panic0() }
func panic0() string                 { panic("String method bug") }

type cyc struct {
	Next *cyc
	N    int
}

func TestCanonRendersCommonShapes(t *testing.T) {
	var nilBig *big.Int
	var nilPtr *int
	type rec struct {
		A int
		B string
	}
	cases := []struct {
		in   any
		want string
	}{
		{true, "true"},
		{false, "false"},
		{bi(-12), "-12"},
		{*bi(7), "7"},
		{nilBig, "nil"},
		{nilPtr, "nil"},
		{nil, "nil"},
		{[]byte{0, 255}, "0x00ff"},
		{[]int{1, 2}, "[1,2]"},
		{[2]int8{-1, 3}, "[-1,3]"},
		{uint64(18446744073709551615), "18446744073709551615"},
		{3.5, "3.5"},
		{"a\"b", `"a\"b"`},
		{map[string]int{"b": 2, "a": 1}, `{"a":1,"b":2}`},
		{rec{3, "x"}, `{3,"x"}`},
		{[]any{bi(1), []byte{1}, "s"}, `[1,0x01,"s"]`},
		{(*int)(new(int)), "0"},
		{make(chan int), "<chan int>"},
		{func() {}, "<func()>"},
		{panickyString{}, "{}"}, // a struct is walked structurally, never by calling String
	}
	for _, c := range cases {
		if got := Canon(c.in); got != c.want {
			t.Errorf("Canon(%#v) = %q, want %q", c.in, got, c.want)
		}
	}
}

func TestCanonIsTotalForHostileWitnesses(t *testing.T) {
	// Depth: a 200,000-deep []any chain renders as "..." past depth 16.
	var deep any = []any{}
	for i := 0; i < 200_000; i++ {
		deep = []any{deep}
	}
	got := Canon(deep)
	if !strings.Contains(got, "...") || len(got) > 200 {
		t.Fatalf("deep witness rendered as %d bytes: %.60s", len(got), got)
	}
	// Maps and pointers nest too.
	var deepMap any = map[string]any{}
	for i := 0; i < 100_000; i++ {
		deepMap = map[string]any{"k": deepMap}
	}
	if got := Canon(deepMap); !strings.Contains(got, "...") {
		t.Fatalf("deep map: %.60s", got)
	}
	// A pointer cycle terminates.
	c := &cyc{N: 1}
	c.Next = c
	if got := Canon(c); !strings.Contains(got, "...") {
		t.Fatalf("cycle: %.80s", got)
	}
	// A wide witness truncates instead of rendering a million nodes.
	wide := make([]int, 1_000_000)
	got = Canon(wide)
	if !strings.Contains(got, "...") || len(got) > 100_000 {
		t.Fatalf("wide witness rendered as %d bytes", len(got))
	}
	// A shared (DAG) witness: 2^40 paths, bounded by the node budget.
	var dag any = 1
	for i := 0; i < 40; i++ {
		dag = []any{dag, dag}
	}
	if got := Canon(dag); len(got) > 200_000 {
		t.Fatalf("DAG witness exploded to %d bytes", len(got))
	}
	// A wide map truncates too.
	m := map[int]int{}
	for i := 0; i < 50_000; i++ {
		m[i] = i
	}
	if got := Canon(m); len(got) > 300_000 {
		t.Fatalf("wide map rendered as %d bytes", len(got))
	}
}

func TestCheckPropertyReturnsAVerdictForAHostileWitness(t *testing.T) {
	// A generator that builds a 100,000-deep witness; redacted mode computes the
	// digest outside any recover, so a crashing encoder would take the runner down.
	deepGen := NewGen(func(*prng.SplitMix64) (any, error) {
		var v any = 0
		for i := 0; i < 100_000; i++ {
			v = []any{v}
		}
		return v, nil
	}, nil)
	for _, opts := range []*Options{ciOpts(), localOpts()} {
		v := mustCheck(t, 1, deepGen, func(any) bool { return false }, opts, nil)
		if v.Passed || v.FailureID != FailureCounterexample {
			t.Fatalf("hostile witness: %+v", v)
		}
	}
	hostile := NewGen(func(*prng.SplitMix64) (panickyString, error) { return panickyString{}, nil }, nil)
	v := mustCheck(t, 1, hostile, func(panickyString) bool { return false }, ciOpts(), nil)
	if v.FailureID != FailureCounterexample || v.Passed {
		t.Fatalf("panicky witness: %+v", v)
	}
}

func TestSafeStringSurvivesAPanickingStringer(t *testing.T) {
	if got := safeString(panickyString{}); !strings.Contains(got, "PANIC") {
		t.Fatalf("safeString = %q", got)
	}
	if got := safeString(errors.New("plain")); got != "plain" {
		t.Fatalf("safeString = %q", got)
	}
	// A property that panics with a hostile value still yields a verdict.
	v := mustCheck(t, 1, Int64(0, 1), func(int64) bool { panic(panickyString{}) }, localOpts(), nil)
	if v.FailureID != FailurePanic || !strings.Contains(v.Message, "PANIC") {
		t.Fatalf("%+v", v)
	}
}
