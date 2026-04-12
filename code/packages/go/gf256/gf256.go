// Package gf256 provides Galois Field GF(2^8) arithmetic.
//
// GF(256) is the finite field with 256 elements (byte values 0..255).
// Arithmetic uses the primitive polynomial:
//
//	p(x) = x^8 + x^4 + x^3 + x^2 + 1  =  0x11D  =  285
//
// Applications:
//   - Reed-Solomon error correction (QR codes, CDs, hard drives)
//   - AES encryption (SubBytes and MixColumns steps)
//   - General error-correcting codes
//
// Key insight: In GF(2^8), addition IS XOR. Subtraction equals addition.
// Multiplication uses precomputed log/antilog tables for O(1) performance.
//
// Layer MA01 in the coding-adventures math stack.
// Enables MA02 (reed-solomon) and MA03 (qr-encoder).
package gf256

// Version of this package.
const Version = "0.1.0"

// GF256 is a byte representing a GF(2^8) field element (0..255).
type GF256 = byte

// PrimitivePolynomial is the irreducible polynomial used for modular reduction.
//
//	p(x) = x^8 + x^4 + x^3 + x^2 + 1
//	Binary: 1_0001_1101 = 0x11D = 285
//
// This polynomial is both irreducible (ensuring every non-zero element has an
// inverse) and primitive (the element g=2 generates the full multiplicative
// group of order 255).
const PrimitivePolynomial = 0x11D

// log and alog tables are built by init() at program startup.
var log [256]byte // log[x] = i such that g^i = x (log[0] unused)
// alog has 256 entries: indices 0..254 hold g^i, and alog[255] = 1
// because the multiplicative group has order 255: g^255 = g^0 = 1.
// This allows Inverse(1) = alog[255-log[1]] = alog[255] = 1 to work.
var alog [256]int

func init() {
	// Build the ALOG (antilogarithm) and LOG (logarithm) tables.
	//
	// Algorithm:
	//   Start with val = 1.
	//   Each step: multiply by 2 (left shift 1 bit).
	//   If bit 8 is set (val >= 256), XOR with 0x11D to reduce.
	//
	// Why shift-left = multiply by 2?
	//   In GF(2^8), elements are polynomials over GF(2). Multiplying by "2"
	//   (the polynomial x) shifts all coefficients up by one degree, which
	//   is a left bit-shift. Overflow means degree-8 coefficient became 1,
	//   so we reduce modulo p(x) by XOR-ing with 0x11D.
	val := 1
	for i := 0; i < 255; i++ {
		alog[i] = val
		log[val] = byte(i)
		val <<= 1
		if val >= 256 {
			val ^= PrimitivePolynomial
		}
	}
	// alog[255] = 1: g^255 = g^0 = 1 (group order 255, wraps around).
	// Critical for Inverse(1): 255 - log[1] = 255 - 0 = 255 → alog[255] = 1.
	alog[255] = 1
}

// LOG returns the discrete logarithm table (256 entries).
// LOG[x] = i such that 2^i = x in GF(256).
// LOG[0] is undefined (zero has no logarithm); its value is 0 by default.
func LOG() [256]byte { return log }

// ALOG returns the antilogarithm table (256 entries; ALOG[255] = ALOG[0] = 1).
// ALOG[i] = 2^i in GF(256).
func ALOG() [256]int { return alog }

// Add adds two GF(256) elements: returns a XOR b.
//
// In characteristic 2, addition is XOR. Each bit represents a GF(2)
// coefficient, and 1+1=0 (mod 2). No carry, no tables needed.
// Every element is its own additive inverse: Add(x, x) = 0.
func Add(a, b GF256) GF256 {
	return a ^ b
}

// Subtract subtracts two GF(256) elements: returns a XOR b.
//
// In characteristic 2, -1 = 1, so subtraction equals addition.
// This identity simplifies error-correction polynomial computations.
func Subtract(a, b GF256) GF256 {
	return a ^ b
}

// Multiply multiplies two GF(256) elements using log/antilog tables.
//
//	a × b = ALOG[(LOG[a] + LOG[b]) mod 255]
//
// Special case: if either operand is 0, the result is 0.
// (Zero has no logarithm; the formula does not apply.)
//
// Time complexity: O(1) — two table lookups and one addition.
func Multiply(a, b GF256) GF256 {
	if a == 0 || b == 0 {
		return 0
	}
	return byte(alog[(int(log[a])+int(log[b]))%255])
}

// Divide divides a by b in GF(256).
//
//	a / b = ALOG[(LOG[a] - LOG[b] + 255) mod 255]
//
// The +255 before % 255 ensures a non-negative result.
// Special case: a = 0 → 0.
//
// Panics if b is 0 (division by zero is undefined in any field).
func Divide(a, b GF256) GF256 {
	if b == 0 {
		panic("gf256: division by zero")
	}
	if a == 0 {
		return 0
	}
	return byte(alog[(int(log[a])-int(log[b])+255)%255])
}

// Power raises a GF(256) element to a non-negative integer power.
//
//	base^exp = ALOG[(LOG[base] * exp) mod 255]
//
// The % 255 reflects the order of the multiplicative group.
// Special cases: 0^0 = 1 by convention; 0^n = 0 for n > 0.
func Power(base GF256, exp int) GF256 {
	if base == 0 {
		if exp == 0 {
			return 1
		}
		return 0
	}
	if exp == 0 {
		return 1
	}
	idx := (int(log[base]) * exp) % 255
	if idx < 0 {
		idx += 255
	}
	return byte(alog[idx])
}

// Inverse returns the multiplicative inverse of a GF(256) element.
//
//	a × Inverse(a) = 1
//	Inverse(a) = ALOG[255 - LOG[a]]
//
// Panics if a is 0 (zero has no multiplicative inverse).
func Inverse(a GF256) GF256 {
	if a == 0 {
		panic("gf256: zero has no multiplicative inverse")
	}
	return byte(alog[255-int(log[a])])
}

// Zero returns the additive identity (0).
func Zero() GF256 { return 0 }

// One returns the multiplicative identity (1).
func One() GF256 { return 1 }

// =============================================================================
// Field — parameterizable GF(2^8) field
// =============================================================================
//
// The functions above are fixed to the Reed-Solomon polynomial 0x11D.
// AES uses the polynomial 0x11B. Rather than inlining a second set of tables,
// Field encapsulates a primitive polynomial with its own log/alog tables.
//
// Usage:
//
//	aesField := gf256.NewField(0x11B)
//	aesField.Multiply(0x53, 0x8C)  // → 1
//	aesField.Inverse(0x53)          // → 0x8C
//
// The module-level functions remain the canonical Reed-Solomon API.

// Field holds precomputed log/antilog tables for a single primitive polynomial.
// Create instances with NewField; zero values are invalid.
type Field struct {
	// PrimitivePoly is the irreducible polynomial used to build this field.
	PrimitivePoly int

	log  [256]byte
	alog [256]int
}

// NewField constructs a GF(2^8) Field for the given primitive polynomial.
//
// primitivePoly is the degree-8 irreducible polynomial as an integer, e.g.:
//
//	0x11D  — Reed-Solomon (same as the module-level polynomial)
//	0x11B  — AES (x^8 + x^4 + x^3 + x + 1)
//
// The log/alog tables are built during construction and reused for all
// subsequent operations. Construction is O(256); all operations are O(1).
func NewField(primitivePoly int) *Field {
	f := &Field{PrimitivePoly: primitivePoly}
	val := 1
	for i := 0; i < 255; i++ {
		f.alog[i] = val
		f.log[val] = byte(i)
		val <<= 1
		if val >= 256 {
			val ^= primitivePoly
		}
	}
	// alog[255] = 1: the multiplicative group has order 255, so g^255 = g^0 = 1.
	f.alog[255] = 1
	return f
}

// Add adds two GF(256) elements: a XOR b.
// Addition is polynomial-independent in GF(2^8); included for API symmetry.
func (f *Field) Add(a, b GF256) GF256 { return a ^ b }

// Subtract subtracts two GF(256) elements: a XOR b (same as Add).
func (f *Field) Subtract(a, b GF256) GF256 { return a ^ b }

// Multiply multiplies two GF(256) elements using this field's log/alog tables.
func (f *Field) Multiply(a, b GF256) GF256 {
	if a == 0 || b == 0 {
		return 0
	}
	return byte(f.alog[(int(f.log[a])+int(f.log[b]))%255])
}

// Divide divides a by b in this GF(256) field.
// Panics if b is 0.
func (f *Field) Divide(a, b GF256) GF256 {
	if b == 0 {
		panic("gf256.Field: division by zero")
	}
	if a == 0 {
		return 0
	}
	return byte(f.alog[(int(f.log[a])-int(f.log[b])+255)%255])
}

// Power raises a GF(256) element to a non-negative integer power.
func (f *Field) Power(base GF256, exp int) GF256 {
	if base == 0 {
		if exp == 0 {
			return 1
		}
		return 0
	}
	if exp == 0 {
		return 1
	}
	idx := (int(f.log[base]) * exp) % 255
	if idx < 0 {
		idx += 255
	}
	return byte(f.alog[idx])
}

// Inverse returns the multiplicative inverse of a in this GF(256) field.
// Panics if a is 0.
func (f *Field) Inverse(a GF256) GF256 {
	if a == 0 {
		panic("gf256.Field: zero has no multiplicative inverse")
	}
	return byte(f.alog[255-int(f.log[a])])
}
