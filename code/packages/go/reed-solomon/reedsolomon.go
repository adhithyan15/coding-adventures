// Package reedsolomon implements Reed-Solomon error-correcting codes over GF(256).
//
// Reed-Solomon (RS) is a block error-correcting code invented by Irving Reed
// and Gustave Solomon in 1960.  You add nCheck redundancy bytes to a message;
// the decoder can recover the original even when up to t = nCheck/2 bytes have
// been corrupted in transit.
//
// Where RS codes are used:
//
//   - QR codes: up to 30% of the symbol can be scratched and still decoded.
//   - CDs / DVDs: CIRC two-level RS corrects scratches and burst errors.
//   - Hard drives: firmware sector-level error correction.
//   - Voyager probes: images sent across 20+ billion kilometres.
//   - RAID-6: the two parity drives ARE an (n, n-2) RS code over GF(256).
//
// Building blocks:
//
//	MA00 polynomial   — coefficient-array polynomial arithmetic
//	MA01 gf256        — GF(2^8) field arithmetic (add=XOR, mul=table lookup)
//	MA02 reedsolomon  — RS encoding / decoding (THIS PACKAGE)
//
// # Polynomial Conventions
//
// Codeword bytes are treated as a big-endian polynomial:
//
//	codeword[0]·x^{n-1} + codeword[1]·x^{n-2} + … + codeword[n-1]
//
// The systematic codeword layout:
//
//	[ message bytes (k) | check bytes (nCheck) ]
//	  degree n-1 … nCheck   degree nCheck-1 … 0
//
// For error position p in a big-endian codeword of length n, the locator
// number is X_p = α^{n-1-p} and its inverse is X_p⁻¹ = α^{(p+256-n) mod 255}.
//
// Internal polynomials (generator, Λ, Ω) use little-endian storage
// (index = degree).  Only the codeword bytes use big-endian order.
package reedsolomon

import (
	"errors"
	"fmt"

	"github.com/adhithyan15/coding-adventures/code/packages/go/gf256"
)

// Version of this package.
const Version = "0.1.0"

// =============================================================================
// Error Types
// =============================================================================

// ErrTooManyErrors is returned when decoding fails because the number of
// corrupted bytes exceeds the correction capacity t = nCheck/2.
//
// The code has a hard limit: given nCheck redundancy bytes it can correct
// at most t byte errors.  If more are present the codeword is unrecoverable
// and this error is returned rather than silently producing wrong data.
var ErrTooManyErrors = errors.New("reed-solomon: too many errors — codeword is unrecoverable")

// InvalidInputError is returned when encode or decode receives bad parameters.
//
// Common causes:
//   - nCheck is 0 or odd (must be a positive even integer)
//   - total codeword length exceeds 255 (GF(256) block size limit)
//   - received slice is shorter than nCheck
type InvalidInputError struct {
	Reason string
}

func (e *InvalidInputError) Error() string {
	return fmt.Sprintf("reed-solomon: invalid input — %s", e.Reason)
}

// =============================================================================
// Generator Polynomial
// =============================================================================

// BuildGenerator constructs the RS generator polynomial for nCheck check bytes.
//
// The generator is the product of nCheck linear factors:
//
//	g(x) = (x + α¹)(x + α²)…(x + α^{nCheck})
//
// where α = 2 is the primitive element of GF(256).
//
// # Return Value
//
// A little-endian coefficient slice (index = degree), length nCheck+1.
// The last element is always 1 (monic polynomial).
//
// # Algorithm
//
// Start with g = [1].  At each step multiply in the next linear factor (αⁱ + x):
//
//	for j, coeff := range g:
//	    newG[j]   ^= Multiply(coeff, α^i)   // coeff·α^i
//	    newG[j+1] ^= coeff                  // coeff·x
//
// Example for nCheck=2:
//
//	Start: g = [1]
//	i=1 (α¹=2): g = [2, 1]
//	i=2 (α²=4): g = [8, 6, 1]
//
// Verify α¹=2 is a root:
//
//	g(2) = 8 ⊕ Mul(6,2) ⊕ Mul(1,4) = 8 ⊕ 12 ⊕ 4 = 0  ✓
func BuildGenerator(nCheck int) ([]byte, error) {
	if nCheck == 0 || nCheck%2 != 0 {
		return nil, &InvalidInputError{
			Reason: fmt.Sprintf("nCheck must be a positive even number, got %d", nCheck),
		}
	}

	g := []byte{1}

	for i := 1; i <= nCheck; i++ {
		alphaI := gf256.Power(2, i)
		newG := make([]byte, len(g)+1)
		for j, coeff := range g {
			newG[j] ^= gf256.Multiply(coeff, alphaI) // coeff · α^i
			newG[j+1] ^= coeff                       // coeff · x
		}
		g = newG
	}

	return g, nil
}

// =============================================================================
// Internal Polynomial Helpers
// =============================================================================

// polyEvalBE evaluates a big-endian GF(256) polynomial at x using Horner's method.
//
// p[0] is the highest-degree coefficient.  Iteration goes left to right:
//
//	acc = 0
//	for each byte b in p:
//	    acc = acc·x + b    (GF(256) arithmetic)
//
// Used for syndrome evaluation: S_j = polyEvalBE(codeword, α^j).
func polyEvalBE(p []byte, x byte) byte {
	var acc byte
	for _, b := range p {
		acc = gf256.Add(gf256.Multiply(acc, x), b)
	}
	return acc
}

// polyEvalLE evaluates a little-endian GF(256) polynomial at x using Horner's method.
//
// p[i] is the coefficient of x^i.  Iteration goes from highest to lowest degree:
//
//	acc = 0
//	for i from len(p)-1 down to 0:
//	    acc = acc·x + p[i]
//
// Used for evaluating Λ(x), Ω(x), and Λ'(x) in Chien search / Forney.
func polyEvalLE(p []byte, x byte) byte {
	var acc byte
	for i := len(p) - 1; i >= 0; i-- {
		acc = gf256.Add(gf256.Multiply(acc, x), p[i])
	}
	return acc
}

// polyMulLE multiplies two little-endian GF(256) polynomials (convolution).
//
// The result has degree deg(a)+deg(b), length len(a)+len(b)-1.
//
//	result[i+j] ^= a[i]·b[j]   for all i, j
//
// In GF(256), addition is XOR, so ^= is correct.
// Used in Forney to compute Ω(x) = S(x)·Λ(x) mod x^{2t}.
func polyMulLE(a, b []byte) []byte {
	if len(a) == 0 || len(b) == 0 {
		return nil
	}
	result := make([]byte, len(a)+len(b)-1)
	for i, ai := range a {
		for j, bj := range b {
			result[i+j] ^= gf256.Multiply(ai, bj)
		}
	}
	return result
}

// polyModBE computes the remainder of big-endian GF(256) polynomial long division.
//
// Both dividend and divisor are big-endian (first = highest degree).
// The divisor must be monic (leading coefficient = 1) — guaranteed because
// the generator polynomial is always monic.
//
// Algorithm: schoolbook long division.
//
//	rem := copy(dividend)
//	for i := 0; i < len(dividend)-len(divisor)+1; i++:
//	    coeff := rem[i]
//	    if coeff == 0: continue
//	    for j, d := range divisor:
//	        rem[i+j] ^= coeff·d
//
// The last len(divisor)-1 bytes of rem are the remainder.
// Returns a slice of length len(divisor)-1.
func polyModBE(dividend, divisor []byte) []byte {
	rem := make([]byte, len(dividend))
	copy(rem, dividend)

	divLen := len(divisor)
	if len(rem) < divLen {
		return rem
	}

	steps := len(rem) - divLen + 1
	for i := 0; i < steps; i++ {
		coeff := rem[i]
		if coeff == 0 {
			continue
		}
		for j, d := range divisor {
			rem[i+j] ^= gf256.Multiply(coeff, d)
		}
	}

	return rem[len(rem)-(divLen-1):]
}

// invLocator returns X_p⁻¹ for byte position p in a codeword of length n.
//
// Big-endian convention: position p corresponds to degree n-1-p.
//
//	X_p  = α^{n-1-p}
//	X_p⁻¹ = α^{(p+256-n) mod 255}
//
// The +256 keeps the exponent non-negative (n ≤ 255, 0 ≤ p < n).
func invLocator(p, n int) byte {
	exp := (p + 256 - n) % 255
	return gf256.Power(2, exp)
}

// =============================================================================
// Encoding
// =============================================================================

// Encode encodes message with Reed-Solomon, producing a systematic codeword.
//
// Systematic means the message bytes appear unchanged in the output, followed
// by nCheck check bytes:
//
//	output = [ message bytes (k) | check bytes (nCheck) ]
//
// # Algorithm
//
//  1. Build the generator polynomial g (little-endian).
//  2. Reverse g to big-endian gBE (gLE[last]=1 becomes gBE[0]=1).
//  3. Append nCheck zero bytes: shifted = message || zeros.
//     This is M(x)·x^{nCheck} in big-endian form.
//  4. Remainder R = shifted mod gBE.
//  5. Output: message || R (R padded to exactly nCheck bytes).
//
// # Why It Works
//
// The codeword polynomial is C(x) = M(x)·x^{nCheck} XOR R(x) = Q(x)·g(x),
// so C(αⁱ) = 0 for i=1…nCheck.  This is the property the decoder exploits.
//
// Encode returns an error if nCheck is 0/odd or len(message)+nCheck > 255.
func Encode(message []byte, nCheck int) ([]byte, error) {
	if nCheck == 0 || nCheck%2 != 0 {
		return nil, &InvalidInputError{
			Reason: fmt.Sprintf("nCheck must be a positive even number, got %d", nCheck),
		}
	}
	n := len(message) + nCheck
	if n > 255 {
		return nil, &InvalidInputError{
			Reason: fmt.Sprintf("total codeword length %d exceeds GF(256) block size limit of 255", n),
		}
	}

	// Build generator in LE, then reverse to BE (monic: gBE[0] = 1).
	gLE, _ := BuildGenerator(nCheck) // nCheck already validated
	gBE := make([]byte, len(gLE))
	for i, b := range gLE {
		gBE[len(gLE)-1-i] = b
	}

	// shifted = message || zeros  (big-endian M(x)·x^{nCheck})
	shifted := make([]byte, n)
	copy(shifted, message)
	// trailing nCheck bytes stay 0

	// Remainder of big-endian division by monic gBE.
	remainder := polyModBE(shifted, gBE)

	// Codeword = message || check bytes (pad remainder to nCheck bytes).
	codeword := make([]byte, n)
	copy(codeword, message)
	pad := nCheck - len(remainder)
	copy(codeword[len(message)+pad:], remainder)

	return codeword, nil
}

// =============================================================================
// Syndromes
// =============================================================================

// Syndromes computes the nCheck syndrome values of a received codeword.
//
//	S_j = received(α^j)   for j = 1, 2, …, nCheck
//
// A valid (uncorrupted) codeword satisfies C(αⁱ) = 0 for all i=1…nCheck,
// because C(x) is divisible by g(x) = ∏(x + αⁱ).
//
// If all syndromes are zero the codeword has no errors.  Any non-zero syndrome
// reveals corruption.
func Syndromes(received []byte, nCheck int) []byte {
	s := make([]byte, nCheck)
	for j := 1; j <= nCheck; j++ {
		s[j-1] = polyEvalBE(received, gf256.Power(2, j))
	}
	return s
}

// =============================================================================
// Berlekamp-Massey Algorithm
// =============================================================================

// berlekampMassey finds the shortest LFSR generating the syndrome sequence.
//
// The LFSR connection polynomial Λ(x) is the error locator polynomial.
// Its roots (where Λ(x)=0) are the inverses of the error locators X_k⁻¹.
// Chien search finds those roots to reveal the error positions.
//
// If errors occurred at positions with locators X₁, X₂, …, Xᵥ, then:
//
//	Λ(x) = ∏_{k=1}^{v} (1 - X_k·x)    Λ(0) = 1
//
// Returns (lambda, numErrors) where lambda is LE and numErrors = degree(Λ).
//
// Algorithm (0-based syndrome indexing):
//
//	C = [1], B = [1], L = 0, xShift = 1, bScale = 1
//
//	for n from 0 to 2t-1:
//	    d = S[n] XOR ∑_{j=1}^{L} C[j]·S[n-j]
//	    if d == 0:
//	        xShift++
//	    elif 2L ≤ n:
//	        T = copy(C)
//	        C = C XOR (d/bScale)·x^{xShift}·B
//	        L = n+1-L; B = T; bScale = d; xShift = 1
//	    else:
//	        C = C XOR (d/bScale)·x^{xShift}·B
//	        xShift++
func berlekampMassey(synds []byte) ([]byte, int) {
	twoT := len(synds)

	c := []byte{1} // current Λ (LE)
	b := []byte{1} // previous Λ (LE)
	bigL := 0      // number of errors found so far
	xShift := 1    // iterations since last update
	bScale := byte(1)

	for n := 0; n < twoT; n++ {

		// -------------------------------------------------------------
		// Discrepancy d = S[n] XOR Σ_{j=1}^{L} C[j]·S[n-j]
		// -------------------------------------------------------------
		d := synds[n]
		for j := 1; j <= bigL; j++ {
			if j < len(c) && n >= j {
				d ^= gf256.Multiply(c[j], synds[n-j])
			}
		}

		// -------------------------------------------------------------
		// Update rule
		// -------------------------------------------------------------
		if d == 0 {
			xShift++

		} else if 2*bigL <= n {
			// Found more errors than modelled — grow Λ.
			tSave := make([]byte, len(c))
			copy(tSave, c)

			scale := gf256.Divide(d, bScale)
			targetLen := xShift + len(b)
			if len(c) < targetLen {
				c = append(c, make([]byte, targetLen-len(c))...)
			}
			for k, bk := range b {
				c[xShift+k] ^= gf256.Multiply(scale, bk)
			}

			bigL = n + 1 - bigL
			b = tSave
			bScale = d
			xShift = 1

		} else {
			// Consistent update — adjust without growing the degree.
			scale := gf256.Divide(d, bScale)
			targetLen := xShift + len(b)
			if len(c) < targetLen {
				c = append(c, make([]byte, targetLen-len(c))...)
			}
			for k, bk := range b {
				c[xShift+k] ^= gf256.Multiply(scale, bk)
			}
			xShift++
		}
	}

	return c, bigL
}

// =============================================================================
// Chien Search
// =============================================================================

// chienSearch finds which byte positions are error locations.
//
// Position p is an error location if and only if Λ(X_p⁻¹) = 0, where:
//
//	X_p⁻¹ = α^{(p+256-n) mod 255}  (invLocator(p, n))
//
// We test all n positions and collect matches.
//
// Correctness: Λ(x) = ∏_k (1 - X_k·x).  This evaluates to zero when
// x = X_k⁻¹ for each error locator X_k.
func chienSearch(lambda []byte, n int) []int {
	positions := make([]int, 0)
	for p := 0; p < n; p++ {
		xiInv := invLocator(p, n)
		if polyEvalLE(lambda, xiInv) == 0 {
			positions = append(positions, p)
		}
	}
	return positions
}

// =============================================================================
// Forney Algorithm
// =============================================================================

// forney computes error magnitudes from known error positions.
//
// For each error at position p:
//
//	e_p = Ω(X_p⁻¹) / Λ'(X_p⁻¹)
//
// where:
//   - Ω(x) = (S(x)·Λ(x)) mod x^{2t}  — error evaluator polynomial
//   - S(x) = S₁ + S₂x + … + S_{2t}x^{2t-1}  — syndrome polynomial (LE)
//   - Λ'(x) — formal derivative of Λ in GF(2^8)
//
// # Formal Derivative in Characteristic 2
//
// In GF(2^8), 2=0, so even-degree terms vanish after differentiation:
//
//	Λ'(x) = Λ₁ + Λ₃x² + Λ₅x⁴ + …
//
// Only odd-indexed Λ coefficients survive; their index is reduced by 1.
//
// # Why This Works
//
// The key identity is S(x)·Λ(x) ≡ Ω(x) (mod x^{2t}).  Forney's formula
// recovers error magnitudes by differentiating Λ and dividing.
func forney(lambda, synds []byte, positions []int, n int) ([]byte, error) {
	twoT := len(synds)

	// Ω = S(x)·Λ(x) mod x^{2t}: truncate to first 2t terms.
	omegaFull := polyMulLE(synds, lambda)
	omega := omegaFull
	if len(omega) > twoT {
		omega = omegaFull[:twoT]
	}

	// Formal derivative Λ'(x): keep odd-indexed coefficients, shift down.
	// Λ'[j-1] = Λ[j]  for j odd.
	lambdaPrimeLen := len(lambda) - 1
	if lambdaPrimeLen < 0 {
		lambdaPrimeLen = 0
	}
	lambdaPrime := make([]byte, lambdaPrimeLen)
	for j := 1; j < len(lambda); j++ {
		if j%2 == 1 { // odd index — survives formal derivative
			lambdaPrime[j-1] ^= lambda[j]
		}
	}

	magnitudes := make([]byte, len(positions))
	for i, pos := range positions {
		xiInv := invLocator(pos, n)
		omegaVal := polyEvalLE(omega, xiInv)
		lpVal := polyEvalLE(lambdaPrime, xiInv)
		if lpVal == 0 {
			return nil, ErrTooManyErrors
		}
		magnitudes[i] = gf256.Divide(omegaVal, lpVal)
	}

	return magnitudes, nil
}

// =============================================================================
// Public API
// =============================================================================

// ErrorLocator computes the error locator polynomial Λ(x) from a syndrome slice.
//
// Runs Berlekamp-Massey and returns Λ in little-endian form with Λ[0]=1.
// Exposed for advanced use (QR decoders, diagnostics).
func ErrorLocator(synds []byte) []byte {
	lambda, _ := berlekampMassey(synds)
	return lambda
}

// Decode decodes a received Reed-Solomon codeword, correcting up to t = nCheck/2 errors.
//
// # Five-Step Pipeline
//
//  1. Compute syndromes S₁…S_{nCheck}.  All zero → return message directly.
//  2. Berlekamp-Massey → Λ(x), error count L.  L > t → ErrTooManyErrors.
//  3. Chien search → error positions {p₁…pᵥ}.  |positions| ≠ L → ErrTooManyErrors.
//  4. Forney → error magnitudes {e₁…eᵥ}.
//  5. Correct: received[p_k] ^= e_k for each k.
//
// Returns the recovered message (length = len(received) - nCheck).
//
// Errors:
//   - *InvalidInputError if nCheck is 0/odd or received is shorter than nCheck.
//   - ErrTooManyErrors if more than t errors are present.
func Decode(received []byte, nCheck int) ([]byte, error) {
	if nCheck == 0 || nCheck%2 != 0 {
		return nil, &InvalidInputError{
			Reason: fmt.Sprintf("nCheck must be a positive even number, got %d", nCheck),
		}
	}
	if len(received) < nCheck {
		return nil, &InvalidInputError{
			Reason: fmt.Sprintf("received length %d < nCheck %d", len(received), nCheck),
		}
	}

	t := nCheck / 2
	n := len(received)
	k := n - nCheck

	// Step 1: Syndromes
	synds := Syndromes(received, nCheck)
	allZero := true
	for _, s := range synds {
		if s != 0 {
			allZero = false
			break
		}
	}
	if allZero {
		return append([]byte(nil), received[:k]...), nil
	}

	// Step 2: Berlekamp-Massey
	lambda, numErrors := berlekampMassey(synds)
	if numErrors > t {
		return nil, ErrTooManyErrors
	}

	// Step 3: Chien Search
	positions := chienSearch(lambda, n)
	if len(positions) != numErrors {
		return nil, ErrTooManyErrors
	}

	// Step 4: Forney
	magnitudes, err := forney(lambda, synds, positions, n)
	if err != nil {
		return nil, err
	}

	// Step 5: Apply corrections
	corrected := make([]byte, n)
	copy(corrected, received)
	for i, pos := range positions {
		corrected[pos] ^= magnitudes[i]
	}

	return corrected[:k], nil
}
