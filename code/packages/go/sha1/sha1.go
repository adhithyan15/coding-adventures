// Package sha1 provides the SHA-1 cryptographic hash function implemented from scratch.
//
// # What Is SHA-1?
//
// SHA-1 (Secure Hash Algorithm 1) takes any sequence of bytes and produces a
// fixed-size 20-byte (160-bit) digest. The same input always gives the same
// digest. Change one bit of input and the entire digest changes — the "avalanche
// effect". You cannot reverse a digest back to the original input.
//
// # The Merkle-Damgård Construction
//
// SHA-1 processes data in 512-bit (64-byte) blocks. The "state" is five 32-bit
// words (H0..H4), initialized to fixed constants. For each block, 80 rounds of
// bit mixing fold the block into the state. The final state is the digest.
//
// Analogy: a blender. Start with a base liquid (initial constants). Add
// ingredients one chunk at a time (message blocks). Each blend mixes the new
// ingredient with everything before it. You cannot un-blend.
//
// # FIPS 180-4 Test Vectors
//
//	Sum1([]byte(""))    = da39a3ee5e6b4b0d3255bfef95601890afd80709
//	Sum1([]byte("abc")) = a9993e364706816aba3e25717850c26c9cd0d89d
//
// Note: This package is named "sha1" but lives in a separate module from Go's
// standard library crypto/sha1. Import it by module path to use it.
package sha1

import (
	"encoding/binary"
	"encoding/hex"
)

// ─── Initialization Constants ─────────────────────────────────────────────────
//
// SHA-1 starts with these five 32-bit words as its initial state. They are
// "nothing up my sleeve" numbers — their obvious counting-sequence structure
// (01234567, 89ABCDEF, ... reversed) proves no backdoor is baked in.
//
//	H0 = 0x67452301 → bytes 67 45 23 01 → reverse: 01 23 45 67
//	H1 = 0xEFCDAB89 → bytes EF CD AB 89 → reverse: 89 AB CD EF

var initState = [5]uint32{
	0x67452301,
	0xEFCDAB89,
	0x98BADCFE,
	0x10325476,
	0xC3D2E1F0,
}

// Round constants — one per 20-round stage, derived from square roots.
var kConst = [4]uint32{
	0x5A827999, // rounds 0–19:  floor(sqrt(2)  * 2^30)
	0x6ED9EBA1, // rounds 20–39: floor(sqrt(3)  * 2^30)
	0x8F1BBCDC, // rounds 40–59: floor(sqrt(5)  * 2^30)
	0xCA62C1D6, // rounds 60–79: floor(sqrt(10) * 2^30)
}

// ─── Helper: Circular Left Shift ──────────────────────────────────────────────
//
// rotl32(n, x) rotates x left by n bit positions. Bits that "fall off" the
// left end reappear on the right, unlike a regular << shift where they are lost.
//
// Example with n=2, x=0b01101001:
//
//	Regular:  01101001 << 2 = 10100100  (01 on the left is gone)
//	Circular: 01101001 ROTL 2 = 10100110  (01 wraps to the right)
func rotl32(n uint, x uint32) uint32 {
	return (x << n) | (x >> (32 - n))
}

// ─── Padding ──────────────────────────────────────────────────────────────────
//
// The compression function needs exactly 64-byte blocks. Padding extends the
// message:
//  1. Append 0x80 (the '1' bit followed by seven '0' bits).
//  2. Append zeros until length ≡ 56 (mod 64).
//  3. Append original bit length as a 64-bit big-endian integer.
//
// Example — "abc" (3 bytes = 24 bits):
//
//	61 62 63 80 [52 zero bytes] 00 00 00 00 00 00 00 18
//	                                              ^^^^
//	                                    24 bits in hex (big-endian)
func pad(data []byte) []byte {
	bitLen := uint64(len(data)) * 8
	// Append the 1-bit
	padded := append([]byte{}, data...)
	padded = append(padded, 0x80)
	// Append zeros until length ≡ 56 (mod 64)
	for len(padded)%64 != 56 {
		padded = append(padded, 0x00)
	}
	// Append 64-bit big-endian length
	var lenBuf [8]byte
	binary.BigEndian.PutUint64(lenBuf[:], bitLen)
	padded = append(padded, lenBuf[:]...)
	return padded
}

// ─── Message Schedule ─────────────────────────────────────────────────────────
//
// Each 64-byte block is parsed as 16 big-endian uint32 words (W[0..15]).
// These are expanded to 80 words:
//
//	W[i] = ROTL(1, W[i-3] XOR W[i-8] XOR W[i-14] XOR W[i-16])  for i ≥ 16
//
// More words = more mixing = better avalanche. A single bit change in the
// input ripples through all 80 words.
func schedule(block []byte) [80]uint32 {
	var W [80]uint32
	for i := 0; i < 16; i++ {
		W[i] = binary.BigEndian.Uint32(block[i*4 : i*4+4])
	}
	for i := 16; i < 80; i++ {
		W[i] = rotl32(1, W[i-3]^W[i-8]^W[i-14]^W[i-16])
	}
	return W
}

// ─── Compression Function ─────────────────────────────────────────────────────
//
// 80 rounds of mixing fold one 64-byte block into the five-word state.
//
// Four stages of 20 rounds, each with a different auxiliary function f:
//
//	Stage  Rounds  f(b,c,d)                    Purpose
//	1      0–19    (b&c) | (~b&d)              Selector (mux)
//	2      20–39   b^c^d                       Parity
//	3      40–59   (b&c)|(b&d)|(c&d)           Majority vote
//	4      60–79   b^c^d                       Parity again
//
// Each round: temp = ROTL(5,a) + f(b,c,d) + e + K + W[t]
//
//	shift state: e=d, d=c, c=ROTL(30,b), b=a, a=temp
func compress(state [5]uint32, block []byte) [5]uint32 {
	W := schedule(block)
	h0, h1, h2, h3, h4 := state[0], state[1], state[2], state[3], state[4]
	a, b, c, d, e := h0, h1, h2, h3, h4

	for t := 0; t < 80; t++ {
		var f, k uint32
		switch {
		case t < 20:
			// Selector: if b=1 output c, if b=0 output d
			f = (b & c) | (^b & d)
			k = kConst[0]
		case t < 40:
			// Parity: 1 if an odd number of inputs are 1
			f = b ^ c ^ d
			k = kConst[1]
		case t < 60:
			// Majority: 1 if at least 2 of the 3 inputs are 1
			f = (b & c) | (b & d) | (c & d)
			k = kConst[2]
		default:
			// Parity again (same formula, different constant)
			f = b ^ c ^ d
			k = kConst[3]
		}
		temp := rotl32(5, a) + f + e + k + W[t]
		e = d
		d = c
		c = rotl32(30, b)
		b = a
		a = temp
	}

	// Davies-Meyer feed-forward: add compressed output back to input state.
	return [5]uint32{
		h0 + a,
		h1 + b,
		h2 + c,
		h3 + d,
		h4 + e,
	}
}

// ─── Public API ───────────────────────────────────────────────────────────────

// Sum1 computes the SHA-1 digest of data and returns it as a [20]byte array.
//
// This is the one-shot API: hash a complete message in a single call.
//
//	digest := sha1.Sum1([]byte("abc"))
//	fmt.Println(hex.EncodeToString(digest[:]))
//	// → a9993e364706816aba3e25717850c26c9cd0d89d
//
// We name this Sum1 (not Sum) to avoid clashing with Go stdlib's crypto/sha1.Sum.
func Sum1(data []byte) [20]byte {
	result, _ := StartNew[[20]byte]("sha1.Sum1", [20]byte{},
		func(op *Operation[[20]byte], rf *ResultFactory[[20]byte]) *OperationResult[[20]byte] {
			op.AddProperty("dataLen", len(data))
			padded := pad(data)
			state := initState
			for i := 0; i < len(padded); i += 64 {
				state = compress(state, padded[i:i+64])
			}
			var digest [20]byte
			for i, w := range state {
				binary.BigEndian.PutUint32(digest[i*4:], w)
			}
			return rf.Generate(true, false, digest)
		}).GetResult()
	return result
}

// HexString computes SHA-1 and returns the 40-character lowercase hex string.
//
//	sha1.HexString([]byte("abc")) → "a9993e364706816aba3e25717850c26c9cd0d89d"
func HexString(data []byte) string {
	result, _ := StartNew[string]("sha1.HexString", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			op.AddProperty("dataLen", len(data))
			digest := Sum1(data)
			return rf.Generate(true, false, hex.EncodeToString(digest[:]))
		}).GetResult()
	return result
}

// Digest is a streaming SHA-1 hasher that accepts data in multiple chunks.
//
// Useful when the full message is not available at once (large files, streams).
// The API is:
//
//	h := sha1.New()
//	h.Write([]byte("ab"))
//	h.Write([]byte("c"))
//	fmt.Println(h.HexDigest())  // → a9993e364706816aba3e25717850c26c9cd0d89d
//
// Multiple Write calls are equivalent to a single Sum1(all_data).
type Digest struct {
	state     [5]uint32
	buf       []byte
	byteCount uint64 // total bytes written (used in padding length field)
}

// New returns a new streaming SHA-1 Digest initialized with the starting constants.
func New() *Digest {
	result, _ := StartNew[*Digest]("sha1.New", nil,
		func(op *Operation[*Digest], rf *ResultFactory[*Digest]) *OperationResult[*Digest] {
			return rf.Generate(true, false, &Digest{state: initState})
		}).GetResult()
	return result
}

// Write feeds more bytes into the hash computation. Always returns len(p), nil.
func (d *Digest) Write(p []byte) (int, error) {
	return StartNew[int]("sha1.Digest.Write", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("pLen", len(p))
			d.buf = append(d.buf, p...)
			d.byteCount += uint64(len(p))
			for len(d.buf) >= 64 {
				d.state = compress(d.state, d.buf[:64])
				d.buf = d.buf[64:]
			}
			return rf.Generate(true, false, len(p))
		}).GetResult()
}

// Sum1 returns the 20-byte digest of all data written so far.
//
// Non-destructive: the internal state is not modified, so you can continue
// writing after calling Sum1.
func (d *Digest) Sum1() [20]byte {
	result, _ := StartNew[[20]byte]("sha1.Digest.Sum1", [20]byte{},
		func(op *Operation[[20]byte], rf *ResultFactory[[20]byte]) *OperationResult[[20]byte] {
			bitLen := d.byteCount * 8
			tail := append([]byte{}, d.buf...)
			tail = append(tail, 0x80)
			for len(tail)%64 != 56 {
				tail = append(tail, 0x00)
			}
			var lenBuf [8]byte
			binary.BigEndian.PutUint64(lenBuf[:], bitLen)
			tail = append(tail, lenBuf[:]...)

			state := d.state
			for i := 0; i < len(tail); i += 64 {
				state = compress(state, tail[i:i+64])
			}

			var digest [20]byte
			for i, w := range state {
				binary.BigEndian.PutUint32(digest[i*4:], w)
			}
			return rf.Generate(true, false, digest)
		}).GetResult()
	return result
}

// HexDigest returns the 40-character hex string of the digest.
func (d *Digest) HexDigest() string {
	result, _ := StartNew[string]("sha1.Digest.HexDigest", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			digest := d.Sum1()
			return rf.Generate(true, false, hex.EncodeToString(digest[:]))
		}).GetResult()
	return result
}
