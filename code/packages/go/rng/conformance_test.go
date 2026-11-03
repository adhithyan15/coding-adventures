package rng

// The shared rng corpus (spec rng.md, "Conformance corpus").
//
// code/specs/fixtures/rng-v1/cases.json pins exact outputs every language's
// rng package must reproduce; its results come from the independent
// reference code/scripts/rng_conformance.py. Passing here means this package
// and that reference agree call for call.

import (
	"encoding/json"
	"fmt"
	"math"
	"os"
	"strconv"
	"testing"
)

const corpusPath = "../../../specs/fixtures/rng-v1/cases.json"

type corpusCall struct {
	Op         string `json:"op"`
	Min        string `json:"min"`
	Max        string `json:"max"`
	Result     string `json:"result"`
	ResultBits string `json:"result_bits"`
	Error      string `json:"error"`
}

type corpusCase struct {
	ID        string       `json:"id"`
	Generator string       `json:"generator"`
	Seed      string       `json:"seed"`
	Calls     []corpusCall `json:"calls"`
}

// generator is the method set the corpus drives.
type generator interface {
	NextU32() uint32
	NextU64() uint64
	NextFloat() float64
	NextIntInRange(min, max int64) int64
}

func newGenerator(t *testing.T, name string, seed uint64) generator {
	switch name {
	case "lcg":
		return NewLCG(seed)
	case "xorshift64":
		return NewXorshift64(seed)
	case "pcg32":
		return NewPCG32(seed)
	}
	t.Fatalf("unknown generator %q", name)
	return nil
}

func mustInt64(t *testing.T, text string) int64 {
	value, err := strconv.ParseInt(text, 10, 64)
	if err != nil {
		t.Fatalf("bad int64 %q: %v", text, err)
	}
	return value
}

// panics reports whether f panics.
func panics(f func()) (didPanic bool) {
	defer func() { didPanic = recover() != nil }()
	f()
	return false
}

func TestCorpus(t *testing.T) {
	data, err := os.ReadFile(corpusPath)
	if err != nil {
		t.Fatal(err)
	}
	var document struct {
		Cases []corpusCase `json:"cases"`
	}
	if err := json.Unmarshal(data, &document); err != nil {
		t.Fatal(err)
	}
	if len(document.Cases) < 27 {
		t.Fatalf("the corpus shrank to %d cases", len(document.Cases))
	}
	for _, c := range document.Cases {
		t.Run(c.ID, func(t *testing.T) {
			seed, err := strconv.ParseUint(c.Seed, 10, 64)
			if err != nil {
				t.Fatal(err)
			}
			g := newGenerator(t, c.Generator, seed)
			for i, call := range c.Calls {
				at := fmt.Sprintf("call %d (%s)", i, call.Op)
				var got, want string
				switch call.Op {
				case "next_u32":
					got, want = strconv.FormatUint(uint64(g.NextU32()), 10), call.Result
				case "next_u64":
					got, want = strconv.FormatUint(g.NextU64(), 10), call.Result
				case "next_float":
					got, want = fmt.Sprintf("%016x", math.Float64bits(g.NextFloat())), call.ResultBits
				case "next_int_in_range":
					lo, hi := mustInt64(t, call.Min), mustInt64(t, call.Max)
					if call.Error != "" {
						// Go reports an invalid range by panicking, before
						// any draw, so the sequence carries on untouched.
						if !panics(func() { g.NextIntInRange(lo, hi) }) {
							t.Fatalf("%s: expected a panic", at)
						}
						continue
					}
					got, want = strconv.FormatInt(g.NextIntInRange(lo, hi), 10), call.Result
				default:
					t.Fatalf("%s: unknown op", at)
				}
				if got != want {
					t.Fatalf("%s: got %s, want %s", at, got, want)
				}
			}
		})
	}
}

// ── Regressions for the v1 range bug ─────────────────────────────────────────

func TestWideRangesReachBeyondTwoToThe32(t *testing.T) {
	// v1 drew 32 bits for every span: 64 draws over 2^40 values all landing
	// below 2^32 has probability 2^-512.
	g := NewPCG32(3)
	for i := 0; i < 64; i++ {
		if g.NextIntInRange(0, 1<<40) >= 1<<32 {
			return
		}
	}
	t.Fatal("no draw above 2^32")
}

func TestWholeInt64DomainNoLongerDividesByZero(t *testing.T) {
	g := NewLCG(3)
	negative, positive := false, false
	for i := 0; i < 64; i++ {
		v := g.NextIntInRange(math.MinInt64, math.MaxInt64)
		negative, positive = negative || v < 0, positive || v > 0
	}
	if !negative || !positive {
		t.Fatal("whole-domain draws did not cover both signs")
	}
}

func TestHalfDomainPlusOneTerminates(t *testing.T) {
	// span 2^63 + 1: v1's threshold exceeded every 32-bit draw and looped
	// forever. Now about half the 64-bit draws are accepted.
	g := NewXorshift64(3)
	for i := 0; i < 100; i++ {
		if v := g.NextIntInRange(math.MinInt64, 0); v > 0 {
			t.Fatalf("out of range: %d", v)
		}
	}
}

func TestMinAboveMaxPanics(t *testing.T) {
	if !panics(func() { NewLCG(1).NextIntInRange(1, 0) }) {
		t.Fatal("expected a panic")
	}
}
