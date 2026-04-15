package gf256_test

import (
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/gf256"
)

// =============================================================================
// Log/Antilog Tables
// =============================================================================

func TestLogAllogTables(t *testing.T) {
	logTable := gf256.LOG()
	alogTable := gf256.ALOG()

	t.Run("ALOG[0] = 1", func(t *testing.T) {
		if alogTable[0] != 1 {
			t.Errorf("ALOG[0] = %d, want 1", alogTable[0])
		}
	})

	t.Run("ALOG[1] = 2", func(t *testing.T) {
		if alogTable[1] != 2 {
			t.Errorf("ALOG[1] = %d, want 2", alogTable[1])
		}
	})

	t.Run("ALOG[8] = 29 (first reduction)", func(t *testing.T) {
		// 2^8 = 256; 256 XOR 0x11D = 0x1D = 29
		if alogTable[8] != 29 {
			t.Errorf("ALOG[8] = %d, want 29", alogTable[8])
		}
	})

	t.Run("ALOG values in range [1,255]", func(t *testing.T) {
		for i := 0; i < 255; i++ {
			v := alogTable[i]
			if v < 1 || v > 255 {
				t.Errorf("ALOG[%d] = %d, out of range [1,255]", i, v)
			}
		}
	})

	t.Run("ALOG is bijection over [0,254]", func(t *testing.T) {
		seen := make(map[int]bool)
		for i := 0; i < 255; i++ {
			v := alogTable[i]
			if seen[v] {
				t.Errorf("ALOG is not a bijection: value %d appears twice", v)
			}
			seen[v] = true
		}
	})

	t.Run("ALOG[LOG[x]] = x for all x in 1..255", func(t *testing.T) {
		for x := 1; x <= 255; x++ {
			if alogTable[int(logTable[x])] != x {
				t.Errorf("ALOG[LOG[%d]] = %d, want %d", x, alogTable[int(logTable[x])], x)
			}
		}
	})

	t.Run("LOG[1] = 0", func(t *testing.T) {
		if logTable[1] != 0 {
			t.Errorf("LOG[1] = %d, want 0", logTable[1])
		}
	})

	t.Run("LOG[2] = 1", func(t *testing.T) {
		if logTable[2] != 1 {
			t.Errorf("LOG[2] = %d, want 1", logTable[2])
		}
	})
}

// =============================================================================
// Add
// =============================================================================

func TestAdd(t *testing.T) {
	t.Run("zero identity", func(t *testing.T) {
		for x := 0; x <= 255; x++ {
			if gf256.Add(0, byte(x)) != byte(x) {
				t.Errorf("Add(0, %d) != %d", x, x)
			}
			if gf256.Add(byte(x), 0) != byte(x) {
				t.Errorf("Add(%d, 0) != %d", x, x)
			}
		}
	})

	t.Run("self is zero (char 2)", func(t *testing.T) {
		for x := 0; x <= 255; x++ {
			if gf256.Add(byte(x), byte(x)) != 0 {
				t.Errorf("Add(%d, %d) != 0", x, x)
			}
		}
	})

	t.Run("commutative", func(t *testing.T) {
		for x := 0; x < 32; x++ {
			for y := 0; y < 32; y++ {
				if gf256.Add(byte(x), byte(y)) != gf256.Add(byte(y), byte(x)) {
					t.Errorf("Add not commutative for %d, %d", x, y)
				}
			}
		}
	})

	t.Run("is XOR", func(t *testing.T) {
		for x := 0; x <= 255; x++ {
			if gf256.Add(byte(x), 0x42) != byte(x)^0x42 {
				t.Errorf("Add(%d, 0x42) != XOR", x)
			}
		}
	})
}

// =============================================================================
// Subtract
// =============================================================================

func TestSubtract(t *testing.T) {
	t.Run("same as add", func(t *testing.T) {
		for x := 0; x < 64; x++ {
			for y := 0; y < 64; y++ {
				if gf256.Subtract(byte(x), byte(y)) != gf256.Add(byte(x), byte(y)) {
					t.Errorf("Subtract != Add for %d, %d", x, y)
				}
			}
		}
	})
}

// =============================================================================
// Multiply
// =============================================================================

func TestMultiply(t *testing.T) {
	t.Run("by zero", func(t *testing.T) {
		for x := 0; x <= 255; x++ {
			if gf256.Multiply(byte(x), 0) != 0 {
				t.Errorf("Multiply(%d, 0) != 0", x)
			}
			if gf256.Multiply(0, byte(x)) != 0 {
				t.Errorf("Multiply(0, %d) != 0", x)
			}
		}
	})

	t.Run("by one (identity)", func(t *testing.T) {
		for x := 0; x <= 255; x++ {
			if gf256.Multiply(byte(x), 1) != byte(x) {
				t.Errorf("Multiply(%d, 1) != %d", x, x)
			}
		}
	})

	t.Run("commutative", func(t *testing.T) {
		for x := 0; x < 32; x++ {
			for y := 0; y < 32; y++ {
				if gf256.Multiply(byte(x), byte(y)) != gf256.Multiply(byte(y), byte(x)) {
					t.Errorf("Multiply not commutative: %d, %d", x, y)
				}
			}
		}
	})

	t.Run("associative", func(t *testing.T) {
		a, b, c := byte(0x53), byte(0xCA), byte(0x3D)
		lhs := gf256.Multiply(gf256.Multiply(a, b), c)
		rhs := gf256.Multiply(a, gf256.Multiply(b, c))
		if lhs != rhs {
			t.Errorf("Multiply not associative: %d != %d", lhs, rhs)
		}
	})

	t.Run("spot check: 0x53 * 0x8C = 0x01 (with 0x11D polynomial)", func(t *testing.T) {
		// With primitive polynomial 0x11D, inverse(0x53) = 0x8C.
		if gf256.Multiply(0x53, 0x8C) != 0x01 {
			t.Errorf("Multiply(0x53, 0x8C) = %d, want 1", gf256.Multiply(0x53, 0x8C))
		}
	})

	t.Run("distributive over add", func(t *testing.T) {
		a, b, c := byte(0x34), byte(0x56), byte(0x78)
		lhs := gf256.Multiply(a, gf256.Add(b, c))
		rhs := gf256.Add(gf256.Multiply(a, b), gf256.Multiply(a, c))
		if lhs != rhs {
			t.Errorf("Distributive law failed: %d != %d", lhs, rhs)
		}
	})
}

// =============================================================================
// Divide
// =============================================================================

func TestDivide(t *testing.T) {
	t.Run("panics on division by zero", func(t *testing.T) {
		defer func() {
			if r := recover(); r == nil {
				t.Error("expected panic for Divide(1, 0)")
			}
		}()
		gf256.Divide(1, 0)
	})

	t.Run("by one", func(t *testing.T) {
		for x := 0; x <= 255; x++ {
			if gf256.Divide(byte(x), 1) != byte(x) {
				t.Errorf("Divide(%d, 1) != %d", x, x)
			}
		}
	})

	t.Run("zero divided by anything", func(t *testing.T) {
		for x := 1; x <= 255; x++ {
			if gf256.Divide(0, byte(x)) != 0 {
				t.Errorf("Divide(0, %d) != 0", x)
			}
		}
	})

	t.Run("divide self is one", func(t *testing.T) {
		for x := 1; x <= 255; x++ {
			if gf256.Divide(byte(x), byte(x)) != 1 {
				t.Errorf("Divide(%d, %d) != 1", x, x)
			}
		}
	})

	t.Run("inverse of multiply", func(t *testing.T) {
		for a := 0; a < 16; a++ {
			for b := 1; b < 16; b++ {
				product := gf256.Multiply(byte(a), byte(b))
				if gf256.Divide(product, byte(b)) != byte(a) {
					t.Errorf("Divide(Multiply(%d,%d), %d) != %d", a, b, b, a)
				}
			}
		}
	})
}

// =============================================================================
// Power
// =============================================================================

func TestPower(t *testing.T) {
	t.Run("any nonzero to 0 is 1", func(t *testing.T) {
		for x := 1; x <= 255; x++ {
			if gf256.Power(byte(x), 0) != 1 {
				t.Errorf("Power(%d, 0) != 1", x)
			}
		}
	})

	t.Run("0^0 = 1", func(t *testing.T) {
		if gf256.Power(0, 0) != 1 {
			t.Error("Power(0, 0) != 1")
		}
	})

	t.Run("0^n = 0 for n > 0", func(t *testing.T) {
		if gf256.Power(0, 1) != 0 || gf256.Power(0, 5) != 0 {
			t.Error("Power(0, n) != 0 for n > 0")
		}
	})

	t.Run("generator order 255", func(t *testing.T) {
		if gf256.Power(2, 255) != 1 {
			t.Errorf("Power(2, 255) = %d, want 1", gf256.Power(2, 255))
		}
	})

	t.Run("power matches ALOG", func(t *testing.T) {
		alogTable := gf256.ALOG()
		for i := 0; i < 255; i++ {
			got := gf256.Power(2, i)
			want := byte(alogTable[i])
			if got != want {
				t.Errorf("Power(2, %d) = %d, want %d", i, got, want)
			}
		}
	})
}

// =============================================================================
// Inverse
// =============================================================================

func TestInverse(t *testing.T) {
	t.Run("panics for 0", func(t *testing.T) {
		defer func() {
			if r := recover(); r == nil {
				t.Error("expected panic for Inverse(0)")
			}
		}()
		gf256.Inverse(0)
	})

	t.Run("Inverse(1) = 1", func(t *testing.T) {
		if gf256.Inverse(1) != 1 {
			t.Error("Inverse(1) != 1")
		}
	})

	t.Run("x * Inverse(x) = 1 for all x", func(t *testing.T) {
		for x := 1; x <= 255; x++ {
			if gf256.Multiply(byte(x), gf256.Inverse(byte(x))) != 1 {
				t.Errorf("x * Inverse(x) != 1 for x = %d", x)
			}
		}
	})

	t.Run("Inverse(Inverse(x)) = x", func(t *testing.T) {
		for x := 1; x <= 255; x++ {
			if gf256.Inverse(gf256.Inverse(byte(x))) != byte(x) {
				t.Errorf("Inverse(Inverse(%d)) != %d", x, x)
			}
		}
	})

	t.Run("spot check: Inverse(0x53) = 0x8C with 0x11D polynomial", func(t *testing.T) {
		if gf256.Inverse(0x53) != 0x8C {
			t.Errorf("Inverse(0x53) = 0x%02X, want 0x8C", gf256.Inverse(0x53))
		}
	})
}

// =============================================================================
// Zero and One
// =============================================================================

func TestZeroAndOne(t *testing.T) {
	if gf256.Zero() != 0 {
		t.Error("Zero() != 0")
	}
	if gf256.One() != 1 {
		t.Error("One() != 1")
	}
	if gf256.Add(gf256.Zero(), 0x42) != 0x42 {
		t.Error("Zero is not additive identity")
	}
	if gf256.Multiply(gf256.One(), 0x42) != 0x42 {
		t.Error("One is not multiplicative identity")
	}
}

// =============================================================================
// Field — parameterizable factory
// =============================================================================

func TestField(t *testing.T) {
	t.Run("AES field: Multiply(0x53, 0xCA) = 1", func(t *testing.T) {
		f := gf256.NewField(0x11B)
		got := f.Multiply(0x53, 0xCA)
		if got != 1 {
			t.Errorf("AES field Multiply(0x53, 0xCA) = 0x%02X, want 0x01", got)
		}
	})

	t.Run("AES field: Multiply(0x57, 0x83) = 0xC1 (FIPS 197 Appendix B)", func(t *testing.T) {
		f := gf256.NewField(0x11B)
		got := f.Multiply(0x57, 0x83)
		if got != 0xC1 {
			t.Errorf("AES field Multiply(0x57, 0x83) = 0x%02X, want 0xC1", got)
		}
	})

	t.Run("AES field: Inverse(0x53) = 0xCA", func(t *testing.T) {
		f := gf256.NewField(0x11B)
		got := f.Inverse(0x53)
		if got != 0xCA {
			t.Errorf("AES field Inverse(0x53) = 0x%02X, want 0xCA", got)
		}
	})

	t.Run("RS field (0x11D) matches module-level Multiply", func(t *testing.T) {
		f := gf256.NewField(0x11D)
		for a := byte(0); a < 32; a++ {
			for b := byte(0); b < 32; b++ {
				want := gf256.Multiply(a, b)
				got := f.Multiply(a, b)
				if got != want {
					t.Errorf("Field(0x11D).Multiply(%d,%d) = %d, want %d", a, b, got, want)
				}
			}
		}
	})

	t.Run("commutativity", func(t *testing.T) {
		f := gf256.NewField(0x11B)
		vals := []byte{0, 1, 0x53, 0xCA, 0xFF}
		for _, a := range vals {
			for _, b := range vals {
				if f.Multiply(a, b) != f.Multiply(b, a) {
					t.Errorf("Multiply(%d,%d) != Multiply(%d,%d)", a, b, b, a)
				}
			}
		}
	})

	t.Run("inverse times self is 1", func(t *testing.T) {
		f := gf256.NewField(0x11B)
		for x := byte(1); x <= 20; x++ {
			if f.Multiply(x, f.Inverse(x)) != 1 {
				t.Errorf("x * Inverse(x) != 1 for x = %d", x)
			}
		}
	})

	t.Run("panics on divide by zero", func(t *testing.T) {
		defer func() {
			if r := recover(); r == nil {
				t.Error("expected panic for Field.Divide(1, 0)")
			}
		}()
		f := gf256.NewField(0x11B)
		f.Divide(1, 0)
	})

	t.Run("panics on inverse of zero", func(t *testing.T) {
		defer func() {
			if r := recover(); r == nil {
				t.Error("expected panic for Field.Inverse(0)")
			}
		}()
		f := gf256.NewField(0x11B)
		f.Inverse(0)
	})

	t.Run("PrimitivePoly stored", func(t *testing.T) {
		f := gf256.NewField(0x11B)
		if f.PrimitivePoly != 0x11B {
			t.Errorf("PrimitivePoly = 0x%03X, want 0x11B", f.PrimitivePoly)
		}
	})
}
