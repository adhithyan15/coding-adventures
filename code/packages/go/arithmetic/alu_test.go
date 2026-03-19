package arithmetic

import (
	"reflect"
	"testing"
)

func TestALUAdd(t *testing.T) {
	alu := NewALU(4)
	
	// 5 + 3 = 8
	// 5 = 0101 -> [1, 0, 1, 0] LSB first
	// 3 = 0011 -> [1, 1, 0, 0]
	res := alu.Execute(ADD, []int{1, 0, 1, 0}, []int{1, 1, 0, 0})
	expected := []int{0, 0, 0, 1} // 8
	
	if !reflect.DeepEqual(res.Value, expected) {
		t.Errorf("expected 8, got %v", res.Value)
	}
	if res.Zero != false { t.Errorf("zero flag wrong") }
	if res.Carry != false { t.Errorf("carry flag wrong") }
	if res.Negative != true { t.Errorf("negative flag wrong, 1000 has MSB 1") }
	if res.Overflow != true { t.Errorf("overflow flag wrong, positive + positive gave negative") }
}

func TestALUSub(t *testing.T) {
	alu := NewALU(4)
	// 5 - 3 = 2
	// 5 = [1, 0, 1, 0]
	// 3 = [1, 1, 0, 0]
	res := alu.Execute(SUB, []int{1, 0, 1, 0}, []int{1, 1, 0, 0})
	expected := []int{0, 1, 0, 0} // 2

	if !reflect.DeepEqual(res.Value, expected) {
		t.Errorf("expected 2, got %v", res.Value)
	}
	if res.Zero != false { t.Errorf("zero flag wrong") }
	if res.Negative != false { t.Errorf("negative flag wrong") }
	if res.Overflow != false { t.Errorf("overflow flag wrong") }
}

func TestALUAnd(t *testing.T) {
	alu := NewALU(4)
	// 1010 AND 1100 = 1000  (10 AND 12 = 8)
	// 10 = [0, 1, 0, 1]
	// 12 = [0, 0, 1, 1]
	res := alu.Execute(AND, []int{0, 1, 0, 1}, []int{0, 0, 1, 1})
	expected := []int{0, 0, 0, 1}
	
	if !reflect.DeepEqual(res.Value, expected) {
		t.Errorf("expected 8, got %v", res.Value)
	}
}

func TestALUOr(t *testing.T) {
	alu := NewALU(4)
	// 1010 OR 1100 = 1110  (10 OR 12 = 14)
	res := alu.Execute(OR, []int{0, 1, 0, 1}, []int{0, 0, 1, 1})
	expected := []int{0, 1, 1, 1}
	
	if !reflect.DeepEqual(res.Value, expected) {
		t.Errorf("expected 14, got %v", res.Value)
	}
}

func TestALUXor(t *testing.T) {
	alu := NewALU(4)
	// 1010 XOR 1100 = 0110 (6)
	res := alu.Execute(XOR, []int{0, 1, 0, 1}, []int{0, 0, 1, 1})
	expected := []int{0, 1, 1, 0}
	
	if !reflect.DeepEqual(res.Value, expected) {
		t.Errorf("expected 6, got %v", res.Value)
	}
}

func TestALUNot(t *testing.T) {
	alu := NewALU(4)
	// NOT 1010 = 0101 (5)
	res := alu.Execute(NOT, []int{0, 1, 0, 1}, []int{})
	expected := []int{1, 0, 1, 0}
	
	if !reflect.DeepEqual(res.Value, expected) {
		t.Errorf("expected 5, got %v", res.Value)
	}
}
