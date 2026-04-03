// Package polynomial provides polynomial arithmetic over float64 coefficients.
//
// A polynomial is represented as a []float64 where the index equals the degree
// of that term's coefficient:
//
//	[]float64{3, 0, 2}  →  3 + 0·x + 2·x²
//	nil or []float64{}  →  the zero polynomial
//
// This "little-endian" representation makes addition position-aligned and
// Horner's method natural to read.
//
// All functions return normalized polynomials — trailing zeros are stripped.
// So []float64{1, 0, 0} and []float64{1} both represent the constant 1.
//
// Layer MA00 in the coding-adventures math stack.
// Enables MA01 (gf256) and MA02 (reed-solomon).
package polynomial

// Version of this package.
const Version = "0.1.0"

// Polynomial is a slice of float64 where index i is the coefficient of x^i.
// The zero polynomial is represented as nil or an empty slice.
type Polynomial = []float64

// Normalize removes trailing zeros from a polynomial.
//
// Trailing zeros represent zero-coefficient high-degree terms. They do not
// change the mathematical value but affect degree comparisons and division.
//
// Example:
//
//	Normalize([]float64{1, 0, 0}) → []float64{1}
//	Normalize([]float64{0})       → []float64{}
//	Normalize(nil)                → []float64{}
func Normalize(p []float64) []float64 {
	n := len(p)
	for n > 0 && p[n-1] == 0 {
		n--
	}
	result := make([]float64, n)
	copy(result, p[:n])
	return result
}

// Degree returns the degree of a polynomial.
//
// The degree is the index of the highest non-zero coefficient.
// By convention, the zero polynomial has degree -1. This sentinel value
// lets polynomial long division terminate cleanly.
//
// Example:
//
//	Degree([]float64{3, 0, 2}) → 2
//	Degree([]float64{7})       → 0
//	Degree(nil)                → -1
func Degree(p []float64) int {
	n := Normalize(p)
	return len(n) - 1
}

// Zero returns the zero polynomial (empty slice).
func Zero() []float64 {
	return []float64{}
}

// One returns the multiplicative identity polynomial [1].
func One() []float64 {
	return []float64{1}
}

// Add adds two polynomials term-by-term.
//
// Shorter polynomial is implicitly zero-padded.
//
// Example:
//
//	Add([]float64{1, 2, 3}, []float64{4, 5}) → []float64{5, 7, 3}
func Add(a, b []float64) []float64 {
	length := len(a)
	if len(b) > length {
		length = len(b)
	}
	result := make([]float64, length)
	for i := range result {
		var ai, bi float64
		if i < len(a) {
			ai = a[i]
		}
		if i < len(b) {
			bi = b[i]
		}
		result[i] = ai + bi
	}
	return Normalize(result)
}

// Subtract subtracts polynomial b from polynomial a term-by-term.
//
// Example:
//
//	Subtract([]float64{5, 7, 3}, []float64{1, 2, 3}) → []float64{4, 5}
func Subtract(a, b []float64) []float64 {
	length := len(a)
	if len(b) > length {
		length = len(b)
	}
	result := make([]float64, length)
	for i := range result {
		var ai, bi float64
		if i < len(a) {
			ai = a[i]
		}
		if i < len(b) {
			bi = b[i]
		}
		result[i] = ai - bi
	}
	return Normalize(result)
}

// Multiply multiplies two polynomials using polynomial convolution.
//
// Each term a[i]·xⁱ of a multiplies each term b[j]·xʲ of b, contributing
// a[i]·b[j] to the result at index i+j.
//
// If a has degree m and b has degree n, the result has degree m+n.
//
// Example:
//
//	Multiply([]float64{1, 2}, []float64{3, 4}) → []float64{3, 10, 8}
//	Because (1+2x)(3+4x) = 3 + 10x + 8x²
func Multiply(a, b []float64) []float64 {
	if len(a) == 0 || len(b) == 0 {
		return []float64{}
	}
	resultLen := len(a) + len(b) - 1
	result := make([]float64, resultLen)
	for i, ai := range a {
		for j, bj := range b {
			result[i+j] += ai * bj
		}
	}
	return Normalize(result)
}

// Divmod performs polynomial long division, returning (quotient, remainder).
//
// Given polynomials a and b (b must not be zero), finds q and r such that:
//
//	a = b * q + r   and   Degree(r) < Degree(b)
//
// Panics if b is the zero polynomial.
func Divmod(a, b []float64) ([]float64, []float64) {
	nb := Normalize(b)
	if len(nb) == 0 {
		panic("polynomial: division by zero polynomial")
	}

	na := Normalize(a)
	degA := len(na) - 1
	degB := len(nb) - 1

	// If dividend has lower degree, quotient is 0, remainder is a.
	if degA < degB {
		return []float64{}, na
	}

	// Mutable copy of remainder.
	rem := make([]float64, len(na))
	copy(rem, na)

	// Quotient coefficients.
	quot := make([]float64, degA-degB+1)
	leadB := nb[degB]
	degRem := degA

	for degRem >= degB {
		leadRem := rem[degRem]
		coeff := leadRem / leadB
		power := degRem - degB
		quot[power] = coeff

		// Subtract coeff * x^power * b from rem.
		for j := 0; j <= degB; j++ {
			rem[power+j] -= coeff * nb[j]
		}

		// Walk back past trailing zeros.
		degRem--
		for degRem >= 0 && rem[degRem] == 0 {
			degRem--
		}
	}

	return Normalize(quot), Normalize(rem)
}

// Divide returns the quotient of Divmod(a, b).
// Panics if b is the zero polynomial.
func Divide(a, b []float64) []float64 {
	q, _ := Divmod(a, b)
	return q
}

// Mod returns the remainder of Divmod(a, b).
// Panics if b is the zero polynomial.
func Mod(a, b []float64) []float64 {
	_, r := Divmod(a, b)
	return r
}

// Evaluate evaluates a polynomial at x using Horner's method.
//
// Horner's method rewrites the polynomial as:
//
//	a₀ + x(a₁ + x(a₂ + ... + x·aₙ))
//
// This requires only n additions and n multiplications — no powers of x.
//
// Example:
//
//	Evaluate([]float64{3, 1, 2}, 4) → 39.0
//	Because 3 + 4 + 2·16 = 39
func Evaluate(p []float64, x float64) float64 {
	n := Normalize(p)
	if len(n) == 0 {
		return 0
	}
	acc := 0.0
	for i := len(n) - 1; i >= 0; i-- {
		acc = acc*x + n[i]
	}
	return acc
}

// GCD computes the greatest common divisor of two polynomials.
//
// Uses the Euclidean algorithm: repeatedly replace (a, b) with (b, a mod b)
// until b is the zero polynomial.
func GCD(a, b []float64) []float64 {
	u := Normalize(a)
	v := Normalize(b)
	for len(v) > 0 {
		r := Mod(u, v)
		u = v
		v = r
	}
	return Normalize(u)
}
