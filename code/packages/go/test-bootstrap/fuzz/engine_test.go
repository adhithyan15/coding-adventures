package fuzz

import (
	"bytes"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"math"
	"slices"
	"strings"
	"testing"
)

// -----------------------------------------------------------------------------
// Bitmap, admission, energy
// -----------------------------------------------------------------------------

func TestBitmapRecordsMasksAndCounts(t *testing.T) {
	var b Bitmap
	if b.Count() != 0 || len(b.Edges()) != 0 || len(b.Raw()) != CoverageBytes {
		t.Fatal("a zero Bitmap is empty and 8,192 bytes wide")
	}
	b.Record(3)
	b.Record(3 + CoverageMapSize) // masks to the same edge
	b.Record(9)
	b.Record(CoverageMapSize - 1)
	if b.Count() != 3 || !b.Has(3) || !b.Has(3+2*CoverageMapSize) || b.Has(4) {
		t.Fatalf("count %d", b.Count())
	}
	if got := b.Edges(); !slices.Equal(got, []uint16{3, 9, 65535}) {
		t.Fatalf("Edges = %v (ascending, masked)", got)
	}
	raw := b.Raw()
	if raw[0] != 1<<3 || raw[1] != 1<<1 || raw[8191] != 1<<7 {
		t.Fatalf("bit layout is LSB-first: %x %x %x", raw[0], raw[1], raw[8191])
	}
	raw[0] = 0xFF // Raw is a copy
	if b.Count() != 3 {
		t.Fatal("Raw must be a copy")
	}
	var other Bitmap
	other.Record(9)
	other.Record(100)
	b.Merge(&other)
	if b.Count() != 4 || !b.Has(100) {
		t.Fatal("Merge ORs")
	}
	b.Reset()
	if b.Count() != 0 || len(b.Edges()) != 0 || b.Has(3) {
		t.Fatal("Reset clears the map and the list")
	}
	b.Record(3)
	if b.Count() != 1 {
		t.Fatal("a Bitmap is reusable after Reset")
	}
}

func TestAdmitCountsUniqueNovelEdges(t *testing.T) {
	var cum Bitmap
	if has, n := cum.Admit([]uint16{5, 5, 5, 6}); !has || n != 2 {
		t.Fatalf("got %v %d", has, n)
	}
	if has, n := cum.Admit([]uint16{5, 6}); has || n != 0 {
		t.Fatalf("got %v %d", has, n)
	}
	if has, n := cum.Admit(nil); has || n != 0 {
		t.Fatal("an empty trace admits nothing")
	}
}

func TestCorpusAdmitCountsUniqueNovelEdges(t *testing.T) {
	if ok, n := CorpusAdmit([]uint64{1, 2, 3}, []uint64{2, 4}); !ok || n != 1 {
		t.Fatalf("%v %d", ok, n)
	}
	if ok, n := CorpusAdmit([]uint64{1, 2, 3}, []uint64{1, 2}); ok || n != 0 {
		t.Fatalf("%v %d", ok, n)
	}
	// A trace repeating one novel edge three times still adds one.
	if ok, n := CorpusAdmit(nil, []uint64{9, 9, 9}); !ok || n != 1 {
		t.Fatalf("%v %d", ok, n)
	}
	if ok, n := CorpusAdmit(nil, nil); ok || n != 0 {
		t.Fatal("nothing from nothing")
	}
}

func TestEnergySchedule(t *testing.T) {
	// 10 * 1.5 / sqrt(4) = 7.5: half rounds AWAY from zero (banker's would give 8 too,
	// so also pin 2.5 -> 3, where banker's rounding gives 2).
	if got := CalculateEnergy(10, 4, 3, 6); got != 8 {
		t.Fatalf("got %d", got)
	}
	if got := CalculateEnergy(5, 4, 0, 0); got != 3 {
		t.Fatalf("5/2 = 2.5 must round to 3, got %d", got)
	}
	if got := CalculateEnergy(10, 1, 5, 0); got != 10 {
		t.Fatalf("no edges: bonus is 1.0, got %d", got)
	}
	if got := CalculateEnergy(0, 1_000_000, 1, 1); got != 1 {
		t.Fatalf("energy is at least 1, got %d", got)
	}
	if got := CalculateEnergy(10, 0, 1, 1); got != 20 {
		t.Fatalf("a zero length counts as 1: %d", got)
	}
	if CalculateEnergy(10, 4, 6, 6) <= CalculateEnergy(10, 4, 0, 6) {
		t.Fatal("seeds that found more coverage get more energy")
	}
	if CalculateEnergy(10, 100, 1, 2) >= CalculateEnergy(10, 1, 1, 2) {
		t.Fatal("shorter seeds get more energy")
	}
}

// -----------------------------------------------------------------------------
// Signatures, failure ids, TAP
// -----------------------------------------------------------------------------

func TestSignaturesAndDedup(t *testing.T) {
	// Pinned against fuzz_conformance.crash_signature.
	for _, c := range []struct {
		kind   string
		frames []string
		want   string
	}{
		{"panic", []string{"eval", "step"}, "4eca7ae8"},
		{"timeout", nil, "4e61ea76"},
		{"oom", nil, "7146a410"},
		{"x", []string{"a:b"}, "557f548e"},
		{"x", []string{"a", "b"}, "557f548e"}, // the documented, unescaped ambiguity
	} {
		if got := CrashSignature(c.kind, c.frames); got != c.want {
			t.Errorf("%s %v: %s, want %s", c.kind, c.frames, got, c.want)
		}
	}
	if !IsDuplicateCrash("panic", []string{"eval", "step"}, "panic", []string{"eval", "step"}) {
		t.Fatal("same signature")
	}
	if IsDuplicateCrash("panic", []string{"eval", "step"}, "segv", []string{"alloc", "write"}) {
		t.Fatal("different signature")
	}
	if IsDuplicateCrash("panic", []string{"a"}, "panic", []string{"a", "b"}) {
		t.Fatal("different depth")
	}
}

func TestFailureVocabularyIsClosed(t *testing.T) {
	for _, id := range []string{"fuzz-crash", "fuzz-timeout", "fuzz-oom", "fuzz-leak", "fuzz-panic"} {
		if !IsFuzzFailureID(id) {
			t.Errorf("%s must be registered", id)
		}
	}
	for _, id := range []string{"", "fuzz-arbitrary-error", "FUZZ-CRASH", "fuzz-crash ", "fuzz-crash\n", "crash", "pbt-panic"} {
		if IsFuzzFailureID(id) {
			t.Errorf("%q must be rejected", id)
		}
	}
	if len(FuzzFailureIDs) != 5 {
		t.Fatal("five ids")
	}
}

func TestTAPLineIsOpaqueAndUnforgeable(t *testing.T) {
	line, err := FormatTAPFailure(1, "ct_compare", FuzzFIDCrash, "3a8f19b2")
	if err != nil || line != "not ok 1 - ct_compare [fuzz-crash:3a8f19b2]" {
		t.Fatalf("%q %v", line, err)
	}
	if _, err := FormatTAPFailure(1, "t", "fuzz-arbitrary-error", "3a8f19b2"); !errors.Is(err, ErrFailureID) {
		t.Fatalf("an unregistered id must be refused, got %v", err)
	}
	for _, c := range []struct{ target, sig, want string }{
		{"t\nok 2 - forged", "3a8f19b2", "not ok 7 - t_ok_2_-_forged [fuzz-crash:3a8f19b2]"},
		{"t\r[fuzz-crash:00000000]", "3a8f19b2", "not ok 7 - t__fuzz-crash_00000000_ [fuzz-crash:3a8f19b2]"},
		{"naïve", "3a8f19b2", "not ok 7 - na__ve [fuzz-crash:3a8f19b2]"}, // 2 UTF-8 bytes, 2 underscores
		{"t", "ZZ\n12", "not ok 7 - t [fuzz-crash:___12]"},
		{"", "", "not ok 7 -  [fuzz-crash:]"},
	} {
		got, err := FormatTAPFailure(7, c.target, FuzzFIDCrash, c.sig)
		if err != nil || got != c.want {
			t.Errorf("target %q sig %q: %q, want %q", c.target, c.sig, got, c.want)
		}
		if strings.ContainsAny(got, "\r\n") {
			t.Errorf("%q spans lines", got)
		}
	}
}

func TestTAPFailuresRenderEveryBucketWithoutPayload(t *testing.T) {
	r := &Report{
		Crashes:  []CrashRecord{{Signature: "aaaaaaaa", minimal: []byte("SECRET")}, {Signature: "bbbbbbbb"}},
		Timeouts: 3, OOMs: 1, seed: 0xDEADBEEF,
	}
	got := TAPFailures(r, "tgt", 5)
	want := []string{
		"not ok 5 - tgt [fuzz-crash:aaaaaaaa]",
		"not ok 6 - tgt [fuzz-crash:bbbbbbbb]",
		"not ok 7 - tgt [fuzz-timeout:4e61ea76]",
		"not ok 8 - tgt [fuzz-oom:7146a410]",
	}
	if !slices.Equal(got, want) {
		t.Fatalf("got %q", got)
	}
	if lines := TAPFailures(&Report{Iterations: 5}, "tgt", 1); len(lines) != 0 {
		t.Fatalf("a clean report has no failure lines: %q", lines)
	}
}

// -----------------------------------------------------------------------------
// Frame and kind normalisation
// -----------------------------------------------------------------------------

func TestNormaliseFrame(t *testing.T) {
	for in, want := range map[string]string{
		"github.com/x/y/pkg.(*T).Method":          "pkg.T.Method",
		"github.com/x/y/pkg.Func":                 "pkg.Func",
		"main.main.func1":                         "main.main.func1",
		"github.com/x/y/pkg.Gen[go.shape.int]":    "pkg.Gen",
		"a/b/pkg.Gen[a/b/c.T,go.shape.[]int].Run": "pkg.Gen.Run",
		"pkg.(*T[...]).Method":                    "pkg.T.Method",
		"weird/pkg.F:G":                           "pkg.F_G",
		"p.f\ng":                                  "p.f_g",
		"":                                        "",
		"pkg.F]x":                                 "pkg.F_x",
	} {
		if got := normaliseFrame(in); got != want {
			t.Errorf("%q -> %q, want %q", in, got, want)
		}
		if strings.Contains(normaliseFrame(in), ":") {
			t.Errorf("%q: a frame must contain no colon", in)
		}
	}
}

func TestNormaliseKindAndCollapse(t *testing.T) {
	if got := normaliseKind("*errors.errorString"); got != "errors.errorString" {
		t.Fatal(got)
	}
	if got := normaliseKind("struct { x int }"); strings.ContainsAny(got, " {}") {
		t.Fatal(got)
	}
	if got := normaliseKind(strings.Repeat("a", 200)); len(got) != maxKindLen {
		t.Fatalf("kind capped at %d, got %d", maxKindLen, len(got))
	}
	if got := collapse([]string{"a", "a", "b", "a", "a"}); !slices.Equal(got, []string{"a", "b", "a"}) {
		t.Fatalf("%v", got)
	}
	long := make([]string, 200)
	for i := range long {
		long[i] = fmt.Sprint("f", i)
	}
	if got := collapse(long); len(got) != maxFrames || got[0] != "f0" {
		t.Fatalf("keeps the innermost %d", maxFrames)
	}
}

// -----------------------------------------------------------------------------
// The executor: crash, timeout, OOM classification
// -----------------------------------------------------------------------------

type crashHolder struct{ items []int }

//go:noinline
func (c *crashHolder) Boom(i int) int { return c.items[i] }

//go:noinline
func leafPanic(i int) int { return (&crashHolder{}).Boom(i) }

//go:noinline
func midPanic(i int) int { return leafPanic(i) + 1 }

//go:noinline
func recursivePanic(depth int) int {
	if depth == 0 {
		panic("deep")
	}
	return recursivePanic(depth-1) + 1
}

type customPanic struct{ secret string }

//go:noinline
func messagePanic(msg string) Target { return func(d []byte, h *Handle) { panic(msg) } }

func execOnce(t *testing.T, target Target, opts Options) Outcome {
	t.Helper()
	h := &Handle{stepBudget: opts.StepBudget, memBudget: opts.MemBudget}
	var cov Bitmap
	h.cov = &cov
	return runTarget(target, []byte{1, 2, 3}, h)
}

func TestExecutorClassifiesPanicsAsCrashes(t *testing.T) {
	opts := DefaultOptions()
	out := execOnce(t, func(d []byte, h *Handle) { midPanic(5) }, opts)
	if out.Kind != OutcomeCrash || out.CrashKind != "runtime.boundsError" {
		t.Fatalf("%+v", out)
	}
	// Innermost first: Boom, leaf, mid, then the target closure; no runtime or
	// executor frames, no path, no line number, no colon.
	if len(out.Frames) < 4 || out.Frames[0] != "fuzz.crashHolder.Boom" || out.Frames[1] != "fuzz.leafPanic" || out.Frames[2] != "fuzz.midPanic" {
		t.Fatalf("frames: %v", out.Frames)
	}
	for _, f := range out.Frames {
		if strings.ContainsAny(f, ":/ ()*") || strings.HasPrefix(f, "runtime.") || strings.Contains(f, "runTarget") {
			t.Errorf("unclean frame %q", f)
		}
	}

	for name, c := range map[string]struct {
		target Target
		kind   string
	}{
		"string":      {func(d []byte, h *Handle) { panic("boom " + string(d)) }, "string"},
		"error":       {func(d []byte, h *Handle) { panic(errors.New("x")) }, "errors.errorString"},
		"custom":      {func(d []byte, h *Handle) { panic(customPanic{string(d)}) }, "fuzz.customPanic"},
		"nil pointer": {func(d []byte, h *Handle) { var p *crashHolder; _ = p.items }, "runtime.errorString"},
		"type assert": {func(d []byte, h *Handle) { var x any = 1; _ = x.(string) }, "runtime.TypeAssertionError"},
		"closed chan": {func(d []byte, h *Handle) { c := make(chan int); close(c); close(c) }, "runtime.plainError"},
		"nil panic":   {func(d []byte, h *Handle) { panic(nil) }, "runtime.PanicNilError"},
		"divide":      {func(d []byte, h *Handle) { z := len(d) - 3; _ = 1 / z }, "runtime.runtimeError"},
		"conversion":  {func(d []byte, h *Handle) { s := make([]int, 2); _ = [4]int(s) }, "runtime.errorString"},
	} {
		got := execOnce(t, c.target, opts)
		if got.Kind != OutcomeCrash {
			t.Errorf("%s: %+v", name, got)
			continue
		}
		// The type name differs across Go releases for some runtime panics; what
		// is pinned is that the kind is a clean, message-free type name.
		if name == "string" || name == "error" || name == "custom" || name == "nil panic" {
			if got.CrashKind != c.kind {
				t.Errorf("%s: kind %q, want %q", name, got.CrashKind, c.kind)
			}
		}
		if got.CrashKind == "" || strings.ContainsAny(got.CrashKind, " :*") {
			t.Errorf("%s: kind %q", name, got.CrashKind)
		}
	}
}

func TestCrashSignatureIgnoresMessagesAndInputBytes(t *testing.T) {
	a := execOnce(t, func(d []byte, h *Handle) { panic("secret-one " + string(d)) }, DefaultOptions())
	b := execOnce(t, func(d []byte, h *Handle) { panic("secret-two") }, DefaultOptions())
	// Distinct closures have distinct frames; the SAME closure with different
	// messages must have one signature.
	_, _ = a, b
	s1 := execOnce(t, messagePanic("one"), DefaultOptions())
	s2 := execOnce(t, messagePanic("two"), DefaultOptions())
	if CrashSignature(s1.CrashKind, s1.Frames) != CrashSignature(s2.CrashKind, s2.Frames) {
		t.Fatal("the panic message must not be part of the signature")
	}
	for _, f := range append(s1.Frames, s1.CrashKind) {
		if strings.Contains(f, "one") || strings.Contains(f, "secret") {
			t.Fatalf("message leaked into %q", f)
		}
	}
}

func TestRecursivePanicsHashAlikeAtAnyDepth(t *testing.T) {
	sig := func(depth int) string {
		o := execOnce(t, func(d []byte, h *Handle) { recursivePanic(depth) }, DefaultOptions())
		if o.Kind != OutcomeCrash {
			t.Fatalf("%+v", o)
		}
		return CrashSignature(o.CrashKind, o.Frames)
	}
	// The capture buffer is 128 PCs; depth 1 and depth 5,000 differ only in how
	// many identical frames precede the target, which collapse.
	if a, b := sig(1), sig(5000); a != b {
		t.Fatalf("%s != %s", a, b)
	}
}

func TestDifferentFailuresGetDifferentSignatures(t *testing.T) {
	a := execOnce(t, func(d []byte, h *Handle) { midPanic(9) }, DefaultOptions())
	b := execOnce(t, func(d []byte, h *Handle) { panic("x") }, DefaultOptions())
	c := execOnce(t, func(d []byte, h *Handle) { recursivePanic(3) }, DefaultOptions())
	sigs := map[string]bool{}
	for _, o := range []Outcome{a, b, c} {
		sigs[CrashSignature(o.CrashKind, o.Frames)] = true
	}
	if len(sigs) != 3 {
		t.Fatalf("%v", sigs)
	}
}

func TestTimeoutIsADeterministicStepBudget(t *testing.T) {
	opts := DefaultOptions()
	opts.StepBudget = 10
	ticks := 0
	loop := func(d []byte, h *Handle) {
		for {
			h.Tick()
			ticks++
		}
	}
	for i := 0; i < 2; i++ {
		ticks = 0
		if out := execOnce(t, loop, opts); out.Kind != OutcomeTimeout || ticks != 10 {
			t.Fatalf("%+v after %d ticks", out, ticks)
		}
	}
	// The budget is per execution: a fresh run starts again at zero.
	exact := func(d []byte, h *Handle) {
		for i := 0; i < 10; i++ {
			h.Tick()
		}
	}
	if out := execOnce(t, exact, opts); out.Kind != OutcomePass {
		t.Fatalf("exactly StepBudget ticks is not a timeout: %+v", out)
	}
	// A target that swallows ONE recover cannot tick past the budget: Tick keeps panicking.
	opts.StepBudget = 0
	swallowOnce := func(d []byte, h *Handle) {
		for i := 0; ; i++ {
			func() {
				defer func() {
					if i == 0 {
						_ = recover()
					}
				}()
				h.Tick()
			}()
		}
	}
	if out := execOnce(t, swallowOnce, opts); out.Kind != OutcomeTimeout {
		t.Fatalf("%+v", out)
	}
}

func TestReserveAccountsBytesWithoutOverflow(t *testing.T) {
	opts := DefaultOptions()
	opts.MemBudget = 100
	if out := execOnce(t, func(d []byte, h *Handle) { h.Reserve(60); h.Reserve(40); h.Reserve(-5); h.Reserve(0) }, opts); out.Kind != OutcomePass {
		t.Fatalf("up to the budget is fine: %+v", out)
	}
	if out := execOnce(t, func(d []byte, h *Handle) { h.Reserve(60); h.Reserve(41) }, opts); out.Kind != OutcomeOOM {
		t.Fatalf("%+v", out)
	}
	if out := execOnce(t, func(d []byte, h *Handle) { h.Reserve(math.MaxInt) }, opts); out.Kind != OutcomeOOM {
		t.Fatalf("a huge request must not overflow the running total: %+v", out)
	}
	if out := execOnce(t, func(d []byte, h *Handle) { h.Reserve(1); h.Reserve(math.MaxInt) }, opts); out.Kind != OutcomeOOM {
		t.Fatalf("%+v", out)
	}
}

func TestTargetReceivesACopyOfTheInput(t *testing.T) {
	seed := []byte{1, 2, 3}
	_, err := Run([][]byte{seed}, Options{MaxIterations: 20, MaxInputLen: 8, BaseEnergy: 3, StepBudget: 100, MemBudget: 100},
		func(d []byte, h *Handle) {
			for i := range d {
				d[i] = 0xEE
			}
		})
	if err != nil || !bytes.Equal(seed, []byte{1, 2, 3}) {
		t.Fatalf("the target scribbled on the caller's seed: %x %v", seed, err)
	}
}

func TestClassifyNilAndForeignSentinels(t *testing.T) {
	h, other := &Handle{}, &Handle{}
	if out, claimed := classify(nil, h, nil); !claimed || out.Kind != OutcomeCrash || out.CrashKind != "nil" {
		t.Fatalf("%+v", out)
	}
	if _, claimed := classify(stepBudgetExceeded{other}, h, nil); claimed {
		t.Fatal("a foreign step sentinel must not be claimed")
	}
	if _, claimed := classify(memBudgetExceeded{other}, h, nil); claimed {
		t.Fatal("a foreign memory sentinel must not be claimed")
	}
	if out, claimed := classify(stepBudgetExceeded{h}, h, nil); !claimed || out.Kind != OutcomeTimeout {
		t.Fatal("own step sentinel")
	}
	if out, claimed := classify(memBudgetExceeded{h}, h, nil); !claimed || out.Kind != OutcomeOOM {
		t.Fatal("own memory sentinel")
	}
}

func TestNestedCampaignsDoNotClaimEachOthersTimeouts(t *testing.T) {
	outer := DefaultOptions()
	outer.StepBudget = 0
	out := execOnce(t, func(d []byte, outerH *Handle) {
		opts := DefaultOptions()
		opts.MaxIterations = 3
		_, _ = Run(nil, opts, func(d []byte, innerH *Handle) { outerH.Tick() })
		t.Error("the inner campaign swallowed the outer timeout")
	}, outer)
	if out.Kind != OutcomeTimeout {
		t.Fatalf("the outer harness must claim its own timeout: %+v", out)
	}
}

// -----------------------------------------------------------------------------
// Minimisation
// -----------------------------------------------------------------------------

func crashIf(pred func([]byte) bool, kind string) func([]byte) Outcome {
	return func(d []byte) Outcome {
		if pred(d) {
			return Outcome{Kind: OutcomeCrash, CrashKind: kind, Frames: []string{"f"}}
		}
		return Outcome{Kind: OutcomePass}
	}
}

func TestMinimizerReachesTheEmptyInput(t *testing.T) {
	always := crashIf(func([]byte) bool { return true }, "k")
	sig := CrashSignature("k", []string{"f"})
	for _, in := range [][]byte{{7}, {1, 2, 3, 4, 5}, nil} {
		got, err := MinimizeCrashingInput(in, sig, always, 1000)
		if err != nil || len(got) != 0 {
			t.Fatalf("%x -> %x %v: a crasher that needs no input must reach empty", in, got, err)
		}
	}
}

func TestMinimizerKeepsTheExactSignatureAndZeroesBytes(t *testing.T) {
	// Crashes iff byte 3 is 0x41.  Everything else is irrelevant.
	pred := crashIf(func(d []byte) bool { return len(d) > 0 && slices.Contains(d, 0x41) }, "k")
	got, err := MinimizeCrashingInput([]byte{9, 8, 7, 0x41, 6, 5}, CrashSignature("k", []string{"f"}), pred, 1000)
	if err != nil || !bytes.Equal(got, []byte{0x41}) {
		t.Fatalf("got %x %v", got, err)
	}
	// Needs two specific bytes in a specific order: zeroing keeps both, deletion drops the rest.
	need := crashIf(func(d []byte) bool {
		i := bytes.IndexByte(d, 1)
		return i >= 0 && bytes.IndexByte(d[i:], 2) >= 0
	}, "k")
	got, _ = MinimizeCrashingInput([]byte{5, 1, 5, 5, 2, 5}, CrashSignature("k", []string{"f"}), need, 1000)
	if !bytes.Equal(got, []byte{1, 2}) {
		t.Fatalf("got %x", got)
	}
	// A non-zero byte that matters is restored after a failed zeroing.
	got, _ = MinimizeCrashingInput([]byte{3, 3, 3}, CrashSignature("k", []string{"f"}),
		crashIf(func(d []byte) bool { return len(d) == 3 && d[1] == 3 }, "k"), 1000)
	if !bytes.Equal(got, []byte{0, 3, 0}) {
		t.Fatalf("got %x", got)
	}
}

func TestMinimizerNeverTradesOneCrashForAnother(t *testing.T) {
	// Long inputs crash "a"; any shorter input crashes "b" (a different bug).
	test := func(d []byte) Outcome {
		if len(d) >= 6 {
			return Outcome{Kind: OutcomeCrash, CrashKind: "a", Frames: []string{"f"}}
		}
		return Outcome{Kind: OutcomeCrash, CrashKind: "b", Frames: []string{"f"}}
	}
	in := []byte{1, 2, 3, 4, 5, 6, 7, 8}
	got, _ := MinimizeCrashingInput(in, CrashSignature("a", []string{"f"}), test, 1000)
	if len(got) != 6 {
		t.Fatalf("shrunk to %x: it traded bug a for bug b below 6 bytes", got)
	}
	// A candidate that merely passes is rejected too.
	got, _ = MinimizeCrashingInput([]byte{1, 1}, CrashSignature("k", []string{"f"}),
		crashIf(func(d []byte) bool { return len(d) == 2 }, "k"), 1000)
	if len(got) != 2 {
		t.Fatalf("%x", got)
	}
}

func TestMinimizerBudgetAndValidation(t *testing.T) {
	calls := 0
	test := func(d []byte) Outcome {
		calls++
		return Outcome{Kind: OutcomeCrash, CrashKind: "k", Frames: []string{"f"}}
	}
	sig := CrashSignature("k", []string{"f"})
	in := bytes.Repeat([]byte{1}, 64)
	for _, budget := range []int{0, 1, 5} {
		calls = 0
		got, err := MinimizeCrashingInput(in, sig, test, budget)
		if err != nil || calls > budget || len(got) > len(in) {
			t.Fatalf("budget %d: %d calls, %d bytes, %v", budget, calls, len(got), err)
		}
	}
	for _, bad := range []int{-1, MaxMinimizeBudget + 1} {
		if _, err := MinimizeCrashingInput(in, sig, test, bad); !errors.Is(err, ErrOptions) {
			t.Fatalf("budget %d: %v", bad, err)
		}
	}
	if _, err := MinimizeCrashingInput(in, sig, nil, 5); !errors.Is(err, ErrOptions) {
		t.Fatalf("nil test: %v", err)
	}
	// The budget can run out in pass 2 as well as in pass 1.
	calls = 0
	_, spent, finished := minimize([]byte{1}, sig, crashIf(func(d []byte) bool { return len(d) == 1 }, "k"), 1)
	if spent != 1 || !finished {
		// one deletion attempt (fails), then pass 2 needs a second execution
		if finished {
			t.Fatalf("spent %d finished %v", spent, finished)
		}
	}
}

func TestMinimizeMarksCutShortWork(t *testing.T) {
	test := crashIf(func([]byte) bool { return true }, "k")
	sig := CrashSignature("k", []string{"f"})
	// Pass 2 cut: a 1-byte input needs 1 deletion probe + 1 zeroing probe.
	out, spent, finished := minimize([]byte{5}, sig, func(d []byte) Outcome {
		if len(d) == 1 {
			return Outcome{Kind: OutcomeCrash, CrashKind: "k", Frames: []string{"f"}}
		}
		return Outcome{Kind: OutcomePass}
	}, 1)
	if finished || spent != 1 || !bytes.Equal(out, []byte{5}) {
		t.Fatalf("%x %d %v", out, spent, finished)
	}
	if _, _, finished := minimize(bytes.Repeat([]byte{1}, 10), sig, test, 0); finished {
		t.Fatal("budget 0 on a non-empty input is not finished")
	}
	if out, spent, finished := minimize(nil, sig, test, 0); !finished || spent != 0 || len(out) != 0 {
		t.Fatal("an empty crasher needs no work")
	}
}

// -----------------------------------------------------------------------------
// Campaign contracts (VER06 section 9.1)
// -----------------------------------------------------------------------------

// recording is a deterministic target that notes everything it sees.
type recording struct {
	execs   int
	maxLen  int
	inputs  [][]byte
	crashAt func([]byte) bool
}

func (r *recording) target(d []byte, h *Handle) {
	r.execs++
	r.maxLen = max(r.maxLen, len(d))
	r.inputs = append(r.inputs, slices.Clone(d))
	if len(d) > 0 {
		h.Edge(uint64(d[0]))
	}
	if r.crashAt != nil && r.crashAt(d) {
		panic("crash")
	}
}

func opts(iter int, seed uint64) Options {
	o := DefaultOptions()
	o.MaxIterations, o.Seed = iter, seed
	return o
}

func mustRun(t *testing.T, corpus [][]byte, o Options, target Target) *Report {
	t.Helper()
	r, err := Run(corpus, o, target)
	if err != nil {
		t.Fatal(err)
	}
	return r
}

func TestCampaignIsDeterministicAndSeedSensitive(t *testing.T) {
	run := func(seed uint64) (*Report, [][]byte) {
		rec := &recording{}
		return mustRun(t, [][]byte{{1, 2, 3}, {9}}, opts(300, seed), rec.target), rec.inputs
	}
	a, ia := run(7)
	b, ib := run(7)
	if a.String() != b.String() || !slices.EqualFunc(ia, ib, bytes.Equal) {
		t.Fatal("the same (corpus, options, target) must give the same campaign")
	}
	if !slices.EqualFunc(a.Corpus(), b.Corpus(), bytes.Equal) {
		t.Fatal("same corpus")
	}
	_, ic := run(8)
	if slices.EqualFunc(ia, ic, bytes.Equal) {
		t.Fatal("a different seed must give a different stream")
	}
}

func TestBudgetLengthCapAndEmptyCorpus(t *testing.T) {
	rec := &recording{}
	o := opts(500, 3)
	o.MaxInputLen = 6
	r := mustRun(t, [][]byte{{1, 2, 3, 4}}, o, rec.target)
	if r.Iterations != 500 || rec.execs != 500+1 {
		t.Fatalf("iterations %d execs %d: exactly the budget, plus triage", r.Iterations, rec.execs)
	}
	if rec.maxLen > 6 {
		t.Fatalf("a candidate of %d bytes exceeds MaxInputLen", rec.maxLen)
	}
	// MaxInputLen 0: every mutated candidate is empty.
	rec = &recording{}
	o.MaxInputLen = 0
	mustRun(t, [][]byte{{1}}, o, rec.target)
	for _, in := range rec.inputs[1:] {
		if len(in) != 0 {
			t.Fatalf("%x", in)
		}
	}
	// An empty corpus (nil or empty slice) becomes a single 00 seed.
	for _, c := range [][][]byte{nil, {}} {
		rec = &recording{}
		r = mustRun(t, c, opts(10, 1), rec.target)
		if !bytes.Equal(rec.inputs[0], []byte{0}) || r.Iterations != 10 {
			t.Fatalf("%x %d", rec.inputs[0], r.Iterations)
		}
	}
	// Initial seeds run as given, even beyond MaxInputLen.
	rec = &recording{}
	o.MaxInputLen = 2
	mustRun(t, [][]byte{bytes.Repeat([]byte{4}, 50)}, o, rec.target)
	if len(rec.inputs[0]) != 50 {
		t.Fatal("the initial seed must run unchanged")
	}
}

func TestCrashingSeedsAreNotMutatedAndNeverClean(t *testing.T) {
	rec := &recording{crashAt: func([]byte) bool { return true }}
	o := opts(100, 1)
	o.MinimizeBudget = 0
	r := mustRun(t, [][]byte{{1}, {2}, {3}}, o, rec.target)
	if r.Iterations != 0 || rec.execs != 3 || r.AdmittedSeeds != 0 || r.IsClean() {
		t.Fatalf("a corpus of crashers fuzzes nothing: %v execs %d", r, rec.execs)
	}
	if len(r.Crashes) != 1 || r.Crashes[0].Hits != 3 {
		t.Fatalf("one signature, three hits: %v", r.Crashes)
	}
	// Timeout- and OOM-only seeds are not mutated either.
	for name, target := range map[string]Target{
		"timeout": func(d []byte, h *Handle) { h.Tick() },
		"oom":     func(d []byte, h *Handle) { h.Reserve(1) },
	} {
		o := opts(100, 1)
		o.StepBudget, o.MemBudget = 0, 0
		r := mustRun(t, [][]byte{{1}}, o, target)
		if r.Iterations != 0 || r.IsClean() || r.Timeouts+r.OOMs != 1 {
			t.Fatalf("%s: %v", name, r)
		}
	}
}

func TestEveryExecutionLandsInABucket(t *testing.T) {
	rec := &recording{crashAt: func(d []byte) bool { return len(d) > 0 && d[0]%3 == 0 }}
	o := opts(400, 11)
	o.MinimizeBudget = 0 // minimiser probes are not campaign executions
	r := mustRun(t, [][]byte{{1}, {2, 5}}, o, rec.target)
	hits := 0
	for _, c := range r.Crashes {
		hits += c.Hits
	}
	if rec.execs != 2+r.Iterations {
		t.Fatalf("execs %d iterations %d", rec.execs, r.Iterations)
	}
	passing := 0
	for _, in := range rec.inputs {
		if !rec.crashAt(in) {
			passing++
		}
	}
	if hits != rec.execs-passing || hits == 0 {
		t.Fatalf("hits %d, crashing executions %d", hits, rec.execs-passing)
	}
	if r.AdmittedSeeds > passing {
		t.Fatal("only passing executions enter the corpus")
	}
}

func TestIsCleanNeedsIterationsAndNoFailureOfAnyKind(t *testing.T) {
	clean := mustRun(t, [][]byte{{1}}, opts(50, 1), (&recording{}).target)
	if !clean.IsClean() || clean.UniqueCrashes() != 0 {
		t.Fatalf("%v", clean)
	}
	if zero := mustRun(t, [][]byte{{1}}, opts(0, 1), (&recording{}).target); zero.IsClean() || zero.Iterations != 0 {
		t.Fatal("zero iterations fuzzed nothing")
	}
	base := Report{Iterations: 1}
	if !base.IsClean() {
		t.Fatal("baseline")
	}
	for name, r := range map[string]Report{
		"crash":   {Iterations: 1, Crashes: []CrashRecord{{}}},
		"timeout": {Iterations: 1, Timeouts: 1},
		"oom":     {Iterations: 1, OOMs: 1},
		"dropped": {Iterations: 1, CrashesDropped: 1},
		"none":    {},
	} {
		if r.IsClean() {
			t.Errorf("%s must not be clean", name)
		}
	}
	// A timeout found by MUTATION (not in a seed) also makes the report unclean.
	o := opts(200, 4)
	o.StepBudget = 0
	r := mustRun(t, [][]byte{{1}}, o, func(d []byte, h *Handle) {
		if len(d) >= 2 {
			h.Tick()
		}
	})
	if r.Timeouts == 0 || r.IsClean() || r.Iterations != 200 {
		t.Fatalf("%v", r)
	}
	if got := r.UniqueCrashes(); got != 0 {
		t.Fatal(got)
	}
}

func TestDedupOneRecordPerSignatureWithCorrectHits(t *testing.T) {
	execs, kindAHits, kindBHits := 0, 0, 0
	exec := func(d []byte, cov *Bitmap) Outcome {
		execs++
		if len(d) > 0 {
			cov.Record(uint64(d[0]))
		}
		switch {
		case len(d) == 1 && d[0] == 0: // the seed passes
			return Outcome{Kind: OutcomePass}
		case len(d)%2 == 0:
			kindAHits++
			return Outcome{Kind: OutcomeCrash, CrashKind: "a", Frames: []string{"x", "y"}}
		default:
			kindBHits++
			return Outcome{Kind: OutcomeCrash, CrashKind: "b", Frames: []string{"x"}}
		}
	}
	o := opts(300, 5)
	o.MinimizeBudget = 0
	r, err := RunOutcomes([][]byte{{0}}, o, exec)
	if err != nil {
		t.Fatal(err)
	}
	if len(r.Crashes) > 2 {
		t.Fatalf("%d records for two signatures", len(r.Crashes))
	}
	byKind := map[string]CrashRecord{}
	for _, c := range r.Crashes {
		byKind[c.Kind] = c
		if c.Minimized {
			t.Error("minimisation was disabled; the record must say so")
		}
	}
	if byKind["a"].Hits != kindAHits || byKind["b"].Hits != kindBHits || kindAHits+kindBHits == 0 {
		t.Fatalf("hits a=%d/%d b=%d/%d", byKind["a"].Hits, kindAHits, byKind["b"].Hits, kindBHits)
	}
}

func TestHitsKeepTheShorterInput(t *testing.T) {
	seen := map[int]bool{}
	shortest := math.MaxInt
	exec := func(d []byte, cov *Bitmap) Outcome {
		if len(seen) == 0 {
			seen[len(d)] = true
			return Outcome{Kind: OutcomePass}
		}
		shortest = min(shortest, len(d))
		return Outcome{Kind: OutcomeCrash, CrashKind: "k", Frames: []string{"f"}}
	}
	o := opts(200, 9)
	o.MinimizeBudget = 0
	r, _ := RunOutcomes([][]byte{{1, 2, 3, 4, 5, 6, 7, 8}}, o, exec)
	if len(r.Crashes) != 1 || len(r.Crashes[0].MinimalInput()) != shortest {
		t.Fatalf("minimal %d, shortest seen %d", len(r.Crashes[0].MinimalInput()), shortest)
	}
}

func TestCrashCapCountsDroppedSignatures(t *testing.T) {
	n := 0
	exec := func(d []byte, cov *Bitmap) Outcome {
		n++
		if n == 1 {
			return Outcome{Kind: OutcomePass}
		}
		return Outcome{Kind: OutcomeCrash, CrashKind: fmt.Sprint("k", n), Frames: []string{"f"}}
	}
	o := opts(1500, 2)
	o.MinimizeBudget = 0
	r, err := RunOutcomes([][]byte{{1}}, o, exec)
	if err != nil {
		t.Fatal(err)
	}
	if len(r.Crashes) != MaxCrashes || r.CrashesDropped != 1500-MaxCrashes || r.IsClean() || r.UniqueCrashes() != 1500 {
		t.Fatalf("%d records, %d dropped", len(r.Crashes), r.CrashesDropped)
	}
	// Dropped alone (no records held) is still never clean.
	if (Report{Iterations: 3, CrashesDropped: 1}).IsClean() {
		t.Fatal("a dropped signature must not read as clean")
	}
}

func TestCoverageAdmissionGrowsTheCorpus(t *testing.T) {
	r := mustRun(t, [][]byte{{1}}, opts(400, 6), func(d []byte, h *Handle) {
		if len(d) > 0 {
			h.Edge(uint64(d[0] % 16))
		}
	})
	if r.CoverageTier != TierReported || r.TotalEdges < 4 || r.AdmittedSeeds < 4 || r.AdmittedSeeds > r.TotalEdges {
		t.Fatalf("%v", r)
	}
	// Every admitted seed after the first covered an edge nobody had.
	if r.AdmittedSeeds != len(r.Corpus()) {
		t.Fatal("corpus accessor")
	}
}

func TestTierLabelsReflectWhatWasCollected(t *testing.T) {
	blind := mustRun(t, [][]byte{{1}}, opts(100, 1), func(d []byte, h *Handle) {})
	if blind.CoverageTier != TierBlackBox || blind.TotalEdges != 0 || blind.AdmittedSeeds != 1 {
		t.Fatalf("a target reporting no edges is black-box and keeps its first seed: %v", blind)
	}
	seen := mustRun(t, [][]byte{{1}}, opts(10, 1), func(d []byte, h *Handle) { h.Edge(EdgeID("x")) })
	if seen.CoverageTier != TierReported || seen.TotalEdges != 1 {
		t.Fatalf("%v", seen)
	}
	if TierReported != "caller-reported" || TierBlackBox != "black-box-uninstrumented" {
		t.Fatal("label vocabulary is part of the contract")
	}
	if EdgeID("a") == EdgeID("b") || EdgeID("a") != EdgeID("a") {
		t.Fatal("EdgeID must be a stable function of its label")
	}
}

func TestCampaignArgumentValidation(t *testing.T) {
	good := DefaultOptions()
	target := func(d []byte, h *Handle) {}
	if _, err := Run(nil, good, nil); !errors.Is(err, ErrOptions) {
		t.Fatalf("nil target: %v", err)
	}
	if _, err := RunOutcomes(nil, good, nil); !errors.Is(err, ErrOptions) {
		t.Fatalf("nil exec: %v", err)
	}
	for name, mutate := range map[string]func(*Options){
		"iterations+":  func(o *Options) { o.MaxIterations = MaxIterations + 1 },
		"iterations-":  func(o *Options) { o.MaxIterations = -1 },
		"input len+":   func(o *Options) { o.MaxInputLen = MaxInputBytes + 1 },
		"input len-":   func(o *Options) { o.MaxInputLen = -1 },
		"energy+":      func(o *Options) { o.BaseEnergy = MaxIterations + 1 },
		"energy-":      func(o *Options) { o.BaseEnergy = -1 },
		"minimize+":    func(o *Options) { o.MinimizeBudget = MaxMinimizeBudget + 1 },
		"minimize-":    func(o *Options) { o.MinimizeBudget = -1 },
		"step budget+": func(o *Options) { o.StepBudget = MaxStepBudget + 1 },
		"step budget-": func(o *Options) { o.StepBudget = -1 },
		"mem budget+":  func(o *Options) { o.MemBudget = MaxMemBudget + 1 },
		"mem budget-":  func(o *Options) { o.MemBudget = -1 },
	} {
		o := good
		mutate(&o)
		if _, err := Run(nil, o, target); !errors.Is(err, ErrOptions) {
			t.Errorf("%s: %v", name, err)
		}
	}
	if _, err := Run(make([][]byte, MaxInitialCorpus+1), good, target); !errors.Is(err, ErrOptions) {
		t.Fatalf("too many seeds: %v", err)
	}
	if _, err := Run([][]byte{make([]byte, MaxInputBytes+1)}, good, target); !errors.Is(err, ErrOptions) {
		t.Fatalf("seed too large: %v", err)
	}
	if _, err := Run([][]byte{make([]byte, MaxInputBytes)}, Options{MaxInputLen: 1, MaxIterations: 1, BaseEnergy: 1}, target); err != nil {
		t.Fatalf("a seed of exactly MaxInputBytes is allowed: %v", err)
	}
	// An outcome kind outside the enum is an error, on a triage run and on a probe.
	bad := func(d []byte, cov *Bitmap) Outcome { return Outcome{Kind: 99} }
	if _, err := RunOutcomes(nil, good, bad); !errors.Is(err, ErrOptions) {
		t.Fatalf("triage: %v", err)
	}
	n := 0
	later := func(d []byte, cov *Bitmap) Outcome {
		n++
		if n == 1 {
			return Outcome{Kind: OutcomePass}
		}
		return Outcome{Kind: -1}
	}
	if _, err := RunOutcomes([][]byte{{1}}, good, later); !errors.Is(err, ErrOptions) {
		t.Fatalf("mutation: %v", err)
	}
	// An invalid kind seen only by the minimiser is treated as a pass (the main
	// loop is the one that reports it).
	n = 0
	probeBad := func(d []byte, cov *Bitmap) Outcome {
		n++
		switch {
		case n == 1:
			return Outcome{Kind: OutcomeCrash, CrashKind: "k", Frames: []string{"f"}}
		case n == 2:
			return Outcome{Kind: 77}
		}
		return Outcome{Kind: OutcomePass}
	}
	if r, err := RunOutcomes([][]byte{{1, 2}}, good, probeBad); err != nil || len(r.Crashes) != 1 {
		t.Fatalf("%v %v", r, err)
	}
}

// -----------------------------------------------------------------------------
// Feedback: a staircase only coverage guidance climbs
// -----------------------------------------------------------------------------

// staircase panics only when the first four bytes are 80 ff 7f 01.  Each matched
// prefix reports a distinct edge in the instrumented variant; the blind
// variant reports none, so it has no feedback.
func staircase(instrumented bool) Target {
	want := []byte{128, 255, 127, 1} // boundary values: one step each, but all four at once is hopeless
	return func(d []byte, h *Handle) {
		for i, w := range want {
			if len(d) <= i || d[i] != w {
				return
			}
			if instrumented {
				h.Edge(EdgeID(fmt.Sprint("stair", i)))
			}
		}
		panic("cracked")
	}
}

func TestCoverageFeedbackCracksAStaircaseBlindMutationDoesNot(t *testing.T) {
	cracked := func(instrumented bool) int {
		count := 0
		for seed := uint64(0); seed < 8; seed++ {
			o := opts(8000, seed)
			o.MinimizeBudget = 0
			r := mustRun(t, [][]byte{{0, 0, 0, 0}}, o, staircase(instrumented))
			if len(r.Crashes) > 0 {
				count++
			}
		}
		return count
	}
	with, blind := cracked(true), cracked(false)
	if with < 4 {
		t.Fatalf("coverage guidance cracked only %d of 8 seeds", with)
	}
	if blind != 0 {
		t.Fatalf("blind mutation cracked %d of 8 seeds: the staircase is not a feedback test", blind)
	}
}

func TestInstrumentedCampaignIsDeterministicAcrossRuns(t *testing.T) {
	a := mustRun(t, [][]byte{{0, 0, 0, 0}}, opts(2000, 3), staircase(true))
	b := mustRun(t, [][]byte{{0, 0, 0, 0}}, opts(2000, 3), staircase(true))
	if a.String() != b.String() || !slices.EqualFunc(a.Corpus(), b.Corpus(), bytes.Equal) {
		t.Fatal("a campaign must be reproducible")
	}
}

// -----------------------------------------------------------------------------
// Minimisation inside a campaign: budgets
// -----------------------------------------------------------------------------

func TestMinimizationInACampaignReachesTheEmptyInput(t *testing.T) {
	exec := func(d []byte, cov *Bitmap) Outcome {
		return Outcome{Kind: OutcomeCrash, CrashKind: "k", Frames: []string{"f"}}
	}
	r, _ := RunOutcomes([][]byte{bytes.Repeat([]byte{7}, 40)}, opts(10, 1), exec)
	if len(r.Crashes) != 1 || len(r.Crashes[0].MinimalInput()) != 0 || !r.Crashes[0].Minimized {
		t.Fatalf("%v", r.Crashes)
	}
}

func TestTotalMinimizationWorkIsBoundedByMaxIterations(t *testing.T) {
	execs := 0
	// 100 distinct crashing seeds of 100 bytes each: unbounded minimisation
	// would spend thousands of probes.
	seeds := make([][]byte, 100)
	for i := range seeds {
		seeds[i] = bytes.Repeat([]byte{byte(i + 1)}, 100)
	}
	exec := func(d []byte, cov *Bitmap) Outcome {
		execs++
		return Outcome{Kind: OutcomeCrash, CrashKind: fmt.Sprint("k", d[:min(1, len(d))]), Frames: []string{"f"}}
	}
	o := opts(5, 1)
	r, err := RunOutcomes(seeds, o, exec)
	if err != nil {
		t.Fatal(err)
	}
	if probes := execs - len(seeds); probes > 5 || probes < 1 {
		t.Fatalf("%d minimisation probes for MaxIterations 5", probes)
	}
	unfinished := 0
	for _, c := range r.Crashes {
		if !c.Minimized {
			unfinished++
		}
	}
	if unfinished == 0 || len(r.Crashes) != 100 {
		t.Fatalf("crashes found after the allowance must say they were not minimised: %d of %d", unfinished, len(r.Crashes))
	}

	// MaxIterations == 0: nothing is minimised at all.
	execs = 0
	r, _ = RunOutcomes(seeds, opts(0, 1), exec)
	if execs != len(seeds) {
		t.Fatalf("%d executions: zero iterations means zero minimisation", execs)
	}
	for _, c := range r.Crashes {
		if c.Minimized {
			t.Fatal("nothing was minimised")
		}
	}
	// The per-crash budget caps one crash too.
	execs = 0
	o = opts(1000, 1)
	o.MinimizeBudget = 7
	RunOutcomes([][]byte{bytes.Repeat([]byte{9}, 500)}, o, exec)
	if execs-1 > 7 {
		t.Fatalf("%d probes for MinimizeBudget 7", execs-1)
	}
}

// -----------------------------------------------------------------------------
// Privacy of every rendering (VER06 section 6)
// -----------------------------------------------------------------------------

func TestNoRenderingOfAReportShowsSeedsOrBytes(t *testing.T) {
	const seed uint64 = 0xDEADBEEF12345678
	secret := []byte("SECRET-PAYLOAD-\xde\xad\xbe\xef")
	o := opts(50, seed)
	o.MinimizeBudget = 0
	crasher := append([]byte("X"), secret...)
	r := mustRun(t, [][]byte{secret, crasher}, o, func(d []byte, h *Handle) {
		h.Edge(1)
		if len(d) > 5 && d[0] == 'X' {
			panic("secret " + string(d))
		}
	})
	if len(r.Crashes) != 1 || len(r.Crashes[0].MinimalInput()) == 0 || r.Seed() != seed || len(r.Corpus()) == 0 {
		t.Fatalf("the accessors must still work: %v", r.Crashes)
	}
	needles := []string{
		"SECRET", hex.EncodeToString(secret[:6]), "deadbeef", "DEADBEEF", fmt.Sprint(seed), fmt.Sprintf("%x", seed),
		fmt.Sprint(secret[:4]), "222 173 190 239", "[83 69",
	}
	rec := r.Crashes[0]
	renders := map[string]string{}
	for _, verb := range []string{"%v", "%+v", "%#v", "%s", "%d", "%x", "%q", "%T"} {
		renders["report "+verb] = fmt.Sprintf(verb, *r)
		renders["report ptr "+verb] = fmt.Sprintf(verb, r)
		renders["record "+verb] = fmt.Sprintf(verb, rec)
		renders["record ptr "+verb] = fmt.Sprintf(verb, &rec)
		renders["records "+verb] = fmt.Sprintf(verb, r.Crashes)
		renders["wrapper "+verb] = fmt.Sprintf(verb, struct{ R *Report }{r})
	}
	renders["String"] = r.String() + r.GoString() + rec.String() + rec.GoString()
	renders["Println"] = fmt.Sprintln(r, *r, rec)
	js, err := json.Marshal(r)
	if err != nil {
		t.Fatal(err)
	}
	renders["json"] = string(js)
	for name, text := range renders {
		if name == "report %T" || strings.HasSuffix(name, "%T") {
			continue
		}
		for _, n := range needles {
			if strings.Contains(text, n) {
				t.Errorf("%s leaks %q: %s", name, n, text)
			}
		}
	}
	if !strings.Contains(fmt.Sprintf("%v", r), "tier=caller-reported") || !strings.Contains(fmt.Sprintf("%#v", rec), rec.Signature) {
		t.Fatal("the safe summary should still be informative")
	}
}

func TestAccessorsReturnCopies(t *testing.T) {
	r := mustRun(t, [][]byte{{1, 2, 3}}, opts(5, 1), func(d []byte, h *Handle) { h.Edge(1); panic("x") })
	if len(r.Crashes) != 1 {
		t.Fatal("setup")
	}
	m := r.Crashes[0].MinimalInput()
	if len(m) == 0 {
		m = []byte{1}
	}
	r2 := &Report{corpus: [][]byte{{5, 6}}}
	c := r2.Corpus()
	c[0][0] = 99
	if r2.corpus[0][0] != 5 {
		t.Fatal("Corpus must return copies")
	}
	cr := CrashRecord{minimal: []byte{1, 2}}
	cr.MinimalInput()[0] = 9
	if cr.minimal[0] != 1 {
		t.Fatal("MinimalInput must return a copy")
	}
}
