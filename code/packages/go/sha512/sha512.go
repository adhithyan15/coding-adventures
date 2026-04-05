// Package sha512 provides the SHA-512 cryptographic hash function implemented from scratch.
//
// # What Is SHA-512?
//
// SHA-512 is the 64-bit sibling of SHA-256 in the SHA-2 family. It takes any
// sequence of bytes and produces a fixed-size 64-byte (512-bit) digest. The same
// input always gives the same digest. Change one bit of input and the entire
// digest changes -- the "avalanche effect".
//
// # How It Differs From SHA-256
//
// SHA-512 shares the same Merkle-Damgard structure as SHA-256 but everything is
// wider:
//
//   - State: 8 x 64-bit words (vs 32-bit)
//   - Block size: 128 bytes (vs 64)
//   - Rounds: 80 (vs 64)
//   - Round constants: 80 x 64-bit (vs 64 x 32-bit)
//   - Length field: 128-bit (vs 64-bit)
//   - Different rotation amounts tuned for 64-bit words
//
// On 64-bit hardware SHA-512 is often faster than SHA-256 because it processes
// data in 128-byte blocks using native 64-bit arithmetic.
//
// # FIPS 180-4 Test Vectors
//
//	Sum512([]byte(""))    = cf83e1357eefb8bd...f927da3e  (128 hex chars)
//	Sum512([]byte("abc")) = ddaf35a193617aba...a54ca49f  (128 hex chars)
package sha512

import (
	"encoding/binary"
	"encoding/hex"
)

// ---- Initial Hash Values (FIPS 180-4 section 5.3.5) ----
//
// Fractional parts of the square roots of the first eight primes (2, 3, 5, 7,
// 11, 13, 17, 19), truncated to 64 bits. "Nothing up my sleeve" numbers.
var initState = [8]uint64{
	0x6A09E667F3BCC908, // frac(sqrt(2))
	0xBB67AE8584CAA73B, // frac(sqrt(3))
	0x3C6EF372FE94F82B, // frac(sqrt(5))
	0xA54FF53A5F1D36F1, // frac(sqrt(7))
	0x510E527FADE682D1, // frac(sqrt(11))
	0x9B05688C2B3E6C1F, // frac(sqrt(13))
	0x1F83D9ABFB41BD6B, // frac(sqrt(17))
	0x5BE0CD19137E2179, // frac(sqrt(19))
}

// ---- Round Constants (FIPS 180-4 section 4.2.3) ----
//
// 80 constants from the fractional parts of the cube roots of the first 80
// primes (2..409), truncated to 64 bits.
var kConst = [80]uint64{
	0x428A2F98D728AE22, 0x7137449123EF65CD, 0xB5C0FBCFEC4D3B2F, 0xE9B5DBA58189DBBC,
	0x3956C25BF348B538, 0x59F111F1B605D019, 0x923F82A4AF194F9B, 0xAB1C5ED5DA6D8118,
	0xD807AA98A3030242, 0x12835B0145706FBE, 0x243185BE4EE4B28C, 0x550C7DC3D5FFB4E2,
	0x72BE5D74F27B896F, 0x80DEB1FE3B1696B1, 0x9BDC06A725C71235, 0xC19BF174CF692694,
	0xE49B69C19EF14AD2, 0xEFBE4786384F25E3, 0x0FC19DC68B8CD5B5, 0x240CA1CC77AC9C65,
	0x2DE92C6F592B0275, 0x4A7484AA6EA6E483, 0x5CB0A9DCBD41FBD4, 0x76F988DA831153B5,
	0x983E5152EE66DFAB, 0xA831C66D2DB43210, 0xB00327C898FB213F, 0xBF597FC7BEEF0EE4,
	0xC6E00BF33DA88FC2, 0xD5A79147930AA725, 0x06CA6351E003826F, 0x142929670A0E6E70,
	0x27B70A8546D22FFC, 0x2E1B21385C26C926, 0x4D2C6DFC5AC42AED, 0x53380D139D95B3DF,
	0x650A73548BAF63DE, 0x766A0ABB3C77B2A8, 0x81C2C92E47EDAEE6, 0x92722C851482353B,
	0xA2BFE8A14CF10364, 0xA81A664BBC423001, 0xC24B8B70D0F89791, 0xC76C51A30654BE30,
	0xD192E819D6EF5218, 0xD69906245565A910, 0xF40E35855771202A, 0x106AA07032BBD1B8,
	0x19A4C116B8D2D0C8, 0x1E376C085141AB53, 0x2748774CDF8EEB99, 0x34B0BCB5E19B48A8,
	0x391C0CB3C5C95A63, 0x4ED8AA4AE3418ACB, 0x5B9CCA4F7763E373, 0x682E6FF3D6B2B8A3,
	0x748F82EE5DEFB2FC, 0x78A5636F43172F60, 0x84C87814A1F0AB72, 0x8CC702081A6439EC,
	0x90BEFFFA23631E28, 0xA4506CEBDE82BDE9, 0xBEF9A3F7B2C67915, 0xC67178F2E372532B,
	0xCA273ECEEA26619C, 0xD186B8C721C0C207, 0xEADA7DD6CDE0EB1E, 0xF57D4F7FEE6ED178,
	0x06F067AA72176FBA, 0x0A637DC5A2C898A6, 0x113F9804BEF90DAE, 0x1B710B35131C471B,
	0x28DB77F523047D84, 0x32CAAB7B40C72493, 0x3C9EBE0A15C9BEBC, 0x431D67C49C100D4C,
	0x4CC5D4BECB3E42B6, 0x597F299CFC657E2A, 0x5FCB6FAB3AD6FAEC, 0x6C44198C4A475817,
}

// ---- Bitwise Helpers ----
//
// SHA-512 uses right-rotations (ROTR) and right-shifts (SHR) on 64-bit words.
// Go's uint64 handles the wrapping naturally -- no masking needed.

// rotr64 rotates x right by n bit positions. Bits that fall off the right
// end wrap around to the left side.
func rotr64(n uint, x uint64) uint64 {
	return (x >> n) | (x << (64 - n))
}

// ---- Sigma Functions ----
//
// Four mixing functions that create avalanche by XOR-ing multiple rotated and
// shifted copies of the same word.
//
// Big-sigma (upper-case) mix working variables during compression.
// Small-sigma (lower-case) mix words during message schedule expansion.

func bigSigma0(x uint64) uint64 {
	return rotr64(28, x) ^ rotr64(34, x) ^ rotr64(39, x)
}

func bigSigma1(x uint64) uint64 {
	return rotr64(14, x) ^ rotr64(18, x) ^ rotr64(41, x)
}

func smallSigma0(x uint64) uint64 {
	return rotr64(1, x) ^ rotr64(8, x) ^ (x >> 7)
}

func smallSigma1(x uint64) uint64 {
	return rotr64(19, x) ^ rotr64(61, x) ^ (x >> 6)
}

// ---- Logical Functions ----
//
// Ch(x,y,z) = "Choice": for each bit, x chooses between y and z.
// Maj(x,y,z) = "Majority": output is 1 if at least 2 of 3 inputs are 1.

func ch(x, y, z uint64) uint64 {
	return (x & y) ^ (^x & z)
}

func maj(x, y, z uint64) uint64 {
	return (x & y) ^ (x & z) ^ (y & z)
}

// ---- Padding ----
//
// Extends the message to a multiple of 128 bytes (1024 bits):
//  1. Append 0x80.
//  2. Append zeros until length == 112 (mod 128).
//  3. Append 128-bit big-endian length (upper 64 bits = 0 for practical sizes).
func pad(data []byte) []byte {
	bitLen := uint64(len(data)) * 8
	padded := append([]byte{}, data...)
	padded = append(padded, 0x80)
	for len(padded)%128 != 112 {
		padded = append(padded, 0x00)
	}
	// 128-bit length: upper 64 bits = 0, lower 64 bits = bit length
	var lenBuf [16]byte
	binary.BigEndian.PutUint64(lenBuf[0:8], 0)
	binary.BigEndian.PutUint64(lenBuf[8:16], bitLen)
	padded = append(padded, lenBuf[:]...)
	return padded
}

// ---- Message Schedule ----
//
// Each 128-byte block is parsed as 16 big-endian uint64 words, then expanded
// to 80 words:
//
//	W[t] = sigma1(W[t-2]) + W[t-7] + sigma0(W[t-15]) + W[t-16]   for t >= 16
func schedule(block []byte) [80]uint64 {
	var W [80]uint64
	for i := 0; i < 16; i++ {
		W[i] = binary.BigEndian.Uint64(block[i*8 : i*8+8])
	}
	for i := 16; i < 80; i++ {
		W[i] = smallSigma1(W[i-2]) + W[i-7] + smallSigma0(W[i-15]) + W[i-16]
	}
	return W
}

// ---- Compression Function ----
//
// 80 rounds of mixing fold one 128-byte block into the eight-word state.
//
// Each round:
//
//	T1 = h + Sigma1(e) + Ch(e,f,g) + K[t] + W[t]
//	T2 = Sigma0(a) + Maj(a,b,c)
//	shift: h=g, g=f, f=e, e=d+T1, d=c, c=b, b=a, a=T1+T2
//
// Davies-Meyer feed-forward adds compressed output back to input state.
func compress(state [8]uint64, block []byte) [8]uint64 {
	W := schedule(block)
	a, b, c, d, e, f, g, h := state[0], state[1], state[2], state[3],
		state[4], state[5], state[6], state[7]

	for t := 0; t < 80; t++ {
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

	return [8]uint64{
		state[0] + a, state[1] + b, state[2] + c, state[3] + d,
		state[4] + e, state[5] + f, state[6] + g, state[7] + h,
	}
}

// ---- Public API ----

// Sum512 computes the SHA-512 digest of data and returns it as a [64]byte array.
//
//	digest := sha512.Sum512([]byte("abc"))
//	fmt.Println(hex.EncodeToString(digest[:]))
func Sum512(data []byte) [64]byte {
	result, _ := StartNew[[64]byte]("sha512.Sum512", [64]byte{},
		func(op *Operation[[64]byte], rf *ResultFactory[[64]byte]) *OperationResult[[64]byte] {
			op.AddProperty("dataLen", len(data))
			padded := pad(data)
			state := initState
			for i := 0; i < len(padded); i += 128 {
				state = compress(state, padded[i:i+128])
			}
			var digest [64]byte
			for i, w := range state {
				binary.BigEndian.PutUint64(digest[i*8:], w)
			}
			return rf.Generate(true, false, digest)
		}).GetResult()
	return result
}

// HexString computes SHA-512 and returns the 128-character lowercase hex string.
//
//	sha512.HexString([]byte("abc")) -> "ddaf35a193617aba..."
func HexString(data []byte) string {
	result, _ := StartNew[string]("sha512.HexString", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			op.AddProperty("dataLen", len(data))
			digest := Sum512(data)
			return rf.Generate(true, false, hex.EncodeToString(digest[:]))
		}).GetResult()
	return result
}

// Digest is a streaming SHA-512 hasher that accepts data in multiple chunks.
//
// Useful when the full message is not available at once (large files, streams).
//
//	h := sha512.New()
//	h.Write([]byte("ab"))
//	h.Write([]byte("c"))
//	fmt.Println(h.HexDigest())
type Digest struct {
	state     [8]uint64
	buf       []byte
	byteCount uint64
}

// New returns a new streaming SHA-512 Digest initialized with the starting constants.
func New() *Digest {
	result, _ := StartNew[*Digest]("sha512.New", nil,
		func(op *Operation[*Digest], rf *ResultFactory[*Digest]) *OperationResult[*Digest] {
			return rf.Generate(true, false, &Digest{state: initState})
		}).GetResult()
	return result
}

// Write feeds more bytes into the hash computation. Always returns len(p), nil.
func (d *Digest) Write(p []byte) (int, error) {
	return StartNew[int]("sha512.Digest.Write", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("pLen", len(p))
			d.buf = append(d.buf, p...)
			d.byteCount += uint64(len(p))
			for len(d.buf) >= 128 {
				d.state = compress(d.state, d.buf[:128])
				d.buf = d.buf[128:]
			}
			return rf.Generate(true, false, len(p))
		}).GetResult()
}

// Sum512 returns the 64-byte digest of all data written so far.
//
// Non-destructive: the internal state is not modified, so you can continue
// writing after calling Sum512.
func (d *Digest) Sum512() [64]byte {
	result, _ := StartNew[[64]byte]("sha512.Digest.Sum512", [64]byte{},
		func(op *Operation[[64]byte], rf *ResultFactory[[64]byte]) *OperationResult[[64]byte] {
			bitLen := d.byteCount * 8
			tail := append([]byte{}, d.buf...)
			tail = append(tail, 0x80)
			for len(tail)%128 != 112 {
				tail = append(tail, 0x00)
			}
			var lenBuf [16]byte
			binary.BigEndian.PutUint64(lenBuf[0:8], 0)
			binary.BigEndian.PutUint64(lenBuf[8:16], bitLen)
			tail = append(tail, lenBuf[:]...)

			state := d.state
			for i := 0; i < len(tail); i += 128 {
				state = compress(state, tail[i:i+128])
			}

			var digest [64]byte
			for i, w := range state {
				binary.BigEndian.PutUint64(digest[i*8:], w)
			}
			return rf.Generate(true, false, digest)
		}).GetResult()
	return result
}

// HexDigest returns the 128-character hex string of the digest.
func (d *Digest) HexDigest() string {
	result, _ := StartNew[string]("sha512.Digest.HexDigest", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			digest := d.Sum512()
			return rf.Generate(true, false, hex.EncodeToString(digest[:]))
		}).GetResult()
	return result
}
