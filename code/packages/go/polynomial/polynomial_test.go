package polynomial_test

import (
	"math"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/polynomial"
)

const delta = 1e-9

func polyEqual(a, b []float64) bool {
	if len(a) != len(b) {
		return false
	}
	for i := range a {
		if math.Abs(a[i]-b[i]) > delta {
			return false
		}
	}
	return true
}

// =============================================================================
// Normalize
// =============================================================================

func TestNormalize(t *testing.T) {
	tests := []struct {
		input []float64
		want  []float64
	}{
		{[]float64{1, 0, 0}, []float64{1}},
		{[]float64{0}, []float64{}},
		{[]float64{0, 0, 0}, []float64{}},
		{[]float64{}, []float64{}},
		{nil, []float64{}},
		{[]float64{1, 2, 3}, []float64{1, 2, 3}},
		{[]float64{1, 0, 2}, []float64{1, 0, 2}},
	}
	for _, tt := range tests {
		got := polynomial.Normalize(tt.input)
		if !polyEqual(got, tt.want) {
			t.Errorf("Normalize(%v) = %v, want %v", tt.input, got, tt.want)
		}
	}
}

// =============================================================================
// Degree
// =============================================================================

func TestDegree(t *testing.T) {
	tests := []struct {
		input []float64
		want  int
	}{
		{nil, -1},
		{[]float64{}, -1},
		{[]float64{0}, -1},
		{[]float64{0, 0}, -1},
		{[]float64{7}, 0},
		{[]float64{1, 2}, 1},
		{[]float64{1, 2, 3}, 2},
		{[]float64{3, 0, 2}, 2},
		{[]float64{3, 0, 0}, 0},
		{[]float64{1, 2, 0}, 1},
	}
	for _, tt := range tests {
		got := polynomial.Degree(tt.input)
		if got != tt.want {
			t.Errorf("Degree(%v) = %d, want %d", tt.input, got, tt.want)
		}
	}
}

// =============================================================================
// Zero and One
// =============================================================================

func TestZero(t *testing.T) {
	z := polynomial.Zero()
	if len(z) != 0 {
		t.Errorf("Zero() = %v, want []", z)
	}
	// Zero is additive identity
	p := []float64{1, 2, 3}
	got := polynomial.Add(polynomial.Zero(), p)
	if !polyEqual(got, p) {
		t.Errorf("Add(Zero(), p) = %v, want %v", got, p)
	}
}

func TestOne(t *testing.T) {
	o := polynomial.One()
	if !polyEqual(o, []float64{1}) {
		t.Errorf("One() = %v, want [1]", o)
	}
	// One is multiplicative identity
	p := []float64{1, 2, 3}
	got := polynomial.Multiply(polynomial.One(), p)
	if !polyEqual(got, p) {
		t.Errorf("Multiply(One(), p) = %v, want %v", got, p)
	}
}

// =============================================================================
// Add
// =============================================================================

func TestAdd(t *testing.T) {
	t.Run("same length", func(t *testing.T) {
		got := polynomial.Add([]float64{1, 2, 3}, []float64{4, 5, 6})
		want := []float64{5, 7, 9}
		if !polyEqual(got, want) {
			t.Errorf("got %v, want %v", got, want)
		}
	})
	t.Run("shorter first", func(t *testing.T) {
		got := polynomial.Add([]float64{4, 5}, []float64{1, 2, 3})
		want := []float64{5, 7, 3}
		if !polyEqual(got, want) {
			t.Errorf("got %v, want %v", got, want)
		}
	})
	t.Run("shorter second", func(t *testing.T) {
		got := polynomial.Add([]float64{1, 2, 3}, []float64{4, 5})
		want := []float64{5, 7, 3}
		if !polyEqual(got, want) {
			t.Errorf("got %v, want %v", got, want)
		}
	})
	t.Run("cancellation", func(t *testing.T) {
		got := polynomial.Add([]float64{1, 2, 3}, []float64{-1, -2, -3})
		if len(got) != 0 {
			t.Errorf("got %v, want []", got)
		}
	})
	t.Run("zero identity", func(t *testing.T) {
		p := []float64{1, 2}
		if !polyEqual(polynomial.Add(polynomial.Zero(), p), p) {
			t.Error("Add(Zero, p) != p")
		}
	})
	t.Run("commutative", func(t *testing.T) {
		a := []float64{1, 2, 3}
		b := []float64{4, 5, 6, 7}
		if !polyEqual(polynomial.Add(a, b), polynomial.Add(b, a)) {
			t.Error("Add is not commutative")
		}
	})
}

// =============================================================================
// Subtract
// =============================================================================

func TestSubtract(t *testing.T) {
	t.Run("basic", func(t *testing.T) {
		got := polynomial.Subtract([]float64{5, 7, 3}, []float64{1, 2, 3})
		want := []float64{4, 5}
		if !polyEqual(got, want) {
			t.Errorf("got %v, want %v", got, want)
		}
	})
	t.Run("self minus self", func(t *testing.T) {
		p := []float64{1, 2, 3}
		got := polynomial.Subtract(p, p)
		if len(got) != 0 {
			t.Errorf("got %v, want []", got)
		}
	})
	t.Run("round trip", func(t *testing.T) {
		p := []float64{3, 1, 4}
		q := []float64{1, 5, 9}
		got := polynomial.Add(polynomial.Subtract(p, q), q)
		if !polyEqual(got, p) {
			t.Errorf("Add(Subtract(p,q),q) = %v, want %v", got, p)
		}
	})
}

// =============================================================================
// Multiply
// =============================================================================

func TestMultiply(t *testing.T) {
	t.Run("two linears", func(t *testing.T) {
		// (1+2x)(3+4x) = 3 + 10x + 8x²
		got := polynomial.Multiply([]float64{1, 2}, []float64{3, 4})
		want := []float64{3, 10, 8}
		if !polyEqual(got, want) {
			t.Errorf("got %v, want %v", got, want)
		}
	})
	t.Run("by zero", func(t *testing.T) {
		got := polynomial.Multiply([]float64{1, 2, 3}, nil)
		if len(got) != 0 {
			t.Errorf("got %v, want []", got)
		}
	})
	t.Run("by one", func(t *testing.T) {
		p := []float64{1, 2, 3}
		got := polynomial.Multiply(p, polynomial.One())
		if !polyEqual(got, p) {
			t.Errorf("got %v, want %v", got, p)
		}
	})
	t.Run("commutative", func(t *testing.T) {
		a := []float64{1, 2, 3}
		b := []float64{4, 5}
		if !polyEqual(polynomial.Multiply(a, b), polynomial.Multiply(b, a)) {
			t.Error("Multiply is not commutative")
		}
	})
	t.Run("result degree", func(t *testing.T) {
		a := []float64{1, 2, 3} // degree 2
		b := []float64{4, 5, 6} // degree 2
		result := polynomial.Multiply(a, b)
		if polynomial.Degree(result) != 4 {
			t.Errorf("degree = %d, want 4", polynomial.Degree(result))
		}
	})
}

// =============================================================================
// Divmod
// =============================================================================

func TestDivmod(t *testing.T) {
	t.Run("panics on zero divisor", func(t *testing.T) {
		defer func() {
			if r := recover(); r == nil {
				t.Error("expected panic for zero divisor")
			}
		}()
		polynomial.Divmod([]float64{1, 2, 3}, nil)
	})

	t.Run("low degree dividend", func(t *testing.T) {
		q, r := polynomial.Divmod([]float64{1, 2}, []float64{0, 0, 1})
		if len(q) != 0 {
			t.Errorf("quotient = %v, want []", q)
		}
		if !polyEqual(r, []float64{1, 2}) {
			t.Errorf("remainder = %v, want [1,2]", r)
		}
	})

	t.Run("zero remainder", func(t *testing.T) {
		product := polynomial.Multiply([]float64{1, 1}, []float64{1, 1})
		q, r := polynomial.Divmod(product, []float64{1, 1})
		if !polyEqual(q, []float64{1, 1}) {
			t.Errorf("quotient = %v, want [1,1]", q)
		}
		if len(r) != 0 {
			t.Errorf("remainder = %v, want []", r)
		}
	})

	t.Run("a = b*q + r", func(t *testing.T) {
		a := []float64{5, 1, 3, 2}
		b := []float64{2, 1}
		q, r := polynomial.Divmod(a, b)
		reconstructed := polynomial.Add(polynomial.Multiply(b, q), r)
		if !polyEqual(reconstructed, polynomial.Normalize(a)) {
			t.Errorf("reconstructed = %v, want %v", reconstructed, a)
		}
	})

	t.Run("spec example", func(t *testing.T) {
		a := []float64{5, 1, 3, 2}
		b := []float64{2, 1}
		q, r := polynomial.Divmod(a, b)
		if !polyEqual(q, []float64{3, -1, 2}) {
			t.Errorf("quotient = %v, want [3,-1,2]", q)
		}
		if !polyEqual(r, []float64{-1}) {
			t.Errorf("remainder = %v, want [-1]", r)
		}
	})
}

// =============================================================================
// Divide and Mod
// =============================================================================

func TestDivide(t *testing.T) {
	t.Run("panics on zero divisor", func(t *testing.T) {
		defer func() {
			if r := recover(); r == nil {
				t.Error("expected panic")
			}
		}()
		polynomial.Divide([]float64{1, 2}, nil)
	})
}

func TestMod(t *testing.T) {
	t.Run("exact division gives zero", func(t *testing.T) {
		p := polynomial.Multiply([]float64{1, 1}, []float64{2, 1})
		r := polynomial.Mod(p, []float64{1, 1})
		if len(r) != 0 {
			t.Errorf("remainder = %v, want []", r)
		}
	})

	t.Run("panics on zero divisor", func(t *testing.T) {
		defer func() {
			if r := recover(); r == nil {
				t.Error("expected panic")
			}
		}()
		polynomial.Mod([]float64{1, 2}, nil)
	})
}

// =============================================================================
// Evaluate
// =============================================================================

func TestEvaluate(t *testing.T) {
	t.Run("zero polynomial", func(t *testing.T) {
		if polynomial.Evaluate(nil, 5) != 0 {
			t.Error("Evaluate(nil, 5) != 0")
		}
	})
	t.Run("constant polynomial", func(t *testing.T) {
		if polynomial.Evaluate([]float64{7}, 100) != 7 {
			t.Error("Evaluate([7], 100) != 7")
		}
	})
	t.Run("spec example", func(t *testing.T) {
		// 3 + x + 2x² at x=4 → 39
		got := polynomial.Evaluate([]float64{3, 1, 2}, 4)
		if math.Abs(got-39) > delta {
			t.Errorf("got %f, want 39", got)
		}
	})
	t.Run("constant term at x=0", func(t *testing.T) {
		got := polynomial.Evaluate([]float64{5, 3, 1}, 0)
		if math.Abs(got-5) > delta {
			t.Errorf("got %f, want 5", got)
		}
	})
	t.Run("matches naive evaluation", func(t *testing.T) {
		p := []float64{1, -3, 2}
		x := 3.0
		naive := p[0] + p[1]*x + p[2]*x*x
		got := polynomial.Evaluate(p, x)
		if math.Abs(got-naive) > delta {
			t.Errorf("got %f, want %f", got, naive)
		}
	})
}

// =============================================================================
// GCD
// =============================================================================

func TestGCD(t *testing.T) {
	t.Run("gcd with zero", func(t *testing.T) {
		p := []float64{1, 2, 3}
		g := polynomial.GCD(p, nil)
		if !polyEqual(g, polynomial.Normalize(p)) {
			t.Errorf("GCD(p, nil) = %v, want %v", g, p)
		}
	})
	t.Run("coprime polynomials", func(t *testing.T) {
		a := []float64{1, 1} // 1 + x
		b := []float64{2, 1} // 2 + x
		g := polynomial.GCD(a, b)
		if polynomial.Degree(g) != 0 {
			t.Errorf("degree = %d, want 0", polynomial.Degree(g))
		}
	})
	t.Run("common factor", func(t *testing.T) {
		f1 := polynomial.Multiply([]float64{1, 1}, []float64{2, 1})
		f2 := polynomial.Multiply([]float64{1, 1}, []float64{3, 1})
		g := polynomial.GCD(f1, f2)
		if polynomial.Degree(g) != 1 {
			t.Errorf("degree = %d, want 1", polynomial.Degree(g))
		}
		// g must divide both f1 and f2
		if len(polynomial.Mod(f1, g)) != 0 {
			t.Error("GCD does not divide f1")
		}
		if len(polynomial.Mod(f2, g)) != 0 {
			t.Error("GCD does not divide f2")
		}
	})
}
