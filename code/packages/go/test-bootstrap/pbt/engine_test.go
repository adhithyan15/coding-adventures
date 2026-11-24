package pbt

import (
	"errors"
	"math/big"
	"slices"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/test-bootstrap/prng"
)

func bi(v int64) *big.Int { return big.NewInt(v) }

func sampleOK[T any](t *testing.T, g Gen[T], rng *prng.SplitMix64) T {
	t.Helper()
	v, err := g.Sample(rng)
	if err != nil {
		t.Fatal(err)
	}
	return v
}

// -----------------------------------------------------------------------------
// Generators
// -----------------------------------------------------------------------------

func TestIntStaysInRangeAcrossSeeds(t *testing.T) {
	ranges := [][2]*big.Int{
		{bi(10), bi(20)},
		{bi(-5), bi(-5)},
		{bi(0), new(big.Int).Lsh(bi(1), 40)},
		{minI128, maxI128},
		{new(big.Int).Neg(new(big.Int).Lsh(bi(1), 200)), new(big.Int).Lsh(bi(1), 200)},
	}
	for _, rg := range ranges {
		g := Int(rg[0], rg[1])
		for seed := uint64(0); seed < 300; seed++ {
			v := sampleOK(t, g, prng.New(seed))
			if v.Cmp(rg[0]) < 0 || v.Cmp(rg[1]) > 0 {
				t.Fatalf("seed %d: %v outside [%v, %v]", seed, v, rg[0], rg[1])
			}
		}
	}
}

func TestIntRejectsBadBounds(t *testing.T) {
	for name, f := range map[string]func(){
		"min above max": func() { Int(bi(2), bi(1)) },
		"nil min":       func() { Int(nil, bi(1)) },
		"nil max":       func() { Int(bi(1), nil) },
		"int64 swapped": func() { Int64(2, 1) },
	} {
		func() {
			defer func() {
				if recover() == nil {
					t.Errorf("%s must panic", name)
				}
			}()
			f()
		}()
	}
}

func TestInt64MatchesIntDraws(t *testing.T) {
	cases := [][2]int64{{-100, 100}, {0, 0}, {-1 << 63, 1<<63 - 1}, {-1 << 63, 0}, {5, 1 << 40}}
	for _, c := range cases {
		a, b := prng.New(77), prng.New(77)
		gi, gb := Int64(c[0], c[1]), Int(bi(c[0]), bi(c[1]))
		for i := 0; i < 50; i++ {
			x := sampleOK(t, gi, a)
			y := sampleOK(t, gb, b)
			if bi(x).Cmp(y) != 0 {
				t.Fatalf("range %v draw %d: %d vs %v", c, i, x, y)
			}
		}
	}
}

func TestI128CoversBothSignsAndStaysInRange(t *testing.T) {
	g := I128()
	neg, pos := false, false
	for seed := uint64(0); seed < 200; seed++ {
		v := sampleOK(t, g, prng.New(seed))
		if v.Cmp(minI128) < 0 || v.Cmp(maxI128) > 0 {
			t.Fatalf("%v outside i128", v)
		}
		neg = neg || v.Sign() < 0
		pos = pos || v.Sign() > 0
	}
	if !neg || !pos {
		t.Fatalf("both signs expected over 200 seeds: neg=%v pos=%v", neg, pos)
	}
}

func TestPrimitivesAndTheirShrinkers(t *testing.T) {
	rng := prng.New(5)
	if v := sampleOK(t, U8(), rng); v > 255 {
		t.Fatal("u8 range")
	}
	_ = sampleOK(t, U64(), rng)
	_ = sampleOK(t, Bool(), rng)

	// u8: x < 7 holds, so 200 shrinks to exactly 7.
	got, steps := U8().Shrink(200, func(x uint8) bool { return x >= 7 }, 1000)
	if got != 7 || steps == 0 {
		t.Fatalf("u8 shrink = %d in %d steps", got, steps)
	}
	// u64 toward 0 with everything failing: the target probe, one step.
	if got, steps := U64().Shrink(1<<63, func(uint64) bool { return true }, 1000); got != 0 || steps != 1 {
		t.Fatalf("u64 shrink = %d in %d", got, steps)
	}
	// i128 at the extremes: no overflow, target 0 fails -> one step.
	got128, steps := I128().Shrink(new(big.Int).Set(minI128), func(*big.Int) bool { return true }, 1000)
	if got128.Sign() != 0 || steps != 1 {
		t.Fatalf("i128 shrink = %v in %d", got128, steps)
	}
	// int64: stays inside its bounds.
	gi := Int64(10, 100)
	if got, _ := gi.Shrink(90, func(x int64) bool { return x > 40 }, 1000); got != 41 {
		t.Fatalf("int64 shrink = %d, want 41", got)
	}
	// Bool shrinks true->false only when false still fails and a budget exists.
	if v, n := Bool().Shrink(true, func(bool) bool { return true }, 5); v || n != 1 {
		t.Fatal("bool should shrink to false")
	}
	if v, n := Bool().Shrink(true, func(b bool) bool { return b }, 5); !v || n != 0 {
		t.Fatal("bool must stay true when false does not fail")
	}
	if v, n := Bool().Shrink(true, func(bool) bool { return true }, 0); !v || n != 0 {
		t.Fatal("bool must not shrink without budget")
	}
	if v, n := Bool().Shrink(false, func(bool) bool { return true }, 5); v || n != 0 {
		t.Fatal("false has nothing to shrink")
	}
}

func TestClampZeroTargets(t *testing.T) {
	cases := []struct{ lo, hi, want int64 }{{5, 10, 5}, {-10, -3, -3}, {-4, 4, 0}}
	for _, c := range cases {
		if got := clampZero(bi(c.lo), bi(c.hi)); got.Int64() != c.want {
			t.Errorf("clampZero(%d,%d) = %v, want %d", c.lo, c.hi, got, c.want)
		}
	}
	// Integrated shrinking never leaves a refinement: 5..10 never becomes 0.
	g := Int(bi(5), bi(10))
	got, _ := g.Shrink(bi(9), func(*big.Int) bool { return true }, 100)
	if got.Int64() != 5 {
		t.Fatalf("Int(5,10) shrank to %v", got)
	}
}

func TestBytesAndListLengthsAndStreamPosition(t *testing.T) {
	for seed := uint64(0); seed < 200; seed++ {
		b := sampleOK(t, Bytes(2, 9), prng.New(seed))
		if len(b) < 2 || len(b) > 9 {
			t.Fatalf("bytes length %d", len(b))
		}
		l := sampleOK(t, List(U8(), 0, 5), prng.New(seed))
		if len(l) > 5 {
			t.Fatalf("list length %d", len(l))
		}
	}
	// A fixed length still spends one word on the length: 1 + 4 words.
	r := prng.New(1)
	sampleOK(t, Bytes(4, 4), r)
	ref := prng.New(1)
	for i := 0; i < 5; i++ {
		ref.NextU64()
	}
	if r.State() != ref.State() {
		t.Fatal("Bytes(4,4) must consume exactly 5 stream words")
	}
	for name, f := range map[string]func(){
		"bytes inverted": func() { Bytes(3, 2) },
		"bytes negative": func() { Bytes(-1, 2) },
		"bytes huge":     func() { Bytes(0, MaxInputBytes+1) },
		"list inverted":  func() { List(U8(), 3, 2) },
		"list huge":      func() { List(U8(), 0, MaxListLen+1) },
	} {
		func() {
			defer func() {
				if recover() == nil {
					t.Errorf("%s must panic", name)
				}
			}()
			f()
		}()
	}
}

func TestListRespectsMinLenWhileShrinking(t *testing.T) {
	g := List(U8(), 3, 8)
	got, _ := g.Shrink([]uint8{9, 9, 9, 9, 9, 9}, func(l []uint8) bool { return len(l) >= 3 }, 1000)
	if len(got) != 3 {
		t.Fatalf("shrunk below minLen: %v", got)
	}
	for _, v := range got {
		if v != 0 {
			t.Fatalf("elements should shrink to 0: %v", got)
		}
	}
	bg := Bytes(2, 8)
	bgot, _ := bg.Shrink([]byte{7, 7, 7, 7}, func(b []byte) bool { return len(b) >= 2 }, 1000)
	if len(bgot) != 2 {
		t.Fatalf("bytes shrunk below minLen: %v", bgot)
	}
	// minLen > 0: the prefix probe is accepted as one step.
	pg, steps := shrinkSequence([]int{5, 6, 7, 8}, func(l []int) bool { return len(l) >= 2 }, 1000, 2, nil)
	if !slices.Equal(pg, []int{5, 6}) || steps != 1 {
		t.Fatalf("prefix probe = %v in %d steps", pg, steps)
	}
}

func TestFilter(t *testing.T) {
	calls := 0
	never := FilterN(U8(), func(uint8) bool { calls++; return false }, 7)
	if _, err := never.Sample(prng.New(1)); !errors.Is(err, ErrExhausted) || calls != 7 {
		t.Fatalf("err=%v calls=%d: want ErrExhausted after exactly 7 samples", err, calls)
	}
	even := Filter(U8(), func(x uint8) bool { return x%2 == 0 })
	for seed := uint64(0); seed < 100; seed++ {
		if v := sampleOK(t, even, prng.New(seed)); v%2 != 0 {
			t.Fatalf("filter let %d through", v)
		}
	}
	// Shrinking is preserved and respects the predicate: odd candidates are
	// not valid inputs, so the shrinker lands on the smallest even failure.
	got, _ := even.Shrink(100, func(x uint8) bool { return x >= 11 }, 1000)
	// 100 -> 50 (even) is accepted; 25 and 49 are odd, so they are not valid
	// inputs and the walk stops: a local minimum, as VER05 section 7.3 says.
	if got != 50 {
		t.Fatalf("filtered shrink gave %d, want the local minimum 50", got)
	}
	// An error from the inner generator propagates.
	bad := FilterN(NewGen(func(*prng.SplitMix64) (int, error) { return 0, errors.New("boom") }, nil), func(int) bool { return true }, 3)
	if _, err := bad.Sample(prng.New(1)); err == nil || err.Error() != "boom" {
		t.Fatalf("inner error lost: %v", err)
	}
	func() {
		defer func() {
			if recover() == nil {
				t.Error("FilterN(0) must panic")
			}
		}()
		FilterN(U8(), func(uint8) bool { return true }, 0)
	}()
}

func TestMapFlatMapConstantAndZeroGen(t *testing.T) {
	doubled := Map(U8(), func(x uint8) int { return int(x) * 2 })
	if v := sampleOK(t, doubled, prng.New(3)); v%2 != 0 {
		t.Fatalf("map result %d", v)
	}
	if got, n := doubled.Shrink(8, func(int) bool { return true }, 10); got != 8 || n != 0 {
		t.Fatal("Map must drop shrinking")
	}
	// Map propagates an inner error.
	failing := Map(NewGen(func(*prng.SplitMix64) (int, error) { return 0, ErrExhausted }, nil), func(int) int { return 1 })
	if _, err := failing.Sample(prng.New(1)); !errors.Is(err, ErrExhausted) {
		t.Fatal("Map must propagate errors")
	}
	// FlatMap: a length, then that many bytes.
	fm := FlatMap(Int64(1, 4), func(n int64) Gen[[]byte] { return Bytes(int(n), int(n)) })
	for seed := uint64(0); seed < 50; seed++ {
		if v := sampleOK(t, fm, prng.New(seed)); len(v) < 1 || len(v) > 4 {
			t.Fatalf("flat map length %d", len(v))
		}
	}
	if _, err := FlatMap(failing, func(int) Gen[int] { return Constant(1) }).Sample(prng.New(1)); err == nil {
		t.Fatal("FlatMap must propagate errors")
	}
	r := prng.New(9)
	before := r.State()
	if v := sampleOK(t, Constant("x"), r); v != "x" || r.State() != before {
		t.Fatal("Constant must draw nothing")
	}
	var zero Gen[int]
	if _, err := zero.Sample(prng.New(1)); !errors.Is(err, ErrExhausted) {
		t.Fatalf("zero Gen must error, got %v", err)
	}
	if v, n := zero.Shrink(4, func(int) bool { return true }, 5); v != 4 || n != 0 {
		t.Fatal("zero Gen shrink must be a no-op")
	}
}

func TestOneOfAndFrequency(t *testing.T) {
	g := OneOf(Constant(0), Constant(1), Constant(2))
	var counts [3]int
	rng := prng.New(4)
	for i := 0; i < 3000; i++ {
		counts[sampleOK(t, g, rng)]++
	}
	for k, c := range counts {
		if c < 800 || c > 1200 {
			t.Fatalf("OneOf arm %d drawn %d times of 3000", k, c)
		}
	}
	// Frequency follows its weights and never selects a zero-weight arm.
	f := Frequency(Arm[int]{1, Constant(0)}, Arm[int]{0, Constant(99)}, Arm[int]{3, Constant(1)})
	var seen [2]int
	rng = prng.New(8)
	for i := 0; i < 4000; i++ {
		v := sampleOK(t, f, rng)
		if v == 99 {
			t.Fatal("zero-weight arm selected")
		}
		seen[v]++
	}
	if seen[1] < 2800 || seen[1] > 3200 {
		t.Fatalf("weight 3 of 4 drawn %d times of 4000", seen[1])
	}
	// A weight-only-on-the-last-arm list always picks it.
	only := Frequency(Arm[int]{0, Constant(7)}, Arm[int]{5, Constant(8)})
	for i := 0; i < 50; i++ {
		if sampleOK(t, only, rng) != 8 {
			t.Fatal("zero-weight first arm selected")
		}
	}
	// Errors from the chosen arm propagate.
	bad := OneOf(NewGen(func(*prng.SplitMix64) (int, error) { return 0, ErrExhausted }, nil))
	if _, err := bad.Sample(prng.New(1)); err == nil {
		t.Fatal("OneOf must propagate errors")
	}
	for name, fn := range map[string]func(){
		"no arms":      func() { OneOf[int]() },
		"no freq arms": func() { Frequency[int]() },
		"zero total":   func() { Frequency(Arm[int]{0, Constant(1)}) },
		"over 2^32":    func() { Frequency(Arm[int]{MaxFrequencyTotal, Constant(1)}, Arm[int]{1, Constant(1)}) },
		"huge weight":  func() { Frequency(Arm[int]{MaxFrequencyTotal + 1, Constant(1)}) },
	} {
		func() {
			defer func() {
				if recover() == nil {
					t.Errorf("%s must panic", name)
				}
			}()
			fn()
		}()
	}
	// Exactly 2^32 total is allowed.
	_ = Frequency(Arm[int]{MaxFrequencyTotal, Constant(1)})
}

func TestTuples(t *testing.T) {
	g2 := Tuple2(Int64(0, 1000), Int64(0, 1000))
	p := sampleOK(t, g2, prng.New(2))
	if p.First < 0 || p.Second > 1000 {
		t.Fatal("tuple2 range")
	}
	// a + b > 100 fails (property: a+b <= 100 holds).  Shrinking one coordinate
	// at a time in repeated passes lands on a local minimum with a+b == 101.
	got, steps := g2.Shrink(Pair[int64, int64]{900, 800},
		func(p Pair[int64, int64]) bool { return p.First+p.Second > 100 }, 1000)
	if got.First+got.Second != 101 || steps == 0 {
		t.Fatalf("tuple2 shrink = %+v in %d", got, steps)
	}
	g3 := Tuple3(Int64(0, 100), Bool(), Int64(0, 100))
	tr := sampleOK(t, g3, prng.New(3))
	if tr.First < 0 || tr.Third > 100 {
		t.Fatal("tuple3 range")
	}
	got3, _ := g3.Shrink(Triple[int64, bool, int64]{50, true, 60},
		func(t Triple[int64, bool, int64]) bool { return t.First+t.Third >= 10 }, 1000)
	if got3.First+got3.Third != 10 || got3.Second {
		t.Fatalf("tuple3 shrink = %+v", got3)
	}
	// Budget respected across passes.
	limited, used := g2.Shrink(Pair[int64, int64]{900, 800},
		func(p Pair[int64, int64]) bool { return true }, 1)
	if used > 1 {
		t.Fatalf("budget 1 used %d steps (%+v)", used, limited)
	}
	// Errors propagate from either side.
	bad := NewGen(func(*prng.SplitMix64) (int64, error) { return 0, ErrExhausted }, nil)
	if _, err := Tuple2(bad, Int64(0, 1)).Sample(prng.New(1)); err == nil {
		t.Fatal("tuple2 first error lost")
	}
	if _, err := Tuple2(Int64(0, 1), bad).Sample(prng.New(1)); err == nil {
		t.Fatal("tuple2 second error lost")
	}
	if _, err := Tuple3(bad, Bool(), Bool()).Sample(prng.New(1)); err == nil {
		t.Fatal("tuple3 first error lost")
	}
	if _, err := Tuple3(Bool(), bad, Bool()).Sample(prng.New(1)); err == nil {
		t.Fatal("tuple3 second error lost")
	}
	if _, err := Tuple3(Bool(), Bool(), bad).Sample(prng.New(1)); err == nil {
		t.Fatal("tuple3 third error lost")
	}
	if _, err := List(bad, 1, 3).Sample(prng.New(1)); err == nil {
		t.Fatal("list element error lost")
	}
}

func TestCorpusEntryPointErrors(t *testing.T) {
	if _, err := GenPrimitive(1, "u7", 1); !errors.Is(err, ErrInvalidArgument) {
		t.Fatal("unknown primitive type")
	}
	if _, err := GenPrimitive(1, "u8", -1); err == nil {
		t.Fatal("negative count")
	}
	if _, err := GenBounded(1, bi(2), bi(1), 1); err == nil {
		t.Fatal("inverted bounds")
	}
	if _, err := GenBounded(1, nil, bi(1), 1); err == nil {
		t.Fatal("nil bound")
	}
	if _, err := GenBytes(1, 3, 2, 1); err == nil {
		t.Fatal("inverted lengths")
	}
	if _, err := GenBytes(1, 0, MaxInputBytes+1, 1); err == nil {
		t.Fatal("oversized lengths")
	}
	if got, err := GenBytes(1, 0, 3, 0); err != nil || len(got) != 0 {
		t.Fatal("zero count is legal")
	}
}

// -----------------------------------------------------------------------------
// Shrinking
// -----------------------------------------------------------------------------

// VER05 section 7.3: for a monotone property the result is exactly the
// threshold t from every failing start, both signs.
func TestMonotoneThresholdsLandExactlyOnT(t *testing.T) {
	for tt := int64(1); tt <= 120; tt++ {
		holdsBelow := func(x *big.Int) bool { return x.Cmp(bi(tt)) < 0 } // x < t holds
		for start := tt; start <= 400; start += 7 {
			got, _, err := ShrinkInt(bi(start), bi(0), bi(-1000), bi(1000), holdsBelow, 1000)
			if err != nil || got.Int64() != tt {
				t.Fatalf("t=%d start=%d: got %v err %v", tt, start, got, err)
			}
		}
		holdsAbove := func(x *big.Int) bool { return x.Cmp(bi(-tt)) > 0 } // x > -t holds
		for start := -tt; start >= -400; start -= 7 {
			got, _, err := ShrinkInt(bi(start), bi(0), bi(-1000), bi(1000), holdsAbove, 1000)
			if err != nil || got.Int64() != -tt {
				t.Fatalf("t=-%d start=%d: got %v err %v", tt, start, got, err)
			}
		}
	}
}

func TestShrinkIntExtremesDoNotOverflow(t *testing.T) {
	never := func(*big.Int) bool { return false } // everything fails
	for _, c := range [][3]*big.Int{
		{maxI128, minI128, minI128}, // opposite-extreme target: the probe wins
		{minI128, maxI128, maxI128},
		{maxI128, bi(0), bi(0)},
	} {
		got, steps, err := ShrinkInt(c[0], c[1], minI128, maxI128, never, 1000)
		if err != nil || got.Cmp(c[2]) != 0 || steps != 1 {
			t.Fatalf("extreme start %v target %v: got %v in %d (err %v)", c[0], c[1], got, steps, err)
		}
	}
	// The walk (no target probe: the target passes) from the extreme.
	holds := func(x *big.Int) bool { return x.Cmp(bi(100)) <= 0 }
	got, _, err := ShrinkInt(maxI128, bi(0), minI128, maxI128, holds, MaxShrinkSteps)
	if err != nil || got.Int64() != 101 {
		t.Fatalf("walk from 2^127-1 = %v err %v", got, err)
	}
	// Result never farther than the start, never outside bounds.
	got, _, _ = ShrinkInt(bi(50), bi(0), bi(10), bi(60), func(x *big.Int) bool { return x.Cmp(bi(20)) < 0 }, 1000)
	if got.Int64() != 20 {
		t.Fatalf("bounded walk = %v", got)
	}
}

func TestShrinkIntBudgetAndErrors(t *testing.T) {
	fails := func(x *big.Int) bool { return x.Cmp(bi(100)) <= 0 }
	if got, steps, err := ShrinkInt(bi(54321), bi(0), bi(0), bi(100000), fails, 0); err != nil || got.Int64() != 54321 || steps != 0 {
		t.Fatalf("budget 0 must return the start: %v %d %v", got, steps, err)
	}
	if got, steps, _ := ShrinkInt(bi(54321), bi(0), bi(0), bi(100000), fails, 1); steps != 1 || fails(got) {
		t.Fatalf("budget 1: %v in %d steps must still falsify", got, steps)
	}
	if _, _, err := ShrinkInt(bi(5), bi(0), bi(0), bi(10), fails, 1000); !errors.Is(err, ErrNotFalsifying) {
		t.Fatalf("a passing start must be ErrNotFalsifying, got %v", err)
	}
	bad := []struct {
		name string
		fn   func() error
	}{
		{"nil initial", func() error { _, _, e := ShrinkInt(nil, bi(0), bi(0), bi(1), fails, 1); return e }},
		{"nil prop", func() error { _, _, e := ShrinkInt(bi(1), bi(0), bi(0), bi(1), nil, 1); return e }},
		{"lo above hi", func() error { _, _, e := ShrinkInt(bi(1), bi(0), bi(2), bi(1), fails, 1); return e }},
		{"start outside", func() error { _, _, e := ShrinkInt(bi(500), bi(0), bi(0), bi(10), fails, 1); return e }},
		{"negative budget", func() error { _, _, e := ShrinkInt(bi(500), bi(0), bi(0), bi(1000), fails, -1); return e }},
		{"huge budget", func() error { _, _, e := ShrinkInt(bi(500), bi(0), bi(0), bi(1000), fails, MaxShrinkSteps+1); return e }},
		{"list nil prop", func() error { _, _, e := ShrinkList(nil, bi(0), nil, 1); return e }},
		{"list nil target", func() error { _, _, e := ShrinkList(nil, nil, func([]*big.Int) bool { return true }, 1); return e }},
		{"list nil element", func() error {
			_, _, e := ShrinkList([]*big.Int{nil}, bi(0), func([]*big.Int) bool { return false }, 1)
			return e
		}},
		{"list budget", func() error {
			_, _, e := ShrinkList(nil, bi(0), func([]*big.Int) bool { return false }, -1)
			return e
		}},
		{"bytes nil prop", func() error { _, _, e := ShrinkBytes(nil, nil, 1); return e }},
		{"bytes budget", func() error {
			_, _, e := ShrinkBytes(nil, func([]byte) bool { return false }, MaxShrinkSteps+1)
			return e
		}},
	}
	for _, b := range bad {
		if err := b.fn(); !errors.Is(err, ErrInvalidArgument) {
			t.Errorf("%s: want ErrInvalidArgument, got %v", b.name, err)
		}
	}
	if _, _, err := ShrinkList([]*big.Int{bi(1)}, bi(0), func([]*big.Int) bool { return true }, 5); !errors.Is(err, ErrNotFalsifying) {
		t.Fatal("list start that passes")
	}
	if _, _, err := ShrinkBytes([]byte{1}, func([]byte) bool { return true }, 5); !errors.Is(err, ErrNotFalsifying) {
		t.Fatal("bytes start that passes")
	}
	// Budget 0 on sequences returns the start.
	if got, steps, _ := ShrinkList([]*big.Int{bi(9), bi(9)}, bi(0), func([]*big.Int) bool { return false }, 0); len(got) != 2 || steps != 0 {
		t.Fatal("list budget 0")
	}
	if got, steps, _ := ShrinkBytes([]byte{9, 9}, func([]byte) bool { return false }, 0); len(got) != 2 || steps != 0 {
		t.Fatal("bytes budget 0")
	}
}

func TestShrinkersDoNotAliasTheCaller(t *testing.T) {
	in := []*big.Int{bi(10), bi(5), bi(8), bi(4)}
	sum := func(l []*big.Int) bool {
		s := new(big.Int)
		for _, v := range l {
			s.Add(s, v)
		}
		if len(l) > 0 {
			l[0].SetInt64(999) // a property that scribbles on its argument
		}
		return s.Cmp(bi(20)) <= 0
	}
	got, _, err := ShrinkList(in, bi(0), sum, 1000)
	if err != nil {
		t.Fatal(err)
	}
	if in[0].Int64() != 10 || in[1].Int64() != 5 {
		t.Fatalf("shrinker mutated its input: %v", in)
	}
	if got[0].Int64() == 999 {
		t.Fatalf("scribble leaked into the result: %v", got)
	}
	raw := []byte{1, 2, 3, 4, 5, 6, 7, 8}
	_, _, _ = ShrinkBytes(raw, func(b []byte) bool {
		if len(b) > 0 {
			b[0] = 0xee
		}
		return len(b) < 5
	}, 1000)
	if raw[0] != 1 {
		t.Fatalf("bytes shrinker mutated its input: %v", raw)
	}
}

func TestSequenceShrinkingIsOneMinimalByDeletion(t *testing.T) {
	type shape struct {
		name  string
		holds func([]int) bool
	}
	shapes := []shape{
		{"len<3", func(l []int) bool { return len(l) < 3 }},
		{"sum<=4", func(l []int) bool {
			s := 0
			for _, v := range l {
				s += v
			}
			return s <= 4
		}},
		{"no zero", func(l []int) bool { return !slices.Contains(l, 0) }},
		{"no consecutive duplicates", func(l []int) bool {
			for i := 0; i+1 < len(l); i++ {
				if l[i] == l[i+1] {
					return false
				}
			}
			return true
		}},
		{"nondecreasing", func(l []int) bool { return slices.IsSorted(l) }},
		{"fewer than two threes", func(l []int) bool {
			n := 0
			for _, v := range l {
				if v == 3 {
					n++
				}
			}
			return n < 2
		}},
	}
	var lists [][]int
	var build func(prefix []int)
	build = func(prefix []int) {
		lists = append(lists, slices.Clone(prefix))
		if len(prefix) == 4 {
			return
		}
		for v := 0; v <= 3; v++ {
			build(append(prefix, v))
		}
	}
	build(nil)
	if len(lists) != 341 {
		t.Fatalf("expected 341 lists, built %d", len(lists))
	}
	for _, s := range shapes {
		for _, start := range lists {
			if s.holds(start) {
				continue
			}
			toBig := func(l []int) []*big.Int {
				out := make([]*big.Int, len(l))
				for i, v := range l {
					out[i] = bi(int64(v))
				}
				return out
			}
			toInt := func(l []*big.Int) []int {
				out := make([]int, len(l))
				for i, v := range l {
					out[i] = int(v.Int64())
				}
				return out
			}
			got, _, err := ShrinkList(toBig(start), bi(0), func(l []*big.Int) bool { return s.holds(toInt(l)) }, 1000)
			if err != nil {
				t.Fatal(err)
			}
			g := toInt(got)
			if s.holds(g) {
				t.Fatalf("%s %v: result %v satisfies the property", s.name, start, g)
			}
			if len(g) > len(start) {
				t.Fatalf("%s %v: result %v is longer", s.name, start, g)
			}
			for i := range g {
				if g[i] < 0 || g[i] > 3 {
					t.Fatalf("%s %v: element left its range: %v", s.name, start, g)
				}
			}
			for i := range g {
				if s.holds(slices.Delete(slices.Clone(g), i, i+1)) {
					continue
				}
				t.Fatalf("%s %v -> %v: deleting index %d still fails, so it is not 1-minimal", s.name, start, g, i)
			}
		}
	}
}

func TestListShrinkWorkedExamples(t *testing.T) {
	// consecutive_duplicates: [1 2 3 3 4 5] shrinks to [3 3], not the global minimum [0 0].
	noDup := func(l []*big.Int) bool {
		for i := 0; i+1 < len(l); i++ {
			if l[i].Cmp(l[i+1]) == 0 {
				return false
			}
		}
		return true
	}
	in := []*big.Int{bi(1), bi(2), bi(3), bi(3), bi(4), bi(5)}
	got, _, err := ShrinkList(in, bi(0), noDup, 1000)
	if err != nil || len(got) != 2 || got[0].Int64() != 3 || got[1].Int64() != 3 {
		t.Fatalf("got %v err %v", got, err)
	}
}

func TestSimulateReplay(t *testing.T) {
	prop := func(x *big.Int) bool { return x.Cmp(bi(100)) <= 0 }
	a1, m1, err := SimulateReplay(42, prop)
	if err != nil || a1.Int64() != 413 || m1.Int64() != 101 {
		t.Fatalf("seed 42 = %v %v %v", a1, m1, err)
	}
	a2, m2, _ := SimulateReplay(42, prop)
	if a1.Cmp(a2) != 0 || m1.Cmp(m2) != 0 {
		t.Fatal("replay must be deterministic")
	}
	if _, _, err := SimulateReplay(1, func(*big.Int) bool { return true }); !errors.Is(err, ErrInvalidArgument) {
		t.Fatalf("a property that never fails cannot be replayed: %v", err)
	}
}

func TestIsCanonicalIntAndHex(t *testing.T) {
	for in, want := range map[string]bool{"0": true, "-1": true, "10": true, "": false, "-": false, "-0": false, "01": false, "+1": false, "1 ": false, "1e3": false} {
		if got := isCanonicalInt(in); got != want {
			t.Errorf("isCanonicalInt(%q) = %v", in, got)
		}
	}
	for in, want := range map[string]bool{"": true, "00ff": true, "0": false, "FF": false, "gg": false} {
		if got := isLowercaseHex(in); got != want {
			t.Errorf("isLowercaseHex(%q) = %v", in, got)
		}
	}
}

func TestSameValueDoesNotConflateKinds(t *testing.T) {
	if sameValue(true, bi(1)) || sameValue(bi(1), true) || sameValue([]byte{1}, []any{bi(1)}) ||
		sameValue("1", bi(1)) || sameValue(bi(1), "1") || sameValue([]any{bi(1)}, []any{bi(1), bi(2)}) ||
		sameValue([]any{bi(1)}, []any{bi(2)}) || sameValue(nil, nil) || sameValue((*big.Int)(nil), bi(0)) {
		t.Fatal("sameValue conflated two different kinds")
	}
	if !sameValue([]any{bi(1), true, "s", []byte{2}}, []any{bi(1), true, "s", []byte{2}}) {
		t.Fatal("sameValue must accept equal trees")
	}
}
