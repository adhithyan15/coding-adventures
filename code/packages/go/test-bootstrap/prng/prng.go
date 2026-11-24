// Package prng is the deterministic randomness shared by every test engine in
// the Go lane: the property engine (VER05d), the fuzz engine (VER06e) and the
// TF01 runner all import it, so there is exactly one SplitMix64 in the lane.
//
// Why a hand-written PRNG instead of math/rand?  A property test is only
// useful if a failure can be replayed.  "Seed 42 failed" must mean the same
// sequence of candidates on every machine, every Go release and every other
// language lane.  math/rand makes no such promise; SplitMix64 is five lines of
// 64-bit arithmetic that any language can reproduce bit for bit, and the shared
// pbt-v1 corpus pins its first outputs.
//
// # SplitMix64
//
// The state is one uint64.  Each step adds the golden-ratio constant and
// scrambles the sum with two xor-shift-multiply rounds:
//
//	state  += 0x9e3779b97f4a7c15
//	z       = state
//	z       = (z ^ (z >> 30)) * 0xbf58476d1ce4e5b9
//	z       = (z ^ (z >> 27)) * 0x94d049bb133111eb
//	output  = z ^ (z >> 31)
//
// Go's unsigned integers wrap modulo 2^64, which is exactly the arithmetic the
// algorithm assumes, so no masking is needed (a Python lane must mask every
// step).  Pinned vector: seed 0 first outputs 0xe220a8397b1dcdaf.
//
// # Bounded draws without bias
//
// "x % n" is biased unless n divides 2^64: the residues below 2^64 mod n occur
// once more often than the rest.  Rejection sampling removes the bias.  With
// the threshold t = (2^64 - n) mod n, every raw value below t is thrown away;
// the 2^64 - t values that remain split into exactly equal residue classes.
//
// Worked example, n = 3.  2^64 = 3*6148914691236517205 + 1, so t = 1.  Only the
// raw value 0 is rejected; the other 2^64-1 values fall evenly into residues
// 0, 1, 2.
package prng

import "math/big"

const (
	goldenGamma    uint64 = 0x9e3779b97f4a7c15
	mixMul1        uint64 = 0xbf58476d1ce4e5b9
	mixMul2        uint64 = 0x94d049bb133111eb
	fnvOffsetBasis uint64 = 0xcbf29ce484222325
	fnvPrime       uint64 = 0x00000100000001b3
)

// SplitMix64 is a deterministic 64-bit pseudo-random generator.  The zero
// value is a valid generator seeded with 0.  It is not safe for concurrent use.
type SplitMix64 struct {
	state uint64
}

// New returns a generator whose stream is fully determined by seed.
func New(seed uint64) *SplitMix64 {
	return &SplitMix64{state: seed}
}

// State returns the current internal state (the next call to NextU64 starts
// from it), so a stream position can be saved and restored.
func (r *SplitMix64) State() uint64 { return r.state }

// SetState restores a state previously returned by State.
func (r *SplitMix64) SetState(state uint64) { r.state = state }

// NextU64 advances the stream and returns the next 64 random bits.
func (r *SplitMix64) NextU64() uint64 {
	r.state += goldenGamma
	z := r.state
	z = (z ^ (z >> 30)) * mixMul1
	z = (z ^ (z >> 27)) * mixMul2
	return z ^ (z >> 31)
}

// Bounded returns a uniform value in [0, n) for 1 <= n <= 2^64-1.
//
// It always draws at least one word, even for n == 1 (where the answer is 0):
// that is the VER05 section 2.2 loop run once, and it keeps the stream
// position of every generator identical across language lanes (a fixed-length
// byte string still spends one word on its length).  n == 0 is a caller bug and
// panics.  A span of exactly 2^64 cannot be written as a uint64: use NextU64.
func (r *SplitMix64) Bounded(n uint64) uint64 {
	if n == 0 {
		panic("prng: Bounded(0) has no valid result")
	}
	// -n on an unsigned integer is 2^64 - n, so this is (2^64 - n) mod n.
	threshold := (-n) % n
	for {
		x := r.NextU64()
		if x >= threshold {
			return x % n
		}
	}
}

var (
	big1     = big.NewInt(1)
	bigTwo64 = new(big.Int).Lsh(big1, 64)
)

// BoundedBig returns a uniform value in [0, span) for any span >= 1.
//
//	span <= 2^64-1   Bounded(span)
//	span == 2^64     one raw word
//	span  > 2^64     k-word rejection sampling
//
// For a wide span, take k = max(2, ceil(bitlen(span-1)/64)) words, first draw
// most significant, giving raw uniform on [0, 2^(64k)).  A plain raw % span
// would over-weight the low residues (for span = 3*2^126 the low third would be
// drawn half the time instead of a third), so raw values below
// t = (2^(64k) - span) mod span are redrawn.  The acceptance probability is at
// least one half.  For 2^64 < span <= 2^128 this is exactly the Rust lane's
// 128-bit rejection.  span < 1 panics.
func (r *SplitMix64) BoundedBig(span *big.Int) *big.Int {
	if span == nil || span.Sign() <= 0 {
		panic("prng: BoundedBig needs span >= 1")
	}
	if span.Cmp(bigTwo64) < 0 {
		return new(big.Int).SetUint64(r.Bounded(span.Uint64()))
	}
	if span.Cmp(bigTwo64) == 0 {
		return new(big.Int).SetUint64(r.NextU64())
	}
	words := (new(big.Int).Sub(span, big1).BitLen() + 63) / 64
	if words < 2 {
		words = 2
	}
	modulus := new(big.Int).Lsh(big1, uint(64*words))
	threshold := new(big.Int).Sub(modulus, span)
	threshold.Mod(threshold, span)
	raw := new(big.Int)
	word := new(big.Int)
	for {
		raw.SetUint64(0)
		for i := 0; i < words; i++ {
			raw.Lsh(raw, 64)
			raw.Or(raw, word.SetUint64(r.NextU64()))
		}
		if raw.Cmp(threshold) >= 0 {
			return raw.Mod(raw, span)
		}
	}
}

// FNV1a64 is the 64-bit FNV-1a hash: start from the offset basis; for each
// byte xor it in, then multiply by the FNV prime (mod 2^64).
//
//	FNV1a64("")  = 0xcbf29ce484222325
//	FNV1a64("a") = 0xaf63dc4c8601ec8c
func FNV1a64(data []byte) uint64 {
	h := fnvOffsetBasis
	for _, b := range data {
		h = (h ^ uint64(b)) * fnvPrime
	}
	return h
}

// FNV1a64String hashes the UTF-8 bytes of s.
func FNV1a64String(s string) uint64 {
	h := fnvOffsetBasis
	for i := 0; i < len(s); i++ {
		h = (h ^ uint64(s[i])) * fnvPrime
	}
	return h
}

// SubSeed derives the isolated seed of one property inside a run
// (VER05 section 2.3):
//
//	SubSeed(run, id) = SplitMix64(run XOR fnv1a64(utf8(id))).NextU64()
//
// Two properties in one run never share a stream, and adding or reordering
// properties never changes another property's draws.  The final NextU64 is a
// scramble: without it two ids whose hashes differ in one bit would produce
// seeds differing in one bit, so their streams would be strongly related.  (The
// TypeScript and Dart lanes omit it; this lane follows the spec, see VER05d
// section 2.2.)
func SubSeed(runSeed uint64, id string) uint64 {
	return New(runSeed ^ FNV1a64String(id)).NextU64()
}
