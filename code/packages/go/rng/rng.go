// Package rng implements three classic pseudorandom number generators.
//
// # The Three Algorithms
//
//   - LCG (Linear Congruential Generator, Knuth 1948) — the simplest useful PRNG.
//     State advances via state = (state × a + c) mod 2^64. Fast, full period,
//     but consecutive outputs are correlated.
//
//   - Xorshift64 (Marsaglia 2003) — three XOR-shift operations on 64-bit state.
//     No multiplication; period 2^64 − 1. Seed 0 is a fixed point and is
//     replaced with 1.
//
//   - PCG32 (O'Neill 2014) — same LCG recurrence plus an XSH RR output
//     permutation (XOR-Shift High / Random Rotate). Passes all known
//     statistical test suites with only 8 bytes of state.
//
// All three generators expose the same API:
//
//	g := rng.NewLCG(42)
//	v := g.NextU32()            // uint32 in [0, 2^32)
//	u := g.NextU64()            // uint64 in [0, 2^64)
//	f := g.NextFloat()          // float64 in [0.0, 1.0)
//	n := g.NextIntInRange(1, 6) // int64 in [1, 6] inclusive
package rng

// lcgMultiplier and lcgIncrement are the Knuth / Numerical Recipes constants.
// Together they satisfy the Hull-Dobell theorem: full period 2^64 (every
// 64-bit value appears exactly once per cycle).
const (
	lcgMultiplier = uint64(6364136223846793005)
	lcgIncrement  = uint64(1442695040888963407)
	floatDiv      = float64(1 << 32) // 2^32 — normalises u32 to [0.0, 1.0)
)

// ── LCG ──────────────────────────────────────────────────────────────────────

// LCG is a Linear Congruential Generator.
//
// Recurrence: state = (state × a + c) mod 2^64
// Output: upper 32 bits of state (lower bits have shorter sub-periods).
type LCG struct {
	state uint64
}

// NewLCG returns an LCG seeded with the given value. Any seed is valid.
func NewLCG(seed uint64) *LCG {
	return &LCG{state: seed}
}

// NextU32 advances the LCG state and returns the upper 32 bits.
func (g *LCG) NextU32() uint32 {
	g.state = g.state*lcgMultiplier + lcgIncrement
	return uint32(g.state >> 32)
}

// NextU64 returns a 64-bit value composed of two consecutive NextU32 calls:
// (hi << 32) | lo.
func (g *LCG) NextU64() uint64 {
	hi := uint64(g.NextU32())
	lo := uint64(g.NextU32())
	return (hi << 32) | lo
}

// NextFloat returns a float64 uniformly distributed in [0.0, 1.0).
func (g *LCG) NextFloat() float64 {
	return float64(g.NextU32()) / floatDiv
}

// NextIntInRange returns a uniform random integer in [min, max] inclusive.
//
// It panics if min > max, before drawing anything. See intInRange for the
// rejection sampling, which is the same for all three generators.
func (g *LCG) NextIntInRange(min, max int64) int64 {
	return intInRange(g.NextU32, min, max)
}

// ── Xorshift64 ────────────────────────────────────────────────────────────────

// Xorshift64 is a Marsaglia (2003) XOR-shift generator.
//
// Three shifts scramble 64-bit state with no multiplication:
//
//	x ^= x << 13
//	x ^= x >> 7
//	x ^= x << 17
//
// Period: 2^64 − 1. State 0 is a fixed point; seed 0 is replaced with 1.
// Output is the lower 32 bits.
type Xorshift64 struct {
	state uint64
}

// NewXorshift64 returns an Xorshift64 seeded with the given value.
// Seed 0 is replaced with 1 to avoid the zero fixed point.
func NewXorshift64(seed uint64) *Xorshift64 {
	if seed == 0 {
		seed = 1
	}
	return &Xorshift64{state: seed}
}

// NextU32 applies the three XOR-shifts and returns the lower 32 bits.
func (g *Xorshift64) NextU32() uint32 {
	x := g.state
	x ^= x << 13
	x ^= x >> 7
	x ^= x << 17
	g.state = x
	return uint32(x)
}

// NextU64 returns a 64-bit value composed of two consecutive NextU32 calls.
func (g *Xorshift64) NextU64() uint64 {
	hi := uint64(g.NextU32())
	lo := uint64(g.NextU32())
	return (hi << 32) | lo
}

// NextFloat returns a float64 uniformly distributed in [0.0, 1.0).
func (g *Xorshift64) NextFloat() float64 {
	return float64(g.NextU32()) / floatDiv
}

// NextIntInRange returns a uniform random integer in [min, max] inclusive
// using rejection sampling (same algorithm as LCG).
func (g *Xorshift64) NextIntInRange(min, max int64) int64 {
	return intInRange(g.NextU32, min, max)
}

// ── PCG32 ─────────────────────────────────────────────────────────────────────

// PCG32 is a Permuted Congruential Generator (O'Neill 2014).
//
// Uses the same LCG recurrence as LCG but applies an XSH RR output
// permutation before returning:
//
//  1. xorshifted = ((old >> 18) ^ old) >> 27   — mix high bits down
//  2. rot        = old >> 59                    — 5-bit rotation amount
//  3. output     = rotr32(xorshifted, rot)      — scatter all bits
//
// Passes all known statistical test suites (TestU01 BigCrush, PractRand).
type PCG32 struct {
	state     uint64
	increment uint64
}

// NewPCG32 returns a PCG32 seeded with the given value.
//
// The "initseq" warm-up is applied so that even seeds 0 and 1 produce
// well-distributed initial sequences:
//
//  1. Advance once from state=0 to incorporate the increment.
//  2. Mix the seed into state.
//  3. Advance once more to scatter seed bits throughout state.
func NewPCG32(seed uint64) *PCG32 {
	inc := lcgIncrement | 1 // increment must be odd for full period
	g := &PCG32{state: 0, increment: inc}
	g.state = g.state*lcgMultiplier + inc
	g.state += seed
	g.state = g.state*lcgMultiplier + inc
	return g
}

// NextU32 advances the PCG32 state and returns the permuted 32-bit output.
func (g *PCG32) NextU32() uint32 {
	oldState := g.state
	g.state = oldState*lcgMultiplier + g.increment

	// XSH RR permutation on old_state (output-before-advance).
	xorshifted := uint32(((oldState >> 18) ^ oldState) >> 27)
	rot := uint32(oldState >> 59)
	return (xorshifted >> rot) | (xorshifted << ((-rot) & 31))
}

// NextU64 returns a 64-bit value composed of two consecutive NextU32 calls.
func (g *PCG32) NextU64() uint64 {
	hi := uint64(g.NextU32())
	lo := uint64(g.NextU32())
	return (hi << 32) | lo
}

// NextFloat returns a float64 uniformly distributed in [0.0, 1.0).
func (g *PCG32) NextFloat() float64 {
	return float64(g.NextU32()) / floatDiv
}

// NextIntInRange returns a uniform random integer in [min, max] inclusive
// using rejection sampling (same algorithm as LCG and Xorshift64).
func (g *PCG32) NextIntInRange(min, max int64) int64 {
	return intInRange(g.NextU32, min, max)
}

// ── Bounded draws (spec rng.md, "Rejection Sampling", v2) ────────────────────

// intInRange returns a uniform integer in [min, max], drawn from a stream of
// 32-bit outputs, identically in every language of the repository.
//
// span = max − min + 1 lies between 1 and 2^64. Taking r % span of a raw draw
// over-samples small offsets whenever the number of possible draws is not a
// multiple of span; rejecting the threshold lowest draws removes exactly the
// excess. Three regimes, chosen by span alone:
//
//	span <= 2^32    draw 32 bits,  threshold = (2^32 − span) mod span
//	span <  2^64    draw 64 bits,  threshold = (2^64 − span) mod span
//	span == 2^64    the whole int64 domain: every uint64 is a valid offset
//
// Example, span = 3: 2^32 = 3 × 1431655765 + 1, so threshold = 1. Rejecting
// r = 0 leaves 2^32 − 1 draws, an exact multiple of 3.
//
// v1 of this package took the threshold modulo 2^64 while drawing 32 bits.
// That was biased, never returned more than 2^32 − 1 above min, divided by
// zero for the whole domain (whose span wraps to 0), and for spans such as
// 2^63 + 1 rejected every possible draw, so it never returned at all.
func intInRange(next32 func() uint32, min, max int64) int64 {
	if min > max {
		panic("rng: NextIntInRange requires min <= max")
	}
	// max − min + 1 in uint64: exact, except that 2^64 wraps to 0.
	span := uint64(max) - uint64(min) + 1
	next64 := func() uint64 {
		hi := uint64(next32())
		return hi<<32 | uint64(next32())
	}
	var offset uint64
	switch {
	case span == 0:
		offset = next64()
	case span <= 1<<32:
		threshold := ((1 << 32) - span) % span
		for {
			r := uint64(next32())
			if r >= threshold {
				offset = r % span
				break
			}
		}
	default:
		threshold := -span % span // (2^64 − span) mod span
		for {
			r := next64()
			if r >= threshold {
				offset = r % span
				break
			}
		}
	}
	// min + offset is within [min, max]; Go's integer arithmetic wraps, so
	// the uint64 sum converts back exactly.
	return int64(uint64(min) + offset)
}
