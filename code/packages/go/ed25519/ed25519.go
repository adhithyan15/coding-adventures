// Package ed25519 implements Ed25519 digital signatures (RFC 8032) from scratch.
//
// # What Is Ed25519?
//
// Ed25519 is an elliptic curve digital signature algorithm (EdDSA) designed by
// Daniel J. Bernstein et al. It uses the twisted Edwards curve:
//
//	-x² + y² = 1 + d·x²·y²    (mod p, where p = 2²⁵⁵ - 19)
//
// Ed25519 provides:
//   - Fast signing and verification
//   - Compact 32-byte keys and 64-byte signatures
//   - 128-bit security level
//   - Deterministic signing (no random nonce needed)
//
// # How It Works
//
//  1. KEY GENERATION: Hash a 32-byte seed with SHA-512. Clamp the first 32 bytes
//     to get the secret scalar. Multiply the base point B by this scalar to get
//     the public key A.
//
//  2. SIGNING: Hash prefix||message for a deterministic nonce r. Compute R = r*B.
//     Hash R||pubkey||message for challenge k. Compute S = (r + k*a) mod L.
//     Signature = R || S.
//
//  3. VERIFICATION: Check S*B == R + k*A.
//
// # The Curve
//
// The twisted Edwards curve -x² + y² = 1 + d·x²·y² over GF(2²⁵⁵ - 19) is
// birationally equivalent to the Montgomery curve used in X25519. The Edwards
// form allows efficient, complete addition formulas — no special cases.
//
// # Extended Coordinates
//
// Points use extended twisted Edwards coordinates (X, Y, Z, T) where:
//
//	x = X/Z,   y = Y/Z,   T = X·Y/Z
//
// This avoids expensive modular divisions during point operations.
package ed25519

import (
	"math/big"

	sha512 "github.com/example/coding-adventures/code/packages/go/sha512"
)

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 1: CONSTANTS
// ═══════════════════════════════════════════════════════════════════════════════

// p is the field prime: 2²⁵⁵ - 19.
// All coordinate arithmetic is done modulo this prime.
var p = new(big.Int).Sub(new(big.Int).Lsh(big.NewInt(1), 255), big.NewInt(19))

// d is the curve parameter in -x² + y² = 1 + d·x²·y².
// It equals -121665/121666 mod p.
var d = bigFromDecimal("37095705934669439343138083508754565189542113879843219016388785533085940283555")

// curveL is the order of the base point subgroup.
// Every valid Ed25519 scalar is reduced modulo L.
// L = 2²⁵² + 27742317777372353535851937790883648493
var curveL = bigFromDecimal("7237005577332262213973186563042994240857116359379907606001950938285454250989")

// baseX is the x-coordinate of the base point B.
var baseX = bigFromDecimal("15112221349535400772501151409588531511454012693041857206046113283949847762202")

// baseY is the y-coordinate of the base point B.
// B_y = 4/5 mod p.
var baseY = bigFromDecimal("46316835694926478169428394003475163141307993866256225615783033603165251855960")

// sqrtM1 is √(-1) mod p. Since p ≡ 5 (mod 8), -1 has a square root in GF(p).
// Used in point decompression when computing field square roots.
var sqrtM1 = bigFromDecimal("19681161376707505956807079304988542015446066515923890162744021073123829784752")

// bigFromDecimal creates a *big.Int from a decimal string.
func bigFromDecimal(s string) *big.Int {
	n, ok := new(big.Int).SetString(s, 10)
	if !ok {
		panic("invalid decimal constant: " + s)
	}
	return n
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 2: FIELD ARITHMETIC IN GF(2²⁵⁵ - 19)
// ═══════════════════════════════════════════════════════════════════════════════
//
// Go's math/big provides arbitrary-precision integers with modular arithmetic.
// We build thin wrappers for the operations Ed25519 needs.

// fieldInv computes a⁻¹ mod p using Fermat's little theorem:
//
//	a^(p-2) ≡ a⁻¹ (mod p)
//
// This works because a^(p-1) ≡ 1 (mod p) for any nonzero a in GF(p).
func fieldInv(a *big.Int) *big.Int {
	pMinus2 := new(big.Int).Sub(p, big.NewInt(2))
	return new(big.Int).Exp(a, pMinus2, p)
}

// fieldSqrt computes √a mod p, or returns nil if a is not a quadratic residue.
//
// Since p ≡ 5 (mod 8), we use Atkin's algorithm:
//
//	candidate = a^((p+3)/8) mod p
//
// Then check:
//   - If candidate² ≡ a: return candidate
//   - If candidate² ≡ -a: return candidate · √(-1)
//   - Otherwise: a is not a quadratic residue (no square root exists)
func fieldSqrt(a *big.Int) *big.Int {
	// exponent = (p + 3) / 8
	exp := new(big.Int).Add(p, big.NewInt(3))
	exp.Rsh(exp, 3)

	candidate := new(big.Int).Exp(a, exp, p)

	// Check: candidate² mod p == a mod p?
	candidateSq := new(big.Int).Mul(candidate, candidate)
	candidateSq.Mod(candidateSq, p)

	aMod := new(big.Int).Mod(a, p)

	if candidateSq.Cmp(aMod) == 0 {
		return candidate
	}

	// Check: candidate² ≡ -a (mod p)?
	negA := new(big.Int).Sub(p, aMod)
	negA.Mod(negA, p)
	if candidateSq.Cmp(negA) == 0 {
		result := new(big.Int).Mul(candidate, sqrtM1)
		result.Mod(result, p)
		return result
	}

	// Not a quadratic residue
	return nil
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 3: EXTENDED TWISTED EDWARDS POINT OPERATIONS
// ═══════════════════════════════════════════════════════════════════════════════
//
// Points on the curve are represented as (X, Y, Z, T) where:
//   x = X/Z,   y = Y/Z,   T = X·Y/Z
//
// The identity point is (0, 1, 1, 0), corresponding to affine (0, 1).

// Point represents a point on the Ed25519 curve in extended twisted Edwards
// coordinates (X, Y, Z, T).
type Point struct {
	X, Y, Z, T *big.Int
}

// newPoint creates a Point from four big.Int values, copying them.
func newPoint(x, y, z, t *big.Int) *Point {
	return &Point{
		X: new(big.Int).Set(x),
		Y: new(big.Int).Set(y),
		Z: new(big.Int).Set(z),
		T: new(big.Int).Set(t),
	}
}

// identity returns the identity point (0, 1, 1, 0).
func identity() *Point {
	return newPoint(big.NewInt(0), big.NewInt(1), big.NewInt(1), big.NewInt(0))
}

// basePoint returns the Ed25519 base point B in extended coordinates.
func basePoint() *Point {
	t := new(big.Int).Mul(baseX, baseY)
	t.Mod(t, p)
	return newPoint(baseX, baseY, big.NewInt(1), t)
}

// pointAdd adds two points on the twisted Edwards curve using the unified
// addition formula for a = -1:
//
//	A = X1·X2,  B = Y1·Y2,  C = T1·d·T2,  D = Z1·Z2
//	E = (X1+Y1)·(X2+Y2) - A - B
//	F = D - C,  G = D + C,  H = B + A  (note: +A because a=-1)
//	X3 = E·F,  Y3 = G·H,  T3 = E·H,  Z3 = F·G
//
// This formula is "complete" — it works for all input pairs, including
// doubling, adding the identity, and adding inverses. No conditional branches
// means no timing side channels.
func pointAdd(p1, p2 *Point) *Point {
	a := new(big.Int).Mul(p1.X, p2.X)
	a.Mod(a, p)

	b := new(big.Int).Mul(p1.Y, p2.Y)
	b.Mod(b, p)

	c := new(big.Int).Mul(p1.T, d)
	c.Mul(c, p2.T)
	c.Mod(c, p)

	dd := new(big.Int).Mul(p1.Z, p2.Z)
	dd.Mod(dd, p)

	// E = (X1+Y1)·(X2+Y2) - A - B
	e := new(big.Int).Add(p1.X, p1.Y)
	tmp := new(big.Int).Add(p2.X, p2.Y)
	e.Mul(e, tmp)
	e.Sub(e, a)
	e.Sub(e, b)
	e.Mod(e, p)

	f := new(big.Int).Sub(dd, c)
	f.Mod(f, p)

	g := new(big.Int).Add(dd, c)
	g.Mod(g, p)

	h := new(big.Int).Add(b, a)
	h.Mod(h, p)

	x3 := new(big.Int).Mul(e, f)
	x3.Mod(x3, p)

	y3 := new(big.Int).Mul(g, h)
	y3.Mod(y3, p)

	t3 := new(big.Int).Mul(e, h)
	t3.Mod(t3, p)

	z3 := new(big.Int).Mul(f, g)
	z3.Mod(z3, p)

	return &Point{X: x3, Y: y3, Z: z3, T: t3}
}

// pointDouble doubles a point using the dedicated doubling formula:
//
//	A = X1²,  B = Y1²,  C = 2·Z1²
//	D = -A  (since a=-1)
//	E = (X1+Y1)² - A - B
//	G = D + B,  F = G - C,  H = D - B
//	X3 = E·F,  Y3 = G·H,  T3 = E·H,  Z3 = F·G
func pointDouble(pt *Point) *Point {
	a := new(big.Int).Mul(pt.X, pt.X)
	a.Mod(a, p)

	b := new(big.Int).Mul(pt.Y, pt.Y)
	b.Mod(b, p)

	c := new(big.Int).Mul(pt.Z, pt.Z)
	c.Mul(c, big.NewInt(2))
	c.Mod(c, p)

	// D = -A (since a = -1 in this curve)
	dd := new(big.Int).Neg(a)
	dd.Mod(dd, p)

	// E = (X1+Y1)² - A - B
	e := new(big.Int).Add(pt.X, pt.Y)
	e.Mul(e, e)
	e.Sub(e, a)
	e.Sub(e, b)
	e.Mod(e, p)

	g := new(big.Int).Add(dd, b)
	g.Mod(g, p)

	f := new(big.Int).Sub(g, c)
	f.Mod(f, p)

	h := new(big.Int).Sub(dd, b)
	h.Mod(h, p)

	x3 := new(big.Int).Mul(e, f)
	x3.Mod(x3, p)

	y3 := new(big.Int).Mul(g, h)
	y3.Mod(y3, p)

	t3 := new(big.Int).Mul(e, h)
	t3.Mod(t3, p)

	z3 := new(big.Int).Mul(f, g)
	z3.Mod(z3, p)

	return &Point{X: x3, Y: y3, Z: z3, T: t3}
}

// scalarMult computes s * pt using the double-and-add algorithm.
//
// This scans bits of the scalar from high to low:
//
//	result = identity
//	for each bit of s (MSB to LSB):
//	    result = double(result)
//	    if bit == 1: result = add(result, pt)
//
// WARNING: Not constant-time. A production implementation would use a
// fixed-window or Montgomery ladder to prevent timing attacks.
func scalarMult(s *big.Int, pt *Point) *Point {
	if s.Sign() == 0 {
		return identity()
	}

	result := identity()
	for i := s.BitLen() - 1; i >= 0; i-- {
		result = pointDouble(result)
		if s.Bit(i) == 1 {
			result = pointAdd(result, pt)
		}
	}
	return result
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 4: POINT ENCODING AND DECODING
// ═══════════════════════════════════════════════════════════════════════════════
//
// Ed25519 uses a compact 32-byte encoding for points:
//   - Store y as 32 bytes, little-endian
//   - Pack the low bit (sign) of x into the high bit of byte[31]
//
// This works because x can be recovered from y using the curve equation.

// pointEncode encodes a curve point as 32 bytes per RFC 8032.
func pointEncode(pt *Point) [32]byte {
	// Convert from projective to affine: x = X·Z⁻¹, y = Y·Z⁻¹
	zInv := fieldInv(pt.Z)

	xAff := new(big.Int).Mul(pt.X, zInv)
	xAff.Mod(xAff, p)

	yAff := new(big.Int).Mul(pt.Y, zInv)
	yAff.Mod(yAff, p)

	// Encode y as 32 bytes, little-endian
	var encoded [32]byte
	yBytes := yAff.Bytes() // big-endian
	// Reverse into little-endian
	for i, b := range yBytes {
		encoded[len(yBytes)-1-i] = b
	}

	// Pack the sign of x (low bit) into the high bit of byte[31]
	if xAff.Bit(0) == 1 {
		encoded[31] |= 0x80
	}

	return encoded
}

// pointDecode decodes a 32-byte compressed point per RFC 8032.
//
// Steps:
//  1. Extract sign bit (high bit of byte[31])
//  2. Clear sign bit and decode y (little-endian)
//  3. Reject if y >= p
//  4. Compute x² = (y² - 1) / (d·y² + 1)
//  5. Compute x = √(x²), reject if no sqrt exists
//  6. If x's low bit != sign, negate x
//  7. If x == 0 and sign == 1, reject
//
// Returns the point and true on success, or nil and false on failure.
func pointDecode(data [32]byte) (*Point, bool) {
	// Step 1: Extract sign bit
	sign := int(data[31] >> 7)

	// Step 2: Clear sign bit, decode y
	data[31] &= 0x7F
	yVal := new(big.Int)
	// Convert from little-endian to big-endian for SetBytes
	reversed := reverseBytes(data[:])
	yVal.SetBytes(reversed)

	// Step 3: Reject if y >= p
	if yVal.Cmp(p) >= 0 {
		return nil, false
	}

	// Step 4: x² = (y² - 1) / (d·y² + 1) mod p
	ySq := new(big.Int).Mul(yVal, yVal)
	ySq.Mod(ySq, p)

	// numerator = y² - 1
	num := new(big.Int).Sub(ySq, big.NewInt(1))
	num.Mod(num, p)

	// denominator = d·y² + 1
	den := new(big.Int).Mul(d, ySq)
	den.Add(den, big.NewInt(1))
	den.Mod(den, p)

	xSq := new(big.Int).Mul(num, fieldInv(den))
	xSq.Mod(xSq, p)

	// Step 5: x = √(x²)
	if xSq.Sign() == 0 {
		if sign == 1 {
			return nil, false // x=0 but sign bit set
		}
		t := new(big.Int).Mul(big.NewInt(0), yVal)
		t.Mod(t, p)
		return newPoint(big.NewInt(0), yVal, big.NewInt(1), t), true
	}

	xVal := fieldSqrt(xSq)
	if xVal == nil {
		return nil, false // not a quadratic residue
	}

	// Step 6: Ensure x has the correct sign (parity)
	if int(xVal.Bit(0)) != sign {
		xVal.Sub(p, xVal)
	}

	t := new(big.Int).Mul(xVal, yVal)
	t.Mod(t, p)

	return newPoint(xVal, yVal, big.NewInt(1), t), true
}

// reverseBytes returns a new slice with bytes in reverse order.
func reverseBytes(b []byte) []byte {
	r := make([]byte, len(b))
	for i, v := range b {
		r[len(b)-1-i] = v
	}
	return r
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 5: CLAMPING
// ═══════════════════════════════════════════════════════════════════════════════
//
// Clamping modifies the raw hash bytes to produce a safe scalar:
//   - Clear low 3 bits: scalar is multiple of 8 (prevents small-subgroup attacks)
//   - Clear bit 255: scalar < 2²⁵⁵
//   - Set bit 254: scalar >= 2²⁵⁴ (constant number of bits, prevents timing attacks)

// clamp takes the first 32 bytes of a SHA-512 hash and returns the clamped scalar.
func clamp(h [64]byte) *big.Int {
	raw := make([]byte, 32)
	copy(raw, h[:32])
	raw[0] &= 248
	raw[31] &= 127
	raw[31] |= 64
	// Decode as little-endian integer
	reversed := reverseBytes(raw)
	return new(big.Int).SetBytes(reversed)
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 6: PUBLIC API
// ═══════════════════════════════════════════════════════════════════════════════

// GenerateKeypair generates an Ed25519 key pair from a 32-byte seed.
//
// The seed is the master secret — it should come from a cryptographically
// secure random source. The seed is hashed with SHA-512 to produce both
// the secret scalar and a prefix for deterministic nonce generation.
//
// Returns (publicKey, secretKey) where:
//   - publicKey is 32 bytes (the encoded curve point A = a·B)
//   - secretKey is 64 bytes (seed || publicKey)
//
// Panics if seed is not exactly 32 bytes.
func GenerateKeypair(seed [32]byte) (publicKey [32]byte, secretKey [64]byte) {
	// Step 1: Hash the seed with SHA-512
	h := sha512.Sum512(seed[:])

	// Step 2: Clamp to get the secret scalar
	a := clamp(h)

	// Step 3: Compute public key A = a · B
	bigA := scalarMult(a, basePoint())
	publicKey = pointEncode(bigA)

	// Step 4: Assemble secret key = seed || publicKey
	copy(secretKey[:32], seed[:])
	copy(secretKey[32:], publicKey[:])

	return publicKey, secretKey
}

// Sign signs a message with an Ed25519 secret key.
//
// Ed25519 signing is deterministic — the same message and key always produce
// the same signature. The nonce is derived from a hash of the secret key
// prefix and message, not from a random source. This prevents catastrophic
// nonce reuse (the attack that broke Sony's PS3 signing key).
//
// Returns a 64-byte signature: R (32 bytes) || S (32 bytes).
func Sign(message []byte, secretKey [64]byte) [64]byte {
	// Extract seed and public key
	seed := secretKey[:32]
	publicKey := secretKey[32:64]

	// Hash the seed to recover scalar and prefix
	h := sha512.Sum512(seed)
	a := clamp(h)
	prefix := h[32:64]

	// Step 1: Deterministic nonce r = SHA-512(prefix || message) mod L
	rInput := make([]byte, 0, 32+len(message))
	rInput = append(rInput, prefix...)
	rInput = append(rInput, message...)
	rHash := sha512.Sum512(rInput)
	r := bytesToBigIntLE(rHash[:])
	r.Mod(r, curveL)

	// Step 2: R = r · B
	bigR := scalarMult(r, basePoint())
	rBytes := pointEncode(bigR)

	// Step 3: k = SHA-512(R || publicKey || message) mod L
	kInput := make([]byte, 0, 32+32+len(message))
	kInput = append(kInput, rBytes[:]...)
	kInput = append(kInput, publicKey...)
	kInput = append(kInput, message...)
	kHash := sha512.Sum512(kInput)
	k := bytesToBigIntLE(kHash[:])
	k.Mod(k, curveL)

	// Step 4: S = (r + k·a) mod L
	sVal := new(big.Int).Mul(k, a)
	sVal.Add(sVal, r)
	sVal.Mod(sVal, curveL)
	sBytes := bigIntToLE32(sVal)

	// Assemble signature: R || S
	var sig [64]byte
	copy(sig[:32], rBytes[:])
	copy(sig[32:], sBytes[:])
	return sig
}

// Verify checks an Ed25519 signature.
//
// The verification equation is:
//
//	S · B == R + k · A
//
// where:
//   - S and R are decoded from the signature
//   - A is decoded from the public key
//   - k = SHA-512(R || publicKey || message) mod L
//
// Returns true if valid, false otherwise. Never panics on invalid input.
func Verify(message []byte, signature [64]byte, publicKey [32]byte) bool {
	// Step 1: Split signature into R and S
	var rBytes, sBytes [32]byte
	copy(rBytes[:], signature[:32])
	copy(sBytes[:], signature[32:])

	// Step 2: Decode S
	sVal := bytesToBigIntLE(sBytes[:])
	if sVal.Cmp(curveL) >= 0 {
		return false
	}

	// Step 3: Decode R and A
	bigR, ok := pointDecode(rBytes)
	if !ok {
		return false
	}

	bigA, ok := pointDecode(publicKey)
	if !ok {
		return false
	}

	// Step 4: k = SHA-512(R || publicKey || message) mod L
	kInput := make([]byte, 0, 32+32+len(message))
	kInput = append(kInput, rBytes[:]...)
	kInput = append(kInput, publicKey[:]...)
	kInput = append(kInput, message...)
	kHash := sha512.Sum512(kInput)
	k := bytesToBigIntLE(kHash[:])
	k.Mod(k, curveL)

	// Step 5: Check S·B == R + k·A
	lhs := scalarMult(sVal, basePoint())
	rhs := pointAdd(bigR, scalarMult(k, bigA))

	// Compare in projective coordinates: X1·Z2 == X2·Z1 and Y1·Z2 == Y2·Z1
	lxrz := new(big.Int).Mul(lhs.X, rhs.Z)
	lxrz.Mod(lxrz, p)
	rxlz := new(big.Int).Mul(rhs.X, lhs.Z)
	rxlz.Mod(rxlz, p)

	lyrz := new(big.Int).Mul(lhs.Y, rhs.Z)
	lyrz.Mod(lyrz, p)
	rylz := new(big.Int).Mul(rhs.Y, lhs.Z)
	rylz.Mod(rylz, p)

	return lxrz.Cmp(rxlz) == 0 && lyrz.Cmp(rylz) == 0
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 7: HELPERS
// ═══════════════════════════════════════════════════════════════════════════════

// bytesToBigIntLE interprets a byte slice as a little-endian unsigned integer.
func bytesToBigIntLE(b []byte) *big.Int {
	reversed := reverseBytes(b)
	return new(big.Int).SetBytes(reversed)
}

// bigIntToLE32 encodes a big.Int as a 32-byte little-endian array.
func bigIntToLE32(n *big.Int) [32]byte {
	b := n.Bytes() // big-endian
	var result [32]byte
	for i, v := range b {
		result[len(b)-1-i] = v
	}
	return result
}
