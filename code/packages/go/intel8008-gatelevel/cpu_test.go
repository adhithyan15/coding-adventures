package intel8008gatelevel

// Tests for the Intel 8008 gate-level simulator.
//
// # Test strategy
//
// 1. Unit tests for individual components (bits, ALU, registers, stack, decoder)
// 2. Integration tests that cross-validate the gate-level CPU against known results
// 3. Example programs that serve as end-to-end tests
//
// The gate-level simulator and the behavioral simulator should produce identical
// results for any program. Where we can't directly compare (the behavioral sim
// is in a separate package), we compare against known-correct expected values.

import (
	"testing"
)

// ─────────────────────────────────────────────────────────────────────────────
// Bit conversion helpers
// ─────────────────────────────────────────────────────────────────────────────

func TestIntToBits_8bit(t *testing.T) {
	// 5 = 00000101 in binary → [1, 0, 1, 0, 0, 0, 0, 0] LSB first
	bits := IntToBits(5, 8)
	if len(bits) != 8 {
		t.Fatalf("expected 8 bits, got %d", len(bits))
	}
	if bits[0] != 1 || bits[1] != 0 || bits[2] != 1 {
		t.Errorf("IntToBits(5, 8) = %v, want [1,0,1,0,0,0,0,0]", bits)
	}
}

func TestIntToBits_14bit(t *testing.T) {
	// 0x3FFF (max 14-bit) should give all ones
	bits := IntToBits(0x3FFF, 14)
	for i, b := range bits {
		if b != 1 {
			t.Errorf("bit %d of 0x3FFF should be 1, got %d", i, b)
		}
	}
}

func TestBitsToInt_roundtrip(t *testing.T) {
	for _, v := range []int{0, 1, 42, 127, 128, 255, 0x100, 0x3FFF} {
		width := 8
		if v > 255 {
			width = 14
		}
		bits := IntToBits(v, width)
		got := BitsToInt(bits)
		if got != v {
			t.Errorf("round-trip failed: input %d, got %d", v, got)
		}
	}
}

func TestComputeParity(t *testing.T) {
	tests := []struct {
		value    int
		expected int // 1 = even parity, 0 = odd parity
	}{
		{0x00, 1}, // 0 ones → even
		{0xFF, 1}, // 8 ones → even
		{0x01, 0}, // 1 one → odd
		{0x03, 1}, // 2 ones → even
		{0x07, 0}, // 3 ones → odd
		{0x0F, 1}, // 4 ones → even
		{0x1F, 0}, // 5 ones → odd
		{0x3F, 1}, // 6 ones → even
		{0x7F, 0}, // 7 ones → odd
		{0x80, 0}, // 1 one → odd
	}
	for _, tt := range tests {
		got := ComputeParity(tt.value)
		if got != tt.expected {
			t.Errorf("ComputeParity(0x%02X) = %d, want %d", tt.value, got, tt.expected)
		}
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// ALU unit tests
// ─────────────────────────────────────────────────────────────────────────────

func TestGateALU_Add_NoCarry(t *testing.T) {
	alu := NewGateALU()
	result, carry := alu.Add(3, 4, 0)
	if result != 7 || carry {
		t.Errorf("Add(3, 4, 0) = (%d, %v), want (7, false)", result, carry)
	}
}

func TestGateALU_Add_WithCarryIn(t *testing.T) {
	alu := NewGateALU()
	// 3 + 4 + 1 = 8
	result, carry := alu.Add(3, 4, 1)
	if result != 8 || carry {
		t.Errorf("Add(3, 4, 1) = (%d, %v), want (8, false)", result, carry)
	}
}

func TestGateALU_Add_Overflow(t *testing.T) {
	alu := NewGateALU()
	// 255 + 1 = 256 = 0 with carry
	result, carry := alu.Add(255, 1, 0)
	if result != 0 || !carry {
		t.Errorf("Add(255, 1, 0) = (%d, %v), want (0, true)", result, carry)
	}
}

func TestGateALU_Subtract_NoBorrow(t *testing.T) {
	alu := NewGateALU()
	// 5 - 3 = 2, no borrow
	result, borrow := alu.Subtract(5, 3, 0)
	if result != 2 || borrow {
		t.Errorf("Subtract(5, 3, 0) = (%d, %v), want (2, false)", result, borrow)
	}
}

func TestGateALU_Subtract_WithBorrow(t *testing.T) {
	alu := NewGateALU()
	// 3 - 5 = -2 = 0xFE (256-2=254), borrow occurred
	result, borrow := alu.Subtract(3, 5, 0)
	if result != 254 || !borrow {
		t.Errorf("Subtract(3, 5, 0) = (%d, %v), want (254, true)", result, borrow)
	}
}

func TestGateALU_Subtract_SBBWithBorrow(t *testing.T) {
	alu := NewGateALU()
	// SBB: 5 - 3 - 1(borrow) = 1
	result, borrow := alu.Subtract(5, 3, 1)
	if result != 1 || borrow {
		t.Errorf("Subtract(5, 3, 1) = (%d, %v), want (1, false)", result, borrow)
	}
}

func TestGateALU_BitwiseAnd(t *testing.T) {
	alu := NewGateALU()
	result := alu.BitwiseAnd(0xF0, 0x0F)
	if result != 0 {
		t.Errorf("BitwiseAnd(0xF0, 0x0F) = %d, want 0", result)
	}
	result = alu.BitwiseAnd(0xFF, 0xAA)
	if result != 0xAA {
		t.Errorf("BitwiseAnd(0xFF, 0xAA) = 0x%02X, want 0xAA", result)
	}
}

func TestGateALU_BitwiseOr(t *testing.T) {
	alu := NewGateALU()
	result := alu.BitwiseOr(0xF0, 0x0F)
	if result != 0xFF {
		t.Errorf("BitwiseOr(0xF0, 0x0F) = 0x%02X, want 0xFF", result)
	}
}

func TestGateALU_BitwiseXor(t *testing.T) {
	alu := NewGateALU()
	result := alu.BitwiseXor(0xFF, 0xFF)
	if result != 0 {
		t.Errorf("BitwiseXor(0xFF, 0xFF) = %d, want 0", result)
	}
	result = alu.BitwiseXor(0xAA, 0x55)
	if result != 0xFF {
		t.Errorf("BitwiseXor(0xAA, 0x55) = 0x%02X, want 0xFF", result)
	}
}

func TestGateALU_Increment(t *testing.T) {
	alu := NewGateALU()
	result, _ := alu.Increment(41)
	if result != 42 {
		t.Errorf("Increment(41) = %d, want 42", result)
	}
	result, carry := alu.Increment(255)
	if result != 0 || !carry {
		t.Errorf("Increment(255) = (%d, %v), want (0, true)", result, carry)
	}
}

func TestGateALU_Decrement(t *testing.T) {
	alu := NewGateALU()
	result, _ := alu.Decrement(42)
	if result != 41 {
		t.Errorf("Decrement(42) = %d, want 41", result)
	}
	result, borrow := alu.Decrement(0)
	if result != 255 || !borrow {
		t.Errorf("Decrement(0) = (%d, %v), want (255, true)", result, borrow)
	}
}

func TestGateALU_RotateLeftCircular(t *testing.T) {
	alu := NewGateALU()
	// 0b10110001 = 0xB1; rotate left circular:
	// bit 7 (1) goes to bit 0 and CY; bits 6-0 shift to 7-1
	// 0b01100011 = 0x63, CY=1
	result, carry := alu.RotateLeftCircular(0xB1)
	if result != 0x63 || !carry {
		t.Errorf("RotateLeftCircular(0xB1) = (0x%02X, %v), want (0x63, true)", result, carry)
	}
}

func TestGateALU_RotateRightCircular(t *testing.T) {
	alu := NewGateALU()
	// 0b10110001 = 0xB1; rotate right circular:
	// bit 0 (1) goes to bit 7 and CY; bits 7-1 shift to 6-0
	// 0b11011000 = 0xD8, CY=1
	result, carry := alu.RotateRightCircular(0xB1)
	if result != 0xD8 || !carry {
		t.Errorf("RotateRightCircular(0xB1) = (0x%02X, %v), want (0xD8, true)", result, carry)
	}
}

func TestGateALU_RotateLeftThroughCarry(t *testing.T) {
	alu := NewGateALU()
	// 0b10110001, carry=0; RAL:
	// new bit 0 = old CY = 0
	// new bits 7-1 = old bits 6-0
	// new CY = old bit 7 = 1
	// result: 0b01100010 = 0x62, CY=1
	result, carry := alu.RotateLeftThroughCarry(0xB1, false)
	if result != 0x62 || !carry {
		t.Errorf("RotateLeftThroughCarry(0xB1, false) = (0x%02X, %v), want (0x62, true)", result, carry)
	}
}

func TestGateALU_RotateRightThroughCarry(t *testing.T) {
	alu := NewGateALU()
	// 0b10110001, carry=0; RAR:
	// new bit 7 = old CY = 0
	// new bits 6-0 = old bits 7-1
	// new CY = old bit 0 = 1
	// result: 0b01011000 = 0x58, CY=1
	result, carry := alu.RotateRightThroughCarry(0xB1, false)
	if result != 0x58 || !carry {
		t.Errorf("RotateRightThroughCarry(0xB1, false) = (0x%02X, %v), want (0x58, true)", result, carry)
	}
}

func TestGateALU_ComputeFlags(t *testing.T) {
	alu := NewGateALU()

	// Result = 0: zero=true, sign=false, parity=true (0 ones = even)
	z, s, c, p := alu.ComputeFlags(0, false)
	if !z || s || c || !p {
		t.Errorf("ComputeFlags(0, false) = (z=%v, s=%v, c=%v, p=%v), want (T,F,F,T)", z, s, c, p)
	}

	// Result = 0x80 (10000000): zero=false, sign=true, parity=false (1 one = odd)
	z, s, c, p = alu.ComputeFlags(0x80, false)
	if z || !s || c || p {
		t.Errorf("ComputeFlags(0x80, false) = (z=%v, s=%v, c=%v, p=%v), want (F,T,F,F)", z, s, c, p)
	}

	// Result = 3 (00000011): zero=false, sign=false, parity=true (2 ones = even)
	z, s, c, p = alu.ComputeFlags(3, true)
	if z || s || !c || !p {
		t.Errorf("ComputeFlags(3, true) = (z=%v, s=%v, c=%v, p=%v), want (F,F,T,T)", z, s, c, p)
	}
}

func TestGateALU_GateCount(t *testing.T) {
	alu := NewGateALU()
	count := alu.GateCount()
	if count <= 0 {
		t.Errorf("GateCount() = %d, want > 0", count)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Register file tests
// ─────────────────────────────────────────────────────────────────────────────

func TestRegisterFile_ReadWrite(t *testing.T) {
	regs := NewRegisterFile()

	// Write and read each register (excluding M=6)
	for i := 0; i < 8; i++ {
		if i == 6 {
			continue
		}
		regs.Write(i, i*10)
		got := regs.Read(i)
		if got != i*10 {
			t.Errorf("Register[%d]: wrote %d, read %d", i, i*10, got)
		}
	}
}

func TestRegisterFile_MaskTo8Bits(t *testing.T) {
	regs := NewRegisterFile()
	// Writing 0x1FF (9 bits) should be masked to 0xFF (8 bits)
	regs.Write(0, 0x1FF)
	got := regs.Read(0)
	if got != 0xFF {
		t.Errorf("Register[0] with value 0x1FF: got 0x%02X, want 0xFF", got)
	}
}

func TestRegisterFile_HLAddress(t *testing.T) {
	regs := NewRegisterFile()
	// H=0x01 (index 4), L=0x23 (index 5)
	// address = (0x01 & 0x3F) << 8 | 0x23 = 0x0123
	regs.Write(4, 0x01) // H
	regs.Write(5, 0x23) // L
	addr := regs.HLAddress()
	if addr != 0x0123 {
		t.Errorf("HLAddress with H=0x01, L=0x23: got 0x%04X, want 0x0123", addr)
	}
}

func TestRegisterFile_HLAddress_MaskH(t *testing.T) {
	regs := NewRegisterFile()
	// H=0xFF: upper 2 bits should be ignored → effective H = 0x3F
	// L=0x00 → address = 0x3F00
	regs.Write(4, 0xFF) // H (masked to 0x3F for address)
	regs.Write(5, 0x00) // L
	addr := regs.HLAddress()
	if addr != 0x3F00 {
		t.Errorf("HLAddress with H=0xFF, L=0x00: got 0x%04X, want 0x3F00", addr)
	}
}

func TestRegisterFile_Reset(t *testing.T) {
	regs := NewRegisterFile()
	regs.Write(0, 42)
	regs.Write(7, 99)
	regs.Reset()
	if regs.Read(0) != 0 || regs.Read(7) != 0 {
		t.Error("Reset() should zero all registers")
	}
}

func TestRegisterFile_GateCount(t *testing.T) {
	regs := NewRegisterFile()
	if regs.GateCount() <= 0 {
		t.Error("GateCount() should be positive")
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Flag register tests
// ─────────────────────────────────────────────────────────────────────────────

func TestFlagRegister_ReadWrite(t *testing.T) {
	flags := NewFlagRegister()

	// Initial state: all false
	c, z, s, p := flags.ReadFlags()
	if c || z || s || p {
		t.Error("Initial flags should all be false")
	}

	// Write specific flags
	flags.WriteFlags(true, false, true, false) // carry=T, zero=F, sign=T, parity=F
	c, z, s, p = flags.ReadFlags()
	if !c || z || !s || p {
		t.Errorf("Flags = (c=%v, z=%v, s=%v, p=%v), want (T,F,T,F)", c, z, s, p)
	}
}

func TestFlagRegister_Reset(t *testing.T) {
	flags := NewFlagRegister()
	flags.WriteFlags(true, true, true, true)
	flags.Reset()
	c, z, s, p := flags.ReadFlags()
	if c || z || s || p {
		t.Error("After Reset(), all flags should be false")
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Push-down stack tests
// ─────────────────────────────────────────────────────────────────────────────

func TestPushDownStack_InitialPC(t *testing.T) {
	stack := NewPushDownStack()
	if stack.PC() != 0 {
		t.Errorf("Initial PC = %d, want 0", stack.PC())
	}
	if stack.Depth() != 0 {
		t.Errorf("Initial depth = %d, want 0", stack.Depth())
	}
}

func TestPushDownStack_Increment(t *testing.T) {
	stack := NewPushDownStack()
	stack.Increment(1)
	if stack.PC() != 1 {
		t.Errorf("After Increment(1): PC = %d, want 1", stack.PC())
	}
	stack.Increment(2)
	if stack.PC() != 3 {
		t.Errorf("After Increment(2): PC = %d, want 3", stack.PC())
	}
}

func TestPushDownStack_Increment_14bitWrap(t *testing.T) {
	stack := NewPushDownStack()
	stack.SetPC(0x3FFF) // max 14-bit address
	stack.Increment(1)
	if stack.PC() != 0 {
		t.Errorf("14-bit wrap: expected PC=0, got PC=%d", stack.PC())
	}
}

func TestPushDownStack_SetPC(t *testing.T) {
	stack := NewPushDownStack()
	stack.SetPC(0x1234)
	if stack.PC() != 0x1234 {
		t.Errorf("SetPC(0x1234): PC = 0x%04X, want 0x1234", stack.PC())
	}
}

func TestPushDownStack_PushPop(t *testing.T) {
	stack := NewPushDownStack()

	// Simulate: PC at 0x0010, call to 0x0100
	stack.SetPC(0x0013) // simulate PC after fetching 3-byte CAL instruction
	returnAddr := stack.PC() // 0x0013 = return address

	// Push: rotate down, entry[0] = 0x0100 (call target)
	stack.Push(0x0100)

	if stack.PC() != 0x0100 {
		t.Errorf("After Push: PC = 0x%04X, want 0x0100", stack.PC())
	}
	if stack.Depth() != 1 {
		t.Errorf("After Push: depth = %d, want 1", stack.Depth())
	}
	if stack.ReadLevel(1) != returnAddr {
		t.Errorf("After Push: level[1] = 0x%04X, want 0x%04X", stack.ReadLevel(1), returnAddr)
	}

	// Pop: rotate up, entry[0] = entry[1] = returnAddr
	stack.Pop()
	if stack.PC() != returnAddr {
		t.Errorf("After Pop: PC = 0x%04X, want 0x%04X", stack.PC(), returnAddr)
	}
	if stack.Depth() != 0 {
		t.Errorf("After Pop: depth = %d, want 0", stack.Depth())
	}
}

func TestPushDownStack_NestedCalls(t *testing.T) {
	stack := NewPushDownStack()

	// Simulate 3 nested calls
	// Call 1: PC=0x10, jump to 0x100
	stack.SetPC(0x13) // after fetching 3-byte CAL
	stack.Push(0x100)

	// Call 2: at 0x100, jump to 0x200
	stack.Increment(3) // advance past second CAL
	stack.Push(0x200)

	// Call 3: at 0x200, jump to 0x300
	stack.Increment(3)
	stack.Push(0x300)

	if stack.Depth() != 3 {
		t.Errorf("Depth after 3 calls = %d, want 3", stack.Depth())
	}
	if stack.PC() != 0x300 {
		t.Errorf("PC after 3 calls = 0x%04X, want 0x300", stack.PC())
	}

	// Return 3 times
	stack.Pop()
	stack.Pop()
	stack.Pop()

	if stack.Depth() != 0 {
		t.Errorf("Depth after 3 returns = %d, want 0", stack.Depth())
	}
}

func TestPushDownStack_Reset(t *testing.T) {
	stack := NewPushDownStack()
	stack.SetPC(0x100)
	stack.Push(0x200)
	stack.Reset()
	if stack.PC() != 0 || stack.Depth() != 0 {
		t.Error("After Reset: PC should be 0, depth should be 0")
	}
}

func TestPushDownStack_GateCount(t *testing.T) {
	stack := NewPushDownStack()
	if stack.GateCount() <= 0 {
		t.Error("GateCount() should be positive")
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Decoder tests
// ─────────────────────────────────────────────────────────────────────────────

func TestDecoder_HLT_0x00(t *testing.T) {
	d := Decode(0x00)
	if d.IsHLT != 1 || d.InstrLen != 1 {
		t.Errorf("Decode(0x00): IsHLT=%d, InstrLen=%d, want (1, 1)", d.IsHLT, d.InstrLen)
	}
}

func TestDecoder_HLT_0x76(t *testing.T) {
	d := Decode(0x76)
	if d.IsHLT != 1 || d.InstrLen != 1 {
		t.Errorf("Decode(0x76): IsHLT=%d, InstrLen=%d, want (1, 1)", d.IsHLT, d.InstrLen)
	}
}

func TestDecoder_HLT_0xFF(t *testing.T) {
	d := Decode(0xFF)
	if d.IsHLT != 1 || d.InstrLen != 1 {
		t.Errorf("Decode(0xFF): IsHLT=%d, InstrLen=%d, want (1, 1)", d.IsHLT, d.InstrLen)
	}
}

func TestDecoder_MVI(t *testing.T) {
	// MVI B, imm = 0x06 (00 000 110)
	d := Decode(0x06)
	if d.IsMVI != 1 || d.InstrLen != 2 || d.RegDst != 0 {
		t.Errorf("Decode(0x06): IsMVI=%d, InstrLen=%d, RegDst=%d, want (1, 2, 0)", d.IsMVI, d.InstrLen, d.RegDst)
	}

	// MVI A, imm = 0x3E (00 111 110)
	d = Decode(0x3E)
	if d.IsMVI != 1 || d.RegDst != 7 {
		t.Errorf("Decode(0x3E): IsMVI=%d, RegDst=%d, want (1, 7)", d.IsMVI, d.RegDst)
	}
}

func TestDecoder_INR(t *testing.T) {
	// INR B = 0x04? No: INR = 00DDD000
	// INR B (B=0): 00 000 000 = 0x00 = HLT... so INR B is not encodable
	// INR C (C=1): 00 001 000 = 0x08
	d := Decode(0x08)
	if d.IsINR != 1 || d.RegDst != 1 {
		t.Errorf("Decode(0x08): IsINR=%d, RegDst=%d, want (1, 1)", d.IsINR, d.RegDst)
	}
}

func TestDecoder_DCR(t *testing.T) {
	// DCR = 00DDD001
	// DCR C (C=1): 00 001 001 = 0x09
	d := Decode(0x09)
	if d.IsDCR != 1 || d.RegDst != 1 {
		t.Errorf("Decode(0x09): IsDCR=%d, RegDst=%d, want (1, 1)", d.IsDCR, d.RegDst)
	}
}

func TestDecoder_RLC(t *testing.T) {
	d := Decode(0x02)
	if d.IsRLC != 1 || d.InstrLen != 1 {
		t.Errorf("Decode(0x02): IsRLC=%d, InstrLen=%d, want (1, 1)", d.IsRLC, d.InstrLen)
	}
}

func TestDecoder_RRC(t *testing.T) {
	d := Decode(0x0A)
	if d.IsRRC != 1 {
		t.Errorf("Decode(0x0A): IsRRC=%d, want 1", d.IsRRC)
	}
}

func TestDecoder_RAL(t *testing.T) {
	d := Decode(0x12)
	if d.IsRAL != 1 {
		t.Errorf("Decode(0x12): IsRAL=%d, want 1", d.IsRAL)
	}
}

func TestDecoder_RAR(t *testing.T) {
	d := Decode(0x1A)
	if d.IsRAR != 1 {
		t.Errorf("Decode(0x1A): IsRAR=%d, want 1", d.IsRAR)
	}
}

func TestDecoder_OUT(t *testing.T) {
	// OUT port: 00DDD010 where DDD >= 4
	// OUT port 0: DDD=100=4, opcode = 00 100 010 = 0x22, port = (0x22 >> 1) & 0x1F = 0x11 = 17?
	// Actually port = (opcode >> 1) & 0x1F:
	// 0x22 = 0b00100010; >> 1 = 0b00010001 = 17; & 0x1F = 17
	// Hmm. Let's check: DDD=4 (100) in opcode bits 5-3; sss=010; opcode = 0b 00 100 010 = 0x22
	// port = (0x22 >> 1) & 0x1F = 0x11 = 17
	d := Decode(0x22)
	if d.IsOUT != 1 {
		t.Errorf("Decode(0x22): IsOUT=%d, want 1", d.IsOUT)
	}
}

func TestDecoder_MOV(t *testing.T) {
	// MOV B, D = 01 000 010 = 0x42? Wait: DDD=B=0, SSS=D=2: 01 000 010 = 0x42
	// But 0x42 is a conditional CAL... let me check the decoder logic
	// sss=010 in group 01 → IsCcond, not MOV
	// MOV B, L = 01 000 101 = 0x45: DDD=0, SSS=5
	// sss=101 is not 000, 001, 010 → should be MOV
	d := Decode(0x45)
	if d.IsMOV != 1 || d.RegDst != 0 || d.RegSrc != 5 {
		t.Errorf("Decode(0x45): IsMOV=%d, RegDst=%d, RegSrc=%d, want (1, 0, 5)", d.IsMOV, d.RegDst, d.RegSrc)
	}
}

func TestDecoder_MOV_AA(t *testing.T) {
	// MOV A, A = 01 111 111 = 0x7F: DDD=7, SSS=7
	// SSS=111 is not 000, 001, 010 → should be MOV
	d := Decode(0x7F)
	if d.IsMOV != 1 || d.RegDst != 7 || d.RegSrc != 7 {
		t.Errorf("Decode(0x7F): IsMOV=%d, RegDst=%d, RegSrc=%d, want (1, 7, 7)", d.IsMOV, d.RegDst, d.RegSrc)
	}
}

func TestDecoder_IN(t *testing.T) {
	// IN: group 01, SSS=001
	// IN port: 01DDD001 where DDD = port number
	// 0x41 = 01 000 001: DDD=0, SSS=1 → IN port 0
	d := Decode(0x41)
	if d.IsIN != 1 || d.PortNum != 0 {
		t.Errorf("Decode(0x41): IsIN=%d, PortNum=%d, want (1, 0)", d.IsIN, d.PortNum)
	}
}

func TestDecoder_JMP(t *testing.T) {
	d := Decode(0x7C)
	if d.IsJMP != 1 || d.InstrLen != 3 {
		t.Errorf("Decode(0x7C): IsJMP=%d, InstrLen=%d, want (1, 3)", d.IsJMP, d.InstrLen)
	}
}

func TestDecoder_CAL(t *testing.T) {
	d := Decode(0x7E)
	if d.IsCAL != 1 || d.InstrLen != 3 {
		t.Errorf("Decode(0x7E): IsCAL=%d, InstrLen=%d, want (1, 3)", d.IsCAL, d.InstrLen)
	}
}

func TestDecoder_Jcond(t *testing.T) {
	// JFZ = 01 001 000 = 0x48: DDD=001=cond FZ, SSS=000 → jump-if-false
	d := Decode(0x48)
	if d.IsJcond != 1 || d.InstrLen != 3 || d.CondCode != CondFZ {
		t.Errorf("Decode(0x48): IsJcond=%d, InstrLen=%d, CondCode=%d, want (1, 3, %d)", d.IsJcond, d.InstrLen, d.CondCode, CondFZ)
	}

	// JTC = 01 100 000 = 0x60: DDD=100=cond TC
	d = Decode(0x60)
	if d.IsJcond != 1 || d.CondCode != CondTC {
		t.Errorf("Decode(0x60): IsJcond=%d, CondCode=%d, want (1, %d)", d.IsJcond, d.CondCode, CondTC)
	}
}

func TestDecoder_Ccond(t *testing.T) {
	// CFZ = 01 001 010 = 0x4A: DDD=001=FZ, SSS=010 → conditional call
	d := Decode(0x4A)
	if d.IsCcond != 1 || d.InstrLen != 3 || d.CondCode != CondFZ {
		t.Errorf("Decode(0x4A): IsCcond=%d, InstrLen=%d, CondCode=%d, want (1, 3, %d)", d.IsCcond, d.InstrLen, d.CondCode, CondFZ)
	}
}

func TestDecoder_Rcond(t *testing.T) {
	// RFZ = 00 001 011 = 0x0B: DDD=001=FZ, SSS=011
	d := Decode(0x0B)
	if d.IsRcond != 1 || d.CondCode != CondFZ {
		t.Errorf("Decode(0x0B): IsRcond=%d, CondCode=%d, want (1, %d)", d.IsRcond, d.CondCode, CondFZ)
	}
}

func TestDecoder_RET(t *testing.T) {
	d := Decode(0xC7)
	if d.IsRET != 1 || d.InstrLen != 1 {
		t.Errorf("Decode(0xC7): IsRET=%d, InstrLen=%d, want (1, 1)", d.IsRET, d.InstrLen)
	}
}

func TestDecoder_RST(t *testing.T) {
	// RST 1 = 11 001 101 = 0xCD: DDD=001=1, SSS=101, vector=8*1=8
	d := Decode(0xCD)
	if d.IsRST != 1 || d.RSTVec != 8 {
		t.Errorf("Decode(0xCD): IsRST=%d, RSTVec=%d, want (1, 8)", d.IsRST, d.RSTVec)
	}
}

func TestDecoder_ALUreg_ADD(t *testing.T) {
	// ADD B = 10 000 000 = 0x80
	d := Decode(0x80)
	if d.IsALUreg != 1 || d.ALUOp != ALUOpADD || d.RegSrc != 0 {
		t.Errorf("Decode(0x80): IsALUreg=%d, ALUOp=%d, RegSrc=%d, want (1, %d, 0)", d.IsALUreg, d.ALUOp, d.RegSrc, ALUOpADD)
	}
}

func TestDecoder_ALUreg_CMP(t *testing.T) {
	// CMP A = 10 111 111 = 0xBF
	d := Decode(0xBF)
	if d.IsALUreg != 1 || d.ALUOp != ALUOpCMP || d.RegSrc != 7 {
		t.Errorf("Decode(0xBF): IsALUreg=%d, ALUOp=%d, RegSrc=%d, want (1, %d, 7)", d.IsALUreg, d.ALUOp, d.RegSrc, ALUOpCMP)
	}
}

func TestDecoder_ALUimm_ADI(t *testing.T) {
	// ADI = 11 000 100 = 0xC4
	d := Decode(0xC4)
	if d.IsALUimm != 1 || d.ALUOp != ALUOpADD || d.InstrLen != 2 {
		t.Errorf("Decode(0xC4): IsALUimm=%d, ALUOp=%d, InstrLen=%d, want (1, %d, 2)", d.IsALUimm, d.ALUOp, d.InstrLen, ALUOpADD)
	}
}

func TestDecoder_ALUimm_ORI(t *testing.T) {
	// ORI = 11 110 100 = 0xF4
	d := Decode(0xF4)
	if d.IsALUimm != 1 || d.ALUOp != ALUOpORA {
		t.Errorf("Decode(0xF4): IsALUimm=%d, ALUOp=%d, want (1, %d)", d.IsALUimm, d.ALUOp, ALUOpORA)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// CPU integration tests
// ─────────────────────────────────────────────────────────────────────────────

func TestCPU_HLT(t *testing.T) {
	cpu := NewIntel8008GateLevel()
	program := []byte{0x00} // HLT
	traces := cpu.Run(program, 100)
	if len(traces) != 1 {
		t.Fatalf("HLT: expected 1 trace, got %d", len(traces))
	}
	if !cpu.Halted() {
		t.Error("CPU should be halted after HLT")
	}
}

func TestCPU_MVI_and_HLT(t *testing.T) {
	cpu := NewIntel8008GateLevel()
	// MVI B, 42; HLT
	program := []byte{0x06, 42, 0x00}
	cpu.Run(program, 100)
	regs := cpu.Registers()
	if regs[0] != 42 {
		t.Errorf("B = %d, want 42", regs[0])
	}
}

func TestCPU_MVI_AllRegisters(t *testing.T) {
	cpu := NewIntel8008GateLevel()
	// MVI B=1, C=2, D=3, E=4, H=5, L=6, A=7, HLT
	program := []byte{
		0x06, 1,  // MVI B, 1  (00 000 110)
		0x0E, 2,  // MVI C, 2  (00 001 110)
		0x16, 3,  // MVI D, 3  (00 010 110)
		0x1E, 4,  // MVI E, 4  (00 011 110)
		0x26, 5,  // MVI H, 5  (00 100 110)
		0x2E, 6,  // MVI L, 6  (00 101 110)
		0x3E, 7,  // MVI A, 7  (00 111 110)
		0x00,     // HLT
	}
	cpu.Run(program, 100)
	regs := cpu.Registers()
	expected := []int{1, 2, 3, 4, 5, 6, 0, 7} // index 6 = M (undefined)
	for i, exp := range expected {
		if i == 6 {
			continue
		}
		if regs[i] != exp {
			t.Errorf("Register[%d] = %d, want %d", i, regs[i], exp)
		}
	}
}

func TestCPU_ADD_1Plus2(t *testing.T) {
	cpu := NewIntel8008GateLevel()
	// Classic 1+2 test:
	//   MVI B, 1
	//   MVI A, 2
	//   ADD B       (A = A + B = 3)
	//   HLT
	program := []byte{
		0x06, 1,   // MVI B, 1  (B = reg 0)
		0x3E, 2,   // MVI A, 2  (A = reg 7)
		0x80,      // ADD B     (10 000 000)
		0x00,      // HLT
	}
	cpu.Run(program, 100)
	regs := cpu.Registers()
	if regs[7] != 3 {
		t.Errorf("A = %d after 1+2, want 3", regs[7])
	}
}

func TestCPU_SUB(t *testing.T) {
	cpu := NewIntel8008GateLevel()
	// A = 10, B = 3, A = A - B = 7
	program := []byte{
		0x3E, 10,  // MVI A, 10
		0x06, 3,   // MVI B, 3
		0x90,      // SUB B (10 010 000)
		0x00,      // HLT
	}
	cpu.Run(program, 100)
	regs := cpu.Registers()
	if regs[7] != 7 {
		t.Errorf("A = %d after 10-3, want 7", regs[7])
	}
	// No borrow: carry should be false
	carry, _, _, _ := cpu.Flags()
	if carry {
		t.Error("SUB 10-3: carry should be false (no borrow)")
	}
}

func TestCPU_INR_DCR(t *testing.T) {
	cpu := NewIntel8008GateLevel()
	// B = 5; INR B (B=6); DCR B (B=5)
	// INR C = 0x08 (00 001 000); DCR C = 0x09 (00 001 001)
	program := []byte{
		0x0E, 5,   // MVI C, 5
		0x08,      // INR C (B? no: 0x08 = 00 001 000 = INR C)
		0x09,      // DCR C (00 001 001)
		0x09,      // DCR C again → 4
		0x00,      // HLT
	}
	cpu.Run(program, 100)
	regs := cpu.Registers()
	if regs[1] != 4 { // C is index 1
		t.Errorf("C = %d, want 4", regs[1])
	}
}

func TestCPU_MOV(t *testing.T) {
	cpu := NewIntel8008GateLevel()
	// MVI B, 99; MOV A, B (copy B to A); HLT
	// MOV A, B: DDD=7, SSS=0 → 01 111 000 = 0x78
	// But SSS=0 is valid (not 000 case conflict because sss=000 in group 01 → JFZ)
	// Wait: sss=000 → IsJcond, not MOV! We need a different SSS.
	// MOV A, L: DDD=7, SSS=5 → 01 111 101 = 0x7D
	program := []byte{
		0x2E, 99,  // MVI L, 99 (L = reg 5)
		0x7D,      // MOV A, L (01 111 101)
		0x00,      // HLT
	}
	cpu.Run(program, 100)
	regs := cpu.Registers()
	if regs[7] != 99 {
		t.Errorf("A = %d after MOV A,L (L=99), want 99", regs[7])
	}
}

func TestCPU_Memory_MVI_MOV_M(t *testing.T) {
	cpu := NewIntel8008GateLevel()
	// Store 42 to memory at address H:L = 0x0000 (H=0, L=0 are default 0)
	// MVI M, 42: 00 110 110 = 0x36
	// MOV A, M to read it back: but 0x7E = CAL, not MOV A,M
	// Instead use MOV B, M: DDD=0, SSS=6 → 01 000 110 = 0x46
	// BUT sss=110 in group 01 is the remaining MOV case (not sss=000/001/010)
	// Let's verify: 0x46 = 01 000 110; sss=110 → not 000/001/010, so it's MOV
	program := []byte{
		0x36, 42,  // MVI M, 42 (store 42 at [H:L]=0x0000)
		0x46,      // MOV B, M  (01 000 110; B = mem[0x0000] = 42)
		0x00,      // HLT
	}
	cpu.Run(program, 100)
	regs := cpu.Registers()
	if regs[0] != 42 {
		t.Errorf("B = %d after MOV B,M (M=42), want 42", regs[0])
	}
}

func TestCPU_JMP(t *testing.T) {
	cpu := NewIntel8008GateLevel()
	// JMP to HLT at 0x0005
	program := []byte{
		0x7C, 0x05, 0x00, // JMP 0x0005 (3 bytes)
		0x3E, 99,          // MVI A, 99 (should be skipped)
		0x00,              // HLT at 0x0005
	}
	cpu.Run(program, 100)
	regs := cpu.Registers()
	if regs[7] != 0 { // A should still be 0 (MVI was skipped)
		t.Errorf("A = %d, want 0 (MVI was skipped)", regs[7])
	}
	if !cpu.Halted() {
		t.Error("CPU should be halted")
	}
}

func TestCPU_CAL_RET(t *testing.T) {
	cpu := NewIntel8008GateLevel()
	// CAL 0x0008 (3 bytes at 0x0000-0x0002)
	// MVI A, 99 (skipped) at 0x0003-0x0004
	// HLT at 0x0005
	// padding at 0x0006-0x0007
	// subroutine at 0x0008: MVI A, 42; RET (0xC7)
	program := []byte{
		0x7E, 0x08, 0x00,  // 0x0000: CAL 0x0008
		0x00,              // 0x0003: HLT (return lands here? no, PC should be 0x0003)
		0x00, 0x00, 0x00, 0x00, // padding
		0x3E, 42,          // 0x0008: MVI A, 42
		0xC7,              // 0x000A: RET (unconditional return)
	}
	traces := cpu.Run(program, 100)
	_ = traces
	regs := cpu.Registers()
	if regs[7] != 42 {
		t.Errorf("A = %d after CAL/RET, want 42", regs[7])
	}
}

func TestCPU_RST(t *testing.T) {
	cpu := NewIntel8008GateLevel()
	// RST 1 calls to address 8
	// At 0x0008: MVI A, 77; RET
	// Main: RST 1 (0xCD); HLT
	program := make([]byte, 20)
	program[0] = 0xCD // RST 1 (11 001 101)
	program[1] = 0x00 // HLT (after return)
	// subroutine at 0x0008
	program[8] = 0x3E  // MVI A, 77
	program[9] = 77
	program[10] = 0xC7 // RET
	cpu.Run(program, 100)
	regs := cpu.Registers()
	if regs[7] != 77 {
		t.Errorf("A = %d after RST 1, want 77", regs[7])
	}
}

func TestCPU_ConditionalJump_JFZ_NotTaken(t *testing.T) {
	cpu := NewIntel8008GateLevel()
	// MVI A, 0 → zero flag set (via ORI 0)
	// JFZ (jump if zero false) → zero IS true, so don't jump
	// MVI A, 42
	// HLT
	program := []byte{
		0x3E, 0,           // MVI A, 0
		0xF4, 0x00,        // ORI 0 — sets zero flag (A=0, Z=1)
		0x48, 0x09, 0x00,  // JFZ 0x0009 (should NOT jump since Z=1)
		0x3E, 42,          // MVI A, 42 (executed)
		0x00,              // HLT at 0x0009
	}
	cpu.Run(program, 100)
	regs := cpu.Registers()
	if regs[7] != 42 {
		t.Errorf("A = %d, want 42 (JFZ not taken when Z=true)", regs[7])
	}
}

func TestCPU_ConditionalJump_JFZ_Taken(t *testing.T) {
	cpu := NewIntel8008GateLevel()
	// MVI B, 5; ORI 0 (sets Z=0); JFZ to HLT (Z false → jump IS taken)
	program := []byte{
		0x06, 5,           // MVI B, 5 (sets Z=0? No, MVI doesn't set flags)
		0x3E, 5,           // MVI A, 5
		0xF4, 0x00,        // ORI 0 — sets Z=0 (A=5, nonzero)
		0x48, 0x0C, 0x00,  // JFZ 0x000C (Z=false → jump taken)
		0x3E, 99,          // MVI A, 99 (should be skipped)
		0x00, 0x00, 0x00,  // padding to 0x000C
		0x00,              // HLT at 0x000C
	}
	cpu.Run(program, 100)
	regs := cpu.Registers()
	if regs[7] != 5 { // A should still be 5
		t.Errorf("A = %d, want 5 (JFZ taken when Z=false)", regs[7])
	}
}

func TestCPU_OUT_IN(t *testing.T) {
	cpu := NewIntel8008GateLevel()
	// OUT port 17 (opcode = 0x22 = 00 100 010):
	// port = (0x22 >> 1) & 0x1F = 0x11 = 17
	cpu.SetInputPort(0, 123) // set input port 0 = 123
	program := []byte{
		0x3E, 55,  // MVI A, 55
		0x22,      // OUT 17  (sends A to port 17)
		0x41,      // IN 0    (01 000 001; reads port 0 into A)
		0x00,      // HLT
	}
	cpu.Run(program, 100)
	// Check output port 17 has 55
	if cpu.GetOutputPort(17) != 55 {
		t.Errorf("Output port 17 = %d, want 55", cpu.GetOutputPort(17))
	}
	// Check A received input port 0 value (123)
	regs := cpu.Registers()
	if regs[7] != 123 {
		t.Errorf("A = %d after IN 0, want 123", regs[7])
	}
}

func TestCPU_RLC(t *testing.T) {
	cpu := NewIntel8008GateLevel()
	// A = 0x81 (10000001); RLC → A = 0x03, CY=1
	// 0x81 rotated left: bit7=1→bit0, bits6-0→bits7-1: 0x03, CY=1
	program := []byte{
		0x3E, 0x81, // MVI A, 0x81
		0x02,       // RLC
		0x00,       // HLT
	}
	cpu.Run(program, 100)
	regs := cpu.Registers()
	if regs[7] != 0x03 {
		t.Errorf("A = 0x%02X after RLC(0x81), want 0x03", regs[7])
	}
	carry, _, _, _ := cpu.Flags()
	if !carry {
		t.Error("Carry should be set after RLC(0x81)")
	}
}

func TestCPU_RAL(t *testing.T) {
	cpu := NewIntel8008GateLevel()
	// A = 0x81, CY=0; RAL → A = 0x02, CY=1
	program := []byte{
		0x3E, 0x81, // MVI A, 0x81 (CY stays 0 from MVI)
		0x12,       // RAL
		0x00,       // HLT
	}
	cpu.Run(program, 100)
	regs := cpu.Registers()
	if regs[7] != 0x02 {
		t.Errorf("A = 0x%02X after RAL(0x81, CY=0), want 0x02", regs[7])
	}
	carry, _, _, _ := cpu.Flags()
	if !carry {
		t.Error("Carry should be set after RAL(0x81)")
	}
}

func TestCPU_ANA(t *testing.T) {
	cpu := NewIntel8008GateLevel()
	// A = 0xFF; ANA B where B=0x0F → A = 0x0F, CY=0
	program := []byte{
		0x3E, 0xFF, // MVI A, 0xFF
		0x06, 0x0F, // MVI B, 0x0F
		0xA0,       // ANA B (10 100 000)
		0x00,       // HLT
	}
	cpu.Run(program, 100)
	regs := cpu.Registers()
	if regs[7] != 0x0F {
		t.Errorf("A = 0x%02X after ANA, want 0x0F", regs[7])
	}
	carry, _, _, _ := cpu.Flags()
	if carry {
		t.Error("ANA should clear carry")
	}
}

func TestCPU_XRA(t *testing.T) {
	cpu := NewIntel8008GateLevel()
	// A XOR A = 0 (zero flag set, carry cleared)
	program := []byte{
		0x3E, 0x55, // MVI A, 0x55
		0xA8,       // XRA A (10 101 111? No: XRA = 10 101 SSS; XRA A = SSS=7 → 10 101 111?
		// Wait: 10 101 111 = 0xAF? Let me recalculate:
		// 10 = bits 7-6; 101 = DDD/ALUOp; SSS = 111 (A=7)
		// Binary: 1 0 1 0 1 1 1 1 = 0xAF
		// Actually that's what we want: XRA A
		0x00,       // HLT
	}
	// Recalculate: XRA = ALUOpXRA = 5; group 10 = 0b10; DDD=5; SSS=7
	// opcode = 10 101 111 = 0b10101111 = 0xAF
	program[2] = 0xAF // fix opcode
	cpu.Run(program, 100)
	regs := cpu.Registers()
	if regs[7] != 0 {
		t.Errorf("A = %d after XRA A, want 0", regs[7])
	}
	_, zero, _, _ := cpu.Flags()
	if !zero {
		t.Error("Zero flag should be set after XRA A (result=0)")
	}
}

func TestCPU_CPI(t *testing.T) {
	cpu := NewIntel8008GateLevel()
	// A = 5; CPI 5 → A unchanged, Zero flag set
	// CPI = 11 111 100 = 0xFC
	program := []byte{
		0x3E, 5,    // MVI A, 5
		0xFC, 5,    // CPI 5
		0x00,       // HLT
	}
	cpu.Run(program, 100)
	regs := cpu.Registers()
	if regs[7] != 5 {
		t.Errorf("A = %d after CPI (should be unchanged), want 5", regs[7])
	}
	_, zero, _, _ := cpu.Flags()
	if !zero {
		t.Error("Zero flag should be set after CPI 5 (A=5, equal)")
	}
}

func TestCPU_GateCount(t *testing.T) {
	cpu := NewIntel8008GateLevel()
	count := cpu.GateCount()
	// Should be at least 1000 gates (we estimated ~1118)
	if count < 500 {
		t.Errorf("GateCount() = %d, expected at least 500", count)
	}
}

func TestCPU_Multiply_4x5(t *testing.T) {
	cpu := NewIntel8008GateLevel()
	// Multiply 4 × 5 = 20 by repeated addition:
	//   B = 4 (multiplicand)
	//   C = 5 (loop counter)
	//   A = 0 (accumulator)
	// loop:
	//   ADD B
	//   DCR C  (00 001 001 = 0x09)
	//   JFZ loop  (jump if zero false = C != 0)
	//   HLT
	//
	// JFZ = 0x48 (01 001 000); loop starts at address 6
	program := []byte{
		0x06, 4,         // 0x00: MVI B, 4
		0x0E, 5,         // 0x02: MVI C, 5
		0x3E, 0,         // 0x04: MVI A, 0
		0x80,            // 0x06: ADD B
		0x09,            // 0x07: DCR C
		0xF4, 0x00,      // 0x08: ORI 0 (sets flags from C value... wait DCR already sets flags)
		// Actually DCR should set flags. Let me not use ORI.
		// Let me remove ORI and use a direct loop:
		// But DCR should set Zero flag when C becomes 0
	}
	// Let me rebuild with correct loop
	program2 := []byte{
		0x06, 4,              // 0x00: MVI B, 4
		0x0E, 5,              // 0x02: MVI C, 5
		0x3E, 0,              // 0x04: MVI A, 0
		0x80,                 // 0x06: ADD B (A += B)
		0x09,                 // 0x07: DCR C (C--, sets flags)
		0x48, 0x06, 0x00,     // 0x08: JFZ 0x0006 (jump back if C != 0)
		0x00,                 // 0x0B: HLT
	}
	cpu.Run(program2, 1000)
	_ = program
	regs := cpu.Registers()
	if regs[7] != 20 {
		t.Errorf("A = %d after 4×5, want 20", regs[7])
	}
}

func TestCPU_StackDepth(t *testing.T) {
	cpu := NewIntel8008GateLevel()
	program := []byte{
		0x7E, 0x05, 0x00,  // 0x00: CAL 0x0005
		0x00,              // 0x03: HLT (return address)
		0x00,              // 0x04: padding
		0xC7,              // 0x05: RET
	}
	cpu.Run(program, 100)
	if cpu.StackDepth() != 0 {
		t.Errorf("Stack depth after CAL+RET = %d, want 0", cpu.StackDepth())
	}
}

func TestCPU_Reset(t *testing.T) {
	cpu := NewIntel8008GateLevel()
	cpu.Run([]byte{0x3E, 42, 0x00}, 100)
	// After run, A=42
	cpu.Reset()
	regs := cpu.Registers()
	if regs[7] != 0 {
		t.Errorf("A = %d after Reset(), want 0", regs[7])
	}
	if cpu.PC() != 0 {
		t.Errorf("PC = %d after Reset(), want 0", cpu.PC())
	}
	if cpu.Halted() {
		t.Error("Halted should be false after Reset()")
	}
}

func TestCPU_Flags_SignBit(t *testing.T) {
	cpu := NewIntel8008GateLevel()
	// A = 0x80 = 10000000; sign flag should be set
	program := []byte{
		0x3E, 0,    // MVI A, 0
		0xF4, 0x80, // ORI 0x80 → A = 0x80, sign=true
		0x00,       // HLT
	}
	cpu.Run(program, 100)
	_, _, sign, _ := cpu.Flags()
	if !sign {
		t.Error("Sign flag should be set when A=0x80")
	}
}

func TestCPU_ADC_WithCarry(t *testing.T) {
	cpu := NewIntel8008GateLevel()
	// Set carry, then ADC
	// ADD 0xFF to A=0: sets carry
	// ADC B (B=1): A = 0 + 1 + 1(carry) = 2
	program := []byte{
		0x3E, 0xFF,  // MVI A, 0xFF
		0x06, 1,     // MVI B, 1
		0x80,        // ADD B → A = 0xFF + 1 = 0x00, CY=1
		0x88,        // ADC B → A = 0x00 + 1 + 1 = 2 (10 001 000)
		0x00,        // HLT
	}
	cpu.Run(program, 100)
	regs := cpu.Registers()
	if regs[7] != 2 {
		t.Errorf("A = %d after ADC with carry, want 2", regs[7])
	}
}
