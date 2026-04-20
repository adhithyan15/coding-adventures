// Package argon2id implements Argon2id (RFC 9106) -- hybrid memory-hard
// password hashing -- from scratch in pure Go.
//
// # What Is Argon2id?
//
// Argon2id is the RFC-recommended memory-hard password-hashing function.
// It is the "hybrid" member of the Argon2 family: the first half of the
// first pass uses side-channel-resistant data-independent addressing
// (Argon2i), and everything afterwards uses GPU/ASIC-resistant
// data-dependent addressing (Argon2d).
//
// Pick this variant unless you have a specific reason to prefer
// ``argon2d`` (proof-of-work, no side-channel threat) or ``argon2i``
// (strict side-channel requirements).
//
// # How It Works
//
// Argon2 allocates a large matrix of 1024-byte "blocks", fills it using
// the compression function G, and derives the final tag from the last
// column of memory.  G is a modified BLAKE2b round: the message schedule
// (SIGMA) is dropped and every quarter-round addition is augmented with
// ``2 * trunc32(a) * trunc32(b)`` -- Argon2's only cryptographic bolt-on
// on top of BLAKE2b.
//
// BLAKE2b is used for H0 (the parameter hash) and H' (the variable-length
// extender); the inner compression round is Argon2-specific and is
// inlined here rather than imported from the BLAKE2b package.
//
// # References
//
//   - RFC 9106: https://datatracker.ietf.org/doc/html/rfc9106
//   - code/specs/KD03-argon2.md (full walk-through)
package argon2id

import (
	"encoding/binary"
	"encoding/hex"
	"fmt"

	"github.com/adhithyan15/coding-adventures/code/packages/go/blake2b"
)

const (
	mask32 = 0xFFFFFFFF

	blockSize         = 1024
	blockWords        = blockSize / 8 // 128 × u64
	syncPoints        = 4             // slices per pass
	addressesPerBlock = blockWords    // 128

	// Version is the only supported Argon2 protocol version (v1.3).
	Version = 0x13

	typeID = 2
)

// rotr64 rotates a 64-bit unsigned integer right by n bits.
func rotr64(x uint64, n uint) uint64 {
	return (x >> n) | (x << (64 - n))
}

// gB is Argon2's G_B quarter-round -- BLAKE2b's mixer with a
// 2*trunc32(a)*trunc32(b) term added to every addition.  That
// multiplicative term is Argon2's only change on top of BLAKE2b and is
// what makes the round non-linear in a way that is not invertible from a
// single operand.
func gB(v []uint64, a, b, c, d int) {
	va, vb, vc, vd := v[a], v[b], v[c], v[d]

	va = va + vb + 2*(va&mask32)*(vb&mask32)
	vd = rotr64(vd^va, 32)
	vc = vc + vd + 2*(vc&mask32)*(vd&mask32)
	vb = rotr64(vb^vc, 24)
	va = va + vb + 2*(va&mask32)*(vb&mask32)
	vd = rotr64(vd^va, 16)
	vc = vc + vd + 2*(vc&mask32)*(vd&mask32)
	vb = rotr64(vb^vc, 63)

	v[a], v[b], v[c], v[d] = va, vb, vc, vd
}

// permutationP applies one BLAKE2b round to 16 u64 words: four columns
// then four diagonals.
func permutationP(v []uint64) {
	gB(v, 0, 4, 8, 12)
	gB(v, 1, 5, 9, 13)
	gB(v, 2, 6, 10, 14)
	gB(v, 3, 7, 11, 15)
	gB(v, 0, 5, 10, 15)
	gB(v, 1, 6, 11, 12)
	gB(v, 2, 7, 8, 13)
	gB(v, 3, 4, 9, 14)
}

// compress is Argon2's compression function G(X, Y).  The block is
// logically an 8x8 matrix of 128-bit registers.  P is applied to each of
// the 8 rows (16 contiguous words) then to each of the 8 "columns"
// (gathered as word pairs at stride 16 starting at offset 2c).  The
// feed-forward ``R XOR Q`` is the Davies-Meyer construction that keeps
// G non-invertible.
func compress(X, Y []uint64) []uint64 {
	R := make([]uint64, blockWords)
	for i := 0; i < blockWords; i++ {
		R[i] = X[i] ^ Y[i]
	}
	Q := make([]uint64, blockWords)
	copy(Q, R)

	// Row pass.
	for i := 0; i < 8; i++ {
		permutationP(Q[i*16 : (i+1)*16])
	}

	// Column pass -- gather pairs at stride 16 starting at offset 2c.
	col := make([]uint64, 16)
	for c := 0; c < 8; c++ {
		for r := 0; r < 8; r++ {
			col[2*r] = Q[r*16+2*c]
			col[2*r+1] = Q[r*16+2*c+1]
		}
		permutationP(col)
		for r := 0; r < 8; r++ {
			Q[r*16+2*c] = col[2*r]
			Q[r*16+2*c+1] = col[2*r+1]
		}
	}

	out := make([]uint64, blockWords)
	for i := 0; i < blockWords; i++ {
		out[i] = R[i] ^ Q[i]
	}
	return out
}

func blockToBytes(block []uint64) []byte {
	out := make([]byte, blockSize)
	for i := 0; i < blockWords; i++ {
		binary.LittleEndian.PutUint64(out[i*8:], block[i])
	}
	return out
}

func bytesToBlock(data []byte) []uint64 {
	out := make([]uint64, blockWords)
	for i := 0; i < blockWords; i++ {
		out[i] = binary.LittleEndian.Uint64(data[i*8:])
	}
	return out
}

func le32(n uint32) []byte {
	b := make([]byte, 4)
	binary.LittleEndian.PutUint32(b, n)
	return b
}

// blake2bLong -- Argon2's variable-length BLAKE2b (H', RFC 9106 §3.3).
// For T <= 64 it is just BLAKE2b-T(LE32(T) || X); for T > 64 it chains
// 64-byte BLAKE2b outputs, keeping the first 32 bytes of each, then
// finishes with a variable-size BLAKE2b for the tail (T - 32r) bytes.
func blake2bLong(T int, X []byte) ([]byte, error) {
	if T <= 0 {
		return nil, fmt.Errorf("H' output length must be positive, got %d", T)
	}
	prefix := le32(uint32(T))
	in := append(append([]byte{}, prefix...), X...)

	if T <= 64 {
		return blake2b.Sum(in, T, nil, nil, nil)
	}

	r := (T+31)/32 - 2
	V, err := blake2b.Sum(in, 64, nil, nil, nil)
	if err != nil {
		return nil, err
	}
	out := make([]byte, 0, T)
	out = append(out, V[:32]...)
	for i := 1; i < r; i++ {
		V, err = blake2b.Sum(V, 64, nil, nil, nil)
		if err != nil {
			return nil, err
		}
		out = append(out, V[:32]...)
	}
	finalSize := T - 32*r
	V, err = blake2b.Sum(V, finalSize, nil, nil, nil)
	if err != nil {
		return nil, err
	}
	out = append(out, V...)
	return out, nil
}

// indexAlpha maps J1 to a reference-block column (RFC 9106 §3.4.1.1).
func indexAlpha(J1 uint64, r, sl, c int, sameLane bool, q, SL int) int {
	var W, start int
	switch {
	case r == 0 && sl == 0:
		W = c - 1
		start = 0
	case r == 0:
		if sameLane {
			W = sl*SL + c - 1
		} else if c == 0 {
			W = sl*SL - 1
		} else {
			W = sl * SL
		}
		start = 0
	default:
		if sameLane {
			W = q - SL + c - 1
		} else if c == 0 {
			W = q - SL - 1
		} else {
			W = q - SL
		}
		start = ((sl + 1) * SL) % q
	}

	// Biased relative position via the J1^2 squashing.  Keep the
	// intermediate in uint64 to avoid 32-bit overflow in W*x.
	x := (J1 * J1) >> 32
	y := (uint64(W) * x) >> 32
	rel := W - 1 - int(y)

	return (start + rel) % q
}

// fillSegment fills one (pass, slice, lane) segment of the memory matrix.
// Argon2id uses data-independent addressing for pass 0 slices 0 and 1,
// then switches to data-dependent addressing for everything after.
func fillSegment(memory [][][]uint64, r, lane, sl, q, SL, p, mPrime, t int) {
	dataIndependent := r == 0 && sl < 2

	var inputBlock, addressBlock [blockWords]uint64
	var zeroBlock [blockWords]uint64
	if dataIndependent {
		inputBlock[0] = uint64(r)
		inputBlock[1] = uint64(lane)
		inputBlock[2] = uint64(sl)
		inputBlock[3] = uint64(mPrime)
		inputBlock[4] = uint64(t)
		inputBlock[5] = typeID
	}

	nextAddresses := func() {
		inputBlock[6]++
		Z := compress(zeroBlock[:], inputBlock[:])
		addr := compress(zeroBlock[:], Z)
		copy(addressBlock[:], addr)
	}

	startingC := 0
	if r == 0 && sl == 0 {
		startingC = 2
	}
	if dataIndependent && startingC != 0 {
		nextAddresses()
	}

	for i := startingC; i < SL; i++ {
		if dataIndependent && i%addressesPerBlock == 0 && !(r == 0 && sl == 0 && i == 2) {
			nextAddresses()
		}

		col := sl*SL + i
		prevCol := col - 1
		if col == 0 {
			prevCol = q - 1
		}
		prevBlock := memory[lane][prevCol]

		var pseudoRand uint64
		if dataIndependent {
			pseudoRand = addressBlock[i%addressesPerBlock]
		} else {
			pseudoRand = prevBlock[0]
		}
		J1 := pseudoRand & mask32
		J2 := (pseudoRand >> 32) & mask32

		lPrime := lane
		if !(r == 0 && sl == 0) {
			lPrime = int(J2 % uint64(p))
		}
		zPrime := indexAlpha(J1, r, sl, i, lPrime == lane, q, SL)
		refBlock := memory[lPrime][zPrime]

		newBlock := compress(prevBlock, refBlock)
		if r == 0 {
			memory[lane][col] = newBlock
		} else {
			existing := memory[lane][col]
			merged := make([]uint64, blockWords)
			for k := 0; k < blockWords; k++ {
				merged[k] = existing[k] ^ newBlock[k]
			}
			memory[lane][col] = merged
		}
	}
}

// Options holds the optional parameters for Argon2id.
type Options struct {
	// Key is the optional secret MAC key (K in the RFC).  Default empty.
	Key []byte
	// AssociatedData is the optional context data (X in the RFC).
	// Default empty.
	AssociatedData []byte
	// Version is the Argon2 protocol version.  Only 0x13 (v1.3) is
	// supported; leave zero to default to Version.
	Version uint32
}

func validate(password, salt []byte, timeCost, memoryCost, parallelism, tagLength int, key, ad []byte, version uint32) error {
	if uint64(len(password)) > 0xFFFFFFFF {
		return fmt.Errorf("password length must fit in 32 bits, got %d", len(password))
	}
	if len(salt) < 8 {
		return fmt.Errorf("salt must be at least 8 bytes, got %d", len(salt))
	}
	if uint64(len(salt)) > 0xFFFFFFFF {
		return fmt.Errorf("salt length must fit in 32 bits, got %d", len(salt))
	}
	if uint64(len(key)) > 0xFFFFFFFF {
		return fmt.Errorf("key length must fit in 32 bits, got %d", len(key))
	}
	if uint64(len(ad)) > 0xFFFFFFFF {
		return fmt.Errorf("associatedData length must fit in 32 bits, got %d", len(ad))
	}
	if tagLength < 4 {
		return fmt.Errorf("tagLength must be >= 4, got %d", tagLength)
	}
	if uint64(tagLength) > 0xFFFFFFFF {
		return fmt.Errorf("tagLength must fit in 32 bits, got %d", tagLength)
	}
	if parallelism < 1 || parallelism > 0xFFFFFF {
		return fmt.Errorf("parallelism must be in [1, 2^24-1], got %d", parallelism)
	}
	if memoryCost < 8*parallelism {
		return fmt.Errorf("memoryCost must be >= 8*parallelism (%d), got %d", 8*parallelism, memoryCost)
	}
	if uint64(memoryCost) > 0xFFFFFFFF {
		return fmt.Errorf("memoryCost must fit in 32 bits, got %d", memoryCost)
	}
	if timeCost < 1 {
		return fmt.Errorf("timeCost must be >= 1, got %d", timeCost)
	}
	if version != Version {
		return fmt.Errorf("only Argon2 v1.3 (0x13) is supported; got 0x%02x", version)
	}
	return nil
}

// Sum computes an Argon2id tag (RFC 9106 §3).  Parameters:
//
//   - password:    the secret input.
//   - salt:        at least 8 bytes; 16+ is recommended.
//   - timeCost:    number of passes over memory (t in the RFC), >= 1.
//   - memoryCost:  target memory in KiB (m in the RFC), >= 8*parallelism.
//     Internally rounded down to a multiple of 4*parallelism.
//   - parallelism: lane count (p in the RFC), 1..2^24-1.
//   - tagLength:   output length in bytes (T in the RFC), >= 4.
//   - opts:        optional Key, AssociatedData, Version.
//
// Returns exactly tagLength bytes.
func Sum(password, salt []byte, timeCost, memoryCost, parallelism, tagLength int, opts *Options) ([]byte, error) {
	var key, ad []byte
	version := uint32(Version)
	if opts != nil {
		key = opts.Key
		ad = opts.AssociatedData
		if opts.Version != 0 {
			version = opts.Version
		}
	}

	if err := validate(password, salt, timeCost, memoryCost, parallelism, tagLength, key, ad, version); err != nil {
		return nil, err
	}

	segmentLength := memoryCost / (syncPoints * parallelism)
	mPrime := segmentLength * syncPoints * parallelism
	q := mPrime / parallelism
	SL := segmentLength
	p := parallelism
	t := timeCost

	// Step 1 -- H0 (64 bytes, RFC 9106 §3.2).
	var h0In []byte
	h0In = append(h0In, le32(uint32(p))...)
	h0In = append(h0In, le32(uint32(tagLength))...)
	h0In = append(h0In, le32(uint32(memoryCost))...)
	h0In = append(h0In, le32(uint32(t))...)
	h0In = append(h0In, le32(version)...)
	h0In = append(h0In, le32(typeID)...)
	h0In = append(h0In, le32(uint32(len(password)))...)
	h0In = append(h0In, password...)
	h0In = append(h0In, le32(uint32(len(salt)))...)
	h0In = append(h0In, salt...)
	h0In = append(h0In, le32(uint32(len(key)))...)
	h0In = append(h0In, key...)
	h0In = append(h0In, le32(uint32(len(ad)))...)
	h0In = append(h0In, ad...)

	h0, err := blake2b.Sum(h0In, 64, nil, nil, nil)
	if err != nil {
		return nil, err
	}

	// Step 2 -- allocate B[p][q].
	memory := make([][][]uint64, p)
	for i := 0; i < p; i++ {
		memory[i] = make([][]uint64, q)
		for j := 0; j < q; j++ {
			memory[i][j] = make([]uint64, blockWords)
		}
	}

	// Step 3 -- first two blocks of every lane via H'.
	for i := 0; i < p; i++ {
		in0 := append(append([]byte{}, h0...), le32(0)...)
		in0 = append(in0, le32(uint32(i))...)
		b0, err := blake2bLong(blockSize, in0)
		if err != nil {
			return nil, err
		}
		in1 := append(append([]byte{}, h0...), le32(1)...)
		in1 = append(in1, le32(uint32(i))...)
		b1, err := blake2bLong(blockSize, in1)
		if err != nil {
			return nil, err
		}
		memory[i][0] = bytesToBlock(b0)
		memory[i][1] = bytesToBlock(b1)
	}

	// Step 4 -- fill the rest of memory, pass by pass, slice by slice.
	for r := 0; r < t; r++ {
		for sl := 0; sl < syncPoints; sl++ {
			for lane := 0; lane < p; lane++ {
				fillSegment(memory, r, lane, sl, q, SL, p, mPrime, t)
			}
		}
	}

	// Step 5 -- XOR the final column across lanes, then H' to T bytes.
	finalBlock := make([]uint64, blockWords)
	copy(finalBlock, memory[0][q-1])
	for lane := 1; lane < p; lane++ {
		for k := 0; k < blockWords; k++ {
			finalBlock[k] ^= memory[lane][q-1][k]
		}
	}

	return blake2bLong(tagLength, blockToBytes(finalBlock))
}

// SumHex is Sum returning lowercase hex.
func SumHex(password, salt []byte, timeCost, memoryCost, parallelism, tagLength int, opts *Options) (string, error) {
	tag, err := Sum(password, salt, timeCost, memoryCost, parallelism, tagLength, opts)
	if err != nil {
		return "", err
	}
	return hex.EncodeToString(tag), nil
}
