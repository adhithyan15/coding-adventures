// Package fuzz is the Go lane of the Level 2 coverage-guided fuzz engine
// (VER06, specified for Go in VER06e).
//
// The package has three layers, each usable alone:
//
//   - the mutator algebra (this file): pure functions from bytes to bytes that
//     match the shared fuzz-v1 corpus byte for byte;
//   - coverage and scheduling (bitmap.go): the 65,536-edge bitmap, admission and
//     the power schedule;
//   - the campaign (engine.go): a deterministic, budgeted loop that runs a
//     target, deduplicates crashes by signature (signature.go) and minimises
//     them.
//
// It does not depend on Go's native fuzzing (go test -fuzz); it is the
// deterministic, cross-lane one.
package fuzz

import (
	"errors"
	"math/big"

	"github.com/adhithyan15/coding-adventures/code/packages/go/test-bootstrap/prng"
)

// Hard ceilings.  Every size an outside caller controls has one (CWE-400).
const (
	// MaxIterations bounds a campaign's mutation executions, MutateSequence
	// steps and the energy base.
	MaxIterations = 1_000_000
	// MaxInputBytes bounds an initial seed and the intermediate size of a
	// mutation sequence.
	MaxInputBytes = 1 << 20
)

// ErrSteps reports a MutateSequence step count above MaxIterations.
var ErrSteps = errors.New("fuzz: steps exceeds MaxIterations")

// Endian selects the byte order of the multi-byte mutators.  It is a closed
// set: any value other than BigEndian behaves as LittleEndian (Rust's rule for
// an unknown endian; the corpus runner rejects any other *text*).
type Endian int

const (
	// LittleEndian stores the least significant byte first.
	LittleEndian Endian = iota
	// BigEndian stores the most significant byte first.
	BigEndian
)

// -----------------------------------------------------------------------------
// Interest lists (VER06 section 3.4)
// -----------------------------------------------------------------------------

var (
	interest8  = []uint64{0, 1, 127, 128, 255}
	interest16 = []uint64{0, 1, 255, 256, 32767, 32768, 65535}
	interest32 = []uint64{0, 1, 65535, 65536, 0x7FFFFFFF, 0x80000000, 0xFFFFFFFF}
	interest64 = []uint64{0, 1, 0xFFFFFFFF, 0x100000000, 0x7FFFFFFFFFFFFFFF, 0x8000000000000000, 0xFFFFFFFFFFFFFFFF}
)

// deltas8 are the arithmetic deltas MutateSequence draws from.
var deltas8 = []int64{-35, -8, -1, 1, 8, 35}

// InterestValues returns a copy of the boundary values for a width in bytes
// (1, 2, 4 or 8), or nil for any other width.
func InterestValues(width int) []uint64 {
	var src []uint64
	switch width {
	case 1:
		src = interest8
	case 2:
		src = interest16
	case 4:
		src = interest32
	case 8:
		src = interest64
	default:
		return nil
	}
	return append([]uint64(nil), src...)
}

func validWidth(w int) bool { return w == 1 || w == 2 || w == 4 || w == 8 }

// -----------------------------------------------------------------------------
// Integer helpers.  Offsets wrap with FLOOR modulo (-1 mod 8 == 7); counts and
// lengths that exceed the machine range saturate.
// -----------------------------------------------------------------------------

// floorMod is a mod m for m > 0, always in [0, m).
func floorMod(a int64, m int) int {
	r := a % int64(m)
	if r < 0 {
		r += int64(m)
	}
	return int(r)
}

// modBig is the same for an arbitrary-size integer (big.Int.Mod is Euclidean,
// which equals floor modulo for a positive modulus).
func modBig(v *big.Int, m int) int {
	return int(new(big.Int).Mod(v, big.NewInt(int64(m))).Int64())
}

// satInt64 clamps a big integer into the int64 range.
func satInt64(v *big.Int) int64 {
	if v.IsInt64() {
		return v.Int64()
	}
	if v.Sign() < 0 {
		return -1 << 63
	}
	return 1<<63 - 1
}

var mask64 = new(big.Int).SetUint64(^uint64(0))

// low64 is v modulo 2^64 in two's complement (so -1 is 0xFFFF...FFFF); modulo
// 2^(8*width) is then a truncating write of the low `width` bytes.
func low64(v *big.Int) uint64 { return new(big.Int).And(v, mask64).Uint64() }

func clone(b []byte) []byte { return append([]byte(nil), b...) }

// -----------------------------------------------------------------------------
// Flips, by parity
// -----------------------------------------------------------------------------

// parityFlip inverts count consecutive units (bits when unitBits is 1, bytes
// when 8) starting at base, wrapping around a buffer of n units.
//
// The window of `count` covers every unit count/n times plus the first
// count%n units (counting from the wrapped start) once more.  A unit flipped an
// even number of times is back where it began (XOR twice), so only parity
// matters:
//
//	flips(step) = count/n + (1 if step < count%n else 0)
//
// That is O(len) work for ANY count, where a loop of `count` single flips would
// never finish for count = 10^18 (VER06 section 9.2).
//
// Worked example, 2 bytes = 16 bits, bit offset 0, count 17: 17/16 = 1 (every
// bit once, odd) and 17%16 = 1 (bit 0 once more, so twice, even): the result
// inverts bits 1..15 and leaves bit 0.
func parityFlip(data []byte, unitBits, base int, oddFull bool, rem int) []byte {
	buf := clone(data)
	if oddFull {
		for i := range buf {
			buf[i] ^= 0xFF
		}
	}
	n := len(buf)
	if unitBits == 1 {
		n *= 8
	}
	for step := 0; step < rem; step++ {
		pos := (base + step) % n
		if unitBits == 1 {
			buf[pos/8] ^= 1 << (pos % 8)
		} else {
			buf[pos] ^= 0xFF
		}
	}
	return buf
}

func unitCount(data []byte, unitBits int) int {
	if unitBits == 1 {
		return len(data) * 8
	}
	return len(data)
}

func flip(data []byte, unitBits int, offset, count int64) []byte {
	if len(data) == 0 || count <= 0 {
		return clone(data)
	}
	n := unitCount(data, unitBits)
	full, rem := count/int64(n), int(count%int64(n))
	return parityFlip(data, unitBits, floorMod(offset, n), full&1 == 1, rem)
}

func flipBig(data []byte, unitBits int, offset, count *big.Int) []byte {
	if len(data) == 0 || count.Sign() <= 0 {
		return clone(data)
	}
	n := unitCount(data, unitBits)
	bn := big.NewInt(int64(n))
	full, rem := new(big.Int).DivMod(count, bn, new(big.Int))
	return parityFlip(data, unitBits, modBig(offset, n), full.Bit(0) == 1, int(rem.Int64()))
}

// BitFlip inverts count consecutive bits from bitOffset mod (8*len).  Bits are
// numbered LSB-first within each byte: bit 9 of 00 00 is bit 1 of byte 1, so
// BitFlip(00 00, 9, 1) is 00 02.  An empty input or count <= 0 is a no-op.
func BitFlip(data []byte, bitOffset, count int64) []byte { return flip(data, 1, bitOffset, count) }

// BitFlipBig is BitFlip for arbitrary-size integers.
func BitFlipBig(data []byte, bitOffset, count *big.Int) []byte {
	return flipBig(data, 1, bitOffset, count)
}

// ByteFlip XORs count consecutive bytes with 0xFF from byteOffset mod len.
func ByteFlip(data []byte, byteOffset, count int64) []byte { return flip(data, 8, byteOffset, count) }

// ByteFlipBig is ByteFlip for arbitrary-size integers.
func ByteFlipBig(data []byte, byteOffset, count *big.Int) []byte {
	return flipBig(data, 8, byteOffset, count)
}

// -----------------------------------------------------------------------------
// Arithmetic and boundary values
// -----------------------------------------------------------------------------

func readUint(b []byte, w int, e Endian) uint64 {
	var v uint64
	for i := 0; i < w; i++ {
		if e == BigEndian {
			v = v<<8 | uint64(b[i])
		} else {
			v |= uint64(b[i]) << (8 * i)
		}
	}
	return v
}

// writeUint stores the low w bytes of v: that is "v mod 2^(8w)".
func writeUint(b []byte, w int, e Endian, v uint64) {
	for i := 0; i < w; i++ {
		shift := 8 * i
		if e == BigEndian {
			shift = 8 * (w - 1 - i)
		}
		b[i] = byte(v >> shift)
	}
}

// arithAt adds delta (mod 2^64, which is also mod 2^(8w) after truncation) to
// the width-byte integer at `at`.  Callers vet width and length.
func arithAt(data []byte, at, w int, delta uint64, e Endian) []byte {
	buf := clone(data)
	writeUint(buf[at:at+w], w, e, readUint(buf[at:at+w], w, e)+delta)
	return buf
}

func boundaryAt(data []byte, at, w int, val uint64, e Endian) []byte {
	buf := clone(data)
	writeUint(buf[at:at+w], w, e, val)
	return buf
}

// ByteArithmetic adds the signed delta modulo 2^(8*width) to the integer at the
// offset.  width is in {1, 2, 4, 8}; any other width, or len(data) < width,
// returns the input unchanged.  The offset wraps over the len-width+1 positions
// where a whole integer fits.
//
// Worked example (u16, little-endian): ff 00 is 0x00ff; adding 1 gives 0x0100,
// stored little-endian as 00 01.
func ByteArithmetic(data []byte, byteOffset, delta int64, width int, e Endian) []byte {
	if !validWidth(width) || len(data) < width {
		return clone(data)
	}
	return arithAt(data, floorMod(byteOffset, len(data)-width+1), width, uint64(delta), e)
}

// ByteArithmeticBig is ByteArithmetic for arbitrary-size integers.
func ByteArithmeticBig(data []byte, byteOffset, delta *big.Int, width int, e Endian) []byte {
	if !validWidth(width) || len(data) < width {
		return clone(data)
	}
	return arithAt(data, modBig(byteOffset, len(data)-width+1), width, low64(delta), e)
}

// BoundaryInsert overwrites width bytes with val mod 2^(8*width) at the offset.
// Same width and short-buffer rules as ByteArithmetic.  Worked example:
// BoundaryInsert(00 00 00 00, 0, 4, 0x80000000, little) is 00 00 00 80.
func BoundaryInsert(data []byte, byteOffset int64, width int, val uint64, e Endian) []byte {
	if !validWidth(width) || len(data) < width {
		return clone(data)
	}
	return boundaryAt(data, floorMod(byteOffset, len(data)-width+1), width, val, e)
}

// BoundaryInsertBig is BoundaryInsert for arbitrary-size integers (a negative
// val is its two's complement).
func BoundaryInsertBig(data []byte, byteOffset *big.Int, width int, val *big.Int, e Endian) []byte {
	if !validWidth(width) || len(data) < width {
		return clone(data)
	}
	return boundaryAt(data, modBig(byteOffset, len(data)-width+1), width, low64(val), e)
}

// -----------------------------------------------------------------------------
// Blocks and splice
// -----------------------------------------------------------------------------

func deleteAt(data []byte, at int, length int64) []byte {
	end := at + int(min(length, int64(len(data)-at)))
	return append(clone(data[:at]), data[end:]...)
}

// BlockDelete deletes up to length bytes at offset mod len, clamped at the end.
func BlockDelete(data []byte, offset, length int64) []byte {
	if len(data) == 0 || length <= 0 {
		return clone(data)
	}
	return deleteAt(data, floorMod(offset, len(data)), length)
}

// BlockDeleteBig is BlockDelete for arbitrary-size integers.
func BlockDeleteBig(data []byte, offset, length *big.Int) []byte {
	if len(data) == 0 || length.Sign() <= 0 {
		return clone(data)
	}
	return deleteAt(data, modBig(offset, len(data)), satInt64(length))
}

func insertAt(data []byte, at int, ins []byte) []byte {
	out := make([]byte, 0, len(data)+len(ins))
	out = append(out, data[:at]...)
	out = append(out, ins...)
	return append(out, data[at:]...)
}

// BlockInsert inserts at offset mod (len+1); an empty input becomes the insert.
func BlockInsert(data []byte, offset int64, insert []byte) []byte {
	if len(data) == 0 {
		return clone(insert)
	}
	return insertAt(data, floorMod(offset, len(data)+1), insert)
}

// BlockInsertBig is BlockInsert for an arbitrary-size offset.
func BlockInsertBig(data []byte, offset *big.Int, insert []byte) []byte {
	if len(data) == 0 {
		return clone(insert)
	}
	return insertAt(data, modBig(offset, len(data)+1), insert)
}

func duplicateAt(data []byte, src int, length int64, dst int) []byte {
	chunk := data[src : src+int(min(length, int64(len(data)-src)))]
	return insertAt(data, dst, chunk)
}

// BlockDuplicate copies up to length bytes from srcOffset mod len and inserts
// them at dstOffset mod (len+1) (the end of the buffer is a valid insertion
// point); the copy is clamped at the end of the source.
func BlockDuplicate(data []byte, srcOffset, length, dstOffset int64) []byte {
	if len(data) == 0 || length <= 0 {
		return clone(data)
	}
	return duplicateAt(data, floorMod(srcOffset, len(data)), length, floorMod(dstOffset, len(data)+1))
}

// BlockDuplicateBig is BlockDuplicate for arbitrary-size integers.
func BlockDuplicateBig(data []byte, srcOffset, length, dstOffset *big.Int) []byte {
	if len(data) == 0 || length.Sign() <= 0 {
		return clone(data)
	}
	return duplicateAt(data, modBig(srcOffset, len(data)), satInt64(length), modBig(dstOffset, len(data)+1))
}

func clampSplit(split int64, n int) int { return int(max(0, min(split, int64(n)))) }

// Splice is target[:ts] + donor[ds:] with both splits clamped into their
// buffers.
func Splice(target, donor []byte, targetSplit, donorSplit int64) []byte {
	t, d := clampSplit(targetSplit, len(target)), clampSplit(donorSplit, len(donor))
	out := make([]byte, 0, t+len(donor)-d)
	out = append(out, target[:t]...)
	return append(out, donor[d:]...)
}

// SpliceBig is Splice for arbitrary-size splits.
func SpliceBig(target, donor []byte, targetSplit, donorSplit *big.Int) []byte {
	return Splice(target, donor, satInt64(targetSplit), satInt64(donorSplit))
}

// -----------------------------------------------------------------------------
// MutateSequence
// -----------------------------------------------------------------------------

// MutateSequence applies `steps` seeded mutations, each chosen by one Bounded(7)
// draw:
//
//	draw  step
//	0     bit flip, count 1
//	1     byte flip, count 1
//	2     byte arithmetic, width 1, delta from (-35 -8 -1 1 8 35)
//	3     boundary insert, width 1, value from (0 1 127 128 255)
//	4     block delete (length 1..len/2); skipped if len <= 1
//	5     block insert of one random byte
//	6     block duplicate (length 1..len/2); skipped if len <= 1
//
// An empty buffer is refilled to 00 before every step.  The draw order is the
// cross-lane contract: the same (data, seed, steps) gives the same bytes in
// every language.  Draws for a skipped step are still consumed.
//
// maxLen is VER06 section 3.5's "maximum length budget": a block insert or
// duplicate that would grow the buffer past it is skipped AFTER its draws are
// consumed, so the stream stays aligned and memory stays bounded (each
// duplicate can add len/2, so 64 of them would otherwise be exponential).
// A negative steps runs none; steps above MaxIterations is ErrSteps.
func MutateSequence(data []byte, seed uint64, steps, maxLen int) ([]byte, error) {
	if steps > MaxIterations {
		return nil, ErrSteps
	}
	rng := prng.New(seed)
	cur := clone(data)
	for i := 0; i < steps; i++ {
		if len(cur) == 0 {
			cur = []byte{0}
		}
		n := len(cur)
		switch rng.Bounded(7) {
		case 0:
			cur = flip(cur, 1, int64(rng.Bounded(uint64(n)*8)), 1)
		case 1:
			cur = flip(cur, 8, int64(rng.Bounded(uint64(n))), 1)
		case 2:
			delta := deltas8[rng.Bounded(uint64(len(deltas8)))]
			cur = arithAt(cur, int(rng.Bounded(uint64(n))), 1, uint64(delta), LittleEndian)
		case 3:
			val := interest8[rng.Bounded(uint64(len(interest8)))]
			cur = boundaryAt(cur, int(rng.Bounded(uint64(n))), 1, val, LittleEndian)
		case 4:
			if n > 1 {
				length := 1 + int64(rng.Bounded(uint64(max(1, n/2))))
				cur = deleteAt(cur, int(rng.Bounded(uint64(n))), length)
			}
		case 5:
			ins := byte(rng.Bounded(256))
			at := int(rng.Bounded(uint64(n + 1)))
			if n+1 <= maxLen {
				cur = insertAt(cur, at, []byte{ins})
			}
		default: // 6
			if n > 1 {
				length := 1 + int64(rng.Bounded(uint64(max(1, n/2))))
				src := int(rng.Bounded(uint64(n)))
				dst := int(rng.Bounded(uint64(n + 1)))
				if n+int(min(length, int64(n-src))) <= maxLen {
					cur = duplicateAt(cur, src, length, dst)
				}
			}
		}
	}
	return cur, nil
}
