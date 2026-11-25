package logicgates

import (
	"errors"
	"fmt"
	"math"
	"testing"
)

// Go's type system makes the "type confusion" matrix (floats, strings,
// booleans, nil) a compile-time matter: every gate takes int and nothing
// else. Untyped constants convert at compile time, so AND(1.0, 0) compiles and
// is exactly AND(1, 0); AND(1.5, 0) does not compile. Hence the only run-time
// rejection domain is the integers, covered exhaustively below.

type bin struct {
	name string
	fn   func(int, int) int
	want [4]int // outputs for (0,0) (0,1) (1,0) (1,1)
}

var binGates = []bin{
	{"AND", AND, [4]int{0, 0, 0, 1}},
	{"OR", OR, [4]int{0, 1, 1, 1}},
	{"XOR", XOR, [4]int{0, 1, 1, 0}},
	{"NAND", NAND, [4]int{1, 1, 1, 0}},
	{"NOR", NOR, [4]int{1, 0, 0, 0}},
	{"XNOR", XNOR, [4]int{1, 0, 0, 1}},
	{"NAND_AND", NAND_AND, [4]int{0, 0, 0, 1}},
	{"NAND_OR", NAND_OR, [4]int{0, 1, 1, 1}},
	{"NAND_XOR", NAND_XOR, [4]int{0, 1, 1, 0}},
}

var unGates = []struct {
	name string
	fn   func(int) int
}{{"NOT", NOT}, {"NAND_NOT", NAND_NOT}}

// badInts is every integer in [-1024,1024] except 0 and 1, plus extremes.
func badInts() []int {
	var out []int
	for i := -1024; i <= 1024; i++ {
		if i != 0 && i != 1 {
			out = append(out, i)
		}
	}
	return append(out, math.MinInt, math.MinInt+1, math.MaxInt-1, math.MaxInt,
		math.MinInt32, math.MaxInt32)
}

func TestLiteralTruthTables(t *testing.T) {
	for _, g := range binGates {
		for i, in := range [][2]int{{0, 0}, {0, 1}, {1, 0}, {1, 1}} {
			if got := g.fn(in[0], in[1]); got != g.want[i] {
				t.Errorf("%s(%d,%d)=%d want %d", g.name, in[0], in[1], got, g.want[i])
			}
		}
	}
	for _, g := range unGates {
		if g.fn(0) != 1 || g.fn(1) != 0 {
			t.Errorf("%s truth table wrong", g.name)
		}
	}
}

func TestNandBuiltEqualsDirect(t *testing.T) {
	pairs := [][2]string{{"NAND_AND", "AND"}, {"NAND_OR", "OR"}, {"NAND_XOR", "XOR"}}
	byName := map[string]func(int, int) int{}
	for _, g := range binGates {
		byName[g.name] = g.fn
	}
	for _, p := range pairs {
		for a := 0; a <= 1; a++ {
			for b := 0; b <= 1; b++ {
				if byName[p[0]](a, b) != byName[p[1]](a, b) {
					t.Errorf("%s != %s at (%d,%d)", p[0], p[1], a, b)
				}
			}
		}
	}
	for a := 0; a <= 1; a++ {
		if NAND_NOT(a) != NOT(a) {
			t.Errorf("NAND_NOT != NOT at %d", a)
		}
	}
}

func TestBinaryRejectionExhaustive(t *testing.T) {
	bad := badInts()
	for _, g := range binGates {
		for _, v := range bad {
			// Bad in position a with b valid or invalid: a is named first.
			assertInvalidBit(t, g.name, "a", v, func() { g.fn(v, 0) })
			assertInvalidBit(t, g.name, "a", v, func() { g.fn(v, 1) })
			assertInvalidBit(t, g.name, "a", v, func() { g.fn(v, 2) })
			// Bad in position b only (a valid).
			assertInvalidBit(t, g.name, "b", v, func() { g.fn(0, v) })
			assertInvalidBit(t, g.name, "b", v, func() { g.fn(1, v) })
		}
	}
}

func TestUnaryRejectionExhaustive(t *testing.T) {
	for _, g := range unGates {
		for _, v := range badInts() {
			assertInvalidBit(t, g.name, "a", v, func() { g.fn(v) })
		}
	}
}

func TestNoValidInputPanicsAndResultsAreBits(t *testing.T) {
	for _, g := range binGates {
		for a := 0; a <= 1; a++ {
			for b := 0; b <= 1; b++ {
				if r := g.fn(a, b); r != 0 && r != 1 {
					t.Errorf("%s(%d,%d)=%d not a bit", g.name, a, b, r)
				}
			}
		}
	}
	for _, g := range unGates {
		for a := 0; a <= 1; a++ {
			if r := g.fn(a); r != 0 && r != 1 {
				t.Errorf("%s(%d)=%d not a bit", g.name, a, r)
			}
		}
	}
}

func TestMultiInputRejectionNamesIndex(t *testing.T) {
	multi := map[string]func(...int) int{"ANDn": ANDn, "ORn": ORn, "XORn": XORn}
	for name, fn := range multi {
		for _, v := range badInts() {
			assertInvalidBit(t, name, "inputs[1]", v, func() { fn(1, v, 0) })
			assertInvalidBit(t, name, "inputs[0]", v, func() { fn(v, 1) })
		}
	}
}

func TestRejectionIdentityIsStable(t *testing.T) {
	var got any
	func() {
		defer func() { got = recover() }()
		AND(7, 0)
	}()
	err, ok := got.(error)
	if !ok {
		t.Fatalf("panic value %T is not an error", got)
	}
	var ibe *InvalidBitError
	if !errors.As(err, &ibe) || ibe.Param != "a" || ibe.Value != 7 {
		t.Fatalf("errors.As failed: %v", err)
	}
	var ie *InternalError
	if errors.As(err, &ie) {
		t.Fatal("input rejection must never be an InternalError")
	}
}

func TestInternalErrorIsDistinct(t *testing.T) {
	cause := fmt.Errorf("boom")
	e := &InternalError{Gate: "AND", Err: cause}
	if e.Error() != "logicgates: AND CMOS evaluation error: boom" {
		t.Errorf("message: %s", e.Error())
	}
	if !errors.Is(e, cause) {
		t.Error("Unwrap should expose cause")
	}
	if _, ok := IsInvalidBit(e); ok {
		t.Error("InternalError must not satisfy IsInvalidBit")
	}
	if _, ok := IsInvalidBit("a string"); ok {
		t.Error("non-error must not satisfy IsInvalidBit")
	}
	if _, ok := IsInvalidBit(nil); ok {
		t.Error("nil must not satisfy IsInvalidBit")
	}
}

func TestMustEvalPanicsWithInternalError(t *testing.T) {
	if mustEval("AND", 1, nil) != 1 {
		t.Fatal("mustEval should pass results through")
	}
	defer func() {
		r := recover()
		if _, ok := r.(*InternalError); !ok {
			t.Fatalf("got %T, want *InternalError", r)
		}
		if _, ok := IsInvalidBit(r); ok {
			t.Fatal("internal failure must not look like rejection")
		}
	}()
	mustEval("AND", 0, fmt.Errorf("boom"))
}

// wideInts are values that a truncating `int32(v)` (or uint32/float32) bit
// check would wrongly map into {0,1}: 2^32 truncates to 0, 2^32+1 to 1, and
// so on. A narrowing validator MUST fail these tests.
func wideInts() []int {
	return []int{
		1 << 31, 1<<31 + 1, 1 << 32, 1<<32 + 1, 1<<53 + 1,
		math.MaxInt64, math.MinInt64, -1,
	}
}

func TestWideIntegerRejectionAllGates(t *testing.T) {
	for _, v := range wideInts() {
		for _, g := range binGates {
			assertInvalidBit(t, g.name, "a", v, func() { g.fn(v, 0) })
			assertInvalidBit(t, g.name, "a", v, func() { g.fn(v, 1) })
			assertInvalidBit(t, g.name, "b", v, func() { g.fn(0, v) })
			assertInvalidBit(t, g.name, "b", v, func() { g.fn(1, v) })
		}
		for _, g := range unGates {
			assertInvalidBit(t, g.name, "a", v, func() { g.fn(v) })
		}
		for name, fn := range map[string]func(...int) int{"ANDn": ANDn, "ORn": ORn, "XORn": XORn} {
			assertInvalidBit(t, name, "inputs[1]", v, func() { fn(1, v, 0) })
			assertInvalidBit(t, name, "inputs[0]", v, func() { fn(v, 1) })
			assertInvalidBit(t, name, "inputs[2]", v, func() { fn(0, 0, v) })
		}
	}
}
