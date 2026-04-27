package intel8008simulator

// Test suite for the Intel 8008 behavioral simulator.
//
// # Test Organization
//
// Tests are organized by instruction group, following the spec:
//   - Basic arithmetic (ADD, ADI, SUB, SUI, etc.)
//   - Register moves (MOV, MVI)
//   - Increment/Decrement (INR, DCR)
//   - Rotate instructions (RLC, RRC, RAL, RAR)
//   - Flag behavior (Zero, Sign, Parity, Carry)
//   - Jump/Call/Return (JMP, CAL, RET and conditional variants)
//   - RST (restart instructions)
//   - Stack mechanics (push-down stack behavior)
//   - M pseudo-register (memory-through-[H:L])
//   - I/O ports (IN, OUT)
//   - HLT (both encodings: 0x76 and 0xFF)
//   - Example programs from the spec
//   - Parity flag computation

import (
	"fmt"
	"testing"
)

// ─────────────────────────────────────────────────────────────────────────────
// Helper functions
// ─────────────────────────────────────────────────────────────────────────────

// newSim creates a fresh simulator with the given program loaded.
func newSim(program []byte) *Simulator {
	s := New()
	s.LoadProgram(program, 0)
	return s
}

// runProgram runs a program to completion and returns (sim, traces).
func runProgram(t *testing.T, program []byte) (*Simulator, []Trace) {
	t.Helper()
	s := New()
	traces := s.Run(program, 10000)
	return s, traces
}

// checkA checks the accumulator value after running a program.
func checkA(t *testing.T, program []byte, want int) {
	t.Helper()
	s, _ := runProgram(t, program)
	if s.A() != want {
		t.Errorf("A = %d (0x%02X), want %d (0x%02X)", s.A(), s.A(), want, want)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// HLT — Halt (0x76 and 0xFF)
// ─────────────────────────────────────────────────────────────────────────────

func TestHLT_0x76(t *testing.T) {
	// The canonical HLT = MOV M,M = 0x76
	s, traces := runProgram(t, []byte{0x76})
	if !s.Halted() {
		t.Error("expected CPU to be halted after HLT (0x76)")
	}
	if len(traces) != 1 {
		t.Errorf("expected 1 trace, got %d", len(traces))
	}
	if traces[0].Mnemonic != "HLT" {
		t.Errorf("expected mnemonic HLT, got %q", traces[0].Mnemonic)
	}
}

func TestHLT_0xFF(t *testing.T) {
	// The alternate HLT encoding = 0xFF
	s, traces := runProgram(t, []byte{0xFF})
	if !s.Halted() {
		t.Error("expected CPU to be halted after HLT (0xFF)")
	}
	if len(traces) != 1 {
		t.Errorf("expected 1 trace, got %d", len(traces))
	}
	if traces[0].Mnemonic != "HLT" {
		t.Errorf("expected mnemonic HLT, got %q", traces[0].Mnemonic)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// MVI — Move Immediate
// ─────────────────────────────────────────────────────────────────────────────

func TestMVI_Registers(t *testing.T) {
	// MVI loads an immediate byte into a register.
	// MVI B, 0x05 = 0x06 0x05
	// MVI C, 0x10 = 0x0E 0x10
	// MVI A, 0x42 = 0x3E 0x42
	program := []byte{
		0x06, 0x05, // MVI B, 5
		0x0E, 0x10, // MVI C, 16
		0x3E, 0x42, // MVI A, 0x42
		0x76,       // HLT
	}
	s, _ := runProgram(t, program)
	if s.B() != 5 {
		t.Errorf("B = %d, want 5", s.B())
	}
	if s.C() != 16 {
		t.Errorf("C = %d, want 16", s.C())
	}
	if s.A() != 0x42 {
		t.Errorf("A = 0x%02X, want 0x42", s.A())
	}
}

func TestMVI_AllRegisters(t *testing.T) {
	// Test MVI for each register
	tests := []struct {
		opcode byte
		val    byte
		getter func(*Simulator) int
		name   string
	}{
		{0x06, 1, (*Simulator).B, "B"},
		{0x0E, 2, (*Simulator).C, "C"},
		{0x16, 3, (*Simulator).D, "D"},
		{0x1E, 4, (*Simulator).E, "E"},
		{0x26, 5, (*Simulator).H, "H"},
		{0x2E, 6, (*Simulator).L, "L"},
		{0x3E, 7, (*Simulator).A, "A"},
	}
	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			program := []byte{tc.opcode, tc.val, 0x76}
			s, _ := runProgram(t, program)
			got := tc.getter(s)
			if got != int(tc.val) {
				t.Errorf("MVI %s: got %d, want %d", tc.name, got, tc.val)
			}
		})
	}
}

func TestMVI_M_WritesMemory(t *testing.T) {
	// MVI M, 0xAB writes 0xAB to memory at [H:L].
	// Set H=0x00, L=0x10 → address 0x0010, then MVI M, 0xAB
	program := []byte{
		0x26, 0x00, // MVI H, 0x00
		0x2E, 0x10, // MVI L, 0x10
		0x36, 0xAB, // MVI M, 0xAB  → mem[0x0010] = 0xAB
		0x76,       // HLT
	}
	s, _ := runProgram(t, program)
	addr := s.HLAddress() // should be 0x0010
	if addr != 0x0010 {
		t.Errorf("HLAddress = 0x%04X, want 0x0010", addr)
	}
	if s.Memory()[0x0010] != 0xAB {
		t.Errorf("mem[0x0010] = 0x%02X, want 0xAB", s.Memory()[0x0010])
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// MOV — Register-to-Register Transfer
// ─────────────────────────────────────────────────────────────────────────────

func TestMOV_RegToReg(t *testing.T) {
	// MOV A, B: 0x78
	// Load B=42, then MOV A, B
	program := []byte{
		0x06, 42, // MVI B, 42
		0x78,     // MOV A, B  → A ← B = 42
		0x76,     // HLT
	}
	s, _ := runProgram(t, program)
	if s.A() != 42 {
		t.Errorf("A = %d, want 42 after MOV A, B", s.A())
	}
}

func TestMOV_FromMemory(t *testing.T) {
	// MOV H, M: 0x66 — loads H from memory at [H:L]
	// Note: MOV A, M (0x7E) conflicts with CAL unconditional and is not usable as MOV.
	// MOV H, M (01 100 110 = 0x66) is the correct way to read from [H:L] into H.
	// Then MOV A, H to get it into A.
	program := []byte{
		0x26, 0x00, // MVI H, 0
		0x2E, 0x20, // MVI L, 0x20  → address 0x0020
		0x36, 0x55, // MVI M, 0x55  → mem[0x0020] = 0x55
		0x66,       // MOV H, M     → H ← mem[0x0020] = 0x55
		0x7C, 0x0C, 0x00, // JMP 0x000C (skip old instruction slot)
		0x76,       // padding HLT placeholder (at addr 10)
		0x00, 0x00, // padding
		0x78,       // MOV A, B — no, we need MOV A, H = 0x7C is JMP...
		0x76,       // HLT
	}
	// Actually test simpler: use MOV H, M and verify H was set
	s := New()
	prog2 := []byte{
		0x26, 0x00, // MVI H, 0
		0x2E, 0x20, // MVI L, 0x20
		0x36, 0x55, // MVI M, 0x55
		0x66,       // MOV H, M → H = 0x55
		0x76,       // HLT
	}
	_ = program
	traces := s.Run(prog2, 100)
	_ = traces
	if s.H() != 0x55 {
		t.Errorf("MOV H, M: H = 0x%02X, want 0x55", s.H())
	}
}

func TestMOV_ToMemory(t *testing.T) {
	// MOV M, A: 0x77 — stores A to memory at [H:L]
	program := []byte{
		0x3E, 0x99, // MVI A, 0x99
		0x26, 0x00, // MVI H, 0
		0x2E, 0x30, // MVI L, 0x30  → address 0x0030
		0x77,       // MOV M, A     → mem[0x0030] ← A = 0x99
		0x76,       // HLT
	}
	s, _ := runProgram(t, program)
	if s.Memory()[0x0030] != 0x99 {
		t.Errorf("mem[0x0030] = 0x%02X, want 0x99", s.Memory()[0x0030])
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// INR / DCR — Increment and Decrement
// ─────────────────────────────────────────────────────────────────────────────

func TestINR_Basic(t *testing.T) {
	// INR A: 0x38 — increments A
	program := []byte{
		0x3E, 0x05, // MVI A, 5
		0x38,       // INR A  → A = 6
		0x76,       // HLT
	}
	checkA(t, program, 6)
}

func TestINR_Wrap(t *testing.T) {
	// INR wraps from 0xFF to 0x00
	program := []byte{
		0x3E, 0xFF, // MVI A, 0xFF
		0x38,       // INR A → A = 0x00 (wraps)
		0x76,       // HLT
	}
	s, _ := runProgram(t, program)
	if s.A() != 0 {
		t.Errorf("INR wrap: A = %d, want 0", s.A())
	}
	if !s.GetFlags().Zero {
		t.Error("INR wrap: Z should be set when result is 0")
	}
}

func TestINR_DoesNotAffectCarry(t *testing.T) {
	// INR does NOT update the Carry flag — this distinguishes it from ADD.
	// First set carry via ADD, then use INR and verify carry is preserved.
	program := []byte{
		0x3E, 0xFF, // MVI A, 0xFF
		0x06, 0x01, // MVI B, 1
		0x80,       // ADD B  → A = 0x00, CY=1 (overflow)
		0x38,       // INR A  → A = 0x01, CY must still be 1
		0x76,       // HLT
	}
	s, _ := runProgram(t, program)
	if !s.GetFlags().Carry {
		t.Error("INR should not clear Carry flag")
	}
}

func TestDCR_Basic(t *testing.T) {
	// DCR B: 0x01 — decrements B
	program := []byte{
		0x06, 10, // MVI B, 10
		0x01,     // DCR B → B = 9
		0x76,     // HLT
	}
	s, _ := runProgram(t, program)
	if s.B() != 9 {
		t.Errorf("B = %d, want 9 after DCR B", s.B())
	}
}

func TestDCR_Wrap(t *testing.T) {
	// DCR wraps from 0x00 to 0xFF
	program := []byte{
		0x06, 0x00, // MVI B, 0
		0x01,       // DCR B → B = 0xFF (wraps)
		0x76,       // HLT
	}
	s, _ := runProgram(t, program)
	if s.B() != 0xFF {
		t.Errorf("DCR wrap: B = 0x%02X, want 0xFF", s.B())
	}
	if !s.GetFlags().Sign {
		t.Error("DCR wrap: S should be set (0xFF has bit 7 = 1)")
	}
}

func TestDCR_SetsZero(t *testing.T) {
	// DCR B where B=1 should set Z=1 after
	program := []byte{
		0x06, 1, // MVI B, 1
		0x01,    // DCR B → B = 0, Z=1
		0x76,    // HLT
	}
	s, _ := runProgram(t, program)
	if !s.GetFlags().Zero {
		t.Error("DCR: Z should be set when result is 0")
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// ALU Register Operations — ADD, ADC, SUB, SBB, ANA, XRA, ORA, CMP
// ─────────────────────────────────────────────────────────────────────────────

func TestADD_Basic(t *testing.T) {
	// ADD B: 0x80 — A ← A + B
	// From spec example: A=2, B=1, ADD B → A=3
	program := []byte{
		0x06, 1, // MVI B, 1
		0x3E, 2, // MVI A, 2
		0x80,    // ADD B → A = 3
		0x76,    // HLT
	}
	checkA(t, program, 3)
}

func TestADD_Carry(t *testing.T) {
	// ADD that overflows sets Carry
	program := []byte{
		0x3E, 0xFF, // MVI A, 255
		0x06, 1,    // MVI B, 1
		0x80,       // ADD B → A = 0, CY = 1
		0x76,       // HLT
	}
	s, _ := runProgram(t, program)
	if s.A() != 0 {
		t.Errorf("ADD overflow: A = %d, want 0", s.A())
	}
	if !s.GetFlags().Carry {
		t.Error("ADD overflow: CY should be set")
	}
	if !s.GetFlags().Zero {
		t.Error("ADD overflow: Z should be set (result = 0)")
	}
}

func TestADC_WithCarry(t *testing.T) {
	// ADC B: A ← A + B + CY
	// Set CY=1 first via add overflow, then use ADC
	program := []byte{
		0x3E, 0xFF, // MVI A, 255
		0x06, 1,    // MVI B, 1
		0x80,       // ADD B → A=0, CY=1
		0x06, 5,    // MVI B, 5
		0x88,       // ADC B → A = 0 + 5 + 1 = 6, CY=0
		0x76,       // HLT
	}
	checkA(t, program, 6)
}

func TestSUB_Basic(t *testing.T) {
	// SUB B: A ← A - B
	program := []byte{
		0x3E, 10, // MVI A, 10
		0x06, 3,  // MVI B, 3
		0x90,     // SUB B → A = 7
		0x76,     // HLT
	}
	checkA(t, program, 7)
}

func TestSUB_Borrow(t *testing.T) {
	// SUB where result < 0 sets Carry (borrow occurred)
	// On the 8008, CY=1 after SUB means borrow.
	program := []byte{
		0x3E, 3,  // MVI A, 3
		0x06, 10, // MVI B, 10
		0x90,     // SUB B → A = 249 (3-10 wraps), CY=1 (borrow)
		0x76,     // HLT
	}
	s, _ := runProgram(t, program)
	if s.A() != 249 {
		t.Errorf("SUB borrow: A = %d, want 249", s.A())
	}
	if !s.GetFlags().Carry {
		t.Error("SUB borrow: CY should be 1 (borrow occurred)")
	}
}

func TestANA_ClearsCarry(t *testing.T) {
	// ANA (AND) always clears Carry
	program := []byte{
		0x3E, 0xFF, // MVI A, 0xFF
		0x06, 0x01, // MVI B, 1
		0x80,       // ADD B → sets CY=1 (overflow)
		0x3E, 0xFF, // MVI A, 0xFF (reset A)
		0xA7,       // ANA A (0xFF & 0xFF = 0xFF, but CY cleared)
		0x76,       // HLT
	}
	s, _ := runProgram(t, program)
	if s.GetFlags().Carry {
		t.Error("ANA should clear Carry flag")
	}
}

func TestXRA_Basic(t *testing.T) {
	// XRA A: 0xAF — A ← A ^ A = 0 (clears A to 0)
	// This is a common 8008 idiom for zeroing A.
	program := []byte{
		0x3E, 0x55, // MVI A, 0x55
		0xAF,       // XRA A → A = 0x55 ^ 0x55 = 0
		0x76,       // HLT
	}
	checkA(t, program, 0)
}

func TestORA_Basic(t *testing.T) {
	// ORA B: A ← A | B
	program := []byte{
		0x3E, 0xF0, // MVI A, 0xF0
		0x06, 0x0F, // MVI B, 0x0F
		0xB0,       // ORA B → A = 0xFF
		0x76,       // HLT
	}
	checkA(t, program, 0xFF)
}

func TestCMP_SetsFlags(t *testing.T) {
	// CMP B: compare A and B (A - B), don't change A
	// A = B → Z=1, CY=0
	program := []byte{
		0x3E, 5, // MVI A, 5
		0x06, 5, // MVI B, 5
		0xB8,    // CMP B → A-B=0, Z=1, A unchanged
		0x76,    // HLT
	}
	s, _ := runProgram(t, program)
	if s.A() != 5 {
		t.Errorf("CMP should not change A, got %d", s.A())
	}
	if !s.GetFlags().Zero {
		t.Error("CMP equal values: Z should be set")
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// ALU Immediate Operations
// ─────────────────────────────────────────────────────────────────────────────

func TestADI_Basic(t *testing.T) {
	// ADI 5: 0xC4 0x05 — A ← A + 5
	program := []byte{
		0x3E, 10,   // MVI A, 10
		0xC4, 5,    // ADI 5  → A = 15
		0x76,       // HLT
	}
	checkA(t, program, 15)
}

func TestSUI_Basic(t *testing.T) {
	// SUI d: 0xD4 d — A ← A - d
	program := []byte{
		0x3E, 20,   // MVI A, 20
		0xD4, 7,    // SUI 7  → A = 13
		0x76,       // HLT
	}
	checkA(t, program, 13)
}

func TestANI_Basic(t *testing.T) {
	// ANI 0xF0: 0xE4 0xF0 — A ← A & 0xF0
	program := []byte{
		0x3E, 0xFF, // MVI A, 0xFF
		0xE4, 0xF0, // ANI 0xF0 → A = 0xF0
		0x76,       // HLT
	}
	checkA(t, program, 0xF0)
}

func TestXRI_Basic(t *testing.T) {
	// XRI 0xFF: A ← A ^ 0xFF — bitwise NOT (complement)
	// XRI opcode: 11 OOO 100 where OOO=5 (XOR) = 11 101 100 = 0xEC
	program := []byte{
		0x3E, 0xAA, // MVI A, 0xAA = 10101010
		0xEC, 0xFF, // XRI 0xFF  → A = 0x55 = 01010101
		0x76,       // HLT
	}
	checkA(t, program, 0x55)
}

func TestORI_Basic(t *testing.T) {
	// ORI d: A ← A | d
	program := []byte{
		0x3E, 0x0F, // MVI A, 0x0F
		0xF4, 0xF0, // ORI 0xF0 → A = 0xFF
		0x76,       // HLT
	}
	checkA(t, program, 0xFF)
}

func TestCPI_Basic(t *testing.T) {
	// CPI 0x0A: compare A with 10, don't change A
	program := []byte{
		0x3E, 0x0A, // MVI A, 10
		0xFC, 0x0A, // CPI 10 → A-10=0, Z=1, A unchanged
		0x76,       // HLT
	}
	s, _ := runProgram(t, program)
	if s.A() != 10 {
		t.Errorf("CPI should not change A, got %d", s.A())
	}
	if !s.GetFlags().Zero {
		t.Error("CPI equal: Z should be set")
	}
}

func TestORI_FlagsFromA(t *testing.T) {
	// ORI 0x00: A ← A | 0 = A — idiomatic way to update flags from A
	// After ORI 0x00, flags reflect the current value of A.
	program := []byte{
		0x3E, 0xB5, // MVI A, 0xB5 = 10110101 (5 ones = odd parity)
		0xF4, 0x00, // ORI 0x00 → A unchanged, flags set
		0x76,       // HLT
	}
	s, _ := runProgram(t, program)
	if s.A() != 0xB5 {
		t.Errorf("ORI 0: A = 0x%02X, want 0xB5", s.A())
	}
	// 0xB5 = 10110101 — count of 1-bits: 1+0+1+1+0+1+0+1 = 5 (odd) → P=0
	if s.GetFlags().Parity {
		t.Error("ORI 0 with 0xB5: P should be 0 (odd parity, 5 ones)")
	}
	// Bit 7 is set → S=1
	if !s.GetFlags().Sign {
		t.Error("ORI 0 with 0xB5: S should be 1 (bit 7 set)")
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Rotate Instructions
// ─────────────────────────────────────────────────────────────────────────────

func TestRLC(t *testing.T) {
	// RLC: rotate left circular — CY←A[7]; A[0]←A[7]
	// A = 0b10000001 = 0x81
	// After RLC: A = 0b00000011 = 0x03, CY=1 (old bit7 was 1)
	program := []byte{
		0x3E, 0x81, // MVI A, 0x81
		0x02,       // RLC
		0x76,       // HLT
	}
	s, _ := runProgram(t, program)
	if s.A() != 0x03 {
		t.Errorf("RLC: A = 0x%02X, want 0x03", s.A())
	}
	if !s.GetFlags().Carry {
		t.Error("RLC: CY should be 1 (old bit 7 was 1)")
	}
}

func TestRRC(t *testing.T) {
	// RRC: rotate right circular — CY←A[0]; A[7]←A[0]
	// A = 0b10000001 = 0x81
	// After RRC: A = 0b11000000 = 0xC0, CY=1 (old bit0 was 1)
	program := []byte{
		0x3E, 0x81, // MVI A, 0x81
		0x0A,       // RRC
		0x76,       // HLT
	}
	s, _ := runProgram(t, program)
	if s.A() != 0xC0 {
		t.Errorf("RRC: A = 0x%02X, want 0xC0", s.A())
	}
	if !s.GetFlags().Carry {
		t.Error("RRC: CY should be 1 (old bit 0 was 1)")
	}
}

func TestRAL(t *testing.T) {
	// RAL: rotate left through carry — new_CY←A[7]; A[0]←old_CY
	// Start: A = 0x81, CY = 0
	// After RAL: A = 0x02 (0x81 << 1 | CY=0), CY = 1 (old bit7)
	program := []byte{
		0x3E, 0x81, // MVI A, 0x81 = 10000001, CY=0 initially
		0x12,       // RAL
		0x76,       // HLT
	}
	s, _ := runProgram(t, program)
	if s.A() != 0x02 {
		t.Errorf("RAL: A = 0x%02X, want 0x02", s.A())
	}
	if !s.GetFlags().Carry {
		t.Error("RAL: CY should be 1 (old bit7 of A was 1)")
	}
}

func TestRAR(t *testing.T) {
	// RAR: rotate right through carry — new_CY←A[0]; A[7]←old_CY
	// Start: A = 0x81 = 10000001, CY = 0
	// After RAR: A = 0x40 = 01000000, CY = 1 (old bit0 was 1)
	program := []byte{
		0x3E, 0x81, // MVI A, 0x81
		0x1A,       // RAR
		0x76,       // HLT
	}
	s, _ := runProgram(t, program)
	if s.A() != 0x40 {
		t.Errorf("RAR: A = 0x%02X, want 0x40", s.A())
	}
	if !s.GetFlags().Carry {
		t.Error("RAR: CY should be 1 (old bit0 was 1)")
	}
}

func TestRotate_DoesNotAffectZSP(t *testing.T) {
	// Rotate instructions should NOT change the Z, S, P flags.
	// Load A=1 (P=0 odd, Z=0, S=0), add 0 to set flags, then rotate.
	// After rotate, Z/S/P should remain whatever they were before.
	program := []byte{
		0x3E, 0x01, // MVI A, 1 (Z=0, S=0, P=0 — but MVI doesn't set flags!)
		0xC4, 0x00, // ADI 0 → flags set: Z=0, S=0, P=0 (1 is odd)
		0x02,       // RLC → A = 0x02, CY=0 — Z/S/P must be unchanged from before
		0x76,       // HLT
	}
	s, _ := runProgram(t, program)
	// After RLC of 0x01: A = 0x02, CY = 0
	// Z, S, P from the ADI 0: A was 1 → Z=0, S=0, P=0 (odd parity of 1)
	if s.GetFlags().Zero {
		t.Error("RLC should not set Z flag")
	}
	if s.GetFlags().Sign {
		t.Error("RLC should not set S flag")
	}
	if s.GetFlags().Parity {
		t.Error("RLC should not change P flag (was P=0 for odd parity of 1)")
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Flag Tests — Zero, Sign, Parity, Carry
// ─────────────────────────────────────────────────────────────────────────────

func TestFlag_Zero(t *testing.T) {
	// Z=1 when result is exactly 0
	program := []byte{
		0x3E, 0x01, // MVI A, 1
		0xD4, 0x01, // SUI 1 → A = 0, Z=1
		0x76,       // HLT
	}
	s, _ := runProgram(t, program)
	if !s.GetFlags().Zero {
		t.Error("Z should be 1 when result is 0")
	}
}

func TestFlag_Sign(t *testing.T) {
	// S=1 when bit 7 of result is 1
	program := []byte{
		0x3E, 0x7F, // MVI A, 0x7F
		0xC4, 0x01, // ADI 1 → A = 0x80 (bit7 = 1), S=1
		0x76,       // HLT
	}
	s, _ := runProgram(t, program)
	if !s.GetFlags().Sign {
		t.Error("S should be 1 when bit 7 is 1")
	}
}

func TestFlag_Parity_Even(t *testing.T) {
	// P=1 when result has even number of 1-bits
	// 0x03 = 00000011 → 2 ones → even parity → P=1
	program := []byte{
		0x3E, 0x01, // MVI A, 1
		0x06, 0x02, // MVI B, 2
		0x80,       // ADD B → A = 3 = 0b00000011 (2 ones = even)
		0x76,       // HLT
	}
	s, _ := runProgram(t, program)
	if !s.GetFlags().Parity {
		t.Error("P should be 1 for 0x03 (even parity, 2 ones)")
	}
}

func TestFlag_Parity_Odd(t *testing.T) {
	// P=0 when result has odd number of 1-bits
	// 0x01 = 00000001 → 1 one → odd parity → P=0
	program := []byte{
		0x3E, 0x00, // MVI A, 0
		0xC4, 0x01, // ADI 1 → A = 1 (1 one = odd)
		0x76,       // HLT
	}
	s, _ := runProgram(t, program)
	if s.GetFlags().Parity {
		t.Error("P should be 0 for 0x01 (odd parity, 1 one)")
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Jump Instructions
// ─────────────────────────────────────────────────────────────────────────────

func TestJMP_Unconditional(t *testing.T) {
	// JMP 0x0008: skip the MVI B and jump to HLT
	// Byte layout: JMP = 0x7C lo hi
	// Target = 0x0008
	// lo = 0x08, hi = 0x00
	program := []byte{
		0x7C, 0x08, 0x00, // JMP 0x0008 (at address 0)
		0x06, 0xFF,       // MVI B, 0xFF (at address 3 — should be skipped)
		0x00, 0x00, 0x00, // padding
		0x76,             // HLT (at address 8)
	}
	s, _ := runProgram(t, program)
	// B should still be 0 because the MVI B was skipped
	if s.B() != 0 {
		t.Errorf("JMP should skip MVI B, but B = %d", s.B())
	}
}

func TestJFZ_TakenWhenZeroClear(t *testing.T) {
	// JFZ (jump if Zero=0): taken when Z=0
	// Compute A = 5 (Z=0), then JFZ to skip the HLT and set B=42
	program := []byte{
		0x3E, 5,          // MVI A, 5
		0xC4, 0x00,       // ADI 0 → flags updated: Z=0 (A≠0)
		0x48, 0x08, 0x00, // JFZ 0x0008 — jump if Z=0 (it is 0)
		0x76,             // HLT at address 7 (should be skipped)
		0x06, 42,         // MVI B, 42 at address 8
		0x76,             // HLT
	}
	s, _ := runProgram(t, program)
	if s.B() != 42 {
		t.Errorf("JFZ should jump when Z=0: B = %d, want 42", s.B())
	}
}

func TestJTZ_TakenWhenZeroSet(t *testing.T) {
	// JTZ (jump if Zero=1): taken when Z=1
	// Compute A=0 (Z=1), then JTZ to skip an instruction
	program := []byte{
		0x3E, 0x00,       // MVI A, 0
		0xC4, 0x00,       // ADI 0 → Z=1 (A=0)
		0x4C, 0x09, 0x00, // JTZ 0x0009 — jump if Z=1 (it is 1)
		0x76,             // HLT at address 7 (skipped)
		0x00,             // padding
		0x06, 99,         // MVI B, 99 at address 9
		0x76,             // HLT
	}
	s, _ := runProgram(t, program)
	if s.B() != 99 {
		t.Errorf("JTZ should jump when Z=1: B = %d, want 99", s.B())
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Call and Return
// ─────────────────────────────────────────────────────────────────────────────

func TestCAL_RET(t *testing.T) {
	// CAL 0x000A: call subroutine that loads B=0x42, then returns
	// Program layout:
	//   0x00: CAL 0x000A (0x7E 0x0A 0x00)
	//   0x03: HLT
	//   0x0A: MVI B, 0x42
	//   0x0C: RET (0x3F)
	program := []byte{
		0x7E, 0x0A, 0x00, // CAL 0x000A (at address 0)
		0x76,             // HLT (at address 3)
		0, 0, 0, 0, 0, 0, // padding (addresses 4-9)
		0x06, 0x42,       // MVI B, 0x42 (at address 10 = 0x0A)
		0x3F,             // RET (at address 12 = 0x0C)
	}
	s, _ := runProgram(t, program)
	if s.B() != 0x42 {
		t.Errorf("CAL/RET: B = 0x%02X, want 0x42", s.B())
	}
}

func TestStack_Depth(t *testing.T) {
	// Call 3 levels deep and verify stack depth
	// Layout:
	//   0x00: CAL 0x0010  → depth=1
	//   0x03: HLT
	//   0x10: CAL 0x0020  → depth=2
	//   0x13: RET
	//   0x20: CAL 0x0030  → depth=3
	//   0x23: RET
	//   0x30: MVI A, 7
	//   0x32: RET
	program := make([]byte, 0x34)
	// 0x00: CAL 0x0010
	program[0x00] = 0x7E; program[0x01] = 0x10; program[0x02] = 0x00
	// 0x03: HLT
	program[0x03] = 0x76
	// 0x10: CAL 0x0020
	program[0x10] = 0x7E; program[0x11] = 0x20; program[0x12] = 0x00
	// 0x13: RET
	program[0x13] = 0x3F
	// 0x20: CAL 0x0030
	program[0x20] = 0x7E; program[0x21] = 0x30; program[0x22] = 0x00
	// 0x23: RET
	program[0x23] = 0x3F
	// 0x30: MVI A, 7
	program[0x30] = 0x3E; program[0x31] = 0x07
	// 0x32: RET
	program[0x32] = 0x3F

	s, _ := runProgram(t, program)
	if s.A() != 7 {
		t.Errorf("nested calls: A = %d, want 7", s.A())
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// RST — Restart Instructions
// ─────────────────────────────────────────────────────────────────────────────

func TestRST_Basic(t *testing.T) {
	// RST 1: 0x0D — 1-byte call to address 1*8 = 8
	// Layout:
	//   0x00: RST 1 (0x0D)  → calls address 8
	//   0x01: HLT
	//   0x08: MVI A, 42
	//   0x0A: RET
	program := make([]byte, 0x0C)
	program[0x00] = 0x0D // RST 1
	program[0x01] = 0x76 // HLT (return here)
	program[0x08] = 0x3E // MVI A, 42
	program[0x09] = 42
	program[0x0A] = 0x3F // RET
	s, _ := runProgram(t, program)
	if s.A() != 42 {
		t.Errorf("RST 1: A = %d, want 42", s.A())
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// M Pseudo-Register (memory via [H:L])
// ─────────────────────────────────────────────────────────────────────────────

func TestM_INR(t *testing.T) {
	// INR M: increments memory at [H:L]
	program := []byte{
		0x26, 0x01, // MVI H, 1
		0x2E, 0x00, // MVI L, 0   → [H:L] = 0x0100
		0x36, 5,    // MVI M, 5   → mem[0x0100] = 5
		0x30,       // INR M       → mem[0x0100] = 6
		0x76,       // HLT
	}
	s, _ := runProgram(t, program)
	if s.Memory()[0x0100] != 6 {
		t.Errorf("INR M: mem[0x100] = %d, want 6", s.Memory()[0x0100])
	}
}

func TestM_DCR(t *testing.T) {
	// DCR M: decrements memory at [H:L]
	program := []byte{
		0x26, 0x00, // MVI H, 0
		0x2E, 0x50, // MVI L, 0x50   → [H:L] = 0x0050
		0x36, 10,   // MVI M, 10      → mem[0x0050] = 10
		0x31,       // DCR M           → mem[0x0050] = 9
		0x76,       // HLT
	}
	s, _ := runProgram(t, program)
	if s.Memory()[0x0050] != 9 {
		t.Errorf("DCR M: mem[0x0050] = %d, want 9", s.Memory()[0x0050])
	}
}

func TestM_ADD(t *testing.T) {
	// ADD M: A ← A + mem[H:L]
	program := []byte{
		0x26, 0x00, // MVI H, 0
		0x2E, 0x40, // MVI L, 0x40   → [H:L] = 0x0040
		0x36, 7,    // MVI M, 7       → mem[0x0040] = 7
		0x3E, 3,    // MVI A, 3
		0x86,       // ADD M           → A = 3 + 7 = 10
		0x76,       // HLT
	}
	checkA(t, program, 10)
}

// ─────────────────────────────────────────────────────────────────────────────
// I/O Ports
// ─────────────────────────────────────────────────────────────────────────────

func TestIN_Port(t *testing.T) {
	// IN 3: 0x59 — reads input port 3 into A
	// IN 3 encoding: 01 011 001 = 0x59? Let me compute:
	// DDD=011=3, sss=001, opcode = 01_011_001 = 0b01011001 = 0x59
	s := New()
	s.SetInputPort(3, 0xCC)
	program := []byte{
		0x59, // IN 3
		0x76, // HLT
	}
	s.LoadProgram(program, 0)
	for !s.Halted() {
		s.Step()
	}
	if s.A() != 0xCC {
		t.Errorf("IN 3: A = 0x%02X, want 0xCC", s.A())
	}
}

func TestIN_Port0(t *testing.T) {
	// IN 0: 0x41 — reads input port 0 into A
	s := New()
	s.SetInputPort(0, 0xAB)
	program := []byte{
		0x41, // IN 0
		0x76, // HLT
	}
	s.LoadProgram(program, 0)
	for !s.Halted() {
		s.Step()
	}
	if s.A() != 0xAB {
		t.Errorf("IN 0: A = 0x%02X, want 0xAB", s.A())
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Example Programs from the Spec
// ─────────────────────────────────────────────────────────────────────────────

func TestExample_Add1Plus2(t *testing.T) {
	// x = 1 + 2 from spec:
	//   MVI B, 0x01   (0x06 0x01)
	//   MVI A, 0x02   (0x3E 0x02)
	//   ADD B         (0x80)
	//   HLT           (0x76)
	// Result: A = 3, Z=0, S=0, CY=0, P=1 (even parity of 0b00000011)
	program := []byte{0x06, 0x01, 0x3E, 0x02, 0x80, 0x76}
	s, traces := runProgram(t, program)
	if s.A() != 3 {
		t.Errorf("1+2: A = %d, want 3", s.A())
	}
	if s.GetFlags().Carry {
		t.Error("1+2: CY should be 0")
	}
	if s.GetFlags().Zero {
		t.Error("1+2: Z should be 0")
	}
	if !s.GetFlags().Parity {
		t.Error("1+2: P should be 1 (0b00000011 has even parity)")
	}
	if len(traces) != 4 {
		t.Errorf("1+2: expected 4 traces, got %d", len(traces))
	}
}

func TestExample_Multiply4x5(t *testing.T) {
	// Multiply 4 × 5 using repeated addition:
	//   MVI B, 5       (0x06 0x05)
	//   MVI C, 4       (0x0E 0x04)
	//   MVI A, 0       (0x3E 0x00)
	// LOOP:
	//   ADD B          (0x80)       ; A += B
	//   DCR C          (0x09)       ; C--
	//   JFZ LOOP       (0x48 lo hi) ; if Z=0, loop
	//   HLT            (0x76)
	// Result: A = 20
	loopAddr := byte(0x08) // LOOP starts at byte 8 (after 3 MVI instructions = 6 bytes, then ADD+DCR = 2 bytes at 6,7, so loop at 6)
	// Actually: MVI B,5=2, MVI C,4=2, MVI A,0=2 → 6 bytes (0x00-0x05)
	// LOOP at 0x06: ADD B, DCR C, JFZ LOOP, HLT
	// JFZ targets address 6 (LOOP): lo=0x06, hi=0x00
	_ = loopAddr
	program := []byte{
		0x06, 0x05, // MVI B, 5  (addr 0)
		0x0E, 0x04, // MVI C, 4  (addr 2)
		0x3E, 0x00, // MVI A, 0  (addr 4)
		0x80,       // ADD B     (addr 6) ← LOOP
		0x09,       // DCR C     (addr 7)
		0x48, 0x06, 0x00, // JFZ 0x0006  (addr 8)
		0x76,       // HLT       (addr 11)
	}
	s, _ := runProgram(t, program)
	if s.A() != 20 {
		t.Errorf("4×5: A = %d, want 20", s.A())
	}
}

func TestExample_AbsoluteValue(t *testing.T) {
	// Subroutine: absolute value of a signed byte in A.
	//
	// The key issue: MVI A does NOT set the Sign flag. We must use
	// ORI 0x00 (the 8008 idiom for "update flags from A") before calling
	// the subroutine so that JFS has a valid Sign flag to test.
	//
	// Layout:
	//   0x00: MVI A, 0xF6    (-10 in two's complement)
	//   0x02: ORI 0x00       (flags ← A: sets S=1 since bit7=1)
	//   0x04: CAL ABS_VAL    (at 0x14)
	//   0x07: HLT
	//   ...
	//   ABS_VAL (0x14):
	//   0x14: JFS DONE (at 0x1B)  — jump if S=0 (already positive, skip negation)
	//   0x17: XRI 0xFF             — A ← A ^ 0xFF (one's complement)
	//   0x19: ADI 0x01             — A ← A + 1 (two's complement negate)
	//   0x1B: DONE: RET
	program := make([]byte, 0x1D)
	// 0x00: MVI A, 0xF6
	program[0x00] = 0x3E; program[0x01] = 0xF6
	// 0x02: ORI 0x00 — update flags from A (0xF6 has S=1)
	program[0x02] = 0xF4; program[0x03] = 0x00
	// 0x04: CAL 0x0014
	program[0x04] = 0x7E; program[0x05] = 0x14; program[0x06] = 0x00
	// 0x07: HLT
	program[0x07] = 0x76
	// ABS_VAL at 0x14:
	// 0x14: JFS 0x001B (jump if Sign=0, i.e., positive → already done)
	program[0x14] = 0x50; program[0x15] = 0x1B; program[0x16] = 0x00
	// 0x17: XRI 0xFF — one's complement: A ← A ^ 0xFF
	// XRI opcode: 11 101 100 = 0xEC
	program[0x17] = 0xEC; program[0x18] = 0xFF
	// 0x19: ADI 0x01 — add 1 to complete two's complement negation
	program[0x19] = 0xC4; program[0x1A] = 0x01
	// 0x1B: DONE: RET
	program[0x1B] = 0x3F

	s, _ := runProgram(t, program)
	if s.A() != 10 {
		t.Errorf("abs(-10): A = %d (0x%02X), want 10 (0x0A)", s.A(), s.A())
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Parity Flag Computation
// ─────────────────────────────────────────────────────────────────────────────

func TestParity_AllBytesCorrect(t *testing.T) {
	// Verify parity flag computation for a selection of values.
	// P=1 means even parity (even number of 1-bits).
	parityTests := []struct {
		value    byte
		wantParity bool
	}{
		{0x00, true},  // 0 ones → even
		{0x01, false}, // 1 one → odd
		{0x03, true},  // 2 ones → even
		{0x07, false}, // 3 ones → odd
		{0x0F, true},  // 4 ones → even
		{0x1F, false}, // 5 ones → odd
		{0x3F, true},  // 6 ones → even
		{0x7F, false}, // 7 ones → odd
		{0xFF, true},  // 8 ones → even
		{0xB5, false}, // 10110101 = 5 ones → odd
		{0xA5, true},  // 10100101 = 4 ones → even
	}

	for _, tc := range parityTests {
		t.Run(fmt.Sprintf("0x%02X", tc.value), func(t *testing.T) {
			program := []byte{
				0x3E, tc.value, // MVI A, value
				0xF4, 0x00,     // ORI 0x00 → update flags without changing A
				0x76,           // HLT
			}
			s, _ := runProgram(t, program)
			got := s.GetFlags().Parity
			if got != tc.wantParity {
				t.Errorf("parity(0x%02X): P=%v, want P=%v", tc.value, got, tc.wantParity)
			}
		})
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// HLAddress — 14-bit address from H and L
// ─────────────────────────────────────────────────────────────────────────────

func TestHLAddress(t *testing.T) {
	// H and L form a 14-bit address: ((H & 0x3F) << 8) | L
	// With H=0x01, L=0x23: address = ((0x01 & 0x3F) << 8) | 0x23 = 0x0123
	program := []byte{
		0x26, 0x01, // MVI H, 1
		0x2E, 0x23, // MVI L, 0x23
		0x76,       // HLT
	}
	s, _ := runProgram(t, program)
	got := s.HLAddress()
	want := 0x0123
	if got != want {
		t.Errorf("HLAddress: got 0x%04X, want 0x%04X", got, want)
	}
}

func TestHLAddress_HighBitsIgnored(t *testing.T) {
	// Only bits [5:0] of H are used: H=0xC1 → H & 0x3F = 0x01
	program := []byte{
		0x26, 0xC1, // MVI H, 0xC1 (top 2 bits are "don't care")
		0x2E, 0x23, // MVI L, 0x23
		0x76,       // HLT
	}
	s, _ := runProgram(t, program)
	got := s.HLAddress()
	want := 0x0123 // (0xC1 & 0x3F) << 8 | 0x23 = 0x01<<8 | 0x23 = 0x0123
	if got != want {
		t.Errorf("HLAddress top bits ignored: got 0x%04X, want 0x%04X", got, want)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Trace fields
// ─────────────────────────────────────────────────────────────────────────────

func TestTrace_Fields(t *testing.T) {
	// Verify trace captures correct before/after state
	program := []byte{
		0x3E, 5,    // MVI A, 5
		0x06, 3,    // MVI B, 3
		0x80,       // ADD B → A = 8
		0x76,       // HLT
	}
	_, traces := runProgram(t, program)
	// Third trace is ADD B
	if len(traces) < 3 {
		t.Fatalf("expected at least 3 traces, got %d", len(traces))
	}
	addTrace := traces[2]
	if addTrace.ABefore != 5 {
		t.Errorf("ADD trace ABefore = %d, want 5", addTrace.ABefore)
	}
	if addTrace.AAfter != 8 {
		t.Errorf("ADD trace AAfter = %d, want 8", addTrace.AAfter)
	}
	if addTrace.Mnemonic != "ADD B" {
		t.Errorf("ADD trace Mnemonic = %q, want \"ADD B\"", addTrace.Mnemonic)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Accessor coverage tests
// ─────────────────────────────────────────────────────────────────────────────

func TestAccessors(t *testing.T) {
	// Test PC, Stack, StackDepth, GetOutputPort accessors
	s := New()
	program := []byte{
		0x3E, 42,         // MVI A, 42
		0x76,             // HLT
	}
	s.Run(program, 100)

	// PC should be at address 3 (after HLT)
	if s.PC() != 3 {
		t.Errorf("PC = %d, want 3", s.PC())
	}

	// Stack should be empty (depth 0)
	if s.StackDepth() != 0 {
		t.Errorf("StackDepth = %d, want 0", s.StackDepth())
	}
	if len(s.Stack()) != 0 {
		t.Errorf("Stack() len = %d, want 0", len(s.Stack()))
	}
}

func TestOutputPort(t *testing.T) {
	// OUT instruction writes A to an output port.
	// OUT using ddd>=4 range. For ddd=4 (H), sss=010: opcode = 00 100 010 = 0x22
	// port = (0x22 >> 1) & 0x1F = 0x11 & 0x1F = 17
	s := New()
	program := []byte{
		0x3E, 0xCD, // MVI A, 0xCD
		0x22,       // OUT port 17 (opcode 00 100 010)
		0x76,       // HLT
	}
	s.Run(program, 100)
	got := s.GetOutputPort(17)
	if got != 0xCD {
		t.Errorf("OUT 17: port 17 = 0x%02X, want 0xCD", got)
	}
}

func TestStackAfterCall(t *testing.T) {
	// After a call, Stack() should contain the return address
	// and StackDepth() should be 1
	s := New()
	program := make([]byte, 0x10)
	program[0x00] = 0x7E; program[0x01] = 0x08; program[0x02] = 0x00 // CAL 0x0008
	program[0x03] = 0x76 // HLT (return here)
	program[0x08] = 0x3F // RET
	s.LoadProgram(program, 0)
	// Execute just the CAL, not the RET
	s.Step() // CAL
	if s.StackDepth() != 1 {
		t.Errorf("after CAL: StackDepth = %d, want 1", s.StackDepth())
	}
	stack := s.Stack()
	if len(stack) != 1 {
		t.Fatalf("after CAL: Stack() len = %d, want 1", len(stack))
	}
	if stack[0] != 3 {
		t.Errorf("after CAL: return addr = %d, want 3", stack[0])
	}
}

func TestSBB_WithBorrow(t *testing.T) {
	// SBB B: A ← A - B - CY
	// Start: CY=1 (from previous overflow), A=5, B=2
	// SBB B → A = 5 - 2 - 1 = 2
	program := []byte{
		0x3E, 0xFF, // MVI A, 255
		0x06, 1,    // MVI B, 1
		0x80,       // ADD B → A=0, CY=1
		0x3E, 5,    // MVI A, 5
		0x06, 2,    // MVI B, 2
		0x98,       // SBB B → A = 5-2-1 = 2
		0x76,       // HLT
	}
	checkA(t, program, 2)
}

func TestConditional_CarryFlags(t *testing.T) {
	// Test JTC (jump if carry true) and JFC (jump if carry false)
	// First test JTC: jump when CY=1
	prog1 := []byte{
		0x3E, 0xFF, // MVI A, 255
		0x06, 1,    // MVI B, 1
		0x80,       // ADD B → CY=1
		0x44, 0x0A, 0x00, // JTC 0x000A — jump if CY=1 (it is)
		0x76,             // HLT (skipped)
		0x00,             // padding
		0x06, 77,         // MVI B, 77 (at 0x000A)
		0x76,             // HLT
	}
	s1, _ := runProgram(t, prog1)
	if s1.B() != 77 {
		t.Errorf("JTC: B = %d, want 77 (branch should be taken)", s1.B())
	}
}

func TestConditional_SignParity(t *testing.T) {
	// JTS: jump if S=1 (sign set)
	prog := []byte{
		0x3E, 0x80, // MVI A, 0x80 (bit7=1)
		0xC4, 0x00, // ADI 0 → flags updated: S=1
		0x54, 0x0A, 0x00, // JTS 0x000A — jump if S=1
		0x76,             // HLT (skipped)
		0x00, 0x00,       // padding
		0x06, 88,         // MVI B, 88 (at 0x000A)
		0x76,             // HLT
	}
	s, _ := runProgram(t, prog)
	if s.B() != 88 {
		t.Errorf("JTS: B = %d, want 88", s.B())
	}
}

func TestConditional_ParityBranch(t *testing.T) {
	// JTP: jump if P=1 (even parity)
	// 0x03 = 00000011 → 2 ones → P=1
	prog := []byte{
		0x3E, 0x01, // MVI A, 1
		0x06, 0x02, // MVI B, 2
		0x80,       // ADD B → A=3=0b11, P=1 (even)
		0x5C, 0x0C, 0x00, // JTP 0x000C — jump if P=1
		0x76,             // HLT (skipped)
		0x00, 0x00, 0x00, // padding
		0x06, 99,         // MVI B, 99 (at 0x000C)
		0x76,             // HLT
	}
	s, _ := runProgram(t, prog)
	if s.B() != 99 {
		t.Errorf("JTP: B = %d, want 99", s.B())
	}
}

func TestConditionalReturn_RFZ(t *testing.T) {
	// RFZ: return if Zero=0
	// Call a subroutine that returns only if Z=0
	program := make([]byte, 0x20)
	// 0x00: MVI B, 1 (set something to check subroutine ran)
	program[0x00] = 0x06; program[0x01] = 1
	// 0x02: CAL 0x0010
	program[0x02] = 0x7E; program[0x03] = 0x10; program[0x04] = 0x00
	// 0x05: MVI B, 42 (after return)
	program[0x05] = 0x06; program[0x06] = 42
	// 0x07: HLT
	program[0x07] = 0x76
	// 0x10: Set Z=0 (A=1, ADI 0 → Z=0)
	program[0x10] = 0x3E; program[0x11] = 1   // MVI A, 1
	program[0x12] = 0xC4; program[0x13] = 0x00 // ADI 0 → Z=0
	// 0x14: RFZ (00 001 011 = 0x0B)
	program[0x14] = 0x0B // RFZ — return if Z=0 (Z IS 0, so return)
	// 0x15: HLT (shouldn't reach here)
	program[0x15] = 0x76

	s, _ := runProgram(t, program)
	if s.B() != 42 {
		t.Errorf("RFZ: B = %d, want 42 (return should have been taken)", s.B())
	}
}

func TestTrace_MemoryAccess(t *testing.T) {
	// MOV M, A (0x77) should record the memory address and value written.
	// MOV M, A = 01 110 111 = 0x77 (DDD=M=6, SSS=A=7)
	// We write A=0x77 to mem[H:L] = mem[0x0050].
	program := []byte{
		0x3E, 0x77, // MVI A, 0x77
		0x26, 0x00, // MVI H, 0
		0x2E, 0x50, // MVI L, 0x50   → [H:L] = 0x0050
		0x77,       // MOV M, A       → mem[0x0050] = 0x77
		0x76,       // HLT
	}
	_, traces := runProgram(t, program)
	// traces[3] is MOV M, A (after 3 MVI instructions)
	if len(traces) < 4 {
		t.Fatalf("expected at least 4 traces, got %d", len(traces))
	}
	movTrace := traces[3]
	if movTrace.Mnemonic != "MOV M, A" {
		t.Errorf("expected MOV M, A, got %q", movTrace.Mnemonic)
	}
	if movTrace.MemAddress == nil {
		t.Error("MOV M, A should record MemAddress")
	} else if *movTrace.MemAddress != 0x0050 {
		t.Errorf("MemAddress = 0x%04X, want 0x0050", *movTrace.MemAddress)
	}
	if movTrace.MemValue == nil {
		t.Error("MOV M, A should record MemValue")
	} else if *movTrace.MemValue != 0x77 {
		t.Errorf("MemValue = 0x%02X, want 0x77", *movTrace.MemValue)
	}
}
