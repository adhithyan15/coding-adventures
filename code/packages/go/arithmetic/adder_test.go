package arithmetic

import (
	"reflect"
	"testing"
)

func TestHalfAdder(t *testing.T) {
	cases := []struct {
		a, b       int
		sum, carry int
	}{
		{0, 0, 0, 0},
		{0, 1, 1, 0},
		{1, 0, 1, 0},
		{1, 1, 0, 1},
	}

	for _, c := range cases {
		s, carry := HalfAdder(c.a, c.b)
		if s != c.sum || carry != c.carry {
			t.Errorf("HalfAdder(%d, %d) == (%d, %d), expected (%d, %d)",
				c.a, c.b, s, carry, c.sum, c.carry)
		}
	}
}

func TestFullAdder(t *testing.T) {
	cases := []struct {
		a, b, cin  int
		sum, carry int
	}{
		{0, 0, 0, 0, 0},
		{0, 0, 1, 1, 0},
		{0, 1, 0, 1, 0},
		{0, 1, 1, 0, 1},
		{1, 0, 0, 1, 0},
		{1, 0, 1, 0, 1},
		{1, 1, 0, 0, 1},
		{1, 1, 1, 1, 1},
	}

	for _, c := range cases {
		s, carry := FullAdder(c.a, c.b, c.cin)
		if s != c.sum || carry != c.carry {
			t.Errorf("FullAdder(%d, %d, %d) == (%d, %d), expected (%d, %d)",
				c.a, c.b, c.cin, s, carry, c.sum, c.carry)
		}
	}
}

func TestRippleCarryAdder(t *testing.T) {
	// 5 + 3 = 8
	// 5 = 0101 -> [1, 0, 1, 0] (LSB first)
	// 3 = 0011 -> [1, 1, 0, 0]
	// 8 = 1000 -> [0, 0, 0, 1]
	a := []int{1, 0, 1, 0}
	b := []int{1, 1, 0, 0}
	
	s, c := RippleCarryAdder(a, b, 0)
	expectedSum := []int{0, 0, 0, 1}
	
	if !reflect.DeepEqual(s, expectedSum) || c != 0 {
		t.Errorf("RippleCarryAdder(5, 3) == (%v, %d), expected (%v, 0)", s, c, expectedSum)
	}

	// 15 + 1 = 16 (overflow 4 bits)
	// 15 = 1111 -> [1, 1, 1, 1]
	// 1 = 0001 -> [1, 0, 0, 0]
	// 16 = 10000 -> [0, 0, 0, 0] carry 1
	a2 := []int{1, 1, 1, 1}
	b2 := []int{1, 0, 0, 0}
	s2, c2 := RippleCarryAdder(a2, b2, 0)
	expectedSum2 := []int{0, 0, 0, 0}

	if !reflect.DeepEqual(s2, expectedSum2) || c2 != 1 {
		t.Errorf("RippleCarryAdder(15, 1) == (%v, %d), expected (%v, 1)", s2, c2, expectedSum2)
	}
}
