// Package md5 provides the MD5 message digest algorithm (RFC 1321) implemented
// entirely from scratch, without using Go's standard library crypto/md5.
//
// # What Is MD5?
//
// MD5 (Message Digest 5) takes any sequence of bytes and produces a fixed-size
// 16-byte (128-bit) "fingerprint" called a digest. The same input always
// produces the same digest. Change even one bit of input and the digest changes
// completely — this is called the avalanche effect.
//
// Ron Rivest designed MD5 in 1991 as an improvement over MD4. It was
// standardized in RFC 1321. MD5 is cryptographically broken: collision attacks
// have been practical since 2004. Do NOT use it for passwords, digital
// signatures, or TLS certificates. It remains valid for: non-security
// checksums, UUID v3, and legacy systems that already use it.
//
// # How MD5 Differs From SHA-1
//
// The single most important difference is byte order:
//
//	Property     SHA-1       MD5
//	──────────   ─────────   ─────────────
//	Output size  20 bytes    16 bytes
//	State words  5 (H₀..H₄)  4 (A,B,C,D)
//	Rounds       80          64
//	Block size   512 bits    512 bits
//	Word order   Big-endian  LITTLE-ENDIAN ← critical difference
//
// Big-endian (SHA-1): most significant byte first.
//
//	0x0A0B0C0D → 0A 0B 0C 0D
//
// Little-endian (MD5): LEAST significant byte first.
//
//	0x0A0B0C0D → 0D 0C 0B 0A
//
// This is the #1 source of MD5 implementation bugs. Concretely:
//   - SHA-1 reads block words with big-endian uint32
//   - MD5 reads block words with binary.LittleEndian.Uint32
//   - SHA-1 writes the final hash with big-endian uint32
//   - MD5 writes the final hash with binary.LittleEndian.PutUint32
//
// # The T-Table (64 Precomputed Constants)
//
// MD5 uses 64 constants T[0..63], one per round. Each is derived from the sine
// function — a transcendental number with unpredictable bit patterns, ensuring
// no hidden mathematical backdoor. These are called "nothing up my sleeve"
// numbers: anyone can verify them independently.
//
//	T[i] = floor(abs(sin(i+1)) × 2³²)   for i = 0..63
//
// Why sine? Because sin(n) for integer n produces pseudo-random values between
// -1 and 1. Scaling by 2³² and flooring gives a 32-bit integer. The pattern is
// derived from a well-known mathematical function, which proves the constants
// were not secretly chosen to weaken the algorithm.
//
// Example derivation:
//
//	sin(1) ≈ 0.8414709848...
//	abs(sin(1)) × 2³² = 0.8414709848 × 4294967296 ≈ 3614090360.02
//	floor(3614090360.02) = 3614090360 = 0xD76AA478 = T[0]
//
// # RFC 1321 Test Vectors
//
//	md5("")              → "d41d8cd98f00b204e9800998ecf8427e"
//	md5("a")             → "0cc175b9c0f1b6a831c399e269772661"
//	md5("abc")           → "900150983cd24fb0d6963f7d28e17f72"
//	md5("message digest") → "f96b697d7cb7938d525a2f31aaf161d0"
package ca_md5

import (
	"encoding/binary"
	"fmt"
	"math"
)

// ── T-Table: 64 Constants Derived From Sine ────────────────────────────────
//
// T[i] = floor(abs(sin(i+1)) × 2³²)  for i in 0..63
//
// These are computed once at package init time. We index from 0 internally
// (our T[i] corresponds to T[i+1] in RFC 1321 notation, since the RFC uses
// 1-based indexing).
//
// Using math.Sin from the standard library is acceptable here — we are
// implementing the MD5 algorithm from scratch, not the sine function.

var tTable [64]uint32

func init() {
	// Pre-compute all 64 sine-derived constants.
	// math.Sin(i+1) for i=0..63 gives us sin(1)..sin(64).
	// math.Abs ensures we handle the negative values of sine correctly.
	for i := 0; i < 64; i++ {
		tTable[i] = uint32(uint64(math.Abs(math.Sin(float64(i+1))) * (1 << 32)))
	}
}

// ── Round Shift Amounts ─────────────────────────────────────────────────────
//
// Each of the 64 rounds has a specific left-rotation amount. These are arranged
// in four groups of 16. The pattern repeats within each group:
//
//	Group 1 (rounds  0-15): [7,12,17,22] repeated 4×
//	Group 2 (rounds 16-31): [5, 9,14,20] repeated 4×
//	Group 3 (rounds 32-47): [4,11,16,23] repeated 4×
//	Group 4 (rounds 48-63): [6,10,15,21] repeated 4×
//
// The RFC provides these as fixed values — they were chosen empirically for
// good bit diffusion across the 32-bit state words.

var s = [64]uint32{
	7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, // rounds 0–15
	5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20, // rounds 16–31
	4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, // rounds 32–47
	6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21, // rounds 48–63
}

// ── Initialization Constants ────────────────────────────────────────────────
//
// The four 32-bit words that prime the MD5 state machine. These are also
// "nothing up my sleeve" numbers. Reading the hex digits in sequence gives a
// simple counting pattern:
//
//	0x67452301 → bytes [67 45 23 01] → reversed nibble pairs: 01 23 45 67
//	0xEFCDAB89 → bytes [EF CD AB 89] → reversed nibble pairs: 89 AB CD EF
//	0x98BADCFE → bytes [98 BA DC FE] → reversed nibble pairs: FE DC BA 98
//	0x10325476 → bytes [10 32 54 76] → reversed nibble pairs: 76 54 32 10

const (
	initA uint32 = 0x67452301
	initB uint32 = 0xEFCDAB89
	initC uint32 = 0x98BADCFE
	initD uint32 = 0x10325476
)

// ── Helper: Circular Left Rotation ─────────────────────────────────────────
//
// rotl32 rotates the bits of x left by n positions within a 32-bit word.
// Bits that fall off the left reappear on the right.
//
// Visualization with a 4-bit word (for clarity):
//
//	x    = 1011  (n=1 left-rotate)
//	→      0111  (shifted left, leftmost bit wraps to right)
//
// For 32 bits: rotl32(n, x) = (x << n) | (x >> (32-n))
//
// The & 0xFFFFFFFF mask is not strictly necessary in Go because uint32
// arithmetic automatically wraps at 32 bits, but it makes the intent explicit.

func rotl32(n, x uint32) uint32 {
	return (x << n) | (x >> (32 - n))
}

// ── Padding ─────────────────────────────────────────────────────────────────
//
// MD5 processes data in 64-byte (512-bit) blocks. If the message is not a
// multiple of 64 bytes, we pad it:
//
//  1. Append a single 0x80 byte (binary: 10000000). This marks the end of the
//     real message.
//  2. Append zero bytes until the total length ≡ 56 (mod 64). This leaves
//     exactly 8 bytes free at the end of the last block.
//  3. Append the original message length in BITS as a 64-bit LITTLE-ENDIAN
//     integer. This uses those 8 bytes from step 2.
//
// The little-endian length is the critical difference from SHA-1's big-endian
// length. It is easy to get wrong.
//
// Example — "abc" (3 bytes = 24 bits):
//
//	61 62 63                   ← "abc"
//	80                         ← end-of-message marker
//	00 00 ... 00               ← 52 zero bytes (to reach 56 mod 64)
//	18 00 00 00 00 00 00 00    ← 24 as 64-bit little-endian (0x18 = 24)
//
// Total: 64 bytes exactly — one block.
//
// Boundary cases to think about:
//   - 55-byte message: append 0x80 → 56 bytes, no extra zeros, append length
//     → 64 bytes (one block)
//   - 56-byte message: append 0x80 → 57 bytes, need 64 more zeros to reach 120
//     bytes ≡ 56 (mod 64), then append length → 128 bytes (two blocks)
//   - 64-byte message: append 0x80 → 65 bytes, need 55 zeros → 120 bytes,
//     append length → 128 bytes (two blocks)

func pad(data []byte) []byte {
	bitLen := uint64(len(data)) * 8

	// Start with the message, then the 0x80 marker.
	msg := make([]byte, len(data)+1)
	copy(msg, data)
	msg[len(data)] = 0x80

	// Append zeros until len ≡ 56 (mod 64).
	for len(msg)%64 != 56 {
		msg = append(msg, 0x00)
	}

	// Append the 64-bit bit-length in LITTLE-ENDIAN byte order.
	// binary.LittleEndian.PutUint64 writes the least significant byte first.
	var lenBuf [8]byte
	binary.LittleEndian.PutUint64(lenBuf[:], bitLen)
	msg = append(msg, lenBuf[:]...)

	return msg
}

// ── Compression Function ────────────────────────────────────────────────────
//
// compress folds one 64-byte block into the four-word state via 64 rounds of
// mixing. This is the core of MD5.
//
// The block is first parsed as 16 LITTLE-ENDIAN 32-bit words M[0..15]:
//
//	Block bytes: [b0 b1 b2 b3 | b4 b5 b6 b7 | ...]
//	M[0] = b0 | (b1<<8) | (b2<<16) | (b3<<24)   ← little-endian
//	M[1] = b4 | (b5<<8) | (b6<<16) | (b7<<24)
//	...
//
// Four stages of 16 rounds each, differing in their auxiliary function and
// message-word selection index g:
//
//	Stage  Rounds  f(B,C,D)              g (word index)    Purpose
//	─────  ──────  ────────────────────  ────────────────  ─────────────────
//	  1    0–15    (B&C)|(^B&D)          i                 Selector
//	  2    16–31   (D&B)|(^D&C)          (5i+1) mod 16     Selector (swapped)
//	  3    32–47   B^C^D                 (3i+5) mod 16     Parity
//	  4    48–63   C^(B|^D)              (7i)   mod 16     "I" function
//
// Stage 1, the F function (B&C)|(^B&D): acts as a multiplexer.
//   - When B=1: result = C (^B=0, second term drops out)
//   - When B=0: result = D (first term drops out, ^B=1)
//   - Truth table:
//     B C D | F
//     ──────────
//     0 0 0 | 0
//     0 0 1 | 1
//     0 1 0 | 0
//     0 1 1 | 1
//     1 0 0 | 0
//     1 0 1 | 0
//     1 1 0 | 1
//     1 1 1 | 1
//
// Stage 4, the I function C^(B|^D): the most unusual auxiliary function.
//   - When D=0 → ^D=1 → B|^D=1 → result = C^1 (flips C regardless of B)
//   - When D=1 → ^D=0 → B|^D=B → result = C^B (parity of B and C)
//   This extra asymmetry between D=0 and D=1 cases increases diffusion.
//
// Each round updates the state as follows:
//
//	f    = auxiliary(B, C, D) according to stage
//	temp = B + rotl32(s[i], A + f + M[g] + T[i])   (all mod 2³²)
//	A, B, C, D = D, temp, B, C
//
// The role rotation (A←D, B←new, C←B, D←old_C) means each word gets updated
// roughly once every four rounds, cascading changes through all four words.
//
// Davies-Meyer feed-forward: after all 64 rounds, add the compressed values
// back to the original state words. This prevents an attacker from inverting
// the compression function even if they know the round function.
//
//	finalA = (a + a0) mod 2³²
//	finalB = (b + b0) mod 2³²
//	finalC = (c + c0) mod 2³²
//	finalD = (d + d0) mod 2³²

func compress(state [4]uint32, block []byte) [4]uint32 {
	// Parse the 64-byte block as 16 little-endian 32-bit words.
	var m [16]uint32
	for i := 0; i < 16; i++ {
		m[i] = binary.LittleEndian.Uint32(block[i*4:])
	}

	// Save the input state for the Davies-Meyer feed-forward at the end.
	a0, b0, c0, d0 := state[0], state[1], state[2], state[3]
	a, b, c, d := a0, b0, c0, d0

	for i := uint32(0); i < 64; i++ {
		var f, g uint32
		switch {
		case i < 16:
			// Stage 1 — F function: selector, output = C if B=1, else D.
			f = (b & c) | (^b & d)
			g = i
		case i < 32:
			// Stage 2 — G function: same selector but B and D roles swapped.
			f = (d & b) | (^d & c)
			g = (5*i + 1) % 16
		case i < 48:
			// Stage 3 — H function: parity, set when an odd number of inputs are 1.
			f = b ^ c ^ d
			g = (3*i + 5) % 16
		default:
			// Stage 4 — I function: C^(B|^D), asymmetric on D.
			f = c ^ (b | ^d)
			g = (7 * i) % 16
		}

		// Core round update:
		//   temp = B + ROTL(s[i],  A + f + M[g] + T[i])
		//   (A, B, C, D) ← (D, temp, B, C)
		//
		// All arithmetic is uint32, so overflow wraps automatically — that is
		// exactly the mod 2³² behavior we need.
		temp := b + rotl32(s[i], a+f+m[g]+tTable[i])
		a, b, c, d = d, temp, b, c
	}

	// Davies-Meyer: add the original state back in to prevent inversion.
	return [4]uint32{
		a + a0,
		b + b0,
		c + c0,
		d + d0,
	}
}

// ── One-Shot API ────────────────────────────────────────────────────────────

// SumMD5 computes the MD5 digest of data and returns it as a 16-byte array.
//
// This is the one-shot API: call it once with the complete message.
//
// NOTE: MD5 is cryptographically broken. Do NOT use for passwords, digital
// signatures, or security-sensitive checksums. Use for UUID v3, non-security
// checksums, or legacy compatibility only.
//
// Example:
//
//	digest := SumMD5([]byte("abc"))
//	// digest == [16]byte{0x90, 0x01, 0x50, 0x98, 0x3c, 0xd2, 0x4f, 0xb0,
//	//                    0xd6, 0x96, 0x3f, 0x7d, 0x28, 0xe1, 0x7f, 0x72}
func SumMD5(data []byte) [16]byte {
	// Pad the data to a multiple of 64 bytes, then process each 64-byte block.
	padded := pad(data)
	state := [4]uint32{initA, initB, initC, initD}
	for i := 0; i < len(padded); i += 64 {
		state = compress(state, padded[i:i+64])
	}

	// Serialize the four 32-bit state words as LITTLE-ENDIAN bytes.
	// The little-endian encoding is what makes MD5 digests look "backwards"
	// compared to SHA-1 or SHA-256.
	var out [16]byte
	binary.LittleEndian.PutUint32(out[0:], state[0])
	binary.LittleEndian.PutUint32(out[4:], state[1])
	binary.LittleEndian.PutUint32(out[8:], state[2])
	binary.LittleEndian.PutUint32(out[12:], state[3])
	return out
}

// HexString computes the MD5 digest and returns it as a 32-character lowercase
// hexadecimal string. This is the format most commonly used for displaying MD5
// checksums.
//
// Example:
//
//	s := HexString([]byte("abc"))
//	// s == "900150983cd24fb0d6963f7d28e17f72"
func HexString(data []byte) string {
	d := SumMD5(data)
	return fmt.Sprintf("%x", d[:])
}

// ── Streaming API ───────────────────────────────────────────────────────────
//
// Digest is a streaming MD5 hasher that accepts data in multiple chunks.
// This is useful when the full message is not available at once — for example,
// when reading a large file in chunks, or when hashing data arriving over a
// network connection.
//
// The streaming API maintains the same state as if all the data had been fed
// at once. Multiple Write calls are equivalent to a single SumMD5 of all the
// concatenated data.
//
// Digest implements io.Writer, so it can be used anywhere an io.Writer is
// accepted (e.g., io.Copy from a file).
//
// Internal structure:
//
//	state     — the four running state words (A, B, C, D)
//	buffer    — incomplete block bytes (0..63 bytes)
//	byteCount — total bytes written so far (used for padding)
//
// The buffer holds data that has not yet formed a complete 64-byte block.
// When a Write call pushes the buffer to 64+ bytes, we compress complete
// blocks immediately and keep the remainder in the buffer.
//
// When finalizing (SumMD5/HexDigest), we apply padding to the buffered
// tail without modifying the state, so the hasher can be reused.

// Digest is a streaming MD5 hasher.
type Digest struct {
	state     [4]uint32
	buffer    []byte
	byteCount uint64
}

// New creates a new MD5 streaming hasher initialized to the MD5 start state.
//
// Example:
//
//	d := md5.New()
//	d.Write([]byte("ab"))
//	d.Write([]byte("c"))
//	fmt.Println(d.HexDigest()) // "900150983cd24fb0d6963f7d28e17f72"
func New() *Digest {
	return &Digest{
		state: [4]uint32{initA, initB, initC, initD},
	}
}

// Write feeds p into the hasher. It implements io.Writer.
//
// Write may be called any number of times. The order of calls matters
// (Write(a) then Write(b) hashes a concatenated with b, same as Write(ab)).
// Write never returns an error.
func (d *Digest) Write(p []byte) (int, error) {
	d.buffer = append(d.buffer, p...)
	d.byteCount += uint64(len(p))

	// Process complete 64-byte blocks eagerly.
	// We only keep incomplete block data in the buffer.
	for len(d.buffer) >= 64 {
		d.state = compress(d.state, d.buffer[:64])
		d.buffer = d.buffer[64:]
	}

	return len(p), nil
}

// SumMD5 returns the MD5 digest of all data written so far as a 16-byte array.
//
// This is non-destructive: calling SumMD5 multiple times returns the same
// result, and subsequent Write calls still work correctly. The internal state
// is not modified.
//
// Internally, we copy the current state, apply padding to the buffered tail,
// process the padded tail blocks, then serialize the result — all without
// touching d.state or d.buffer.
func (d *Digest) SumMD5() [16]byte {
	// Build the padded tail from the buffered bytes and the byte count.
	// We do NOT modify d.state here — this is a snapshot operation.
	bitLen := d.byteCount * 8

	// Start with the buffered bytes, append the 0x80 end-marker.
	tail := make([]byte, len(d.buffer)+1)
	copy(tail, d.buffer)
	tail[len(d.buffer)] = 0x80

	// Pad to ≡ 56 (mod 64).
	for len(tail)%64 != 56 {
		tail = append(tail, 0x00)
	}

	// Append the 64-bit bit-length in little-endian.
	var lenBuf [8]byte
	binary.LittleEndian.PutUint64(lenBuf[:], bitLen)
	tail = append(tail, lenBuf[:]...)

	// Run the compression over the tail blocks using a copy of the state.
	state := d.state
	for i := 0; i < len(tail); i += 64 {
		state = compress(state, tail[i:i+64])
	}

	// Serialize as little-endian.
	var out [16]byte
	binary.LittleEndian.PutUint32(out[0:], state[0])
	binary.LittleEndian.PutUint32(out[4:], state[1])
	binary.LittleEndian.PutUint32(out[8:], state[2])
	binary.LittleEndian.PutUint32(out[12:], state[3])
	return out
}

// HexDigest returns the MD5 digest as a 32-character lowercase hex string.
//
// Like SumMD5, this is non-destructive.
func (d *Digest) HexDigest() string {
	digest := d.SumMD5()
	return fmt.Sprintf("%x", digest[:])
}
