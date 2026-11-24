// Package pbt is the Go lane of the VER05 property-based testing engine
// (specification: code/specs/VER05d-go-pbt-engine.md).
//
// A property-based test does not check one example, it checks a claim about
// every input a generator can produce:
//
//	for every list xs of small ints:   sorted(sorted(xs)) == sorted(xs)
//
// The engine has three jobs, and the files are organised around them:
//
//  1. Generate candidate inputs from a deterministic random stream (gen.go).
//     The stream is SplitMix64 (package prng), so the same seed gives the same
//     candidates on every machine and in every language lane; the shared
//     pbt-v1 corpus pins the first outputs.
//  2. Shrink a failing candidate to a witness a human can read (shrink.go).
//     A counterexample of [913 -4 77 0 2] is noise; [0] is a diagnosis.
//     Shrinking is integrated: the generator that made a value knows its own
//     bounds, so a shrunk Int(5, 10) value never becomes 0.
//  3. Judge honestly (runner.go).  A verdict is one of a closed set of failure
//     ids, and the engine refuses to report a pass unless every case actually
//     ran and held (VER05 section 7.1, "verdict honesty").
//
// Integers are *big.Int wherever the corpus or the full 128-bit range is
// involved (Go has no 128-bit integer), and values handed around are never
// mutated.  The package performs no file, clock, network or environment access.
package pbt

import (
	"fmt"
	"math/big"

	"github.com/adhithyan15/coding-adventures/code/packages/go/test-bootstrap/prng"
)

// newRNG builds the stream for a seed.  It is a variable so that the negative
// controls can swap in a shifted stream and prove the corpus run notices.
var newRNG = prng.New

// Gen generates values of type T from a PRNG stream and knows how to shrink
// them.  Sampling returns an error because generation can fail on data (a
// Filter that never accepts); every combinator propagates it.  The zero Gen
// has no sampler and every Sample returns an error.
type Gen[T any] struct {
	sample func(*prng.SplitMix64) (T, error)
	shrink ShrinkFunc[T]
}

// NewGen builds a generator from a sampler and an optional shrinker (nil means
// "values are reported unshrunk").
func NewGen[T any](sample func(*prng.SplitMix64) (T, error), shrink ShrinkFunc[T]) Gen[T] {
	return Gen[T]{sample: sample, shrink: shrink}
}

// Sample draws one value.
func (g Gen[T]) Sample(rng *prng.SplitMix64) (T, error) {
	if g.sample == nil {
		var zero T
		return zero, fmt.Errorf("%w: generator has no sampler", ErrExhausted)
	}
	return g.sample(rng)
}

// Shrink shrinks value while falsifies holds; (value, 0) if the generator has
// no shrinker.
func (g Gen[T]) Shrink(value T, falsifies func(T) bool, maxSteps int) (T, int) {
	if g.shrink == nil {
		return value, 0
	}
	return g.shrink(value, falsifies, maxSteps)
}

// DefaultMaxRetries is how many samples Filter tries before giving up.
const DefaultMaxRetries = 100

func ok[T any](v T) (T, error) { return v, nil }

// =============================================================================
// Primitive generators (VER05 section 3.1)
//
// Draw order is the compatibility contract between language lanes: the pbt-v1
// vectors pin exact outputs per seed, so reordering a draw breaks conformance.
// =============================================================================

// Constant always generates v and draws nothing.
func Constant[T any](v T) Gen[T] {
	return NewGen(func(*prng.SplitMix64) (T, error) { return v, nil }, nil)
}

// Bool generates true or false: Bounded(2) == 1.  It shrinks true to false.
func Bool() Gen[bool] {
	return NewGen(
		func(r *prng.SplitMix64) (bool, error) { return r.Bounded(2) == 1, nil },
		func(value bool, falsifies func(bool) bool, maxSteps int) (bool, int) {
			if value && maxSteps > 0 && falsifies(false) {
				return false, 1
			}
			return value, 0
		})
}

// bigShrinker adapts the big-integer walk to a native integer type.
func bigShrinker[T any](lo, hi *big.Int, toBig func(T) *big.Int, fromBig func(*big.Int) T) ShrinkFunc[T] {
	target := clampZero(lo, hi)
	return func(value T, falsifies func(T) bool, maxSteps int) (T, int) {
		got, steps := shrinkIntCore(toBig(value), target, lo, hi,
			func(b *big.Int) bool { return falsifies(fromBig(b)) }, maxSteps)
		return fromBig(got), steps
	}
}

// clampZero is the in-range point nearest zero: the integer shrink target.
func clampZero(lo, hi *big.Int) *big.Int {
	switch {
	case lo.Sign() > 0:
		return new(big.Int).Set(lo)
	case hi.Sign() < 0:
		return new(big.Int).Set(hi)
	}
	return new(big.Int)
}

// U8 generates [0, 255]: Bounded(256).
func U8() Gen[uint8] {
	return NewGen(
		func(r *prng.SplitMix64) (uint8, error) { return uint8(r.Bounded(256)), nil },
		bigShrinker(big.NewInt(0), big.NewInt(255),
			func(v uint8) *big.Int { return big.NewInt(int64(v)) },
			func(b *big.Int) uint8 { return uint8(b.Uint64()) }))
}

var maxU64 = new(big.Int).SetUint64(^uint64(0))

// U64 generates a full 64-bit word: NextU64.
func U64() Gen[uint64] {
	return NewGen(
		func(r *prng.SplitMix64) (uint64, error) { return r.NextU64(), nil },
		bigShrinker(big.NewInt(0), maxU64,
			func(v uint64) *big.Int { return new(big.Int).SetUint64(v) },
			func(b *big.Int) uint64 { return b.Uint64() }))
}

var (
	minI128   = new(big.Int).Neg(new(big.Int).Lsh(bigOne, 127))
	maxI128   = new(big.Int).Sub(new(big.Int).Lsh(bigOne, 127), bigOne)
	twoPow128 = new(big.Int).Lsh(bigOne, 128)
)

func sampleI128(r *prng.SplitMix64) *big.Int {
	hi := new(big.Int).SetUint64(r.NextU64())
	lo := new(big.Int).SetUint64(r.NextU64())
	v := hi.Lsh(hi, 64)
	v.Or(v, lo)
	if v.Cmp(new(big.Int).Lsh(bigOne, 127)) >= 0 {
		v.Sub(v, twoPow128)
	}
	return v
}

// I128 generates a signed 128-bit integer from two stream words, high word
// first, read as two's complement:
//
//	words 0x8000...0 : 0   -> -2^127   (top bit set => subtract 2^128)
//	words 0x7fff...f : f.. ->  2^127 - 1
func I128() Gen[*big.Int] {
	return NewGen(
		func(r *prng.SplitMix64) (*big.Int, error) { return sampleI128(r), nil },
		bigShrinker(minI128, maxI128, func(v *big.Int) *big.Int { return v }, func(b *big.Int) *big.Int { return b }))
}

// Int generates a uniform integer over the inclusive range [min, max], any
// width.  It panics if min > max or either is nil.
//
// The width is span = max - min + 1 and the offset from min is drawn by
// prng.BoundedBig (VER05 section 7.2): Bounded for spans below 2^64, one raw
// word for exactly 2^64, k-word rejection sampling above.  It shrinks toward the
// in-range point nearest zero and never leaves [min, max].
func Int(min, max *big.Int) Gen[*big.Int] {
	if min == nil || max == nil || min.Cmp(max) > 0 {
		panic("pbt: Int needs non-nil min <= max")
	}
	lo, hi := new(big.Int).Set(min), new(big.Int).Set(max)
	return NewGen(
		func(r *prng.SplitMix64) (*big.Int, error) { return boundedInt(r, lo, hi), nil },
		bigShrinker(lo, hi, func(v *big.Int) *big.Int { return v }, func(b *big.Int) *big.Int { return b }))
}

func boundedInt(r *prng.SplitMix64, lo, hi *big.Int) *big.Int {
	span := new(big.Int).Sub(hi, lo)
	span.Add(span, bigOne)
	return span.Add(lo, r.BoundedBig(span))
}

// Int64 is Int over native int64 bounds: the same draws, a native result.
func Int64(min, max int64) Gen[int64] {
	if min > max {
		panic("pbt: Int64 needs min <= max")
	}
	// uint64 arithmetic wraps, so a full-range span is 0 (meaning 2^64).
	span := uint64(max) - uint64(min) + 1
	return NewGen(
		func(r *prng.SplitMix64) (int64, error) {
			var offset uint64
			if span == 0 {
				offset = r.NextU64()
			} else {
				offset = r.Bounded(span)
			}
			return int64(uint64(min) + offset), nil
		},
		bigShrinker(big.NewInt(min), big.NewInt(max),
			func(v int64) *big.Int { return big.NewInt(v) },
			func(b *big.Int) int64 { return b.Int64() }))
}

func checkLengths(minLen, maxLen, limit int, what string) {
	if minLen < 0 || maxLen < minLen {
		panic(fmt.Sprintf("pbt: %s needs 0 <= minLen <= maxLen", what))
	}
	if maxLen > limit {
		panic(fmt.Sprintf("pbt: %s maxLen %d exceeds %d", what, maxLen, limit))
	}
}

// Bytes generates a byte string: L = minLen + Bounded(span), then L draws of
// Bounded(256).  It shrinks by sequence shrinking, respecting minLen.
func Bytes(minLen, maxLen int) Gen[[]byte] {
	checkLengths(minLen, maxLen, MaxInputBytes, "Bytes")
	span := uint64(maxLen-minLen) + 1
	return NewGen(
		func(r *prng.SplitMix64) ([]byte, error) {
			n := minLen + int(r.Bounded(span))
			out := make([]byte, n)
			for i := range out {
				out[i] = byte(r.Bounded(256))
			}
			return out, nil
		},
		func(value []byte, falsifies func([]byte) bool, maxSteps int) ([]byte, int) {
			return shrinkSequence(value, falsifies, maxSteps, minLen, byteWalk)
		})
}

// =============================================================================
// Combinators (VER05 section 3.2)
// =============================================================================

// Map transforms generated values.  It drops shrinking: f has no inverse, so
// the engine cannot move a transformed value back into the source domain.
func Map[T, U any](g Gen[T], f func(T) U) Gen[U] {
	return NewGen(func(r *prng.SplitMix64) (U, error) {
		v, err := g.Sample(r)
		if err != nil {
			var zero U
			return zero, err
		}
		return f(v), nil
	}, nil)
}

// Filter keeps only values satisfying pred, trying DefaultMaxRetries samples.
func Filter[T any](g Gen[T], pred func(T) bool) Gen[T] {
	return FilterN(g, pred, DefaultMaxRetries)
}

// FilterN is Filter with an explicit retry count.  After exactly maxRetries
// rejected samples Sample returns an error wrapping ErrExhausted.  Shrinking is
// preserved: a shrink candidate that fails pred is simply not a valid input, so
// it is treated as "does not falsify".  maxRetries < 1 panics.
func FilterN[T any](g Gen[T], pred func(T) bool, maxRetries int) Gen[T] {
	if maxRetries < 1 {
		panic("pbt: FilterN needs maxRetries >= 1")
	}
	return NewGen(
		func(r *prng.SplitMix64) (T, error) {
			for i := 0; i < maxRetries; i++ {
				v, err := g.Sample(r)
				if err != nil {
					return v, err
				}
				if pred(v) {
					return v, nil
				}
			}
			var zero T
			return zero, fmt.Errorf("%w: filter rejected %d samples", ErrExhausted, maxRetries)
		},
		func(value T, falsifies func(T) bool, maxSteps int) (T, int) {
			return g.Shrink(value, func(c T) bool { return pred(c) && falsifies(c) }, maxSteps)
		})
}

// FlatMap is dependent generation: sample, then sample from f(value).  No
// shrinking.
func FlatMap[T, U any](g Gen[T], f func(T) Gen[U]) Gen[U] {
	return NewGen(func(r *prng.SplitMix64) (U, error) {
		v, err := g.Sample(r)
		if err != nil {
			var zero U
			return zero, err
		}
		return f(v).Sample(r)
	}, nil)
}

// OneOf picks one generator uniformly (Bounded(len), then that arm's draws).
// No shrinking.  Zero arms panic.
func OneOf[T any](gens ...Gen[T]) Gen[T] {
	if len(gens) == 0 {
		panic("pbt: OneOf needs at least one generator")
	}
	arms := append([]Gen[T](nil), gens...)
	return NewGen(func(r *prng.SplitMix64) (T, error) {
		return arms[r.Bounded(uint64(len(arms)))].Sample(r)
	}, nil)
}

// Arm is one weighted alternative of Frequency.
type Arm[T any] struct {
	Weight uint64
	Gen    Gen[T]
}

// Frequency is a weighted choice: pick = Bounded(total); arms are walked in
// order, subtracting each weight, so Frequency(Arm{1, a}, Arm{3, b}) picks b
// three times in four.  A zero-weight arm can never be selected (pick < 0 is
// never true).  An empty list, a zero total or a total above 2^32 panics.
func Frequency[T any](arms ...Arm[T]) Gen[T] {
	if len(arms) == 0 {
		panic("pbt: Frequency needs at least one arm")
	}
	var total uint64
	for _, a := range arms {
		if a.Weight > MaxFrequencyTotal {
			panic("pbt: Frequency weight exceeds 2^32")
		}
		total += a.Weight // at most len(arms) * 2^32: cannot wrap a uint64
		if total > MaxFrequencyTotal {
			panic("pbt: Frequency total weight exceeds 2^32")
		}
	}
	if total == 0 {
		panic("pbt: Frequency total weight must be > 0")
	}
	owned := append([]Arm[T](nil), arms...)
	return NewGen(func(r *prng.SplitMix64) (T, error) {
		pick := r.Bounded(total)
		for _, a := range owned {
			if pick < a.Weight {
				return a.Gen.Sample(r)
			}
			pick -= a.Weight
		}
		panic("pbt: unreachable, pick < total")
	}, nil)
}

// List generates a list: L = minLen + Bounded(span), then L elements in order.
// It shrinks by sequence shrinking, respecting minLen, with the element
// generator's shrinker for the element-wise phase.
func List[T any](elem Gen[T], minLen, maxLen int) Gen[[]T] {
	checkLengths(minLen, maxLen, MaxListLen, "List")
	span := uint64(maxLen-minLen) + 1
	return NewGen(
		func(r *prng.SplitMix64) ([]T, error) {
			n := minLen + int(r.Bounded(span))
			out := make([]T, 0, min(n, 1024))
			for i := 0; i < n; i++ {
				v, err := elem.Sample(r)
				if err != nil {
					return nil, err
				}
				out = append(out, v)
			}
			return out, nil
		},
		func(value []T, falsifies func([]T) bool, maxSteps int) ([]T, int) {
			return shrinkSequence(value, falsifies, maxSteps, minLen, elem.Shrink)
		})
}

// Pair is the value of Tuple2.
type Pair[A, B any] struct {
	First  A
	Second B
}

// Triple is the value of Tuple3.
type Triple[A, B, C any] struct {
	First  A
	Second B
	Third  C
}

// Tuple2 samples a then b, and shrinks one coordinate at a time.
func Tuple2[A, B any](ga Gen[A], gb Gen[B]) Gen[Pair[A, B]] {
	return NewGen(
		func(r *prng.SplitMix64) (Pair[A, B], error) {
			var p Pair[A, B]
			var err error
			if p.First, err = ga.Sample(r); err != nil {
				return p, err
			}
			p.Second, err = gb.Sample(r)
			return p, err
		},
		func(value Pair[A, B], falsifies func(Pair[A, B]) bool, maxSteps int) (Pair[A, B], int) {
			cur, steps := value, 0
			// Repeat passes until a whole pass makes no progress: shrinking one
			// coordinate can unlock more shrinking of the other.  Every pass that
			// continues accepted at least one move, so the budget bounds it.
			for progress := true; progress && steps < maxSteps; {
				progress = false
				if remaining := maxSteps - steps; remaining > 0 {
					got, used := ga.Shrink(cur.First, func(a A) bool { return falsifies(Pair[A, B]{a, cur.Second}) }, remaining)
					if used > 0 {
						cur.First, steps, progress = got, steps+used, true
					}
				}
				if remaining := maxSteps - steps; remaining > 0 {
					got, used := gb.Shrink(cur.Second, func(b B) bool { return falsifies(Pair[A, B]{cur.First, b}) }, remaining)
					if used > 0 {
						cur.Second, steps, progress = got, steps+used, true
					}
				}
			}
			return cur, steps
		})
}

// Tuple3 samples a, b, c in order and shrinks one coordinate at a time.
func Tuple3[A, B, C any](ga Gen[A], gb Gen[B], gc Gen[C]) Gen[Triple[A, B, C]] {
	return NewGen(
		func(r *prng.SplitMix64) (Triple[A, B, C], error) {
			var t Triple[A, B, C]
			var err error
			if t.First, err = ga.Sample(r); err != nil {
				return t, err
			}
			if t.Second, err = gb.Sample(r); err != nil {
				return t, err
			}
			t.Third, err = gc.Sample(r)
			return t, err
		},
		func(value Triple[A, B, C], falsifies func(Triple[A, B, C]) bool, maxSteps int) (Triple[A, B, C], int) {
			cur, steps := value, 0
			for progress := true; progress && steps < maxSteps; {
				progress = false
				if remaining := maxSteps - steps; remaining > 0 {
					got, used := ga.Shrink(cur.First, func(a A) bool { return falsifies(Triple[A, B, C]{a, cur.Second, cur.Third}) }, remaining)
					if used > 0 {
						cur.First, steps, progress = got, steps+used, true
					}
				}
				if remaining := maxSteps - steps; remaining > 0 {
					got, used := gb.Shrink(cur.Second, func(b B) bool { return falsifies(Triple[A, B, C]{cur.First, b, cur.Third}) }, remaining)
					if used > 0 {
						cur.Second, steps, progress = got, steps+used, true
					}
				}
				if remaining := maxSteps - steps; remaining > 0 {
					got, used := gc.Shrink(cur.Third, func(c C) bool { return falsifies(Triple[A, B, C]{cur.First, cur.Second, c}) }, remaining)
					if used > 0 {
						cur.Third, steps, progress = got, steps+used, true
					}
				}
			}
			return cur, steps
		})
}

// =============================================================================
// Corpus entry points (pbt-v1)
// =============================================================================

// GenPrimitive returns count primitives of typeName ("bool", "u8", "u64",
// "i128") drawn from seed.  Booleans are bool; every integer is *big.Int.
func GenPrimitive(seed uint64, typeName string, count int) ([]any, error) {
	if count < 0 {
		return nil, argErr("negative count")
	}
	rng := newRNG(seed)
	out := make([]any, 0, min(count, 1024))
	for i := 0; i < count; i++ {
		switch typeName {
		case "bool":
			out = append(out, rng.Bounded(2) == 1)
		case "u8":
			out = append(out, new(big.Int).SetUint64(rng.Bounded(256)))
		case "u64":
			out = append(out, new(big.Int).SetUint64(rng.NextU64()))
		case "i128":
			out = append(out, sampleI128(rng))
		default:
			return nil, argErr("unknown primitive type %q", typeName)
		}
	}
	return out, nil
}

// GenBounded returns count uniform integers over [lo, hi] drawn from seed.
func GenBounded(seed uint64, lo, hi *big.Int, count int) ([]*big.Int, error) {
	if lo == nil || hi == nil || lo.Cmp(hi) > 0 || count < 0 {
		return nil, argErr("need non-nil lo <= hi and count >= 0")
	}
	rng := newRNG(seed)
	out := make([]*big.Int, 0, min(count, 1024))
	for i := 0; i < count; i++ {
		out = append(out, boundedInt(rng, lo, hi))
	}
	return out, nil
}

// GenBytes returns count byte strings of length in [minLen, maxLen] from seed.
func GenBytes(seed uint64, minLen, maxLen, count int) ([][]byte, error) {
	if minLen < 0 || maxLen < minLen || maxLen > MaxInputBytes || count < 0 {
		return nil, argErr("need 0 <= minLen <= maxLen <= %d and count >= 0", MaxInputBytes)
	}
	gen := Bytes(minLen, maxLen)
	rng := newRNG(seed)
	out := make([][]byte, 0, min(count, 1024))
	for i := 0; i < count; i++ {
		v, err := gen.Sample(rng)
		if err != nil {
			return nil, err
		}
		out = append(out, v)
	}
	return out, nil
}

// SimulateReplay replays a failure from nothing but seed.
//
// It draws up to 1000 values Bounded(1000); the first that falsifies prop is
// initialFail; that value is then shrunk toward 0 over [0, 1000].  The same
// seed gives the same pair forever.  prop is true when the value is fine.  An
// error means no draw falsified prop.
func SimulateReplay(seed uint64, prop func(*big.Int) bool) (initialFail, minimal *big.Int, err error) {
	rng := newRNG(seed)
	for i := 0; i < 1000; i++ {
		v := new(big.Int).SetUint64(rng.Bounded(1000))
		if !prop(new(big.Int).Set(v)) {
			initialFail = v
			break
		}
	}
	if initialFail == nil {
		return nil, nil, argErr("property was not falsified within 1000 draws")
	}
	minimal, _, err = ShrinkInt(initialFail, big.NewInt(0), big.NewInt(0), big.NewInt(1000), prop, 1000)
	if err != nil {
		return nil, nil, err
	}
	return initialFail, minimal, nil
}
