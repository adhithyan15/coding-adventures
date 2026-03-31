// Package trig provides trigonometric functions built from first principles.
//
// # Why Build Trig from Scratch?
//
// Every programming language ships with sin() and cos() in its standard library.
// So why would we reimplement them? Because understanding *how* these functions
// work deepens our appreciation for mathematics and computing. The techniques
// here — Taylor/Maclaurin series, range reduction, and iterative term computation
// — are the same ideas used inside real math libraries (just with additional
// optimizations like Chebyshev polynomials and table lookups).
//
// # The Core Idea: Maclaurin Series
//
// A Maclaurin series is a way to express a function as an infinite sum of terms
// computed from the function's derivatives at zero. For sine and cosine:
//
//	sin(x) = x - x^3/3! + x^5/5! - x^7/7! + ...
//	cos(x) = 1 - x^2/2! + x^4/4! - x^6/6! + ...
//
// Each successive term is smaller than the last (for reasonable x), so after
// enough terms the sum converges to the true value. We use 20 terms, which
// gives us double-precision accuracy for inputs near zero.
//
// # Range Reduction
//
// The Maclaurin series converges quickly for small x, but slowly for large x.
// Since sin and cos are periodic (they repeat every 2*pi), we can always reduce
// any input to the range [-pi, pi] without changing the result. This is called
// "range reduction" and it's a critical step in every real trig implementation.
//
// # Operations
//
// Every public function is wrapped in an Operation, giving each call
// automatic timing, structured logging, and panic recovery.
package trig

// ============================================================================
// Constants
// ============================================================================

// PI is the ratio of a circle's circumference to its diameter.
//
// This is one of the most fundamental constants in mathematics. It appears
// everywhere: geometry, trigonometry, calculus, physics, statistics, and more.
//
// We define it to the full precision of a float64 (about 15-16 significant
// digits). This matches the value in Go's standard math package.
const PI = 3.141592653589793

// TwoPI is the full period of sine and cosine.
//
// Sine and cosine are "periodic" functions — they repeat the same pattern
// over and over. The length of one complete cycle is 2*pi radians (360 degrees).
// We precompute this constant to avoid repeated multiplication.
const TwoPI = 2 * PI

// ============================================================================
// Range Reduction
// ============================================================================

// rangeReduce normalizes an angle x (in radians) to the range [-pi, pi].
//
// # Why Is This Necessary?
//
// The Maclaurin series for sin and cos converges fastest when x is close to
// zero. For large values of x (like 1000*pi), the series terms start out
// enormous before cancelling, which causes floating-point precision loss.
//
// Since sin and cos repeat every 2*pi, we can subtract multiples of 2*pi
// to bring x into [-pi, pi] without changing the function's value:
//
//	sin(x) = sin(x - 2*pi*k)  for any integer k
//	cos(x) = cos(x - 2*pi*k)  for any integer k
//
// # How It Works
//
// We use Go's modulo-like approach:
//  1. Divide x by 2*pi to find how many full cycles it contains.
//  2. Subtract those full cycles.
//  3. If the result is still outside [-pi, pi], adjust by one more 2*pi.
//
// This is the same approach used in production math libraries, though they
// use more sophisticated techniques (like Cody-Waite reduction) for extreme
// inputs to minimize rounding error.
func rangeReduce(x float64) float64 {
	// Step 1: Remove full rotations.
	// We compute x mod 2*pi manually using division and truncation.
	//
	// int(x / TwoPI) gives us the number of complete cycles (truncated
	// toward zero). Subtracting those cycles leaves the remainder.
	x = x - TwoPI*float64(int(x/TwoPI))

	// Step 2: Ensure we're in [-pi, pi].
	//
	// After removing full rotations, x is in (-2*pi, 2*pi). We may need
	// one more adjustment to land in [-pi, pi].
	//
	//   If x > pi:  subtract 2*pi  (e.g., 3.5 -> 3.5 - 6.28 = -2.78)
	//   If x < -pi: add 2*pi       (e.g., -3.5 -> -3.5 + 6.28 = 2.78)
	if x > PI {
		x -= TwoPI
	} else if x < -PI {
		x += TwoPI
	}

	return x
}

// ============================================================================
// Sin — The Sine Function
// ============================================================================

// Sin computes the sine of x (in radians) using a Maclaurin series.
//
// # The Maclaurin Series for Sine
//
// The sine function can be expressed as an infinite polynomial:
//
//	sin(x) = x - x^3/3! + x^5/5! - x^7/7! + x^9/9! - ...
//
// Written more compactly:
//
//	sin(x) = sum_{n=0}^{inf} (-1)^n * x^(2n+1) / (2n+1)!
//
// Each term uses odd powers of x (1, 3, 5, 7, ...) and alternates in sign.
//
// # Iterative Term Computation
//
// A naive implementation would compute x^n and n! separately for each term.
// This is wasteful and can overflow for large n. Instead, we compute each
// term from the previous one:
//
//	term_0 = x
//	term_n = term_{n-1} * (-x^2) / ((2n)(2n+1))
//
// This works because:
//
//	term_n / term_{n-1} = (-1) * x^2 / ((2n)(2n+1))
//
// The factor (2n)(2n+1) in the denominator comes from the factorial growth:
// (2n+1)! / (2n-1)! = (2n)(2n+1).
//
// This "iterative term computation" trick avoids computing large factorials
// and keeps every intermediate value small enough for floating-point.
//
// # Example: Sin(pi/6) = 0.5
//
//	x = 0.5236 (pi/6)
//	term 0: +0.5236
//	term 1: -0.0239  (multiply by -x^2 / (2*3))
//	term 2: +0.0003  (multiply by -x^2 / (4*5))
//	...
//	sum converges to 0.5000
func Sin(x float64) float64 {
	result, _ := StartNew[float64]("trig.Sin", 0,
		func(op *Operation[float64], rf *ResultFactory[float64]) *OperationResult[float64] {
			op.AddProperty("x", x)

			// Step 1: Range reduction.
			// Bring x into [-pi, pi] so the series converges quickly.
			x = rangeReduce(x)

			// Step 2: Initialize the series.
			// The first term of the Maclaurin series for sin is just x itself.
			term := x   // Current term in the series: starts at x (the n=0 term)
			sum := term // Running total: accumulates all terms

			// Step 3: Accumulate terms.
			// We compute 20 terms, which is more than enough for double precision.
			// (In practice, the series converges in about 10-12 terms for |x| <= pi,
			// but extra terms cost almost nothing and ensure accuracy.)
			for n := 1; n <= 20; n++ {
				// Compute the ratio between consecutive terms:
				//   term_n = term_{n-1} * (-x^2) / ((2n)(2n+1))
				//
				// The denominator (2n)(2n+1) grows quadratically, so each term
				// shrinks rapidly. The minus sign creates the alternating pattern.
				denom := float64(2*n) * float64(2*n+1)
				term = term * (-x * x) / denom
				sum += term
			}

			return rf.Generate(true, false, sum)
		}).GetResult()
	return result
}

// ============================================================================
// Cos — The Cosine Function
// ============================================================================

// Cos computes the cosine of x (in radians) using a Maclaurin series.
//
// # The Maclaurin Series for Cosine
//
// The cosine function's Maclaurin series uses even powers of x:
//
//	cos(x) = 1 - x^2/2! + x^4/4! - x^6/6! + x^8/8! - ...
//
// Written more compactly:
//
//	cos(x) = sum_{n=0}^{inf} (-1)^n * x^(2n) / (2n)!
//
// # Comparison with Sine
//
// Notice the structural similarity:
//
//	sin: odd powers   (1, 3, 5, 7, ...)   starting with x
//	cos: even powers  (0, 2, 4, 6, ...)   starting with 1
//
// The iterative term computation works the same way, but the ratio between
// consecutive terms is:
//
//	term_n = term_{n-1} * (-x^2) / ((2n-1)(2n))
//
// The denominator factors differ slightly because cosine uses even powers.
//
// # The Pythagorean Identity
//
// One of the most important identities in trigonometry is:
//
//	sin^2(x) + cos^2(x) = 1
//
// This holds for ALL values of x. Our test suite verifies this identity
// as a way to check that both functions are implemented correctly.
func Cos(x float64) float64 {
	result, _ := StartNew[float64]("trig.Cos", 0,
		func(op *Operation[float64], rf *ResultFactory[float64]) *OperationResult[float64] {
			op.AddProperty("x", x)

			// Step 1: Range reduction.
			x = rangeReduce(x)

			// Step 2: Initialize the series.
			// The first term of the Maclaurin series for cos is 1 (the constant term).
			term := 1.0 // Current term: starts at 1 (the n=0 term)
			sum := term // Running total

			// Step 3: Accumulate terms.
			// Same strategy as Sin, but with even-power denominators.
			for n := 1; n <= 20; n++ {
				// The ratio between consecutive cosine terms:
				//   term_n = term_{n-1} * (-x^2) / ((2n-1)(2n))
				//
				// Why (2n-1)(2n)? Because:
				//   (2n)! / (2(n-1))! = (2n-1)(2n)
				denom := float64(2*n-1) * float64(2*n)
				term = term * (-x * x) / denom
				sum += term
			}

			return rf.Generate(true, false, sum)
		}).GetResult()
	return result
}

// ============================================================================
// Angle Conversion
// ============================================================================

// Radians converts an angle from degrees to radians.
//
// # Degrees vs Radians
//
// Degrees and radians are two ways to measure angles:
//   - Degrees: a full circle = 360 degrees (arbitrary, from Babylonian astronomy)
//   - Radians: a full circle = 2*pi radians (natural, based on the circle's geometry)
//
// The conversion formula comes from setting up a proportion:
//
//	degrees / 360 = radians / (2*pi)
//
// Solving for radians:
//
//	radians = degrees * (2*pi / 360) = degrees * (pi / 180)
//
// # Examples
//
//	Radians(0)   = 0
//	Radians(90)  = pi/2  (a right angle)
//	Radians(180) = pi    (a straight line)
//	Radians(360) = 2*pi  (a full circle)
func Radians(deg float64) float64 {
	result, _ := StartNew[float64]("trig.Radians", 0,
		func(op *Operation[float64], rf *ResultFactory[float64]) *OperationResult[float64] {
			op.AddProperty("deg", deg)
			return rf.Generate(true, false, deg*PI/180.0)
		}).GetResult()
	return result
}

// Degrees converts an angle from radians to degrees.
//
// This is the inverse of [Radians]. The conversion formula is:
//
//	degrees = radians * (180 / pi)
//
// # Examples
//
//	Degrees(0)      = 0
//	Degrees(pi/2)   = 90
//	Degrees(pi)     = 180
//	Degrees(2*pi)   = 360
func Degrees(rad float64) float64 {
	result, _ := StartNew[float64]("trig.Degrees", 0,
		func(op *Operation[float64], rf *ResultFactory[float64]) *OperationResult[float64] {
			op.AddProperty("rad", rad)
			return rf.Generate(true, false, rad*180.0/PI)
		}).GetResult()
	return result
}
