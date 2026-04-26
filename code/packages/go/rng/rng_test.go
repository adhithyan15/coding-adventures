package rng

import (
	"math"
	"testing"
)

// knownLCG, knownXS, knownPCG are the reference outputs for seed=1.
// All language implementations must produce exactly these values.
var (
	knownLCG = []uint32{1817669548, 2187888307, 2784682393}
	knownXS  = []uint32{1082269761, 201397313, 1854285353}
	knownPCG = []uint32{1412771199, 1791099446, 124312908}
)

// ── LCG ──────────────────────────────────────────────────────────────────────

func TestLCG_Determinism(t *testing.T) {
	g1, g2 := NewLCG(42), NewLCG(42)
	for i := 0; i < 100; i++ {
		if g1.NextU32() != g2.NextU32() {
			t.Fatal("same seed diverged")
		}
	}
}

func TestLCG_KnownValues(t *testing.T) {
	g := NewLCG(1)
	for i, want := range knownLCG {
		if got := g.NextU32(); got != want {
			t.Errorf("call %d: got %d, want %d", i+1, got, want)
		}
	}
}

func TestLCG_Seed0(t *testing.T) {
	g := NewLCG(0)
	want := uint32(lcgIncrement >> 32)
	if got := g.NextU32(); got != want {
		t.Errorf("seed=0 first output: got %d, want %d", got, want)
	}
}

func TestLCG_DifferentSeedsDiverge(t *testing.T) {
	g1, g2 := NewLCG(1), NewLCG(2)
	same := true
	for i := 0; i < 10; i++ {
		if g1.NextU32() != g2.NextU32() {
			same = false
			break
		}
	}
	if same {
		t.Fatal("seeds 1 and 2 produced identical outputs for 10 calls")
	}
}

func TestLCG_NextU64Composition(t *testing.T) {
	g1, g2 := NewLCG(99), NewLCG(99)
	u64 := g1.NextU64()
	hi, lo := uint64(g2.NextU32()), uint64(g2.NextU32())
	if u64 != (hi<<32)|lo {
		t.Errorf("NextU64 = %d; want (hi<<32)|lo = %d", u64, (hi<<32)|lo)
	}
}

func TestLCG_FloatRange(t *testing.T) {
	g := NewLCG(12345)
	for i := 0; i < 100000; i++ {
		f := g.NextFloat()
		if f < 0 || f >= 1 {
			t.Fatalf("float out of [0,1): %v", f)
		}
	}
}

func TestLCG_IntRangeBounds(t *testing.T) {
	g := NewLCG(777)
	for i := 0; i < 10000; i++ {
		v := g.NextIntInRange(1, 6)
		if v < 1 || v > 6 {
			t.Fatalf("die roll out of [1,6]: %d", v)
		}
	}
}

func TestLCG_SingleValueRange(t *testing.T) {
	g := NewLCG(42)
	for i := 0; i < 100; i++ {
		if v := g.NextIntInRange(7, 7); v != 7 {
			t.Fatalf("single-value range returned %d", v)
		}
	}
}

func TestLCG_Distribution(t *testing.T) {
	g := NewLCG(54321)
	counts := make([]int, 10)
	const n = 100000
	for i := 0; i < n; i++ {
		counts[g.NextIntInRange(0, 9)]++
	}
	expected := float64(n) / 10
	for i, c := range counts {
		if ratio := float64(c) / expected; ratio < 0.7 || ratio > 1.3 {
			t.Errorf("bucket %d: count %d, ratio %.2f outside ±30%%", i, c, ratio)
		}
	}
}

// ── Xorshift64 ────────────────────────────────────────────────────────────────

func TestXS_Determinism(t *testing.T) {
	g1, g2 := NewXorshift64(42), NewXorshift64(42)
	for i := 0; i < 100; i++ {
		if g1.NextU32() != g2.NextU32() {
			t.Fatal("same seed diverged")
		}
	}
}

func TestXS_KnownValues(t *testing.T) {
	g := NewXorshift64(1)
	for i, want := range knownXS {
		if got := g.NextU32(); got != want {
			t.Errorf("call %d: got %d, want %d", i+1, got, want)
		}
	}
}

func TestXS_Seed0ReplacedWith1(t *testing.T) {
	g0, g1 := NewXorshift64(0), NewXorshift64(1)
	for i := 0; i < 10; i++ {
		if g0.NextU32() != g1.NextU32() {
			t.Fatalf("seed=0 diverged from seed=1 at call %d", i+1)
		}
	}
}

func TestXS_StateNeverZero(t *testing.T) {
	g := NewXorshift64(1)
	for i := 0; i < 100000; i++ {
		g.NextU32()
		if g.state == 0 {
			t.Fatalf("state became 0 after %d calls", i+1)
		}
	}
}

func TestXS_FloatRange(t *testing.T) {
	g := NewXorshift64(99)
	for i := 0; i < 100000; i++ {
		f := g.NextFloat()
		if f < 0 || f >= 1 {
			t.Fatalf("float out of [0,1): %v", f)
		}
	}
}

func TestXS_IntRangeBounds(t *testing.T) {
	g := NewXorshift64(7)
	for i := 0; i < 10000; i++ {
		v := g.NextIntInRange(0, 100)
		if v < 0 || v > 100 {
			t.Fatalf("out of [0,100]: %d", v)
		}
	}
}

func TestXS_NextU64Composition(t *testing.T) {
	g1, g2 := NewXorshift64(99), NewXorshift64(99)
	u64 := g1.NextU64()
	hi, lo := uint64(g2.NextU32()), uint64(g2.NextU32())
	if u64 != (hi<<32)|lo {
		t.Errorf("NextU64 mismatch")
	}
}

// ── PCG32 ─────────────────────────────────────────────────────────────────────

func TestPCG_Determinism(t *testing.T) {
	g1, g2 := NewPCG32(42), NewPCG32(42)
	for i := 0; i < 100; i++ {
		if g1.NextU32() != g2.NextU32() {
			t.Fatal("same seed diverged")
		}
	}
}

func TestPCG_KnownValues(t *testing.T) {
	g := NewPCG32(1)
	for i, want := range knownPCG {
		if got := g.NextU32(); got != want {
			t.Errorf("call %d: got %d, want %d", i+1, got, want)
		}
	}
}

func TestPCG_DifferentSeedsDiverge(t *testing.T) {
	g1, g2 := NewPCG32(1), NewPCG32(2)
	same := true
	for i := 0; i < 10; i++ {
		if g1.NextU32() != g2.NextU32() {
			same = false
			break
		}
	}
	if same {
		t.Fatal("PCG seeds 1 and 2 produced identical outputs for 10 calls")
	}
}

func TestPCG_FloatRange(t *testing.T) {
	g := NewPCG32(8675309)
	for i := 0; i < 100000; i++ {
		f := g.NextFloat()
		if f < 0 || f >= 1 {
			t.Fatalf("float out of [0,1): %v", f)
		}
	}
}

func TestPCG_IntRangeBounds(t *testing.T) {
	g := NewPCG32(0)
	for i := 0; i < 10000; i++ {
		v := g.NextIntInRange(-10, 10)
		if v < -10 || v > 10 {
			t.Fatalf("out of [-10,10]: %d", v)
		}
	}
}

func TestPCG_Distribution(t *testing.T) {
	g := NewPCG32(112233)
	counts := make([]int, 10)
	const n = 100000
	for i := 0; i < n; i++ {
		counts[g.NextIntInRange(0, 9)]++
	}
	expected := float64(n) / 10
	for i, c := range counts {
		if ratio := float64(c) / expected; ratio < 0.7 || ratio > 1.3 {
			t.Errorf("bucket %d: count %d, ratio %.2f outside ±30%%", i, c, ratio)
		}
	}
}

func TestPCG_NextU64Composition(t *testing.T) {
	g1, g2 := NewPCG32(5), NewPCG32(5)
	u64 := g1.NextU64()
	hi, lo := uint64(g2.NextU32()), uint64(g2.NextU32())
	if u64 != (hi<<32)|lo {
		t.Errorf("NextU64 mismatch")
	}
}

func TestPCG_SingleValueRange(t *testing.T) {
	g := NewPCG32(1234)
	for i := 0; i < 50; i++ {
		if v := g.NextIntInRange(42, 42); v != 42 {
			t.Fatalf("single-value range returned %d", v)
		}
	}
}

// ── Cross-generator ───────────────────────────────────────────────────────────

func TestAllFloatPrecision(t *testing.T) {
	// NextFloat must never reach exactly 1.0.
	// With 2^32 possible u32 values and divisor 2^32, max = (2^32-1)/2^32 < 1.
	g := NewPCG32(999)
	for i := 0; i < 100000; i++ {
		f := g.NextFloat()
		if f < 0 || f >= 1 {
			t.Fatalf("float out of [0,1): %v", f)
		}
		if math.IsNaN(f) || math.IsInf(f, 0) {
			t.Fatalf("float is NaN or Inf")
		}
	}
}

func TestAllU32InRange(t *testing.T) {
	lcg := NewLCG(7)
	xs := NewXorshift64(7)
	pcg := NewPCG32(7)
	for i := 0; i < 1000; i++ {
		// All outputs must fit in uint32 — the type already enforces this,
		// but the test documents the contract.
		_ = lcg.NextU32()
		_ = xs.NextU32()
		_ = pcg.NextU32()
	}
}
