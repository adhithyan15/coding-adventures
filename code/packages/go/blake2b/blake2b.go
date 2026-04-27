// Package blake2b implements the BLAKE2b cryptographic hash function (RFC 7693)
// from scratch in pure Go.
//
// # What Is BLAKE2b?
//
// BLAKE2b is a modern cryptographic hash function that is both faster than MD5
// on 64-bit hardware and as secure as SHA-3 against known attacks. It was
// designed in 2012 by Aumasson, Neves, Wilcox-O'Hearn, and Winnerlein.
//
// Four features distinguish it from the SHA family shipped alongside:
//
//  1. Variable output length, 1..64 bytes.
//  2. Built-in keyed mode (single-pass MAC, replaces HMAC).
//  3. Salt and personalization parameters folded into the initial state.
//  4. ARX-only core (Add, Rotate, XOR) -- no table lookups or S-boxes.
//
// # Why This Repo Cares
//
// BLAKE2b is a prerequisite for Argon2 (memory-hard password hashing), which
// uses it both for H0 and for the BLAKE2b-long expansion that we will ship
// inside the Argon2 packages. It is also used by libsodium, WireGuard, Noise
// Protocol, and IPFS.
//
// # Key Invariant (and common bug)
//
// The last real block must be the one compressed with the final flag set. For
// message lengths that are an exact multiple of 128 bytes, do NOT add an
// empty padding block -- just flag the last real block. This streaming
// implementation enforces that invariant by holding at least one byte in the
// buffer across update() calls until digest() time.
//
// Reference: https://datatracker.ietf.org/doc/html/rfc7693
package blake2b

import (
	"encoding/binary"
	"encoding/hex"
	"fmt"
)

// BlockSize is the BLAKE2b compression block size in bytes.
const BlockSize = 128

// MaxDigestSize is the largest digest BLAKE2b can produce without XOF extension.
const MaxDigestSize = 64

// MaxKeySize is the largest key accepted in keyed mode.
const MaxKeySize = 64

// iv is the BLAKE2b initial hash values, identical to SHA-512's IVs.
// "Nothing up my sleeve": fractional parts of sqrt of the first 8 primes.
var iv = [8]uint64{
	0x6A09E667F3BCC908, // frac(sqrt(2))
	0xBB67AE8584CAA73B, // frac(sqrt(3))
	0x3C6EF372FE94F82B, // frac(sqrt(5))
	0xA54FF53A5F1D36F1, // frac(sqrt(7))
	0x510E527FADE682D1, // frac(sqrt(11))
	0x9B05688C2B3E6C1F, // frac(sqrt(13))
	0x1F83D9ABFB41BD6B, // frac(sqrt(17))
	0x5BE0CD19137E2179, // frac(sqrt(19))
}

// sigma holds the ten message-schedule permutations. Round i uses sigma[i%10];
// rounds 10 and 11 reuse sigma[0] and sigma[1].
var sigma = [10][16]int{
	{0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15},
	{14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3},
	{11, 8, 12, 0, 5, 2, 15, 13, 10, 14, 3, 6, 7, 1, 9, 4},
	{7, 9, 3, 1, 13, 12, 11, 14, 2, 6, 5, 10, 4, 0, 15, 8},
	{9, 0, 5, 7, 2, 4, 10, 15, 14, 1, 11, 12, 6, 8, 3, 13},
	{2, 12, 6, 10, 0, 11, 8, 3, 4, 13, 7, 5, 15, 14, 1, 9},
	{12, 5, 1, 15, 14, 13, 4, 10, 0, 7, 6, 3, 9, 2, 8, 11},
	{13, 11, 7, 14, 12, 1, 3, 9, 5, 0, 15, 4, 8, 6, 2, 10},
	{6, 15, 14, 9, 11, 3, 0, 8, 12, 2, 13, 7, 1, 4, 10, 5},
	{10, 2, 8, 4, 7, 6, 1, 5, 15, 11, 9, 14, 3, 12, 13, 0},
}

// rotr64 performs a circular right shift of x by n bits within a 64-bit word.
func rotr64(x uint64, n uint) uint64 {
	return (x >> n) | (x << (64 - n))
}

// mix is the BLAKE2b G quarter-round: mutate v[a], v[b], v[c], v[d] using
// message words x and y through ARX steps with rotation constants
// (32, 24, 16, 63).
func mix(v *[16]uint64, a, b, c, d int, x, y uint64) {
	v[a] = v[a] + v[b] + x
	v[d] = rotr64(v[d]^v[a], 32)
	v[c] = v[c] + v[d]
	v[b] = rotr64(v[b]^v[c], 24)
	v[a] = v[a] + v[b] + y
	v[d] = rotr64(v[d]^v[a], 16)
	v[c] = v[c] + v[d]
	v[b] = rotr64(v[b]^v[c], 63)
}

// compress mixes one 128-byte block into the 8-word state.
//
// t is the total byte count fed into the hash so far (including this block's
// bytes; counters are advanced before compress is called). final=true triggers
// the v[14] inversion that makes the final compression distinguishable from
// any intermediate one, defeating length-extension at the construction level.
func compress(h *[8]uint64, block []byte, t uint64, final bool) {
	var m [16]uint64
	for i := 0; i < 16; i++ {
		m[i] = binary.LittleEndian.Uint64(block[i*8 : (i+1)*8])
	}

	var v [16]uint64
	copy(v[:8], h[:])
	copy(v[8:], iv[:])

	// Fold the (128-bit) byte counter into v[12..13]. Messages >2^64 bytes
	// are not supported here; v[13] stays zero for any realistic input.
	v[12] ^= t
	// v[13] ^= 0  (upper 64 bits of counter are always 0 for our sizes)

	if final {
		v[14] ^= 0xFFFFFFFFFFFFFFFF
	}

	// 12 rounds, each a "column then diagonal" pair like ChaCha20's double round.
	for i := 0; i < 12; i++ {
		s := sigma[i%10]
		mix(&v, 0, 4, 8, 12, m[s[0]], m[s[1]])
		mix(&v, 1, 5, 9, 13, m[s[2]], m[s[3]])
		mix(&v, 2, 6, 10, 14, m[s[4]], m[s[5]])
		mix(&v, 3, 7, 11, 15, m[s[6]], m[s[7]])
		mix(&v, 0, 5, 10, 15, m[s[8]], m[s[9]])
		mix(&v, 1, 6, 11, 12, m[s[10]], m[s[11]])
		mix(&v, 2, 7, 8, 13, m[s[12]], m[s[13]])
		mix(&v, 3, 4, 9, 14, m[s[14]], m[s[15]])
	}

	// Davies-Meyer feed-forward: XOR both halves of v back into the state.
	for i := 0; i < 8; i++ {
		h[i] ^= v[i] ^ v[i+8]
	}
}

// initialState builds the IV-XOR-parameter-block starting state.
func initialState(digestSize, keyLen int, salt, personal []byte) [8]uint64 {
	var p [64]byte
	p[0] = byte(digestSize)
	p[1] = byte(keyLen)
	p[2] = 1 // fanout = 1 (sequential)
	p[3] = 1 // depth  = 1 (sequential)
	// bytes 4..31 stay zero for sequential mode
	if len(salt) > 0 {
		copy(p[32:48], salt)
	}
	if len(personal) > 0 {
		copy(p[48:64], personal)
	}

	var h [8]uint64
	for i := 0; i < 8; i++ {
		h[i] = iv[i] ^ binary.LittleEndian.Uint64(p[i*8:(i+1)*8])
	}
	return h
}

// Hasher is a streaming BLAKE2b hasher.
//
// Usage:
//
//	h, _ := New(32, nil, nil, nil)
//	h.Update([]byte("hello "))
//	h.Update([]byte("world"))
//	sum := h.Digest()   // 32 bytes
//
// Digest() is non-destructive: repeated calls return the same bytes and the
// hasher remains usable for further Update() calls.
type Hasher struct {
	state      [8]uint64
	buffer     []byte // always held strictly below BlockSize after Update
	byteCount  uint64 // total bytes fed through compress (excludes unflushed)
	digestSize int
}

// New creates a new streaming BLAKE2b hasher.
//
// digestSize must be in [1, 64]. key, salt, and personal are optional:
//   - key may be empty or up to 64 bytes.
//   - salt must be empty or exactly 16 bytes.
//   - personal must be empty or exactly 16 bytes.
func New(digestSize int, key, salt, personal []byte) (*Hasher, error) {
	if digestSize < 1 || digestSize > MaxDigestSize {
		return nil, fmt.Errorf("blake2b: digest size must be in [1, 64], got %d", digestSize)
	}
	if len(key) > MaxKeySize {
		return nil, fmt.Errorf("blake2b: key length must be in [0, 64], got %d", len(key))
	}
	if len(salt) != 0 && len(salt) != 16 {
		return nil, fmt.Errorf("blake2b: salt must be exactly 16 bytes (or empty), got %d", len(salt))
	}
	if len(personal) != 0 && len(personal) != 16 {
		return nil, fmt.Errorf("blake2b: personal must be exactly 16 bytes (or empty), got %d", len(personal))
	}

	h := &Hasher{
		state:      initialState(digestSize, len(key), salt, personal),
		digestSize: digestSize,
	}
	if len(key) > 0 {
		// Keyed mode: prepend the key, zero-padded to a full block.
		keyBlock := make([]byte, BlockSize)
		copy(keyBlock, key)
		h.buffer = keyBlock
	}
	return h, nil
}

// Update feeds more bytes into the hash. It compresses any full blocks it can
// prove are not the last one -- i.e., it only flushes when more than one full
// block of data is buffered. That guarantees a non-empty final block at
// Digest() time, which is the block that must be flagged final.
func (h *Hasher) Update(data []byte) *Hasher {
	h.buffer = append(h.buffer, data...)
	for len(h.buffer) > BlockSize {
		h.byteCount += BlockSize
		compress(&h.state, h.buffer[:BlockSize], h.byteCount, false)
		h.buffer = h.buffer[BlockSize:]
	}
	return h
}

// Digest returns the hash of all bytes fed so far. It does not modify the
// hasher: repeated calls return the same bytes.
func (h *Hasher) Digest() []byte {
	// Work on a copy so repeated digest calls produce identical output.
	state := h.state
	final := make([]byte, BlockSize)
	copy(final, h.buffer)
	byteCount := h.byteCount + uint64(len(h.buffer))
	compress(&state, final, byteCount, true)

	out := make([]byte, 8*8)
	for i := 0; i < 8; i++ {
		binary.LittleEndian.PutUint64(out[i*8:(i+1)*8], state[i])
	}
	return out[:h.digestSize]
}

// HexDigest returns the hash as a lowercase hex string.
func (h *Hasher) HexDigest() string {
	return hex.EncodeToString(h.Digest())
}

// Copy returns a deep copy of the hasher suitable for hashing messages that
// share a common prefix.
func (h *Hasher) Copy() *Hasher {
	clone := &Hasher{
		state:      h.state,
		byteCount:  h.byteCount,
		digestSize: h.digestSize,
	}
	clone.buffer = append([]byte(nil), h.buffer...)
	return clone
}

// Sum is the one-shot BLAKE2b hash. It is equivalent to:
//
//	h, _ := New(digestSize, key, salt, personal)
//	h.Update(data)
//	sum := h.Digest()
func Sum(data []byte, digestSize int, key, salt, personal []byte) ([]byte, error) {
	h, err := New(digestSize, key, salt, personal)
	if err != nil {
		return nil, err
	}
	h.Update(data)
	return h.Digest(), nil
}

// SumHex is Sum's hex-encoded counterpart.
func SumHex(data []byte, digestSize int, key, salt, personal []byte) (string, error) {
	out, err := Sum(data, digestSize, key, salt, personal)
	if err != nil {
		return "", err
	}
	return hex.EncodeToString(out), nil
}
