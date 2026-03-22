package intel4004gatelevel

// Tests for the Intel 4004 gate-level simulator.
//
// These tests verify that every instruction works correctly when routed
// through real logic gates. The test structure mirrors the behavioral
// simulator's tests — same programs, same expected results.

import (
	"testing"
)

// ===================================================================
// Basic instructions
// ===================================================================

func TestNOPDoesNothing(t *testing.T) {
	cpu := NewIntel4004GateLevel()
	traces := cpu.Run([]byte{0x00, 0x01}, 10000)
	if cpu.Accumulator() != 0 {
		t.Errorf("expected accumulator=0, got %d", cpu.Accumulator())
	}
	if traces[0].Mnemonic != "NOP" {
		t.Errorf("expected NOP, got %s", traces[0].Mnemonic)
	}
}

func TestMultipleNOPs(t *testing.T) {
	cpu := NewIntel4004GateLevel()
	traces := cpu.Run([]byte{0x00, 0x00, 0x00, 0x01}, 10000)
	if len(traces) != 4 {
		t.Errorf("expected 4 traces, got %d", len(traces))
	}
}

func TestHLTStops(t *testing.T) {
	cpu := NewIntel4004GateLevel()
	traces := cpu.Run([]byte{0x01}, 10000)
	if !cpu.Halted() {
		t.Error("expected CPU to be halted")
	}
	if len(traces) != 1 {
		t.Errorf("expected 1 trace, got %d", len(traces))
	}
}

func TestLDMValues(t *testing.T) {
	for n := 0; n < 16; n++ {
		cpu := NewIntel4004GateLevel()
		cpu.Run([]byte{byte(0xD0 | n), 0x01}, 10000)
		if cpu.Accumulator() != n {
			t.Errorf("LDM %d: expected accumulator=%d, got %d", n, n, cpu.Accumulator())
		}
	}
}

func TestLDReadsRegister(t *testing.T) {
	cpu := NewIntel4004GateLevel()
	cpu.Run([]byte{0xD7, 0xB0, 0xA0, 0x01}, 10000) // LDM 7, XCH R0, LD R0
	if cpu.Accumulator() != 7 {
		t.Errorf("expected accumulator=7, got %d", cpu.Accumulator())
	}
}

func TestXCHSwaps(t *testing.T) {
	cpu := NewIntel4004GateLevel()
	cpu.Run([]byte{0xD7, 0xB0, 0x01}, 10000)
	if cpu.Registers()[0] != 7 {
		t.Errorf("expected R0=7, got %d", cpu.Registers()[0])
	}
	if cpu.Accumulator() != 0 {
		t.Errorf("expected accumulator=0, got %d", cpu.Accumulator())
	}
}

func TestINCWraps(t *testing.T) {
	cpu := NewIntel4004GateLevel()
	cpu.Run([]byte{0xDF, 0xB0, 0x60, 0x01}, 10000) // LDM 15, XCH R0, INC R0
	if cpu.Registers()[0] != 0 {
		t.Errorf("expected R0=0, got %d", cpu.Registers()[0])
	}
}

func TestINCNoCarry(t *testing.T) {
	cpu := NewIntel4004GateLevel()
	// Set carry, then INC — carry should stay
	cpu.Run([]byte{0xDF, 0xB1, 0xDF, 0x81, 0x60, 0x01}, 10000)
	if !cpu.Carry() {
		t.Error("expected carry=true")
	}
}

// ===================================================================
// Arithmetic
// ===================================================================

func TestADDBasic(t *testing.T) {
	cpu := NewIntel4004GateLevel()
	cpu.Run([]byte{0xD3, 0xB0, 0xD2, 0x80, 0x01}, 10000)
	if cpu.Accumulator() != 5 {
		t.Errorf("expected accumulator=5, got %d", cpu.Accumulator())
	}
	if cpu.Carry() {
		t.Error("expected carry=false")
	}
}

func TestADDOverflow(t *testing.T) {
	cpu := NewIntel4004GateLevel()
	cpu.Run([]byte{0xD1, 0xB0, 0xDF, 0x80, 0x01}, 10000)
	if cpu.Accumulator() != 0 {
		t.Errorf("expected accumulator=0, got %d", cpu.Accumulator())
	}
	if !cpu.Carry() {
		t.Error("expected carry=true")
	}
}

func TestADDCarryIn(t *testing.T) {
	cpu := NewIntel4004GateLevel()
	cpu.Run([]byte{
		0xDF, 0xB0, 0xDF, 0x80, // 15+15 -> carry=1
		0xD1, 0xB1, 0xD1, 0x81, // 1+1+carry = 3
		0x01,
	}, 10000)
	if cpu.Accumulator() != 3 {
		t.Errorf("expected accumulator=3, got %d", cpu.Accumulator())
	}
}

func TestSUBBasic(t *testing.T) {
	cpu := NewIntel4004GateLevel()
	cpu.Run([]byte{0xD3, 0xB0, 0xD5, 0x90, 0x01}, 10000)
	if cpu.Accumulator() != 2 {
		t.Errorf("expected accumulator=2, got %d", cpu.Accumulator())
	}
	if !cpu.Carry() {
		t.Error("expected carry=true (no borrow)")
	}
}

func TestSUBUnderflow(t *testing.T) {
	cpu := NewIntel4004GateLevel()
	cpu.Run([]byte{0xD1, 0xB0, 0xD0, 0x90, 0x01}, 10000)
	if cpu.Accumulator() != 15 {
		t.Errorf("expected accumulator=15, got %d", cpu.Accumulator())
	}
	if cpu.Carry() {
		t.Error("expected carry=false (borrow)")
	}
}

// ===================================================================
// Accumulator operations
// ===================================================================

func TestCLB(t *testing.T) {
	cpu := NewIntel4004GateLevel()
	cpu.Run([]byte{0xDF, 0xB0, 0xDF, 0x80, 0xF0, 0x01}, 10000)
	if cpu.Accumulator() != 0 {
		t.Errorf("expected accumulator=0, got %d", cpu.Accumulator())
	}
	if cpu.Carry() {
		t.Error("expected carry=false")
	}
}

func TestCLC(t *testing.T) {
	cpu := NewIntel4004GateLevel()
	cpu.Run([]byte{0xDF, 0xB0, 0xDF, 0x80, 0xF1, 0x01}, 10000)
	if cpu.Carry() {
		t.Error("expected carry=false")
	}
}

func TestIAC(t *testing.T) {
	cpu := NewIntel4004GateLevel()
	cpu.Run([]byte{0xD5, 0xF2, 0x01}, 10000)
	if cpu.Accumulator() != 6 {
		t.Errorf("expected accumulator=6, got %d", cpu.Accumulator())
	}
}

func TestIACOverflow(t *testing.T) {
	cpu := NewIntel4004GateLevel()
	cpu.Run([]byte{0xDF, 0xF2, 0x01}, 10000)
	if cpu.Accumulator() != 0 {
		t.Errorf("expected accumulator=0, got %d", cpu.Accumulator())
	}
	if !cpu.Carry() {
		t.Error("expected carry=true")
	}
}

func TestCMC(t *testing.T) {
	cpu := NewIntel4004GateLevel()
	cpu.Run([]byte{0xF3, 0x01}, 10000)
	if !cpu.Carry() {
		t.Error("expected carry=true")
	}
}

func TestCMA(t *testing.T) {
	cpu := NewIntel4004GateLevel()
	cpu.Run([]byte{0xD5, 0xF4, 0x01}, 10000)
	if cpu.Accumulator() != 10 {
		t.Errorf("expected accumulator=10, got %d", cpu.Accumulator())
	}
}

func TestRAL(t *testing.T) {
	cpu := NewIntel4004GateLevel()
	cpu.Run([]byte{0xD5, 0xF5, 0x01}, 10000) // 0101 -> 1010
	if cpu.Accumulator() != 0b1010 {
		t.Errorf("expected accumulator=10, got %d", cpu.Accumulator())
	}
}

func TestRAR(t *testing.T) {
	cpu := NewIntel4004GateLevel()
	cpu.Run([]byte{0xD4, 0xF6, 0x01}, 10000) // 0100 -> 0010
	if cpu.Accumulator() != 2 {
		t.Errorf("expected accumulator=2, got %d", cpu.Accumulator())
	}
}

func TestTCC(t *testing.T) {
	cpu := NewIntel4004GateLevel()
	cpu.Run([]byte{0xFA, 0xF7, 0x01}, 10000)
	if cpu.Accumulator() != 1 {
		t.Errorf("expected accumulator=1, got %d", cpu.Accumulator())
	}
	if cpu.Carry() {
		t.Error("expected carry=false")
	}
}

func TestDAC(t *testing.T) {
	cpu := NewIntel4004GateLevel()
	cpu.Run([]byte{0xD5, 0xF8, 0x01}, 10000)
	if cpu.Accumulator() != 4 {
		t.Errorf("expected accumulator=4, got %d", cpu.Accumulator())
	}
	if !cpu.Carry() {
		t.Error("expected carry=true")
	}
}

func TestDACZero(t *testing.T) {
	cpu := NewIntel4004GateLevel()
	cpu.Run([]byte{0xD0, 0xF8, 0x01}, 10000)
	if cpu.Accumulator() != 15 {
		t.Errorf("expected accumulator=15, got %d", cpu.Accumulator())
	}
	if cpu.Carry() {
		t.Error("expected carry=false")
	}
}

func TestTCS(t *testing.T) {
	cpu := NewIntel4004GateLevel()
	cpu.Run([]byte{0xFA, 0xF9, 0x01}, 10000)
	if cpu.Accumulator() != 10 {
		t.Errorf("expected accumulator=10, got %d", cpu.Accumulator())
	}
}

func TestSTC(t *testing.T) {
	cpu := NewIntel4004GateLevel()
	cpu.Run([]byte{0xFA, 0x01}, 10000)
	if !cpu.Carry() {
		t.Error("expected carry=true")
	}
}

func TestDAA(t *testing.T) {
	cpu := NewIntel4004GateLevel()
	cpu.Run([]byte{0xDC, 0xFB, 0x01}, 10000)
	if cpu.Accumulator() != 2 {
		t.Errorf("expected accumulator=2, got %d", cpu.Accumulator())
	}
	if !cpu.Carry() {
		t.Error("expected carry=true")
	}
}

func TestKBPAllValues(t *testing.T) {
	expected := map[int]int{0: 0, 1: 1, 2: 2, 4: 3, 8: 4, 3: 15, 15: 15}
	for inp, out := range expected {
		cpu := NewIntel4004GateLevel()
		cpu.Run([]byte{byte(0xD0 | inp), 0xFC, 0x01}, 10000)
		if cpu.Accumulator() != out {
			t.Errorf("KBP(%d)=%d, expected %d", inp, cpu.Accumulator(), out)
		}
	}
}

func TestDCL(t *testing.T) {
	cpu := NewIntel4004GateLevel()
	cpu.Run([]byte{0xD2, 0xFD, 0x01}, 10000)
	if cpu.RAMBank() != 2 {
		t.Errorf("expected RAM bank=2, got %d", cpu.RAMBank())
	}
}

// ===================================================================
// Jump instructions
// ===================================================================

func TestJUN(t *testing.T) {
	cpu := NewIntel4004GateLevel()
	cpu.Run([]byte{0x40, 0x04, 0xD5, 0x01, 0x01}, 10000)
	if cpu.Accumulator() != 0 {
		t.Errorf("expected accumulator=0 (LDM 5 skipped), got %d", cpu.Accumulator())
	}
}

func TestJCNZero(t *testing.T) {
	cpu := NewIntel4004GateLevel()
	cpu.Run([]byte{0x14, 0x04, 0xD5, 0x01, 0x01}, 10000)
	if cpu.Accumulator() != 0 {
		t.Errorf("expected accumulator=0 (A==0 -> jump), got %d", cpu.Accumulator())
	}
}

func TestJCNNonzeroNoJump(t *testing.T) {
	cpu := NewIntel4004GateLevel()
	cpu.Run([]byte{0xD3, 0x14, 0x06, 0xD5, 0x01, 0x01, 0x01}, 10000)
	if cpu.Accumulator() != 5 {
		t.Errorf("expected accumulator=5, got %d", cpu.Accumulator())
	}
}

func TestJCNInvert(t *testing.T) {
	cpu := NewIntel4004GateLevel()
	cpu.Run([]byte{0xD3, 0x1C, 0x06, 0xD5, 0x01, 0x01, 0x01}, 10000)
	if cpu.Accumulator() != 3 {
		t.Errorf("expected accumulator=3 (A!=0 -> jump), got %d", cpu.Accumulator())
	}
}

func TestISZLoop(t *testing.T) {
	cpu := NewIntel4004GateLevel()
	cpu.Run([]byte{0xDE, 0xB0, 0x70, 0x02, 0x01}, 10000)
	if cpu.Registers()[0] != 0 {
		t.Errorf("expected R0=0, got %d", cpu.Registers()[0])
	}
}

// ===================================================================
// Subroutines
// ===================================================================

func TestJMSBBL(t *testing.T) {
	cpu := NewIntel4004GateLevel()
	cpu.Run([]byte{
		0x50, 0x04, // JMS 0x004
		0x01,       // HLT (returned here)
		0x00,       // padding
		0xC5,       // BBL 5
	}, 10000)
	if cpu.Accumulator() != 5 {
		t.Errorf("expected accumulator=5, got %d", cpu.Accumulator())
	}
}

func TestNested(t *testing.T) {
	cpu := NewIntel4004GateLevel()
	cpu.Run([]byte{
		0x50, 0x06, // JMS sub1
		0xB0, 0x01, // XCH R0, HLT
		0x00, 0x00, // padding
		0x50, 0x0C, // sub1: JMS sub2
		0xB1,       // XCH R1
		0xD9, 0xC0, // LDM 9, BBL 0
		0x00,       // padding
		0xC3,       // sub2: BBL 3
	}, 10000)
	if cpu.Registers()[1] != 3 {
		t.Errorf("expected R1=3, got %d", cpu.Registers()[1])
	}
}

// ===================================================================
// Register pairs
// ===================================================================

func TestFIM(t *testing.T) {
	cpu := NewIntel4004GateLevel()
	cpu.Run([]byte{0x20, 0xAB, 0x01}, 10000)
	regs := cpu.Registers()
	if regs[0] != 0xA {
		t.Errorf("expected R0=0xA, got %d", regs[0])
	}
	if regs[1] != 0xB {
		t.Errorf("expected R1=0xB, got %d", regs[1])
	}
}

func TestSRCWRMRDM(t *testing.T) {
	cpu := NewIntel4004GateLevel()
	cpu.Run([]byte{
		0x20, 0x00, 0x21, 0xD7, 0xE0, // SRC P0, LDM 7, WRM
		0xD0,                           // LDM 0
		0x20, 0x00, 0x21, 0xE9,         // SRC P0, RDM
		0x01,
	}, 10000)
	if cpu.Accumulator() != 7 {
		t.Errorf("expected accumulator=7, got %d", cpu.Accumulator())
	}
}

func TestJIN(t *testing.T) {
	cpu := NewIntel4004GateLevel()
	cpu.Run([]byte{0x22, 0x06, 0x33, 0xD5, 0x01, 0x00, 0x01}, 10000)
	if cpu.Accumulator() != 0 {
		t.Errorf("expected accumulator=0 (LDM 5 skipped), got %d", cpu.Accumulator())
	}
}

// ===================================================================
// RAM I/O
// ===================================================================

func TestStatusWriteRead(t *testing.T) {
	cpu := NewIntel4004GateLevel()
	cpu.Run([]byte{
		0x20, 0x00, 0x21, // SRC P0
		0xD3, 0xE4,       // LDM 3, WR0
		0xD0,             // LDM 0
		0x20, 0x00, 0x21, // SRC P0
		0xEC,             // RD0
		0x01,
	}, 10000)
	if cpu.Accumulator() != 3 {
		t.Errorf("expected accumulator=3, got %d", cpu.Accumulator())
	}
}

func TestWRRRDR(t *testing.T) {
	cpu := NewIntel4004GateLevel()
	cpu.Run([]byte{0xDB, 0xE2, 0xD0, 0xEA, 0x01}, 10000)
	if cpu.Accumulator() != 11 {
		t.Errorf("expected accumulator=11, got %d", cpu.Accumulator())
	}
}

func TestRAMBanking(t *testing.T) {
	cpu := NewIntel4004GateLevel()
	cpu.Run([]byte{
		0xD0, 0xFD,       // DCL bank 0
		0x20, 0x00, 0x21, // SRC P0
		0xD5, 0xE0,       // LDM 5, WRM
		0xD1, 0xFD,       // DCL bank 1
		0x20, 0x00, 0x21,
		0xD9, 0xE0, // LDM 9, WRM
		0xD0, 0xFD, // DCL bank 0
		0x20, 0x00, 0x21,
		0xE9, // RDM
		0x01,
	}, 10000)
	if cpu.Accumulator() != 5 {
		t.Errorf("expected accumulator=5, got %d", cpu.Accumulator())
	}
}

// ===================================================================
// End-to-end programs
// ===================================================================

func TestXEquals1Plus2(t *testing.T) {
	cpu := NewIntel4004GateLevel()
	cpu.Run([]byte{0xD1, 0xB0, 0xD2, 0x80, 0xB1, 0x01}, 10000)
	if cpu.Registers()[1] != 3 {
		t.Errorf("expected R1=3, got %d", cpu.Registers()[1])
	}
	if !cpu.Halted() {
		t.Error("expected CPU to be halted")
	}
}

func TestMultiply3x4(t *testing.T) {
	cpu := NewIntel4004GateLevel()
	cpu.Run([]byte{
		0xD3, 0xB0, 0xDC, 0xB1,
		0xD0, 0x80, 0x71, 0x05,
		0xB2, 0x01,
	}, 10000)
	if cpu.Registers()[2] != 12 {
		t.Errorf("expected R2=12, got %d", cpu.Registers()[2])
	}
}

func TestBCD7Plus8(t *testing.T) {
	cpu := NewIntel4004GateLevel()
	cpu.Run([]byte{
		0xD8, 0xB0, 0xD7, 0x80, 0xFB, 0x01,
	}, 10000)
	if cpu.Accumulator() != 5 {
		t.Errorf("expected accumulator=5, got %d", cpu.Accumulator())
	}
	if !cpu.Carry() {
		t.Error("expected carry=true")
	}
}

func TestCountdown(t *testing.T) {
	cpu := NewIntel4004GateLevel()
	cpu.Run([]byte{0xD5, 0xF8, 0x1C, 0x01, 0x01}, 10000)
	if cpu.Accumulator() != 0 {
		t.Errorf("expected accumulator=0, got %d", cpu.Accumulator())
	}
}

func TestMaxSteps(t *testing.T) {
	cpu := NewIntel4004GateLevel()
	traces := cpu.Run([]byte{0x40, 0x00}, 10)
	if len(traces) != 10 {
		t.Errorf("expected 10 traces, got %d", len(traces))
	}
}

func TestGateCount(t *testing.T) {
	cpu := NewIntel4004GateLevel()
	count := cpu.GateCount()
	if count <= 500 {
		t.Errorf("expected gate count > 500, got %d", count)
	}
}

// ===================================================================
// Component tests
// ===================================================================

func TestBitsRoundtrip(t *testing.T) {
	for val := 0; val < 16; val++ {
		if BitsToInt(IntToBits(val, 4)) != val {
			t.Errorf("4-bit roundtrip failed for %d", val)
		}
	}
	for val := 0; val < 4096; val++ {
		if BitsToInt(IntToBits(val, 12)) != val {
			t.Errorf("12-bit roundtrip failed for %d", val)
		}
	}
}

func TestALUAdd(t *testing.T) {
	alu := NewGateALU()
	result, carry := alu.Add(5, 3, 0)
	if result != 8 {
		t.Errorf("expected 8, got %d", result)
	}
	if carry {
		t.Error("expected no carry")
	}
}

func TestALUSub(t *testing.T) {
	alu := NewGateALU()
	result, carry := alu.Subtract(5, 3, 1)
	if result != 2 {
		t.Errorf("expected 2, got %d", result)
	}
	if !carry {
		t.Error("expected carry (no borrow)")
	}
}

func TestRegisterFileReadWrite(t *testing.T) {
	rf := NewRegisterFile()
	rf.Write(5, 11)
	if rf.Read(5) != 11 {
		t.Errorf("expected 11, got %d", rf.Read(5))
	}
	if rf.Read(0) != 0 {
		t.Errorf("expected 0, got %d", rf.Read(0))
	}
}

func TestPCIncrement(t *testing.T) {
	pc := NewProgramCounter()
	if pc.Read() != 0 {
		t.Errorf("expected 0, got %d", pc.Read())
	}
	pc.Increment()
	if pc.Read() != 1 {
		t.Errorf("expected 1, got %d", pc.Read())
	}
	pc.Increment()
	if pc.Read() != 2 {
		t.Errorf("expected 2, got %d", pc.Read())
	}
}

func TestStackPushPop(t *testing.T) {
	stack := NewHardwareStack()
	stack.Push(0x100)
	stack.Push(0x200)
	if stack.Pop() != 0x200 {
		t.Error("expected 0x200")
	}
	if stack.Pop() != 0x100 {
		t.Error("expected 0x100")
	}
}

func TestDecoder(t *testing.T) {
	d := Decode(0xD5, -1)
	if d.IsLDM != 1 {
		t.Error("expected IsLDM=1")
	}
	if d.Immediate != 5 {
		t.Errorf("expected Immediate=5, got %d", d.Immediate)
	}

	d = Decode(0x80, -1)
	if d.IsADD != 1 {
		t.Error("expected IsADD=1")
	}
	if d.RegIndex != 0 {
		t.Errorf("expected RegIndex=0, got %d", d.RegIndex)
	}
}

// ===================================================================
// Additional edge-case tests for comprehensive coverage
// ===================================================================

func TestALUComplement(t *testing.T) {
	alu := NewGateALU()
	// NOT(5) = NOT(0101) = 1010 = 10
	if alu.Complement(5) != 10 {
		t.Errorf("expected 10, got %d", alu.Complement(5))
	}
	// NOT(0) = 15
	if alu.Complement(0) != 15 {
		t.Errorf("expected 15, got %d", alu.Complement(0))
	}
}

func TestALUIncrement(t *testing.T) {
	alu := NewGateALU()
	result, carry := alu.Increment(14)
	if result != 15 || carry {
		t.Errorf("expected (15, false), got (%d, %v)", result, carry)
	}
	result, carry = alu.Increment(15)
	if result != 0 || !carry {
		t.Errorf("expected (0, true), got (%d, %v)", result, carry)
	}
}

func TestALUDecrement(t *testing.T) {
	alu := NewGateALU()
	result, carry := alu.Decrement(5)
	if result != 4 || !carry {
		t.Errorf("expected (4, true), got (%d, %v)", result, carry)
	}
	result, carry = alu.Decrement(0)
	if result != 15 || carry {
		t.Errorf("expected (15, false), got (%d, %v)", result, carry)
	}
}

func TestRegisterPairs(t *testing.T) {
	rf := NewRegisterFile()
	rf.WritePair(0, 0xAB)
	if rf.Read(0) != 0xA {
		t.Errorf("expected R0=0xA, got %d", rf.Read(0))
	}
	if rf.Read(1) != 0xB {
		t.Errorf("expected R1=0xB, got %d", rf.Read(1))
	}
	if rf.ReadPair(0) != 0xAB {
		t.Errorf("expected pair 0=0xAB, got 0x%X", rf.ReadPair(0))
	}
}

func TestPCLoad(t *testing.T) {
	pc := NewProgramCounter()
	pc.Load(0x123)
	if pc.Read() != 0x123 {
		t.Errorf("expected 0x123, got 0x%X", pc.Read())
	}
}

func TestPCIncrement2(t *testing.T) {
	pc := NewProgramCounter()
	pc.Increment2()
	if pc.Read() != 2 {
		t.Errorf("expected 2, got %d", pc.Read())
	}
}

func TestStackWrap(t *testing.T) {
	stack := NewHardwareStack()
	stack.Push(0x100)
	stack.Push(0x200)
	stack.Push(0x300)
	// Push a 4th — silently wraps, overwriting slot 0
	stack.Push(0x400)
	// Pop 3 values — we should get 0x400, 0x300, 0x200
	if stack.Pop() != 0x400 {
		t.Error("expected 0x400")
	}
	if stack.Pop() != 0x300 {
		t.Error("expected 0x300")
	}
	if stack.Pop() != 0x200 {
		t.Error("expected 0x200")
	}
}

func TestRAMReadWrite(t *testing.T) {
	ram := NewRAM()
	ram.WriteMain(0, 0, 0, 7)
	if ram.ReadMain(0, 0, 0) != 7 {
		t.Errorf("expected 7, got %d", ram.ReadMain(0, 0, 0))
	}
	ram.WriteStatus(1, 2, 3, 12)
	if ram.ReadStatus(1, 2, 3) != 12 {
		t.Errorf("expected 12, got %d", ram.ReadStatus(1, 2, 3))
	}
}

func TestRAMOutput(t *testing.T) {
	ram := NewRAM()
	ram.WriteOutput(2, 9)
	if ram.ReadOutput(2) != 9 {
		t.Errorf("expected 9, got %d", ram.ReadOutput(2))
	}
}

func TestDecoderTwoByte(t *testing.T) {
	// JUN is two-byte
	d := Decode(0x40, -1)
	if d.IsTwoByte != 1 {
		t.Error("JUN should be two-byte")
	}
	if d.IsJUN != 1 {
		t.Error("expected IsJUN=1")
	}

	// FIM is two-byte
	d = Decode(0x20, -1)
	if d.IsTwoByte != 1 {
		t.Error("FIM should be two-byte")
	}

	// LD is one-byte
	d = Decode(0xA0, -1)
	if d.IsTwoByte != 0 {
		t.Error("LD should be one-byte")
	}
}

func TestAccumulatorReset(t *testing.T) {
	acc := NewAccumulator()
	acc.Write(7)
	if acc.Read() != 7 {
		t.Errorf("expected 7, got %d", acc.Read())
	}
	acc.Reset()
	if acc.Read() != 0 {
		t.Errorf("expected 0, got %d", acc.Read())
	}
}

func TestCarryFlagReset(t *testing.T) {
	cf := NewCarryFlag()
	cf.Write(true)
	if !cf.Read() {
		t.Error("expected carry=true")
	}
	cf.Reset()
	if cf.Read() {
		t.Error("expected carry=false after reset")
	}
}

func TestWMP(t *testing.T) {
	cpu := NewIntel4004GateLevel()
	cpu.Run([]byte{
		0x20, 0x00, 0x21, // SRC P0
		0xD7, 0xE1,       // LDM 7, WMP
		0x01,
	}, 10000)
	output := cpu.RAMOutput()
	if output[0] != 7 {
		t.Errorf("expected RAM output[0]=7, got %d", output[0])
	}
}

func TestSBM(t *testing.T) {
	cpu := NewIntel4004GateLevel()
	cpu.Run([]byte{
		// Store 3 in RAM[0][0][0]
		0x20, 0x00, 0x21, // SRC P0
		0xD3, 0xE0,       // LDM 3, WRM
		// Now A=5, subtract RAM value
		0xD5,             // LDM 5
		0x20, 0x00, 0x21, // SRC P0
		0xE8,             // SBM
		0x01,
	}, 10000)
	if cpu.Accumulator() != 2 {
		t.Errorf("expected accumulator=2, got %d", cpu.Accumulator())
	}
}

func TestADM(t *testing.T) {
	cpu := NewIntel4004GateLevel()
	cpu.Run([]byte{
		// Store 3 in RAM[0][0][0]
		0x20, 0x00, 0x21, // SRC P0
		0xD3, 0xE0,       // LDM 3, WRM
		// Now A=5, add RAM value
		0xD5,             // LDM 5
		0x20, 0x00, 0x21, // SRC P0
		0xEB,             // ADM
		0x01,
	}, 10000)
	if cpu.Accumulator() != 8 {
		t.Errorf("expected accumulator=8, got %d", cpu.Accumulator())
	}
}

func TestFIN(t *testing.T) {
	cpu := NewIntel4004GateLevel()
	// Load P0 with address 0x06, then FIN P1 reads ROM[0x06]
	cpu.Run([]byte{
		0x20, 0x06, // FIM P0,0x06
		0x32,       // FIN P1
		0x01,       // HLT
		0x00, 0x00, // padding
		0xCD,       // ROM[0x06] = 0xCD
	}, 10000)
	regs := cpu.Registers()
	if regs[2] != 0xC {
		t.Errorf("expected R2=0xC, got %d", regs[2])
	}
	if regs[3] != 0xD {
		t.Errorf("expected R3=0xD, got %d", regs[3])
	}
}

func TestJCNCarry(t *testing.T) {
	cpu := NewIntel4004GateLevel()
	// Set carry, then JCN carry test
	cpu.Run([]byte{
		0xFA,       // STC
		0x12, 0x06, // JCN 2, 0x06 (test carry=1 -> jump)
		0xD5, 0x01, // LDM 5, HLT (skipped)
		0x00,       // padding
		0x01,       // HLT (jumped here)
	}, 10000)
	if cpu.Accumulator() != 0 {
		t.Errorf("expected accumulator=0 (carry jump taken), got %d", cpu.Accumulator())
	}
}

func TestResetClears(t *testing.T) {
	cpu := NewIntel4004GateLevel()
	cpu.Run([]byte{0xD5, 0xB0, 0xFA, 0x01}, 10000)
	// acc should be 0 (was 5, then STC then XCH), carry=true
	cpu.Reset()
	if cpu.Accumulator() != 0 {
		t.Errorf("expected accumulator=0 after reset, got %d", cpu.Accumulator())
	}
	if cpu.Carry() {
		t.Error("expected carry=false after reset")
	}
	if cpu.PC() != 0 {
		t.Errorf("expected PC=0 after reset, got %d", cpu.PC())
	}
	if cpu.Halted() {
		t.Error("expected halted=false after reset")
	}
}

func TestHWStackAccessor(t *testing.T) {
	cpu := NewIntel4004GateLevel()
	cpu.Run([]byte{
		0x50, 0x04, // JMS 0x004
		0x01,       // HLT
		0x00,       // padding
		0xC0,       // BBL 0
	}, 10000)
	// After BBL, stack should still have the address in slot 0
	stack := cpu.HWStack()
	if len(stack) != 3 {
		t.Errorf("expected 3 stack levels, got %d", len(stack))
	}
}

func TestBitwiseAnd(t *testing.T) {
	alu := NewGateALU()
	if alu.BitwiseAnd(0xF, 0x5) != 0x5 {
		t.Errorf("expected 5, got %d", alu.BitwiseAnd(0xF, 0x5))
	}
}

func TestBitwiseOr(t *testing.T) {
	alu := NewGateALU()
	if alu.BitwiseOr(0x5, 0xA) != 0xF {
		t.Errorf("expected 15, got %d", alu.BitwiseOr(0x5, 0xA))
	}
}

func TestALUGateCount(t *testing.T) {
	alu := NewGateALU()
	if alu.GateCount() != 32 {
		t.Errorf("expected 32, got %d", alu.GateCount())
	}
}

func TestRegisterFileGateCount(t *testing.T) {
	rf := NewRegisterFile()
	if rf.GateCount() != 480 {
		t.Errorf("expected 480, got %d", rf.GateCount())
	}
}

func TestRAMGateCount(t *testing.T) {
	ram := NewRAM()
	if ram.GateCount() != 7880 {
		t.Errorf("expected 7880, got %d", ram.GateCount())
	}
}

func TestRegisterFileReset(t *testing.T) {
	rf := NewRegisterFile()
	for i := 0; i < 16; i++ {
		rf.Write(i, i)
	}
	rf.Reset()
	for i := 0; i < 16; i++ {
		if rf.Read(i) != 0 {
			t.Errorf("expected R%d=0 after reset, got %d", i, rf.Read(i))
		}
	}
}

func TestRAMReset(t *testing.T) {
	ram := NewRAM()
	ram.WriteMain(0, 0, 0, 7)
	ram.WriteStatus(1, 2, 3, 12)
	ram.WriteOutput(2, 9)
	ram.Reset()
	if ram.ReadMain(0, 0, 0) != 0 {
		t.Errorf("expected 0 after reset, got %d", ram.ReadMain(0, 0, 0))
	}
	if ram.ReadStatus(1, 2, 3) != 0 {
		t.Errorf("expected 0 after reset, got %d", ram.ReadStatus(1, 2, 3))
	}
	if ram.ReadOutput(2) != 0 {
		t.Errorf("expected 0 after reset, got %d", ram.ReadOutput(2))
	}
}
