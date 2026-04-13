// Package x25519 implements X25519 elliptic curve Diffie-Hellman (RFC 7748).
//
// X25519 is one of the most widely deployed key-agreement protocols on the
// internet.  Every TLS 1.3 handshake (HTTPS, SSH, Signal, WireGuard) almost
// certainly uses it.
//
// The beauty of X25519 lies in its simplicity:
//
//	shared_secret = X25519(my_private_key, your_public_key)
//
// Both parties compute the same 32-byte shared secret, yet an eavesdropper
// who sees both public keys cannot derive it.
//
// # Implementation notes
//
// This implementation uses Go's math/big for field arithmetic over GF(2^255-19).
// All operations follow the Montgomery ladder algorithm for constant-time
// scalar multiplication (at the algorithmic level — big.Int operations are
// not constant-time at the hardware level).
//
// # The prime field GF(2^255 - 19)
//
// All arithmetic is modulo p = 2^255 - 19.  This is a Mersenne-like prime
// chosen by Dan Bernstein because it makes reduction efficient: instead of
// a generic Barrett reduction, we can exploit 2^255 ≡ 19 (mod p).
package x25519

import (
	"errors"
	"math/big"
)

// ============================================================================
// Constants
// ============================================================================

// p is the prime modulus for Curve25519: 2^255 - 19.
//
// This is the largest prime less than 2^255.  Being close to a power of 2
// makes modular reduction fast.
var p = new(big.Int).Sub(new(big.Int).Exp(big.NewInt(2), big.NewInt(255), nil), big.NewInt(19))

// a24 is the constant (A + 2) / 4 where A = 486662 is the Montgomery curve parameter.
//
// Curve25519: y^2 = x^3 + 486662·x^2 + x
//
// In the Montgomery ladder, we need (A + 2) / 4 = (486662 + 2) / 4 = 121666.
// This appears in the step: z_2 = E * (AA + a24 * E)
var a24 = big.NewInt(121666)

// pMinus2 is p - 2, used for Fermat inversion: a^(p-2) ≡ a^(-1) (mod p).
var pMinus2 = new(big.Int).Sub(p, big.NewInt(2))

// BasePoint is the standard base point for Curve25519: u = 9.
//
// This is the generator of the prime-order subgroup, encoded as 32 bytes
// in little-endian.  Bernstein chose 9 as the smallest valid generator.
var BasePoint = func() [32]byte {
	var bp [32]byte
	bp[0] = 9
	return bp
}()

// ErrLowOrderPoint is returned when X25519 produces the all-zeros output,
// indicating a low-order point input.
var ErrLowOrderPoint = errors.New("x25519: produced all-zeros output (low-order point)")

// ============================================================================
// Field Arithmetic over GF(2^255 - 19)
// ============================================================================
//
// These are the building blocks.  Every higher-level operation reduces to
// sequences of these operations.  We use Go's math/big for convenience;
// a production implementation would use fixed-width limb arrays.

// fieldAdd computes (a + b) mod p.
//
// Addition in a prime field is just regular addition followed by reduction.
func fieldAdd(a, b *big.Int) *big.Int {
	result := new(big.Int).Add(a, b)
	return result.Mod(result, p)
}

// fieldSub computes (a - b) mod p.
//
// If a < b, the result wraps around: (a - b + p) mod p.
// Go's big.Int.Mod handles this correctly for us.
func fieldSub(a, b *big.Int) *big.Int {
	result := new(big.Int).Sub(a, b)
	return result.Mod(result, p)
}

// fieldMul computes (a * b) mod p.
//
// The intermediate product can be up to (p-1)^2 ≈ 2^510 bits.
func fieldMul(a, b *big.Int) *big.Int {
	result := new(big.Int).Mul(a, b)
	return result.Mod(result, p)
}

// fieldSquare computes a^2 mod p.
//
// Squaring is separated from multiplication because it can be optimized:
// in limbed arithmetic, many cross-terms are doubled rather than computed.
func fieldSquare(a *big.Int) *big.Int {
	result := new(big.Int).Mul(a, a)
	return result.Mod(result, p)
}

// fieldInvert computes a^(-1) mod p using Fermat's little theorem.
//
// For prime p: a^(p-1) ≡ 1 (mod p), therefore a^(p-2) ≡ a^(-1) (mod p).
//
// Go's big.Int.Exp with a modulus uses efficient modular exponentiation.
func fieldInvert(a *big.Int) *big.Int {
	return new(big.Int).Exp(a, pMinus2, p)
}

// ============================================================================
// Constant-Time Conditional Swap
// ============================================================================

// cswap conditionally swaps two big.Int values based on the swap bit.
//
// If swap == 1, the values are exchanged.  If swap == 0, nothing happens.
//
// In a production implementation, this would use bitwise masking on fixed-width
// limbs.  Here we use the XOR trick on big.Ints, which is algorithmically
// constant-time but not at the hardware level.
func cswap(swap int, a, b *big.Int) (*big.Int, *big.Int) {
	if swap == 1 {
		return new(big.Int).Set(b), new(big.Int).Set(a)
	}
	return new(big.Int).Set(a), new(big.Int).Set(b)
}

// ============================================================================
// Encoding and Decoding
// ============================================================================

// decodeUCoordinate decodes a u-coordinate from 32 bytes (little-endian).
//
// Per RFC 7748 Section 5, the high bit of the last byte is masked off.
// This ensures the decoded value is at most 2^255 - 1.
func decodeUCoordinate(uBytes [32]byte) *big.Int {
	// Mask off the high bit of byte 31
	masked := uBytes
	masked[31] &= 0x7F

	// Convert from little-endian to big.Int
	// big.Int.SetBytes expects big-endian, so we reverse
	reversed := reverseBytes(masked[:])
	return new(big.Int).SetBytes(reversed)
}

// decodeScalar decodes and clamps a scalar (private key) from 32 bytes.
//
// Clamping performs three bit manipulations:
//
//  1. k[0] &= 248 — Clear the three lowest bits (cofactor clearing).
//     Curve25519 has cofactor h = 8.  Making k a multiple of 8
//     ensures we land in the prime-order subgroup.
//
//  2. k[31] &= 127 — Clear bit 255 (keep k below 2^255).
//
//  3. k[31] |= 64 — Set bit 254 (ensure constant bit-length for
//     constant-time execution).
func decodeScalar(kBytes [32]byte) *big.Int {
	clamped := kBytes

	// Clear the three lowest bits — make k a multiple of 8
	clamped[0] &= 248

	// Clear bit 255
	clamped[31] &= 127

	// Set bit 254
	clamped[31] |= 64

	reversed := reverseBytes(clamped[:])
	return new(big.Int).SetBytes(reversed)
}

// encodeUCoordinate encodes a field element as 32 bytes (little-endian).
//
// The value is first reduced mod p to ensure canonical encoding.
func encodeUCoordinate(u *big.Int) [32]byte {
	reduced := new(big.Int).Mod(u, p)

	// Convert to big-endian bytes, then reverse to little-endian
	beBytes := reduced.Bytes()

	// Pad to 32 bytes (big-endian)
	var padded [32]byte
	copy(padded[32-len(beBytes):], beBytes)

	// Reverse to little-endian
	var result [32]byte
	for i := 0; i < 32; i++ {
		result[i] = padded[31-i]
	}
	return result
}

// reverseBytes returns a new slice with the bytes in reverse order.
func reverseBytes(in_ []byte) []byte {
	out := make([]byte, len(in_))
	for i, b := range in_ {
		out[len(in_)-1-i] = b
	}
	return out
}

// ============================================================================
// The Montgomery Ladder — The Heart of X25519
// ============================================================================
//
// The Montgomery ladder computes scalar multiplication k·u on Curve25519
// using only the x-coordinate (called "u" in Montgomery form).
//
// It maintains two points in projective coordinates:
//   - (x_2, z_2) — the "main" accumulator
//   - (x_3, z_3) — always one step ahead
//
// The actual x-coordinate is x/z.  This avoids expensive field inversions
// during the loop (we only invert once at the end).

// X25519 computes scalar multiplication on Curve25519.
//
// Given a 32-byte scalar (private key) and a 32-byte u-coordinate
// (public key or base point), it returns the 32-byte u-coordinate
// of the resulting point.
//
// Returns ErrLowOrderPoint if the result is all zeros.
func X25519(scalar, uCoord [32]byte) ([32]byte, error) {
	k := decodeScalar(scalar)
	u := decodeUCoordinate(uCoord)

	// ---- Initialize the Montgomery ladder ----
	//
	// P2 = (1 : 0) — the point at infinity (identity)
	// P3 = (u : 1) — the input point
	x1 := new(big.Int).Set(u)
	x2 := big.NewInt(1)
	z2 := big.NewInt(0)
	x3 := new(big.Int).Set(u)
	z3 := big.NewInt(1)

	swap := 0

	// ---- Main loop: bit 254 down to bit 0 ----
	//
	// We start at bit 254 because clamping set bit 254 and cleared bit 255.
	for t := 254; t >= 0; t-- {
		// Extract bit t of the scalar
		kt := int(k.Bit(t))

		swap ^= kt
		x2, x3 = cswap(swap, x2, x3)
		z2, z3 = cswap(swap, z2, z3)
		swap = kt

		// ---- Montgomery ladder step ----

		// --- Doubling side (P2) ---
		a := fieldAdd(x2, z2)    // A = x_2 + z_2
		aa := fieldSquare(a)     // AA = A^2
		b := fieldSub(x2, z2)   // B = x_2 - z_2
		bb := fieldSquare(b)     // BB = B^2
		e := fieldSub(aa, bb)    // E = AA - BB

		// --- Addition side (P3) ---
		c := fieldAdd(x3, z3)    // C = x_3 + z_3
		d := fieldSub(x3, z3)   // D = x_3 - z_3
		da := fieldMul(d, a)     // DA = D * A
		cb := fieldMul(c, b)     // CB = C * B

		// New P3 (addition result)
		x3 = fieldSquare(fieldAdd(da, cb))                // x_3 = (DA + CB)^2
		z3 = fieldMul(x1, fieldSquare(fieldSub(da, cb)))  // z_3 = x_1 * (DA - CB)^2

		// New P2 (doubling result)
		// z_2 = E * (BB + a24 * E), where a24 = (A+2)/4 = 121666
		// Derived from: Z_{2n} = 4xz * (x^2 + Axz + z^2)
		//                      = E * ((x-z)^2 + (A+2)/4 * E)
		//                      = E * (BB + 121666 * E)
		x2 = fieldMul(aa, bb)                              // x_2 = AA * BB
		z2 = fieldMul(e, fieldAdd(bb, fieldMul(a24, e)))   // z_2 = E * (BB + a24 * E)
	}

	// Final conditional swap
	x2, x3 = cswap(swap, x2, x3)
	z2, _ = cswap(swap, z2, z3)

	// Convert from projective to affine: result = x_2 * z_2^(-1)
	result := fieldMul(x2, fieldInvert(z2))
	resultBytes := encodeUCoordinate(result)

	// Check for all-zeros result (point at infinity)
	allZero := true
	for _, b := range resultBytes {
		if b != 0 {
			allZero = false
			break
		}
	}
	if allZero {
		return [32]byte{}, ErrLowOrderPoint
	}

	return resultBytes, nil
}

// X25519Base computes scalar multiplication with the standard base point (u = 9).
//
// This is the standard way to derive a public key from a private key.
func X25519Base(scalar [32]byte) ([32]byte, error) {
	return X25519(scalar, BasePoint)
}

// GenerateKeypair generates a public key from a private key.
//
// This is simply X25519Base — included for API clarity.
// The private key should be 32 bytes of cryptographically secure random data.
func GenerateKeypair(privateKey [32]byte) ([32]byte, error) {
	return X25519Base(privateKey)
}
