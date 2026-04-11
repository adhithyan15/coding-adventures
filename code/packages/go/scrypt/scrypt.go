// Package scrypt implements the scrypt password-based key derivation function
// as specified in RFC 7914 (Colin Percival, 2009).
//
// # What Is scrypt?
//
// scrypt is a deliberately memory-hard key derivation function. Its goal is
// to make brute-force attacks expensive in both CPU time AND memory, forcing
// attackers to use large amounts of RAM — a resource that is harder to
// parallelize cheaply on ASICs or FPGAs than pure computation.
//
// Compare to PBKDF2: PBKDF2 iterates a hash many times, but each iteration
// requires only a few kilobytes of state, making it highly parallelisable on
// GPUs. scrypt fills a large random-access table (V) and then revisits entries
// pseudo-randomly, so reducing memory means more I/O — memory and sequential
// computation trade directly.
//
// # Where Is scrypt Used?
//
//   - Litecoin and many other cryptocurrencies use scrypt as a proof-of-work
//   - Password managers (e.g., tarsnap by the algorithm's author)
//   - Operating system credential storage
//
// # The Three Layers of scrypt (RFC 7914 §3)
//
//  1. PBKDF2-HMAC-SHA256 (1 iteration) expands the password+salt into a
//     p×128r-byte block B.
//
//  2. ROMix (§5) applies sequential-memory-hard mixing to each 128r-byte
//     sub-block of B.  ROMix fills a table V of N pseudo-random blocks then
//     makes N random-access lookups driven by Integerify(X).  N is the memory
//     parameter — larger N ⇒ more RAM required.
//
//  3. PBKDF2-HMAC-SHA256 (1 iteration again) condenses the mixed B back into
//     a dkLen-byte derived key.
//
// # Parameters
//
//	N  — CPU/memory cost factor. Must be a power of 2 ≥ 2. Typical: 32768.
//	r  — Block size factor. Typical: 8.  Memory = O(N * r).
//	p  — Parallelisation factor. Typical: 1.
//	dkLen — Output key length in bytes. Typical: 32 or 64.
//
// # Memory Usage
//
// The ROMix step allocates N blocks of 2r × 64 bytes each:
//
//	memory ≈ N × 128 × r  bytes
//
// For N=32768, r=8: 32768 × 128 × 8 = 32 MiB per parallel lane (p=1).
// For N=1048576 (2²⁰), r=8: 1 GiB.
//
// # RFC 7914 Test Vectors
//
// These values are verified against Python's hashlib.scrypt,
// golang.org/x/crypto/scrypt, and OpenSSL.
//
//	Scrypt("", "", 16, 1, 1, 64)
//	→ 77d6576238657b203b19ca42c18a0497f16b4844e3074ae8dfdffa3fede21442
//	  fcd0069ded0948f8326a753a0fc81f17e8d3e0fb2e0d3628cf35e20c38d18906
//
//	Scrypt("password", "NaCl", 1024, 8, 16, 64)
//	→ fdbabe1c9d3472007856e7190d01e9fe7c6ad7cbc8237830e77376634b373162
//	  2eaf30d92e22a3886ff109279d9830dac727afb94a83ee6d8360cbdfa2cc0640
package scrypt

import (
	"encoding/binary"
	"errors"
	"fmt"

	hmacpkg "github.com/adhithyan15/coding-adventures/code/packages/go/hmac"
	gsha256 "github.com/adhithyan15/coding-adventures/code/packages/go/sha256"
)

// ===========================================================================
// Error variables
// ===========================================================================
//
// Explicit, named errors let callers distinguish configuration mistakes from
// programmer errors without string matching.

var (
	// ErrInvalidN is returned when N is not a power of 2 or is less than 2.
	// N must be a power of 2 so that `Integerify(X) mod N` can be computed
	// quickly with a bitmask (or at least avoids biased modular reduction).
	ErrInvalidN = errors.New("scrypt: N must be a power of 2 and >= 2")

	// ErrNTooLarge caps N at 2^20 to prevent accidental multi-gigabyte
	// allocations.  2^20 at r=8 already requires 1 GiB.
	ErrNTooLarge = errors.New("scrypt: N must not exceed 2^20")

	// ErrInvalidR is returned when r < 1.  r=0 would produce 0-byte blocks.
	ErrInvalidR = errors.New("scrypt: r must be a positive integer")

	// ErrInvalidP is returned when p < 1.  p=0 means no parallel lanes,
	// producing no work at all.
	ErrInvalidP = errors.New("scrypt: p must be a positive integer")

	// ErrInvalidKeyLength is returned when dkLen < 1.
	ErrInvalidKeyLength = errors.New("scrypt: dk_len must be a positive integer")

	// ErrKeyLengthTooLarge caps dkLen at 2^20 bytes to prevent huge outputs.
	ErrKeyLengthTooLarge = errors.New("scrypt: dk_len must not exceed 2^20")

	// ErrPRTooLarge is returned when p*r exceeds 2^30.
	// RFC 7914 §2 requires p * (128*r) < 2^32, so p*r < 2^25.  We use a
	// tighter cap of 2^30 to prevent int64 overflow on 32-bit systems.
	ErrPRTooLarge = errors.New("scrypt: p * r exceeds limit (2^30)")
)

// ===========================================================================
// salsa20_8 — The Salsa20/8 Core (RFC 7914 §3, §4)
// ===========================================================================
//
// Salsa20/8 is a reduced-round variant of the Salsa20 stream cipher core.
// It operates on a 64-byte (16 × uint32) block.
//
// The "8" means 8 rounds (4 double-rounds), which is half the 16 rounds of
// full Salsa20 but sufficient for mixing purposes inside BlockMix.
//
// # Data Layout
//
// The 64 input bytes are loaded as 16 little-endian uint32 words:
//
//	x[0]  x[1]  x[2]  x[3]
//	x[4]  x[5]  x[6]  x[7]
//	x[8]  x[9]  x[10] x[11]
//	x[12] x[13] x[14] x[15]
//
// # Quarter-Round (QR)
//
// The quarter-round mixes four words a, b, c, d:
//
//	b ^= ROTL(a + d, 7)
//	c ^= ROTL(b + a, 9)
//	d ^= ROTL(c + b, 13)
//	a ^= ROTL(d + c, 18)
//
// # Double-Round Structure (column then row)
//
// Column round — mixes the four "column" diagonals:
//
//	QR(0, 4, 8, 12)   QR(5, 9, 13, 1)   QR(10, 14, 2, 6)   QR(15, 3, 7, 11)
//
// Row round — mixes the four rows:
//
//	QR(0, 1, 2, 3)   QR(5, 6, 7, 4)   QR(10, 11, 8, 9)   QR(15, 12, 13, 14)
//
// # Final Step
//
// The output is: initial_state + mixing_result (word-wise uint32 addition,
// wrapping).  This "add-back" prevents the output from being trivially
// invertible, making Salsa20/8 a pseudo-random permutation.
//
// All arithmetic is uint32 wrapping — Go handles this natively for uint32.
func salsa20_8(b []byte) []byte {
	// Load 16 little-endian uint32 words from the 64-byte input.
	var x [16]uint32
	for i := 0; i < 16; i++ {
		x[i] = binary.LittleEndian.Uint32(b[i*4:])
	}
	z := x // save initial state for the final add-back

	// rotl32 rotates a 32-bit value left by n bits.
	// In Go, uint32 arithmetic wraps at 2^32 automatically.
	rotl32 := func(v uint32, n uint) uint32 {
		return (v << n) | (v >> (32 - n))
	}

	// qr applies one quarter-round to four words in the state array.
	// Parameters are indices into the xi array.
	qr := func(xi *[16]uint32, a, b, c, d int) {
		xi[b] ^= rotl32(xi[a]+xi[d], 7)
		xi[c] ^= rotl32(xi[b]+xi[a], 9)
		xi[d] ^= rotl32(xi[c]+xi[b], 13)
		xi[a] ^= rotl32(xi[d]+xi[c], 18)
	}

	// Apply 4 double-rounds (= 8 total rounds = "Salsa20/8").
	// Each double-round is one column round followed by one row round.
	for i := 0; i < 4; i++ {
		// Column rounds — mix along the 4×4 grid columns.
		// The column diagonals of the 4×4 layout are:
		//   col 0: indices  0, 4,  8, 12
		//   col 1: indices  5, 9, 13,  1
		//   col 2: indices 10,14,  2,  6
		//   col 3: indices 15, 3,  7, 11
		qr(&x, 0, 4, 8, 12)
		qr(&x, 5, 9, 13, 1)
		qr(&x, 10, 14, 2, 6)
		qr(&x, 15, 3, 7, 11)

		// Row rounds — mix along the 4×4 grid rows.
		//   row 0: indices  0, 1,  2,  3
		//   row 1: indices  5, 6,  7,  4
		//   row 2: indices 10,11,  8,  9
		//   row 3: indices 15,12, 13, 14
		qr(&x, 0, 1, 2, 3)
		qr(&x, 5, 6, 7, 4)
		qr(&x, 10, 11, 8, 9)
		qr(&x, 15, 12, 13, 14)
	}

	// Produce output: x[i] = x[i] + z[i] (wrapping uint32 addition).
	// This prevents the round function from being trivially invertible.
	out := make([]byte, 64)
	for i := 0; i < 16; i++ {
		binary.LittleEndian.PutUint32(out[i*4:], x[i]+z[i])
	}
	return out
}

// xor64 XORs 64 bytes from src1 and src2 into dst.
// Used in BlockMix to XOR the running state X with each block before hashing.
func xor64(dst, src1, src2 []byte) {
	for i := 0; i < 64; i++ {
		dst[i] = src1[i] ^ src2[i]
	}
}

// ===========================================================================
// blockMix — BlockMix (RFC 7914 §4)
// ===========================================================================
//
// BlockMix mixes a sequence of 2r 64-byte blocks (total: 128r bytes).
//
// # Algorithm
//
//	X = blocks[2r-1]      // start from the last block
//	for i = 0 to 2r-1:
//	    X = Salsa20/8(X XOR blocks[i])
//	    Y[i] = X
//
//	// Interleave: even indices go to first half, odd to second half
//	return [Y[0], Y[2], ..., Y[2r-2], Y[1], Y[3], ..., Y[2r-1]]
//
// # Why the Interleave?
//
// The interleaving ensures that the output blocks are maximally mixed —
// every output block depends on every input block through the chain of XOR
// and Salsa20/8 applications.  It also sets up the ROMix table entries in an
// order that makes cache behaviour harder to predict.
//
// # Memory Observation
//
// Each Salsa20/8 call takes O(1) memory (64 bytes).
// BlockMix takes O(r) memory for the Y array.
// The expensive memory comes from ROMix, which stores N copies of BlockMix output.
func blockMix(blocks [][]byte, r int) [][]byte {
	twoR := 2 * r

	// X starts as the last block of the input.
	x := make([]byte, 64)
	copy(x, blocks[twoR-1])

	// Y holds the 2r output blocks in processing order.
	y := make([][]byte, twoR)
	for i := 0; i < twoR; i++ {
		// X = Salsa20/8(X XOR blocks[i])
		xor64(x, x, blocks[i])
		x = salsa20_8(x)
		// Store a fresh copy — we will read x again in the next iteration.
		y[i] = append([]byte(nil), x...)
	}

	// Interleave: even-indexed Y entries into the first half,
	// odd-indexed Y entries into the second half.
	out := make([][]byte, twoR)
	for i := 0; i < r; i++ {
		out[i] = y[2*i]
		out[r+i] = y[2*i+1]
	}
	return out
}

// ===========================================================================
// roMix — ROMix (RFC 7914 §5)
// ===========================================================================
//
// ROMix is the sequential-memory-hard core of scrypt.
//
// # Why Is It Memory-Hard?
//
// Phase 1 (fill) builds a table V of N = 2^k block-mix outputs sequentially.
// Phase 2 (mix) makes N pseudo-random lookups into V, XORs into X, and
// applies BlockMix again.  Each lookup's index depends on the current state X,
// so lookups cannot be reordered — the adversary must either keep all N blocks
// in memory or recompute them from scratch on every access.
//
// # Algorithm (RFC 7914 §5)
//
//	V[0]     = X
//	V[1]     = BlockMix(V[0])
//	...
//	V[N-1]   = BlockMix(V[N-2])
//	X        = BlockMix(V[N-1])   // one more step after fill
//
//	for i = 0 to N-1:
//	    j = Integerify(X) mod N
//	    X = BlockMix(X XOR V[j])
//
//	return X
//
// # Integerify (RFC 7914 §4)
//
// Integerify reads the first 8 bytes of the last block of X as a little-endian
// uint64.  This gives a pseudo-random index into V.
//
// Parameters:
//   - bBytes: a 128*r byte slice (one parallel lane from the outer PBKDF2 output)
//   - n:      the CPU/memory cost parameter (must be a power of 2)
//   - r:      the block size parameter
func roMix(bBytes []byte, n, r int) []byte {
	twoR := 2 * r

	// Slice bBytes into 2r separate 64-byte blocks.
	// We copy each block so that ROMix does not mutate the input slice.
	blocks := make([][]byte, twoR)
	for i := 0; i < twoR; i++ {
		blocks[i] = append([]byte(nil), bBytes[i*64:(i+1)*64]...)
	}

	// ---------------------------------------------------------------------------
	// Phase 1: Fill table V
	//
	// V[0] = X (initial state)
	// V[i] = BlockMix(V[i-1])
	//
	// After this loop:
	//   V[0] = initial X
	//   V[1] = BlockMix(V[0])
	//   ...
	//   V[N-1] = BlockMix(V[N-2])
	//   x = BlockMix(V[N-1])    ← one step beyond the table
	// ---------------------------------------------------------------------------
	v := make([][][]byte, n)
	x := blocks
	for i := 0; i < n; i++ {
		v[i] = copyBlocks(x) // snapshot current X into V[i]
		x = blockMix(x, r)   // advance X by one BlockMix step
	}
	// After the loop, x = BlockMix(V[N-1]), which is the correct starting
	// state for Phase 2 per the RFC.

	// ---------------------------------------------------------------------------
	// Phase 2: Pseudo-random lookup and mix
	//
	// For each of N steps:
	//   j = Integerify(X) mod N   (pseudo-random index into V)
	//   X = BlockMix(X XOR V[j])
	//
	// Because j depends on the current X, each step's lookup depends on the
	// previous result — this enforces sequential computation.
	// ---------------------------------------------------------------------------
	for i := 0; i < n; i++ {
		j := integerify(x) % uint64(n)
		xorBlocks(x, x, v[j])
		x = blockMix(x, r)
	}

	// Flatten the 2r blocks back into a single 128r-byte slice.
	out := make([]byte, twoR*64)
	for i, blk := range x {
		copy(out[i*64:], blk)
	}
	return out
}

// copyBlocks returns a deep copy of a slice of 64-byte blocks.
// We need deep copies when saving snapshots into the V table, because blockMix
// creates new backing arrays but we reuse the x variable.
func copyBlocks(blocks [][]byte) [][]byte {
	cp := make([][]byte, len(blocks))
	for i, b := range blocks {
		cp[i] = append([]byte(nil), b...)
	}
	return cp
}

// xorBlocks XORs every byte of src2 into dst (via src1), in-place.
// src1 and dst may alias — we use src1[i][k] before writing dst[i][k].
// (When dst == src1, the read and write are of the same byte — that is fine
//
//	because XOR is applied with src2[i][k], so dst[i][k] ^= src2[i][k].)
func xorBlocks(dst, src1 [][]byte, src2 [][]byte) {
	for i := range dst {
		for k := 0; k < 64; k++ {
			dst[i][k] = src1[i][k] ^ src2[i][k]
		}
	}
}

// integerify reads the first 8 bytes of the LAST block of x as a little-endian
// uint64, per RFC 7914 §4.
//
// The last block is chosen because BlockMix always terminates with a Salsa20/8
// output that depends on all previous blocks — it is the most "mixed" block.
func integerify(x [][]byte) uint64 {
	lastBlock := x[len(x)-1]
	return binary.LittleEndian.Uint64(lastBlock[:8])
}

// ===========================================================================
// pbkdf2Sha256 — Internal PBKDF2-HMAC-SHA256 (RFC 2898 §5.2)
// ===========================================================================
//
// This is a private PBKDF2 implementation that does NOT enforce a non-empty
// password restriction.  The published hmac.HmacSHA256 function returns
// ErrEmptyKey for empty passwords, but RFC 7914 test vector 1 uses
// password="" and salt="".  We implement PBKDF2 here using hmac.HMAC
// directly (which is exported and does not check for empty key), passing it
// a SHA-256 hash function.
//
// # PBKDF2 Algorithm (RFC 2898 §5.2, simplified for c=1)
//
// For each 1-indexed block i from 1 to ⌈keyLength/hLen⌉:
//
//	U_1 = PRF(password, salt || INT(i))    // PRF = HMAC-SHA256, hLen = 32
//	T_i = U_1 XOR U_2 XOR ... XOR U_c     // c = iterations
//
// DK = T_1 || T_2 || ... || T_⌈keyLength/hLen⌉  (truncated to keyLength)
//
// In scrypt, PBKDF2 is called with c=1 (a single PRF application per block),
// so the XOR loop body runs just once and T_i = U_1 = PRF(password, salt||INT(i)).
//
// # Why Not Use the pbkdf2 Package?
//
// The published pbkdf2 package calls hmac.HmacSHA256, which rejects empty
// keys.  Since RFC 7914 vector 1 exercises password="", we need a bypass.
// Implementing PBKDF2 inline avoids modifying the published package and keeps
// the security invariant (empty key rejection) intact for all other callers.
func pbkdf2Sha256(password, salt []byte, iterations, keyLength int) ([]byte, error) {
	const hLen = 32 // SHA-256 output size in bytes

	// sha256Fn wraps gsha256.Sum256 (which returns [32]byte) into the
	// hmacpkg.HashFn signature (func([]byte) []byte).
	sha256Fn := func(data []byte) []byte {
		s := gsha256.Sum256(data)
		return s[:]
	}

	// Compute the number of 32-byte blocks needed to produce keyLength bytes.
	numBlocks := (keyLength + hLen - 1) / hLen
	dk := make([]byte, 0, numBlocks*hLen)

	// blockIdx is a 4-byte big-endian counter appended to salt to
	// differentiate blocks (per PBKDF2 spec: INT(i) is big-endian 32-bit).
	blockIdx := make([]byte, 4)

	for i := 1; i <= numBlocks; i++ {
		binary.BigEndian.PutUint32(blockIdx, uint32(i))

		// seed = salt || INT(i)
		// Allocate fresh; do NOT use append(salt, blockIdx...) as that
		// could mutate the caller's salt slice if it has spare capacity.
		seed := make([]byte, len(salt)+4)
		copy(seed, salt)
		copy(seed[len(salt):], blockIdx)

		// U_1 = HMAC-SHA256(password, seed)
		// We call hmacpkg.HMAC directly (not HmacSHA256) to bypass the
		// empty-key restriction — scrypt allows password="".
		u := hmacpkg.HMAC(sha256Fn, 64, password, seed)
		if u == nil {
			return nil, fmt.Errorf("scrypt: PBKDF2 PRF returned nil on block %d", i)
		}

		// t = U_1 XOR U_2 XOR ... XOR U_c
		// For iterations=1 (scrypt's case), this loop body never executes
		// and t = U_1.
		t := make([]byte, hLen)
		copy(t, u)
		for j := 1; j < iterations; j++ {
			// U_{j+1} = HMAC-SHA256(password, U_j)
			u = hmacpkg.HMAC(sha256Fn, 64, password, u)
			if u == nil {
				return nil, fmt.Errorf("scrypt: PBKDF2 PRF returned nil on block %d iteration %d", i, j)
			}
			for k := range t {
				t[k] ^= u[k]
			}
		}
		dk = append(dk, t...)
	}
	return dk[:keyLength], nil
}

// ===========================================================================
// Public API
// ===========================================================================

// Scrypt derives a cryptographic key from a password using the scrypt KDF
// (RFC 7914).
//
// # Parameters
//
//   - password: the user's password (any bytes, including empty)
//   - salt:     a random salt (typically 16–32 bytes); reuse with the same
//     password always produces the same key, so store the salt
//   - n:        CPU/memory cost factor; must be a power of 2 ≥ 2 and ≤ 2^20.
//     Higher N ⇒ more time and memory.  Common values: 16384 or 32768.
//   - r:        block size multiplier; must be ≥ 1.  Typically 8.
//   - p:        parallelisation factor; must be ≥ 1.  Typically 1.
//   - dkLen:    desired output key length in bytes (1 to 2^20).
//
// # Memory Cost
//
//	memory ≈ N × 128 × r  bytes
//
// # Security Guidance
//
// Choose N, r, p so that the function takes ≥ 100 ms on your hardware.
// For interactive logins with r=8, p=1: N=32768 (requires ~32 MiB).
// For offline encryption: N=1048576 (1 GiB) gives much stronger guarantees.
//
// # Example
//
//	key, err := scrypt.Scrypt([]byte("hunter2"), salt, 32768, 8, 1, 32)
func Scrypt(password, salt []byte, n, r, p, dkLen int) ([]byte, error) {
	// ---------------------------------------------------------------------------
	// Validate parameters per RFC 7914 §2
	// ---------------------------------------------------------------------------

	// N must be a power of 2 and ≥ 2.
	// (n & (n-1)) == 0 is the standard "is power of 2" bit trick.
	// n < 2 catches N=0 and N=1.
	if n < 2 || (n&(n-1)) != 0 {
		return nil, ErrInvalidN
	}
	if n > 1<<20 {
		return nil, ErrNTooLarge
	}
	if r < 1 {
		return nil, ErrInvalidR
	}
	if p < 1 {
		return nil, ErrInvalidP
	}
	if dkLen < 1 {
		return nil, ErrInvalidKeyLength
	}
	if dkLen > 1<<20 {
		return nil, ErrKeyLengthTooLarge
	}
	// p*r product limit.  Cast to int64 to prevent overflow before comparison.
	if int64(p)*int64(r) > 1<<30 {
		return nil, ErrPRTooLarge
	}

	// ---------------------------------------------------------------------------
	// Step 1: Expand password+salt into p×128r bytes via PBKDF2-HMAC-SHA256.
	//
	// The output B is split into p lanes of 128r bytes each.  Each lane is
	// processed independently by ROMix.
	// ---------------------------------------------------------------------------
	bLen := p * 128 * r
	b, err := pbkdf2Sha256(password, salt, 1, bLen)
	if err != nil {
		return nil, fmt.Errorf("scrypt: initial PBKDF2 failed: %w", err)
	}

	// ---------------------------------------------------------------------------
	// Step 2: Apply ROMix to each of the p lanes of B.
	//
	// Each lane is 128*r bytes.  ROMix is sequential-memory-hard and produces
	// a fresh 128*r-byte block that replaces the lane in B.
	//
	// In a production implementation, p lanes can be processed in parallel
	// (goroutines).  We keep it sequential here for clarity.
	// ---------------------------------------------------------------------------
	for i := 0; i < p; i++ {
		chunk := b[i*128*r : (i+1)*128*r]
		mixed := roMix(chunk, n, r)
		copy(b[i*128*r:], mixed)
	}

	// ---------------------------------------------------------------------------
	// Step 3: Condense the fully-mixed B into dkLen output bytes.
	//
	// Another round of PBKDF2-HMAC-SHA256 with the original password and the
	// mixed B as the "salt".  The output is the derived key.
	// ---------------------------------------------------------------------------
	return pbkdf2Sha256(password, b, 1, dkLen)
}

// ScryptHex is a convenience wrapper around Scrypt that returns the derived
// key as a lowercase hexadecimal string instead of raw bytes.
//
// Useful for storing keys as text (e.g., in configuration files or databases).
//
// Example:
//
//	hex, err := scrypt.ScryptHex([]byte("password"), []byte("salt"), 16, 1, 1, 32)
func ScryptHex(password, salt []byte, n, r, p, dkLen int) (string, error) {
	dk, err := Scrypt(password, salt, n, r, p, dkLen)
	if err != nil {
		return "", err
	}
	return fmt.Sprintf("%x", dk), nil
}
