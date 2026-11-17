// Package trig provides trigonometric functions built from first principles.
//
// # One algorithm, every language (TRIG-REF-1)
//
// This package is the Go port of the reference algorithm of PHY00a
// (code/specs/PHY00a-trig-convergence.md). Every port of trig, in every
// language of this repository, performs the same sequence of IEEE binary64
// operations, so that Sin(x) is the same 64 bits everywhere. The shared vectors in
// code/specs/fixtures/trig-reference-v1 check that bit for bit.
//
// Every public function is still wrapped in an Operation, which gives each call
// automatic timing, structured logging and panic recovery.
//
// # The Core Idea: Maclaurin Series, After Range Reduction
//
// A Maclaurin series expresses a function as an infinite sum computed from its
// derivatives at zero:
//
//	sin(x) = x - x^3/3! + x^5/5! - x^7/7! + ...
//	cos(x) = 1 - x^2/2! + x^4/4! - x^6/6! + ...
//
// The terms shrink fast when |x| is small, so we first *reduce* the argument:
// sin and cos repeat every quarter turn (pi/2) up to a sign swap, so we find how
// many quarter turns fit in x, subtract them with a very accurate pi/2 (the
// Cody-Waite method), and run the series on what is left, which is at most about
// pi/4. Then ten terms of the series, summed by Horner's rule, are more than
// double precision can resolve.
//
// # The domain
//
// Sin, Cos and Tan are defined for |x| <= 2^29 (about 5.4e8 radians). Beyond that,
// and for NaN and the infinities, they return NaN: the reduction is only exact up
// to there, and a finite wrong answer would be worse than none. A caller with a
// phase that grows without bound must reduce it first (see go/oscillator).
//
// # No fused multiply-add
//
// The algorithm is defined with one rounding per operation. Go is allowed by its
// specification to fuse x*y + z into a single rounding (it does so on arm64,
// ppc64, s390x and riscv64) and there is no flag to forbid it. The one thing the
// specification guarantees is that an explicit conversion rounds: float64(x*y) is
// always a rounded product. So every product that feeds an addition or
// subtraction below is written float64(x*y). Do not remove those conversions: on
// arm64 the results change, and CI checks the generated assembly for FMADD.
package trig

import "math"

// ============================================================================
// Constants (PHY00a 3.2)
// ============================================================================
//
// Every constant is pinned by its IEEE bit pattern and built with
// math.Float64frombits. We never combine them in constant expressions: Go
// evaluates those exactly, with arbitrary precision, at compile time, so a
// product of two constants would not be rounded the way the algorithm says. A
// test asserts the bits of every one of these against the shared manifest.

// PI is the ratio of a circle's circumference to its diameter (0x400921FB54442D18).
const PI = 3.141592653589793

// TwoPI is the full period of sine and cosine (0x401921FB54442D18).
const TwoPI = 2 * PI

// halfPI is pi/2 (0x3FF921FB54442D18), used by atan and atan2.
var halfPI = math.Float64frombits(0x3FF921FB54442D18)

var (
	// invPio2 is the double nearest 2/pi.
	invPio2 = math.Float64frombits(0x3FE45F306DC9C883)

	// The first four pieces of pi/2 have at most 24 significant bits each, so
	// the product k*piece is EXACT whenever |k| < 2^29 (24 + 29 = 53 bits).
	// The fifth is the remainder rounded to 53 bits. Together they carry pi/2 to
	// 149 bits.
	pio2P1 = math.Float64frombits(0x3FF921FB40000000)
	pio2P2 = math.Float64frombits(0x3E74442D00000000)
	pio2P3 = math.Float64frombits(0x3CF8469880000000)
	pio2P4 = math.Float64frombits(0x3B78CC5160000000)
	pio2P5 = math.Float64frombits(0x39F01B839A25204A)

	// magic is 1.5 * 2^52. (t + magic) - magic rounds t to the nearest whole
	// number (ties to even) using only an add and a subtract.
	magic = math.Float64frombits(0x4338000000000000)

	// limit is 2^29: the largest |x| that sin, cos and tan accept.
	limit = math.Float64frombits(0x41C0000000000000)

	// tiny is 2^-27: below it sin(x) == x and cos(x) == 1 in binary64.
	tiny = math.Float64frombits(0x3E40000000000000)

	// radPerDeg is pi/180 and degPerRad is 180/pi, each rounded once.
	radPerDeg = math.Float64frombits(0x3F91DF46A2529D39)
	degPerRad = math.Float64frombits(0x404CA5DC1A63C1F8)
)

// sinCoef[i] is S[i+1] = (-1)^(i+1) / (2i+3)!, the Maclaurin coefficients of
// sin(r) = r + r*z*(S1 + S2*z + ... + S9*z^8), with z = r*r. Reading the table
// you can see the factorials:
//
//	S1 = -1/3!   S2 = +1/5!   S3 = -1/7!   S4 = +1/9!   S5 = -1/11!
//	S6 = +1/13!  S7 = -1/15!  S8 = +1/17!  S9 = -1/19!
var sinCoef = [9]float64{
	math.Float64frombits(0xBFC5555555555555),
	math.Float64frombits(0x3F81111111111111),
	math.Float64frombits(0xBF2A01A01A01A01A),
	math.Float64frombits(0x3EC71DE3A556C734),
	math.Float64frombits(0xBE5AE64567F544E4),
	math.Float64frombits(0x3DE6124613A86D09),
	math.Float64frombits(0xBD6AE7F3E733B81F),
	math.Float64frombits(0x3CE952C77030AD4A),
	math.Float64frombits(0xBC62F49B46814157),
}

// cosCoef[i] is C[i+2] = (-1)^i / (2i+4)!, the coefficients of
// cos(r) = 1 - z/2 + z^2*(C2 + C3*z + ... + C9*z^7). (The -1/2 term is applied
// as 0.5*z, which is exact.)
//
//	C2 = +1/4!  C3 = -1/6!  C4 = +1/8!  C5 = -1/10!
//	C6 = +1/12! C7 = -1/14! C8 = +1/16! C9 = -1/18!
var cosCoef = [8]float64{
	math.Float64frombits(0x3FA5555555555555),
	math.Float64frombits(0xBF56C16C16C16C17),
	math.Float64frombits(0x3EFA01A01A01A01A),
	math.Float64frombits(0xBE927E4FB7789F5C),
	math.Float64frombits(0x3E21EED8EFF8D898),
	math.Float64frombits(0xBDA93974A8C07C9D),
	math.Float64frombits(0x3D2AE7F3E733B81F),
	math.Float64frombits(0xBCA6827863B97D97),
}

// ============================================================================
// Range Reduction (PHY00a 3.4)
// ============================================================================

// reduce splits x (with |x| <= 2^29) into a quarter-turn count and a remainder:
//
//	x = k * (pi/2) + r,   |r| <= about pi/4,   quadrant = k mod 4
//
// Step 1, the quotient. t = x * (2/pi) is roughly "how many quarter turns".
// Adding magic and subtracting it again rounds t to the nearest whole number k
// with nothing but + and -: no floor, no round, no float-to-int conversion, hence
// no behaviour to worry about for NaN or out-of-range values.
//
// Step 2, the remainder, by Cody-Waite subtraction. pi/2 is irrational, so
// k*(pi/2) cannot be formed exactly in 53 bits. Instead pi/2 is stored as a sum
// of five pieces, and each of the first four is so short (24 bits) that k times it
// is exact. We subtract the slices one at a time:
//
//	x ---------------------------------------------------------- the angle
//	|-- k*pio2P1 --|                                              24 bits of k*pi/2
//	               |-- k*pio2P2 --|                               next 24
//	                              |-- k*pio2P3 --|                next 24
//	                                             |- k*pio2P4 -|   next 24
//	                                                          |k*pio2P5| last, rounds
//	r = what is left
//
// The first two subtractions are exact; the last three round (PHY00a 3.4).
//
// Every product below feeds an addition or subtraction, so each is wrapped in
// float64(): on arm64 Go would otherwise fuse "r - k*piece" into one FMSUB and
// the last bits of r would differ from every other language. The same goes for
// t + magic.
func reduce(x float64) (k, r float64) {
	t := float64(x * invPio2)
	k = (t + magic) - magic
	r = x - float64(k*pio2P1)
	r = r - float64(k*pio2P2)
	r = r - float64(k*pio2P3)
	r = r - float64(k*pio2P4)
	r = r - float64(k*pio2P5)
	return k, r
}

// ============================================================================
// The series kernels (PHY00a 3.5)
// ============================================================================

// sinKernel computes sin(r) for |r| <= pi/4 by Horner's rule on z = r*r:
//
//	sin(r) = r + (r*z) * (S1 + z*(S2 + z*(... + z*S9)))
//
// Horner adds the smallest terms first, which is the accurate order, and needs no
// division. The leading r is added last and unrounded in the polynomial, so the
// big part of the answer carries a single rounding.
func sinKernel(r float64) float64 {
	z := float64(r * r)
	p := sinCoef[8]
	for n := 7; n >= 0; n-- {
		p = float64(p*z) + sinCoef[n]
	}
	w := float64(r * z)
	w = float64(w * p)
	return r + w
}

// cosKernel computes cos(r) for |r| <= pi/4:
//
//	cos(r) = (1 - z/2) + z^2 * (C2 + z*(C3 + z*(... + z*C9)))
//
// 0.5*z is exact, and (1 - z/2) is the dominant part, added to the much smaller
// correction last.
func cosKernel(r float64) float64 {
	z := float64(r * r)
	q := cosCoef[7]
	for n := 6; n >= 0; n-- {
		q = float64(q*z) + cosCoef[n]
	}
	u := float64(0.5 * z)
	u = 1.0 - u
	v := float64(z * z)
	v = float64(v * q)
	return u + v
}

// ============================================================================
// Sin and Cos (PHY00a 3.6)
// ============================================================================

// Sin computes the sine of x (in radians).
//
// # Special values
//
//	NaN, +-Inf, |x| > 2^29   NaN  (outside the domain)
//	+0, -0                   +0, -0 (the sign of zero is kept)
//	|x| < 2^-27              x itself (the next series term is below half an ulp)
//
// # The quadrant table
//
// With k the nearest whole number of quarter turns and r the remainder,
// sin(r + k*pi/2) is one of four things depending on k mod 4:
//
//	k mod 4 | 0       | 1       | 2        | 3
//	sin     | sin(r)  | cos(r)  | -sin(r)  | -cos(r)
//
// "k & 3" on a two's-complement integer is the floor modulus, so k = -1 gives 3.
//
// # Accuracy
//
// Within 3 ulp of the correctly rounded sine on |x| <= 2^29 (measured worst 2.38).
func Sin(x float64) float64 {
	result, _ := StartNew[float64]("trig.Sin", 0,
		func(op *Operation[float64], rf *ResultFactory[float64]) *OperationResult[float64] {
			op.AddProperty("x", x)

			// The negated comparison is deliberate: a NaN fails every
			// comparison, so this one test rejects NaN, both infinities and
			// every finite value beyond the limit. Writing "ax > limit" would
			// let a NaN through.
			ax := x
			if ax < 0 {
				ax = -ax
			}
			if !(ax <= limit) {
				return rf.Generate(true, false, math.NaN())
			}
			if ax < tiny {
				return rf.Generate(true, false, x)
			}

			k, r := reduce(x)
			var v float64
			switch int64(k) & 3 {
			case 0:
				v = sinKernel(r)
			case 1:
				v = cosKernel(r)
			case 2:
				v = -sinKernel(r)
			default:
				v = -cosKernel(r)
			}
			return rf.Generate(true, false, v)
		}).GetResult()
	return result
}

// Cos computes the cosine of x (in radians).
//
// Special values mirror [Sin]: NaN outside |x| <= 2^29, and exactly 1 for
// |x| < 2^-27. The quadrant table is:
//
//	k mod 4 | 0       | 1        | 2        | 3
//	cos     | cos(r)  | -sin(r)  | -cos(r)  | sin(r)
//
// # The Pythagorean Identity
//
// sin^2(x) + cos^2(x) = 1 holds for every x, which the tests use as a sanity
// check on both functions.
func Cos(x float64) float64 {
	result, _ := StartNew[float64]("trig.Cos", 0,
		func(op *Operation[float64], rf *ResultFactory[float64]) *OperationResult[float64] {
			op.AddProperty("x", x)

			ax := x
			if ax < 0 {
				ax = -ax
			}
			if !(ax <= limit) {
				return rf.Generate(true, false, math.NaN())
			}
			if ax < tiny {
				return rf.Generate(true, false, 1.0)
			}

			k, r := reduce(x)
			var v float64
			switch int64(k) & 3 {
			case 0:
				v = cosKernel(r)
			case 1:
				v = -sinKernel(r)
			case 2:
				v = -cosKernel(r)
			default:
				v = sinKernel(r)
			}
			return rf.Generate(true, false, v)
		}).GetResult()
	return result
}

// ============================================================================
// Angle Conversion (PHY00a 3.7)
// ============================================================================

// Radians converts an angle from degrees to radians with ONE multiplication by
// the stored constant pi/180.
//
// A full circle is 360 degrees or 2*pi radians, so radians = degrees * (pi/180).
// One rounding (not the two of "deg*PI/180"), no intermediate that overflows, and
// better accuracy (worst 0.65 ulp). NaN propagates, infinities map to
// infinities, and zero keeps its sign.
//
//	Radians(0)   = 0
//	Radians(90)  = pi/2
//	Radians(180) = pi
func Radians(deg float64) float64 {
	result, _ := StartNew[float64]("trig.Radians", 0,
		func(op *Operation[float64], rf *ResultFactory[float64]) *OperationResult[float64] {
			op.AddProperty("deg", deg)
			return rf.Generate(true, false, deg*radPerDeg)
		}).GetResult()
	return result
}

// Degrees converts an angle from radians to degrees with ONE multiplication by
// the stored constant 180/pi. It is the inverse of [Radians].
//
//	Degrees(0)    = 0
//	Degrees(pi/2) = 90
//	Degrees(pi)   = 180
func Degrees(rad float64) float64 {
	result, _ := StartNew[float64]("trig.Degrees", 0,
		func(op *Operation[float64], rf *ResultFactory[float64]) *OperationResult[float64] {
			op.AddProperty("rad", rad)
			return rf.Generate(true, false, rad*degPerRad)
		}).GetResult()
	return result
}

// ============================================================================
// Sqrt — Newton's (Babylonian) Method
// ============================================================================

// Sqrt computes the square root of x using Newton's iterative method.
//
// # Newton's Method
//
// The Babylonian method, known for over 3,000 years, says:
//
//	If `guess` approximates sqrt(x), then (guess + x/guess) / 2 is better.
//
// Intuition: if guess < sqrt(x), then x/guess > sqrt(x). Their average
// "squeezes" both over/underestimates closer together each step.
//
// The method has *quadratic convergence*: each iteration doubles the number
// of correct digits. Convergence table for sqrt(2):
//
//	iter | guess          | digits correct
//	-----|----------------|---------------
//	0    | 2.000000       | 0
//	1    | 1.500000       | 1
//	2    | 1.416667       | 2
//	3    | 1.414216       | 5
//	4    | 1.41421356237  | 11  (full precision)
//
// Typically converges in 10–15 iterations for any normal float64 input.
func Sqrt(x float64) float64 {
	if x < 0 {
		panic("trig.Sqrt: domain error — input is negative")
	}
	result, _ := StartNew[float64]("trig.Sqrt", 0,
		func(op *Operation[float64], rf *ResultFactory[float64]) *OperationResult[float64] {
			op.AddProperty("x", x)

			// sqrt(0) = 0 exactly.
			if x == 0.0 {
				return rf.Generate(true, false, x)
			}
			if x > 1.7976931348623157e308 {
				return rf.Generate(true, false, x)
			}

			scaled := x
			resultScale := 1.0
			for scaled < 0.25 {
				scaled *= 4.0
				resultScale *= 0.5
			}
			for scaled >= 4.0 {
				scaled *= 0.25
				resultScale *= 2.0
			}

			// Initial guess: x itself for large values (saves a few iterations),
			// 1.0 for values in (0, 1) (avoids an expensive first step).
			guess := scaled
			if scaled < 1.0 {
				guess = 1.0
			}

			// Iterate until convergence. The safety cap of 60 is extreme;
			// quadratic convergence means real-world termination in ~15 steps.
			for i := 0; i < 60; i++ {
				// Halving is exact, but the compiler turns "/ 2.0" into "* 0.5" and
				// arm64 would then fuse that product into the subtraction below
				// (harmless numerically, since the product is exact, but it would
				// fail the no-FMA assembly check). The conversion keeps them apart.
				next := float64((guess + scaled/guess) / 2.0)

				// Stop when improvement is below the precision floor.
				// 1e-15*guess handles relative precision for large values.
				// 1e-300 is an absolute floor for subnormal inputs.
				improvement := next - guess
				if improvement < 0 {
					improvement = -improvement
				}
				if improvement < float64(1e-15*guess)+1e-300 {
					return rf.Generate(true, false, next*resultScale)
				}

				guess = next
			}

			return rf.Generate(true, false, guess*resultScale)
		}).GetResult()
	return result
}

// ============================================================================
// Tan — Tangent as Sine / Cosine
// ============================================================================

// Tan computes the tangent of x (in radians).
//
// # Definition
//
// Tangent is the ratio of sine to cosine:
//
//	tan(x) = sin(x) / cos(x)
//
// On the unit circle, a ray at angle x meets the circle at (cos x, sin x),
// and meets the vertical tangent line at x=1 at height sin(x)/cos(x).
// This geometric picture is the origin of the name "tangent."
//
// # Undefined Points (Poles)
//
// tan is undefined at x = π/2 + k·π for any integer k, because cos(x) = 0
// there. Our implementation detects |cos(x)| < 1e-15 and returns the pinned
// sentinel +-1e308 (not an infinity), with the sign of sin(x). That is a legacy
// contract (PHY00 3.6), not an approximation: tan(fl(pi/2)) is about 1.6e16.
//
// Tan has the same domain as [Sin]: NaN for NaN, infinities and |x| > 2^29 (both
// s and c are NaN, the sentinel test is false and NaN/NaN is NaN). No accuracy
// claim is made for Tan (PHY00a 3.8).
//
// We call our own Sin and Cos — not math.Sin or math.Cos.
func Tan(x float64) float64 {
	result, _ := StartNew[float64]("trig.Tan", 0,
		func(op *Operation[float64], rf *ResultFactory[float64]) *OperationResult[float64] {
			op.AddProperty("x", x)

			s := Sin(x) // our own Sin
			c := Cos(x) // our own Cos

			// Guard against division by near-zero cosine.
			// |cos(x)| < 1e-15 means we're within ~1e-15 radians of a pole
			// (x ≈ π/2 + k·π). Return a large finite value with the sign that
			// indicates the direction of divergence.
			absC := c
			if absC < 0 {
				absC = -absC
			}
			if absC < 1e-15 {
				var val float64
				if s > 0 {
					val = 1.0e308
				} else {
					val = -1.0e308
				}
				return rf.Generate(true, false, val)
			}

			return rf.Generate(true, false, s/c)
		}).GetResult()
	return result
}

// ============================================================================
// atan — Arctangent via Taylor Series with Range Reduction
// ============================================================================

// atanCore computes atan for |x| <= 1 using half-angle reduction + Taylor series.
//
// This is an internal helper; users call Atan or Atan2.
//
// Half-angle reduction:
//
//	atan(x) = 2·atan( x / (1 + sqrt(1 + x²)) )
//
// This shrinks |x| <= 1 to |y| <= tan(π/8) ≈ 0.414, where the Taylor
// series atan(t) = t - t³/3 + t⁵/5 - ... converges in ~15 terms.
//
// Iterative term computation:
//
//	term_0 = t
//	term_n = term_{n-1} * (-t²) * (2n-1) / (2n+1)
func atanCore(x float64) float64 {
	// Half-angle: shrink x to a smaller argument.
	// We call our own Sqrt here — no math.Sqrt.
	reduced := x / (1.0 + Sqrt(1.0+float64(x*x)))

	t := reduced
	tSq := t * t
	term := t
	result := t

	for n := 1; n <= 30; n++ {
		// term_n = term_{n-1} * (-t²) * (2n-1) / (2n+1)
		// The ratio (2n-1)/(2n+1) connects consecutive odd-denominator terms:
		//   atan series denominators: 1, 3, 5, 7, ...
		//   consecutive ratio: (2n-1) / (2n+1)
		term = term * (-tSq) * float64(2*n-1) / float64(2*n+1)
		result += term

		// Early exit when term's magnitude is negligibly small.
		absterm := term
		if absterm < 0 {
			absterm = -absterm
		}
		if absterm < 1e-17 {
			break
		}
	}

	// Undo the half-angle halving: atan(x) = 2·atan(reduced).
	return 2.0 * result
}

// Atan computes the arctangent of x (in radians).
//
// # Returns
//
// A value in the open interval (-π/2, π/2).
//
// # Range Reduction
//
// The Taylor series atan(x) = x - x³/3 + x⁵/5 - ... converges only for
// |x| <= 1. For |x| > 1 we use the complementary identity:
//
//	atan(x)  = π/2 - atan(1/x)    for x > 1
//	atan(x)  = -π/2 - atan(1/x)   for x < -1
//
// Inside atan_core, a further half-angle reduction halves the argument,
// giving fast convergence for the Taylor series.
//
// # Examples
//
//	Atan(0)    = 0
//	Atan(1)    = π/4   (45 degrees)
//	Atan(-1)   = -π/4
func Atan(x float64) float64 {
	result, _ := StartNew[float64]("trig.Atan", 0,
		func(op *Operation[float64], rf *ResultFactory[float64]) *OperationResult[float64] {
			op.AddProperty("x", x)

			absX := x
			if absX < 0.0 {
				absX = -absX
			}
			// Atan(x) rounds exactly to x here; avoid halving subnormals and retain -0.
			if absX <= 7.450580596923828e-9 {
				return rf.Generate(true, false, x)
			}
			if x > 1.0 {
				return rf.Generate(true, false, halfPI-atanCore(1.0/x))
			}
			if x < -1.0 {
				return rf.Generate(true, false, -halfPI-atanCore(1.0/x))
			}
			return rf.Generate(true, false, atanCore(x))
		}).GetResult()
	return result
}

// Atan2 computes the four-quadrant arctangent of (y, x).
//
// # Returns
//
// The angle in radians that the point (x, y) makes with the positive x-axis,
// in the range (-π, π].
//
// # Why atan2 and not atan(y/x)?
//
// atan(y/x) only returns angles in (-π/2, π/2) — the right half-plane.
// Crucially, it cannot distinguish (y=1, x=-1) from (y=-1, x=1), because
// both give y/x = ±1. atan2 uses the signs of y and x separately:
//
//	Quadrant I   (x>0, y>0):  atan2 ∈ (0,    π/2)
//	Quadrant II  (x<0, y>0):  atan2 ∈ (π/2,  π  ]
//	Quadrant III (x<0, y<0):  atan2 ∈ (-π,  -π/2)
//	Quadrant IV  (x>0, y<0):  atan2 ∈ (-π/2,  0 )
//
// # Examples
//
//	Atan2(0,  1) = 0       (positive x-axis)
//	Atan2(1,  0) = π/2     (positive y-axis)
//	Atan2(0, -1) = π       (negative x-axis)
//	Atan2(-1, 0) = -π/2    (negative y-axis)
func Atan2(y, x float64) float64 {
	result, _ := StartNew[float64]("trig.Atan2", 0,
		func(op *Operation[float64], rf *ResultFactory[float64]) *OperationResult[float64] {
			op.AddProperty("y", y)
			op.AddProperty("x", x)

			// A NaN argument has no angle. Without this line a NaN x falls
			// through every comparison below to the last branch and returns
			// 0.0, a finite answer for an undefined question (PHY00a 3.9).
			if math.IsNaN(x) || math.IsNaN(y) {
				return rf.Generate(true, false, math.NaN())
			}

			var val float64
			if x > 0.0 {
				val = Atan(y / x)
			} else if x < 0.0 && y >= 0.0 {
				val = Atan(y/x) + PI
			} else if x < 0.0 && y < 0.0 {
				val = Atan(y/x) - PI
			} else if x == 0.0 && y > 0.0 {
				val = halfPI
			} else if x == 0.0 && y < 0.0 {
				val = -halfPI
			} else {
				val = 0.0 // both zero: undefined, return 0 by convention
			}

			return rf.Generate(true, false, val)
		}).GetResult()
	return result
}
