package prng

import (
	"math/big"
	"testing"
)

func mustBig(t *testing.T, s string) *big.Int {
	t.Helper()
	v, ok := new(big.Int).SetString(s, 10)
	if !ok {
		t.Fatalf("bad integer literal %q", s)
	}
	return v
}

// Vectors below were produced by the independent reference
// code/scripts/pbt_conformance.py, never by this package.

func TestSplitMix64PinnedVector(t *testing.T) {
	r := New(0)
	for i, want := range []uint64{0xe220a8397b1dcdaf, 0x6e789e6aa1b965f4, 0x06c45d188009454f} {
		if got := r.NextU64(); got != want {
			t.Fatalf("output %d = %#x, want %#x", i, got, want)
		}
	}
}

func TestZeroValueIsSeedZero(t *testing.T) {
	var r SplitMix64
	if got := r.NextU64(); got != 0xe220a8397b1dcdaf {
		t.Fatalf("zero value first output %#x", got)
	}
}

func TestStateRoundTrip(t *testing.T) {
	r := New(7)
	r.NextU64()
	saved := r.State()
	a := r.NextU64()
	r.SetState(saved)
	if b := r.NextU64(); a != b {
		t.Fatalf("restored state gave %#x, want %#x", b, a)
	}
}

func TestBoundedSmallVector(t *testing.T) {
	r := New(5)
	for i, want := range []uint64{3, 5, 2, 2, 3} {
		if got := r.Bounded(7); got != want {
			t.Fatalf("draw %d = %d, want %d", i, got, want)
		}
	}
}

// A span of 2^63+1 rejects raw values below (2^64 - n) mod n = 2^63 - 1, about
// half of all draws.  A threshold that is always 0 (the Python lane's old bug)
// would change these outputs.
func TestBoundedWideThresholdIsApplied(t *testing.T) {
	n := uint64(1)<<63 + 1
	r := New(5)
	want := []uint64{4654242949169100535, 8957066056171264800, 204786321411665706, 1908141438795372386, 6903704994719435514}
	for i, w := range want {
		if got := r.Bounded(n); got != w {
			t.Fatalf("draw %d = %d, want %d", i, got, w)
		}
	}
}

func TestBoundedOneConsumesOneWord(t *testing.T) {
	r := New(5)
	for i := 0; i < 3; i++ {
		if got := r.Bounded(1); got != 0 {
			t.Fatalf("Bounded(1) = %d", got)
		}
	}
	if r.State() != 15755400384260043844 {
		t.Fatalf("state after three Bounded(1) = %d: each must spend exactly one word", r.State())
	}
}

func TestBoundedZeroPanics(t *testing.T) {
	defer func() {
		if recover() == nil {
			t.Fatal("Bounded(0) must panic")
		}
	}()
	New(1).Bounded(0)
}

func TestBoundedStaysInRangeAcrossSeeds(t *testing.T) {
	for seed := uint64(0); seed < 2000; seed++ {
		r := New(seed)
		for _, n := range []uint64{1, 2, 3, 7, 255, 256, 1000, 1<<32 + 1, 1<<63 + 1, ^uint64(0)} {
			if got := r.Bounded(n); got >= n {
				t.Fatalf("seed %d: Bounded(%d) = %d out of range", seed, n, got)
			}
		}
	}
}

func TestBoundedIsRoughlyUniform(t *testing.T) {
	r := New(99)
	var counts [3]int
	const draws = 30000
	for i := 0; i < draws; i++ {
		counts[r.Bounded(3)]++
	}
	for k, c := range counts {
		if c < draws/3-600 || c > draws/3+600 {
			t.Fatalf("residue %d drawn %d times of %d", k, c, draws)
		}
	}
}

func TestBoundedBigSmallSpansMatchBounded(t *testing.T) {
	a, b := New(11), New(11)
	for i := 0; i < 50; i++ {
		got := a.BoundedBig(big.NewInt(1000))
		want := b.Bounded(1000)
		if got.Uint64() != want {
			t.Fatalf("draw %d: %v vs %d", i, got, want)
		}
	}
}

func TestBoundedBigFullWordSpanIsRawWord(t *testing.T) {
	a, b := New(3), New(3)
	got := a.BoundedBig(new(big.Int).Lsh(big.NewInt(1), 64))
	if got.Uint64() != b.NextU64() {
		t.Fatal("span 2^64 must be one raw word")
	}
}

func TestBoundedBigWideVectors(t *testing.T) {
	r := New(9)
	span := new(big.Int).Add(new(big.Int).Lsh(big.NewInt(1), 130), big.NewInt(7))
	for i, want := range []string{
		"255448235011303640280899557894340534535",
		"89342299821301620937131352229074699094",
		"334684220971821213071345962061252959748",
	} {
		if got := r.BoundedBig(span); got.Cmp(mustBig(t, want)) != 0 {
			t.Fatalf("2^130+7 draw %d = %v, want %s", i, got, want)
		}
	}
	r = New(9)
	span = new(big.Int).Lsh(big.NewInt(3), 126)
	for i, want := range []string{
		"232196006557297741543928818731175635554",
		"90284548084330202170444353266735210592",
		"89342299821301620962466552557356346366",
	} {
		if got := r.BoundedBig(span); got.Cmp(mustBig(t, want)) != 0 {
			t.Fatalf("3*2^126 draw %d = %v, want %s", i, got, want)
		}
	}
}

// VER05 section 7.2: raw % span over-weights the low third of a 3*2^126 span
// (it would be drawn about half the time).  Rejection sampling gives thirds.
func TestBoundedBigThirdsAreUnbiased(t *testing.T) {
	span := new(big.Int).Lsh(big.NewInt(3), 126)
	third := new(big.Int).Lsh(big.NewInt(1), 126)
	r := New(2024)
	var counts [3]int
	const draws = 30000
	for i := 0; i < draws; i++ {
		v := r.BoundedBig(span)
		if v.Sign() < 0 || v.Cmp(span) >= 0 {
			t.Fatalf("draw %v out of range", v)
		}
		counts[new(big.Int).Div(v, third).Int64()]++
	}
	for k, c := range counts {
		if c < draws/3-700 || c > draws/3+700 {
			t.Fatalf("third %d drawn %d times of %d", k, c, draws)
		}
	}
}

func TestBoundedBigRejectsBadSpan(t *testing.T) {
	for _, span := range []*big.Int{nil, big.NewInt(0), big.NewInt(-5)} {
		func() {
			defer func() {
				if recover() == nil {
					t.Fatalf("span %v must panic", span)
				}
			}()
			New(1).BoundedBig(span)
		}()
	}
}

func TestFNV1a64Vectors(t *testing.T) {
	for in, want := range map[string]uint64{
		"":       0xcbf29ce484222325,
		"a":      0xaf63dc4c8601ec8c,
		"foobar": 0x85944171f73967e8,
	} {
		if got := FNV1a64([]byte(in)); got != want {
			t.Errorf("FNV1a64(%q) = %#x, want %#x", in, got, want)
		}
		if got := FNV1a64String(in); got != want {
			t.Errorf("FNV1a64String(%q) = %#x, want %#x", in, got, want)
		}
	}
}

func TestSubSeedVectors(t *testing.T) {
	cases := []struct {
		run  uint64
		id   string
		want uint64
	}{
		{0, "x", 0x338262d8f096398f},
		{42, "prop/ct_compare_eq_reflexive", 0xcca3a0330363d462},
		{^uint64(0), "", 0x30ef66c3e79dda4e},
	}
	for _, c := range cases {
		if got := SubSeed(c.run, c.id); got != c.want {
			t.Errorf("SubSeed(%d, %q) = %#x, want %#x", c.run, c.id, got, c.want)
		}
	}
	// Without the final scramble the answer would be run XOR hash, which is
	// what the TypeScript and Dart lanes compute; this lane must differ.
	if SubSeed(0, "x") == FNV1a64String("x") {
		t.Fatal("SubSeed must scramble with a final NextU64")
	}
}
