// Package sha256 provides the SHA-256 cryptographic hash function implemented from scratch.
//
// # What Is SHA-256?
//
// SHA-256 (Secure Hash Algorithm 256) belongs to the SHA-2 family, designed by
// the NSA and published by NIST in 2001. It produces a 256-bit (32-byte) digest
// and is the workhorse of modern cryptography -- TLS, Bitcoin, git, code signing,
// and password hashing all rely on SHA-256.
//
// # How It Differs from SHA-1
//
// SHA-256 shares the same Merkle-Damgard construction as SHA-1 but with:
//   - 8 state words (not 5), each 32 bits wide
//   - 64 rounds (not 80) per block
//   - 64 unique round constants from cube roots of first 64 primes
//   - A more complex message schedule with two "small sigma" functions
//   - Two "big Sigma" functions and uniform Ch/Maj usage every round
//
// # The Merkle-Damgard Construction
//
// SHA-256 processes data in 512-bit (64-byte) blocks. The "state" is eight 32-bit
// words (H0..H7), initialized from fractional parts of square roots of the first
// 8 primes. For each block, 64 rounds of bit mixing fold the block into the state.
// The final state is the digest.
//
// # FIPS 180-4 Test Vectors
//
//	Sum256([]byte(""))    = e3b0c442...7852b855
//	Sum256([]byte("abc")) = ba7816bf...f20015ad
package sha256

import (
	"encoding/binary"
	"encoding/hex"
)

// === Initialization Constants ================================================
//
// Eight 32-bit words derived from the FRACTIONAL parts of the square roots of
// the first 8 prime numbers (2, 3, 5, 7, 11, 13, 17, 19).
//
// For prime p, take sqrt(p), keep only the fractional part, multiply by 2^32,
// and take the floor. Example:
//
//	sqrt(2)  = 1.4142135623...
//	frac     = 0.4142135623...
//	* 2^32   = 1779033703.952... -> floor = 0x6A09E667
//
// These are "nothing up my sleeve" numbers -- their derivation is transparent,
// proving no hidden mathematical backdoor exists.
var initState = [8]uint32{
	0x6A09E667, // sqrt(2)
	0xBB67AE85, // sqrt(3)
	0x3C6EF372, // sqrt(5)
	0xA54FF53A, // sqrt(7)
	0x510E527F, // sqrt(11)
	0x9B05688C, // sqrt(13)
	0x1F83D9AB, // sqrt(17)
	0x5BE0CD19, // sqrt(19)
}

// 64 round constants from the FRACTIONAL parts of the cube roots of the first
// 64 prime numbers (2, 3, 5, ..., 311). Having unique constants for each round
// prevents round symmetry attacks.
var kConst = [64]uint32{
	0x428A2F98, 0x71374491, 0xB5C0FBCF, 0xE9B5DBA5,
	0x3956C25B, 0x59F111F1, 0x923F82A4, 0xAB1C5ED5,
	0xD807AA98, 0x12835B01, 0x243185BE, 0x550C7DC3,
	0x72BE5D74, 0x80DEB1FE, 0x9BDC06A7, 0xC19BF174,
	0xE49B69C1, 0xEFBE4786, 0x0FC19DC6, 0x240CA1CC,
	0x2DE92C6F, 0x4A7484AA, 0x5CB0A9DC, 0x76F988DA,
	0x983E5152, 0xA831C66D, 0xB00327C8, 0xBF597FC7,
	0xC6E00BF3, 0xD5A79147, 0x06CA6351, 0x14292967,
	0x27B70A85, 0x2E1B2138, 0x4D2C6DFC, 0x53380D13,
	0x650A7354, 0x766A0ABB, 0x81C2C92E, 0x92722C85,
	0xA2BFE8A1, 0xA81A664B, 0xC24B8B70, 0xC76C51A3,
	0xD192E819, 0xD6990624, 0xF40E3585, 0x106AA070,
	0x19A4C116, 0x1E376C08, 0x2748774C, 0x34B0BCB5,
	0x391C0CB3, 0x4ED8AA4A, 0x5B9CCA4F, 0x682E6FF3,
	0x748F82EE, 0x78A5636F, 0x84C87814, 0x8CC70208,
	0x90BEFFFA, 0xA4506CEB, 0xBEF9A3F7, 0xC67178F2,
}

// === Bit Manipulation Helpers ================================================
//
// SHA-256 uses six auxiliary functions combining rotations, shifts, and boolean
// operations to create non-linear mixing that resists cryptanalysis.

// rotr32 performs a circular right rotation of a 32-bit word by n positions.
// Bits that "fall off" the right end reappear on the left.
//
// Example with n=3, x=0b11010010:
//
//	Shift:  11010010 >> 3  = 00011010  (110 is lost)
//	Rotate: 11010010 ROTR3 = 01011010  (010 wraps to the left)
//
// Go's uint32 arithmetic naturally wraps at 32 bits, so no masking is needed.
func rotr32(n uint, x uint32) uint32 {
	return (x >> n) | (x << (32 - n))
}

// ch is the Choice function: for each bit, if x=1 choose y, else choose z.
//
// Truth table:
//
//	x | y | z | Ch
//	--+---+---+----
//	0 | * | z |  z    (x=0: output follows z)
//	1 | y | * |  y    (x=1: output follows y)
//
// Think of it as a 1-bit multiplexer: x selects between y and z.
func ch(x, y, z uint32) uint32 {
	return (x & y) ^ (^x & z)
}

// maj is the Majority function: output is 1 if at least 2 of 3 inputs are 1.
//
// Ensures that even if one variable is "stuck", the other two still influence
// the output.
func maj(x, y, z uint32) uint32 {
	return (x & y) ^ (x & z) ^ (y & z)
}

// bigSigma0 is used on working variable 'a' in the round function.
// Sigma0(x) = ROTR(2,x) XOR ROTR(13,x) XOR ROTR(22,x)
func bigSigma0(x uint32) uint32 {
	return rotr32(2, x) ^ rotr32(13, x) ^ rotr32(22, x)
}

// bigSigma1 is used on working variable 'e' in the round function.
// Sigma1(x) = ROTR(6,x) XOR ROTR(11,x) XOR ROTR(25,x)
func bigSigma1(x uint32) uint32 {
	return rotr32(6, x) ^ rotr32(11, x) ^ rotr32(25, x)
}

// smallSigma0 is used in the message schedule expansion.
// sigma0(x) = ROTR(7,x) XOR ROTR(18,x) XOR SHR(3,x)
//
// Note the SHR (shift, not rotate) in the third term -- it destroys
// information intentionally, making the schedule a one-way function.
func smallSigma0(x uint32) uint32 {
	return rotr32(7, x) ^ rotr32(18, x) ^ (x >> 3)
}

// smallSigma1 is used in the message schedule expansion.
// sigma1(x) = ROTR(17,x) XOR ROTR(19,x) XOR SHR(10,x)
func smallSigma1(x uint32) uint32 {
	return rotr32(17, x) ^ rotr32(19, x) ^ (x >> 10)
}

// === Padding =================================================================
//
// Extends the message to a multiple of 64 bytes per FIPS 180-4 section 5.1.1:
//  1. Append 0x80 (the '1' bit followed by seven '0' bits)
//  2. Append zeros until length == 56 (mod 64)
//  3. Append original bit length as a 64-bit big-endian integer
//
// Why 56 mod 64? Because 56 + 8 = 64 -- room for the 8-byte length field.
func pad(data []byte) []byte {
	bitLen := uint64(len(data)) * 8
	padded := append([]byte{}, data...)
	padded = append(padded, 0x80)
	for len(padded)%64 != 56 {
		padded = append(padded, 0x00)
	}
	var lenBuf [8]byte
	binary.BigEndian.PutUint64(lenBuf[:], bitLen)
	padded = append(padded, lenBuf[:]...)
	return padded
}

// === Message Schedule ========================================================
//
// Each 64-byte block is expanded into a 64-word schedule W[0..63].
// The first 16 words come from the block (big-endian uint32). Words 16..63:
//
//	W[t] = sigma1(W[t-2]) + W[t-7] + sigma0(W[t-15]) + W[t-16]
//
// The two sigma functions create stronger diffusion than SHA-1's simple XOR.
func schedule(block []byte) [64]uint32 {
	var W [64]uint32
	for i := 0; i < 16; i++ {
		W[i] = binary.BigEndian.Uint32(block[i*4 : i*4+4])
	}
	for i := 16; i < 64; i++ {
		W[i] = smallSigma1(W[i-2]) + W[i-7] + smallSigma0(W[i-15]) + W[i-16]
	}
	return W
}

// === Compression Function ====================================================
//
// The heart of SHA-256. Each 64-byte block is "compressed" into the 8-word
// state through 64 rounds.
//
// Working variables a..h are initialized from the state. Each round:
//
//	T1 = h + Sigma1(e) + Ch(e,f,g) + K[t] + W[t]
//	T2 = Sigma0(a) + Maj(a,b,c)
//	shift: h=g, g=f, f=e, e=d+T1, d=c, c=b, b=a, a=T1+T2
//
// Davies-Meyer feed-forward: add working variables back to input state.
func compress(state [8]uint32, block []byte) [8]uint32 {
	W := schedule(block)
	h0, h1, h2, h3 := state[0], state[1], state[2], state[3]
	h4, h5, h6, h7 := state[4], state[5], state[6], state[7]
	a, b, c, d, e, f, g, h := h0, h1, h2, h3, h4, h5, h6, h7

	for t := 0; t < 64; t++ {
		t1 := h + bigSigma1(e) + ch(e, f, g) + kConst[t] + W[t]
		t2 := bigSigma0(a) + maj(a, b, c)
		h = g
		g = f
		f = e
		e = d + t1
		d = c
		c = b
		b = a
		a = t1 + t2
	}

	return [8]uint32{
		h0 + a, h1 + b, h2 + c, h3 + d,
		h4 + e, h5 + f, h6 + g, h7 + h,
	}
}

// === Public API ==============================================================

// Sum256 computes the SHA-256 digest of data and returns it as a [32]byte array.
//
// This is the one-shot API: hash a complete message in a single call.
//
//	digest := sha256.Sum256([]byte("abc"))
//	fmt.Println(hex.EncodeToString(digest[:]))
//	// -> ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad
func Sum256(data []byte) [32]byte {
	result, _ := StartNew[[32]byte]("sha256.Sum256", [32]byte{},
		func(op *Operation[[32]byte], rf *ResultFactory[[32]byte]) *OperationResult[[32]byte] {
			op.AddProperty("dataLen", len(data))
			padded := pad(data)
			state := initState
			for i := 0; i < len(padded); i += 64 {
				state = compress(state, padded[i:i+64])
			}
			var digest [32]byte
			for i, w := range state {
				binary.BigEndian.PutUint32(digest[i*4:], w)
			}
			return rf.Generate(true, false, digest)
		}).GetResult()
	return result
}

// HexString computes SHA-256 and returns the 64-character lowercase hex string.
//
//	sha256.HexString([]byte("abc")) -> "ba7816bf..."
func HexString(data []byte) string {
	result, _ := StartNew[string]("sha256.HexString", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			op.AddProperty("dataLen", len(data))
			digest := Sum256(data)
			return rf.Generate(true, false, hex.EncodeToString(digest[:]))
		}).GetResult()
	return result
}

// Digest is a streaming SHA-256 hasher that accepts data in multiple chunks.
//
// Useful when the full message is not available at once (large files, streams).
// The API:
//
//	h := sha256.New()
//	h.Write([]byte("ab"))
//	h.Write([]byte("c"))
//	fmt.Println(h.HexDigest())  // -> ba7816bf...
//
// Multiple Write calls are equivalent to a single Sum256(all_data).
type Digest struct {
	state     [8]uint32
	buf       []byte
	byteCount uint64 // total bytes written (used in padding length field)
}

// New returns a new streaming SHA-256 Digest initialized with the starting constants.
func New() *Digest {
	result, _ := StartNew[*Digest]("sha256.New", nil,
		func(op *Operation[*Digest], rf *ResultFactory[*Digest]) *OperationResult[*Digest] {
			return rf.Generate(true, false, &Digest{state: initState})
		}).GetResult()
	return result
}

// Write feeds more bytes into the hash computation. Always returns len(p), nil.
func (d *Digest) Write(p []byte) (int, error) {
	return StartNew[int]("sha256.Digest.Write", 0,
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

// Sum256 returns the 32-byte digest of all data written so far.
//
// Non-destructive: the internal state is not modified, so you can continue
// writing after calling Sum256.
func (d *Digest) Sum256() [32]byte {
	result, _ := StartNew[[32]byte]("sha256.Digest.Sum256", [32]byte{},
		func(op *Operation[[32]byte], rf *ResultFactory[[32]byte]) *OperationResult[[32]byte] {
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

			var digest [32]byte
			for i, w := range state {
				binary.BigEndian.PutUint32(digest[i*4:], w)
			}
			return rf.Generate(true, false, digest)
		}).GetResult()
	return result
}

// HexDigest returns the 64-character hex string of the digest.
func (d *Digest) HexDigest() string {
	result, _ := StartNew[string]("sha256.Digest.HexDigest", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			digest := d.Sum256()
			return rf.Generate(true, false, hex.EncodeToString(digest[:]))
		}).GetResult()
	return result
}

// Copy returns an independent deep copy of the current Digest state.
//
// Useful for computing multiple hashes that share a common prefix:
//
//	h := sha256.New()
//	h.Write(commonPrefix)
//	h1 := h.Copy(); h1.Write([]byte("suffix_a"))
//	h2 := h.Copy(); h2.Write([]byte("suffix_b"))
func (d *Digest) Copy() *Digest {
	result, _ := StartNew[*Digest]("sha256.Digest.Copy", nil,
		func(op *Operation[*Digest], rf *ResultFactory[*Digest]) *OperationResult[*Digest] {
			other := &Digest{
				state:     d.state,
				byteCount: d.byteCount,
			}
			other.buf = make([]byte, len(d.buf))
			copy(other.buf, d.buf)
			return rf.Generate(true, false, other)
		}).GetResult()
	return result
}
