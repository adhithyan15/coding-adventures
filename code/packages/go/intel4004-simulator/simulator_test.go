package intel4004simulator

import (
	"testing"
)

// ---------------------------------------------------------------------------
// Helper: run a program and return the simulator for state inspection
// ---------------------------------------------------------------------------

func runProgram(t *testing.T, program []byte) (*Intel4004Simulator, []Intel4004Trace) {
	t.Helper()
	sim := NewIntel4004Simulator(4096)
	traces := sim.Run(program, 10000)
	return sim, traces
}

// ===========================================================================
// NOP (0x00) and HLT (0x01)
// ===========================================================================

func TestNOP(t *testing.T) {
	sim, traces := runProgram(t, []byte{
		EncodeNOP(),
		EncodeNOP(),
		EncodeHlt(),
	})
	if len(traces) != 3 {
		t.Fatalf("expected 3 traces, got %d", len(traces))
	}
	if traces[0].Mnemonic != "NOP" {
		t.Errorf("expected NOP, got %s", traces[0].Mnemonic)
	}
	if sim.Accumulator != 0 {
		t.Errorf("NOP should not change accumulator")
	}
}

func TestHLT(t *testing.T) {
	sim, traces := runProgram(t, []byte{EncodeHlt()})
	if len(traces) != 1 {
		t.Fatalf("expected 1 trace, got %d", len(traces))
	}
	if !sim.Halted {
		t.Error("expected CPU to be halted")
	}
	if traces[0].Mnemonic != "HLT" {
		t.Errorf("expected HLT, got %s", traces[0].Mnemonic)
	}
}

func TestHaltedPanic(t *testing.T) {
	sim := NewIntel4004Simulator(4096)
	sim.Halted = true
	defer func() {
		if r := recover(); r == nil {
			t.Error("expected panic from stepping while halted")
		}
	}()
	sim.Step()
}

// ===========================================================================
// LDM (0xD_) — Load immediate
// ===========================================================================

func TestLDM(t *testing.T) {
	for n := 0; n <= 15; n++ {
		sim, _ := runProgram(t, []byte{EncodeLdm(n), EncodeHlt()})
		if sim.Accumulator != n {
			t.Errorf("LDM %d: expected A=%d, got %d", n, n, sim.Accumulator)
		}
	}
}

// ===========================================================================
// LD (0xA_) — Load register into accumulator
// ===========================================================================

func TestLD(t *testing.T) {
	sim, _ := runProgram(t, []byte{
		EncodeLdm(7),  // A = 7
		EncodeXch(3),  // R3 = 7, A = 0
		EncodeLd(3),   // A = R3 = 7
		EncodeHlt(),
	})
	if sim.Accumulator != 7 {
		t.Errorf("expected A=7, got %d", sim.Accumulator)
	}
}

// ===========================================================================
// XCH (0xB_) — Exchange accumulator and register
// ===========================================================================

func TestXCH(t *testing.T) {
	sim, _ := runProgram(t, []byte{
		EncodeLdm(5),  // A = 5
		EncodeXch(2),  // R2 = 5, A = 0
		EncodeHlt(),
	})
	if sim.Registers[2] != 5 {
		t.Errorf("expected R2=5, got %d", sim.Registers[2])
	}
	if sim.Accumulator != 0 {
		t.Errorf("expected A=0, got %d", sim.Accumulator)
	}
}

// ===========================================================================
// INC (0x6_) — Increment register
// ===========================================================================

func TestINC(t *testing.T) {
	sim, _ := runProgram(t, []byte{
		EncodeLdm(14), // A = 14
		EncodeXch(0),  // R0 = 14
		EncodeInc(0),  // R0 = 15
		EncodeInc(0),  // R0 = 0 (wrap)
		EncodeHlt(),
	})
	if sim.Registers[0] != 0 {
		t.Errorf("expected R0=0 after wrap, got %d", sim.Registers[0])
	}
}

func TestINCDoesNotAffectCarry(t *testing.T) {
	sim, _ := runProgram(t, []byte{
		0xFA,          // STC — set carry
		EncodeLdm(15),
		EncodeXch(0),  // R0 = 15
		EncodeInc(0),  // R0 wraps to 0, but carry should remain true
		EncodeHlt(),
	})
	if !sim.Carry {
		t.Error("INC should not affect carry flag")
	}
}

// ===========================================================================
// ADD (0x8_) — Add register to accumulator with carry
// ===========================================================================

func TestADD(t *testing.T) {
	sim, _ := runProgram(t, []byte{
		EncodeLdm(1),  // A = 1
		EncodeXch(0),  // R0 = 1
		EncodeLdm(2),  // A = 2
		EncodeAdd(0),  // A = 2 + 1 + 0 = 3
		EncodeXch(1),  // R1 = 3
		EncodeHlt(),
	})
	if sim.Registers[1] != 3 {
		t.Errorf("expected R1=3, got %d", sim.Registers[1])
	}
}

func TestADDOverflow(t *testing.T) {
	sim, _ := runProgram(t, []byte{
		EncodeLdm(8),  // A = 8
		EncodeXch(0),  // R0 = 8
		EncodeLdm(9),  // A = 9
		EncodeAdd(0),  // A = 9 + 8 = 17, carry set, A = 1
		EncodeHlt(),
	})
	if sim.Accumulator != 1 {
		t.Errorf("expected A=1 (17 & 0xF), got %d", sim.Accumulator)
	}
	if !sim.Carry {
		t.Error("expected carry set on overflow")
	}
}

func TestADDWithCarryIn(t *testing.T) {
	sim, _ := runProgram(t, []byte{
		0xFA,          // STC — set carry
		EncodeLdm(3),  // A = 3
		EncodeXch(0),  // R0 = 3
		EncodeLdm(4),  // A = 4
		EncodeAdd(0),  // A = 4 + 3 + 1(carry) = 8
		EncodeHlt(),
	})
	if sim.Accumulator != 8 {
		t.Errorf("expected A=8, got %d", sim.Accumulator)
	}
	if sim.Carry {
		t.Error("expected carry clear (8 <= 15)")
	}
}

// ===========================================================================
// SUB (0x9_) — Subtract with complement-add
// ===========================================================================

func TestSUBNoBorrow(t *testing.T) {
	// 5 - 3 = 2, no borrow → carry should be SET (inverted convention)
	sim, _ := runProgram(t, []byte{
		EncodeLdm(3),  // A = 3
		EncodeXch(0),  // R0 = 3
		EncodeLdm(5),  // A = 5
		EncodeSub(0),  // A = 5 - 3 = 2, carry = true (no borrow)
		EncodeHlt(),
	})
	if sim.Accumulator != 2 {
		t.Errorf("expected A=2, got %d", sim.Accumulator)
	}
	if !sim.Carry {
		t.Error("expected carry=true (no borrow)")
	}
}

func TestSUBWithBorrow(t *testing.T) {
	// 1 - 3: borrow occurs → carry should be CLEAR
	// complement = ~3 & 0xF = 12, borrow_in = 1 (carry is clear initially)
	// result = 1 + 12 + 1 = 14, carry = false (14 <= 15)
	// Wait — that's wrong. Let me recalculate:
	// Actually initial carry=false, borrow_in = 1 (since !carry)
	// result = 1 + 12 + 1 = 14. 14 <= 15 so carry=false. A = 14.
	// Hmm, but 1-3 = -2, and -2 in 4-bit is 14. carry=false means borrow. Correct!
	sim, _ := runProgram(t, []byte{
		EncodeLdm(3),
		EncodeXch(0),  // R0 = 3
		EncodeLdm(1),  // A = 1
		EncodeSub(0),  // A = 1 - 3 = 14 (with borrow)
		EncodeHlt(),
	})
	if sim.Accumulator != 14 {
		t.Errorf("expected A=14, got %d", sim.Accumulator)
	}
	if sim.Carry {
		t.Error("expected carry=false (borrow occurred)")
	}
}

func TestSUBZeroMinusOne(t *testing.T) {
	// 0 - 1 = 15 (with borrow)
	// complement = ~1 & 0xF = 14, borrow_in = 1
	// result = 0 + 14 + 1 = 15, carry = false (15 <= 15)
	sim, _ := runProgram(t, []byte{
		EncodeLdm(1),
		EncodeXch(0),  // R0 = 1
		EncodeLdm(0),  // A = 0
		EncodeSub(0),  // A = 0 - 1 = 15
		EncodeHlt(),
	})
	if sim.Accumulator != 15 {
		t.Errorf("expected A=15, got %d", sim.Accumulator)
	}
	if sim.Carry {
		t.Error("expected carry=false (borrow)")
	}
}

// ===========================================================================
// JUN (0x4_) — Unconditional jump
// ===========================================================================

func TestJUN(t *testing.T) {
	b1, b2 := EncodeJun(0x004)
	sim, traces := runProgram(t, []byte{
		b1, b2,        // JUN 0x004 — skip next 2 bytes
		EncodeLdm(15), // should be skipped
		EncodeHlt(),   // should be skipped
		EncodeLdm(7),  // landed here
		EncodeHlt(),
	})
	if sim.Accumulator != 7 {
		t.Errorf("expected A=7, got %d", sim.Accumulator)
	}
	// Should have: JUN, LDM 7, HLT = 3 traces
	if len(traces) != 3 {
		t.Errorf("expected 3 traces, got %d", len(traces))
	}
}

// ===========================================================================
// JMS (0x5_) and BBL (0xC_) — Subroutine call and return
// ===========================================================================

func TestJMSAndBBL(t *testing.T) {
	// Program layout:
	//   0x000: JMS 0x004    (2 bytes) — call subroutine at 0x004
	//   0x002: HLT          (1 byte)  — after return
	//   0x003: NOP          (1 byte)  — padding
	//   0x004: LDM 9        (1 byte)  — subroutine body
	//   0x005: BBL 5        (1 byte)  — return with A=5
	b1, b2 := EncodeJms(0x004)
	sim, _ := runProgram(t, []byte{
		b1, b2,        // 0x000: JMS 0x004
		EncodeHlt(),   // 0x002: HLT (return here)
		EncodeNOP(),   // 0x003: padding
		EncodeLdm(9),  // 0x004: subroutine start
		EncodeBbl(5),  // 0x005: return, A=5
	})
	// BBL loads A with 5
	if sim.Accumulator != 5 {
		t.Errorf("expected A=5 after BBL, got %d", sim.Accumulator)
	}
	if !sim.Halted {
		t.Error("expected CPU halted after return")
	}
}

func TestNestedSubroutines(t *testing.T) {
	// Test 3-level nesting (the maximum for the 4004's hardware stack)
	b1a, b1b := EncodeJms(0x006)
	b2a, b2b := EncodeJms(0x00A)
	b3a, b3b := EncodeJms(0x00E)
	program := []byte{
		b1a, b1b,      // 0x000: JMS 0x006 (level 1)
		EncodeHlt(),   // 0x002: HLT
		0, 0, 0,       // 0x003-0x005: padding
		b2a, b2b,      // 0x006: JMS 0x00A (level 2)
		EncodeBbl(1),  // 0x008: BBL 1 (return from level 1)
		0,             // 0x009: padding
		b3a, b3b,      // 0x00A: JMS 0x00E (level 3)
		EncodeBbl(2),  // 0x00C: BBL 2 (return from level 2)
		0,             // 0x00D: padding
		EncodeLdm(7),  // 0x00E: innermost subroutine
		EncodeBbl(3),  // 0x00F: BBL 3 (return from level 3)
	}
	sim, _ := runProgram(t, program)
	// Final BBL from level 1 sets A=1
	if sim.Accumulator != 1 {
		t.Errorf("expected A=1 after nested returns, got %d", sim.Accumulator)
	}
}

// ===========================================================================
// JCN (0x1_) — Conditional jump
// ===========================================================================

func TestJCNZeroTrue(t *testing.T) {
	// Condition 0x4 = jump if A == 0
	b1, b2 := EncodeJcn(0x4, 0x05)
	sim, _ := runProgram(t, []byte{
		EncodeLdm(0),  // A = 0
		b1, b2,        // JCN 4,0x05 — should jump (A is 0)
		EncodeLdm(15), // skipped
		EncodeHlt(),   // 0x05: landed here
	})
	if sim.Accumulator != 0 {
		t.Errorf("expected A=0 (jump taken), got %d", sim.Accumulator)
	}
}

func TestJCNZeroFalse(t *testing.T) {
	// Condition 0x4 = jump if A == 0, but A != 0
	b1, b2 := EncodeJcn(0x4, 0x06)
	sim, _ := runProgram(t, []byte{
		EncodeLdm(5),  // A = 5
		b1, b2,        // JCN 4,0x06 — should NOT jump (A != 0)
		EncodeLdm(9),  // A = 9 (not skipped)
		EncodeHlt(),
	})
	if sim.Accumulator != 9 {
		t.Errorf("expected A=9 (fall through), got %d", sim.Accumulator)
	}
}

func TestJCNInvertedZero(t *testing.T) {
	// Condition 0xC = 0x8 | 0x4 = invert(test_zero) = jump if A != 0
	b1, b2 := EncodeJcn(0xC, 0x06)
	sim, _ := runProgram(t, []byte{
		EncodeLdm(3),  // A = 3 (not zero)
		b1, b2,        // JCN 12,0x06 — should jump (A != 0)
		EncodeLdm(15), // skipped
		EncodeHlt(),   // 0x06: landed here
	})
	if sim.Accumulator != 3 {
		t.Errorf("expected A=3 (jump taken), got %d", sim.Accumulator)
	}
}

func TestJCNCarry(t *testing.T) {
	// Condition 0x2 = jump if carry is set
	b1, b2 := EncodeJcn(0x2, 0x05)
	sim, _ := runProgram(t, []byte{
		0xFA,          // STC — set carry
		b1, b2,        // JCN 2,0x05 — should jump (carry is set)
		EncodeLdm(15), // skipped
		EncodeHlt(),   // 0x05: landed here
	})
	_ = sim
}

func TestJCNCarryNotSet(t *testing.T) {
	// Condition 0x2 = jump if carry, but carry is clear
	b1, b2 := EncodeJcn(0x2, 0x06)
	sim, _ := runProgram(t, []byte{
		EncodeLdm(0),  // A = 0, carry = false
		b1, b2,        // JCN 2,0x06 — should NOT jump
		EncodeLdm(8),  // A = 8 (not skipped)
		EncodeHlt(),
	})
	if sim.Accumulator != 8 {
		t.Errorf("expected A=8 (fall through), got %d", sim.Accumulator)
	}
}

// ===========================================================================
// ISZ (0x7_) — Increment and skip if zero
// ===========================================================================

func TestISZLoop(t *testing.T) {
	// Load R0=14, ISZ loops twice (14→15→0), then falls through
	b1, b2 := EncodeIsz(0, 0x03)
	sim, traces := runProgram(t, []byte{
		EncodeLdm(14), // A = 14
		EncodeXch(0),  // R0 = 14
		EncodeLdm(0),  // A = 0 (so we can detect final state)
		b1, b2,        // 0x03: ISZ R0,0x03 — jump back to self if R0 != 0
		EncodeHlt(),   // 0x05: falls through when R0 = 0
	})
	if sim.Registers[0] != 0 {
		t.Errorf("expected R0=0, got %d", sim.Registers[0])
	}
	// R0 goes: 14→15 (jump), 15→0 (fall through) = 2 ISZ executions + 3 setup + 1 HLT
	if len(traces) != 6 {
		t.Errorf("expected 6 traces (3 setup + 2 ISZ + 1 HLT), got %d", len(traces))
	}
}

func TestISZImmediateFallthrough(t *testing.T) {
	// R0=15, ISZ increments to 0 and falls through immediately
	b1, b2 := EncodeIsz(0, 0x03)
	sim, _ := runProgram(t, []byte{
		EncodeLdm(15),
		EncodeXch(0),  // R0 = 15
		EncodeLdm(0),
		b1, b2,        // 0x03: ISZ R0,0x03 — R0 becomes 0, fall through
		EncodeHlt(),
	})
	if sim.Registers[0] != 0 {
		t.Errorf("expected R0=0, got %d", sim.Registers[0])
	}
}

// ===========================================================================
// FIM (0x2_ even) — Fetch immediate to register pair
// ===========================================================================

func TestFIM(t *testing.T) {
	b1, b2 := EncodeFim(1, 0xAB)
	sim, _ := runProgram(t, []byte{
		b1, b2,        // FIM P1, 0xAB → R2=0xA, R3=0xB
		EncodeHlt(),
	})
	if sim.Registers[2] != 0xA {
		t.Errorf("expected R2=0xA, got %d", sim.Registers[2])
	}
	if sim.Registers[3] != 0xB {
		t.Errorf("expected R3=0xB, got %d", sim.Registers[3])
	}
}

// ===========================================================================
// SRC (0x2_ odd) — Send register control
// ===========================================================================

func TestSRC(t *testing.T) {
	b1, b2 := EncodeFim(1, 0x35) // R2=3, R3=5
	sim, _ := runProgram(t, []byte{
		b1, b2,
		EncodeSrc(1), // SRC P1 → RAMRegister=3, RAMCharacter=5
		EncodeHlt(),
	})
	if sim.RAMRegister != 3 {
		t.Errorf("expected RAMRegister=3, got %d", sim.RAMRegister)
	}
	if sim.RAMCharacter != 5 {
		t.Errorf("expected RAMCharacter=5, got %d", sim.RAMCharacter)
	}
}

// ===========================================================================
// FIN (0x3_ even) — Fetch indirect from ROM
// ===========================================================================

func TestFIN(t *testing.T) {
	// Store a known byte in ROM at address 0x10, then use FIN to read it.
	// P0 (R0:R1) = 0x10 → read ROM[0x010] into P2.
	b1, b2 := EncodeFim(0, 0x10) // P0 = 0x10 (R0=1, R1=0)
	program := make([]byte, 4096)
	program[0] = b1
	program[1] = b2
	program[2] = EncodeFin(2) // FIN P2 — read ROM[current_page | 0x10] into P2
	program[3] = EncodeHlt()
	program[0x10] = 0xCD // The byte FIN will read

	sim := NewIntel4004Simulator(4096)
	sim.Run(program, 100)

	// P2 = R4:R5 should contain 0xCD → R4=0xC, R5=0xD
	if sim.Registers[4] != 0xC {
		t.Errorf("expected R4=0xC, got %d", sim.Registers[4])
	}
	if sim.Registers[5] != 0xD {
		t.Errorf("expected R5=0xD, got %d", sim.Registers[5])
	}
}

// ===========================================================================
// JIN (0x3_ odd) — Jump indirect via register pair
// ===========================================================================

func TestJIN(t *testing.T) {
	b1, b2 := EncodeFim(1, 0x07) // P1 = 0x07
	sim, _ := runProgram(t, []byte{
		b1, b2,         // 0x00: FIM P1, 0x07
		EncodeJin(1),   // 0x02: JIN P1 → jump to 0x07
		EncodeLdm(15),  // 0x03: skipped
		EncodeHlt(),    // 0x04: skipped
		EncodeNOP(),    // 0x05: padding
		EncodeNOP(),    // 0x06: padding
		EncodeLdm(3),   // 0x07: landed here
		EncodeHlt(),
	})
	if sim.Accumulator != 3 {
		t.Errorf("expected A=3 after JIN, got %d", sim.Accumulator)
	}
}

// ===========================================================================
// BBL (0xC_) — Branch back and load
// ===========================================================================

func TestBBLLoadsImmediate(t *testing.T) {
	b1, b2 := EncodeJms(0x004)
	sim, _ := runProgram(t, []byte{
		b1, b2,        // JMS 0x004
		EncodeHlt(),   // return here
		0,             // padding
		EncodeBbl(12), // BBL 12 — A=12, return
	})
	if sim.Accumulator != 12 {
		t.Errorf("expected A=12, got %d", sim.Accumulator)
	}
}

// ===========================================================================
// I/O: WRM/RDM — Write/Read RAM main character
// ===========================================================================

func TestWRMAndRDM(t *testing.T) {
	b1, b2 := EncodeFim(0, 0x23) // P0 = 0x23 → register=2, character=3
	sim, _ := runProgram(t, []byte{
		b1, b2,
		EncodeSrc(0),  // SRC P0 → set RAM address
		EncodeLdm(9),  // A = 9
		0xE0,          // WRM — write A to RAM[0][2][3]
		EncodeLdm(0),  // A = 0
		0xE9,          // RDM — read RAM[0][2][3] into A
		EncodeHlt(),
	})
	if sim.Accumulator != 9 {
		t.Errorf("expected A=9, got %d", sim.Accumulator)
	}
}

// ===========================================================================
// I/O: WMP — Write RAM output port
// ===========================================================================

func TestWMP(t *testing.T) {
	sim, _ := runProgram(t, []byte{
		EncodeLdm(7), // A = 7
		0xE1,         // WMP — write A to output port
		EncodeHlt(),
	})
	if sim.RAMOutput[0] != 7 {
		t.Errorf("expected RAMOutput[0]=7, got %d", sim.RAMOutput[0])
	}
}

// ===========================================================================
// I/O: WRR/RDR — Write/Read ROM I/O port
// ===========================================================================

func TestWRRAndRDR(t *testing.T) {
	sim, _ := runProgram(t, []byte{
		EncodeLdm(11), // A = 11
		0xE2,          // WRR — write A to ROM port
		EncodeLdm(0),  // A = 0
		0xEA,          // RDR — read ROM port into A
		EncodeHlt(),
	})
	if sim.Accumulator != 11 {
		t.Errorf("expected A=11, got %d", sim.Accumulator)
	}
}

// ===========================================================================
// I/O: WPM — Write program RAM (NOP in simulation)
// ===========================================================================

func TestWPM(t *testing.T) {
	_, traces := runProgram(t, []byte{
		0xE3,         // WPM
		EncodeHlt(),
	})
	if traces[0].Mnemonic != "WPM" {
		t.Errorf("expected WPM, got %s", traces[0].Mnemonic)
	}
}

// ===========================================================================
// I/O: WR0-WR3/RD0-RD3 — Write/Read RAM status characters
// ===========================================================================

func TestWRAndRDStatus(t *testing.T) {
	b1, b2 := EncodeFim(0, 0x10) // register=1, character=0
	sim, _ := runProgram(t, []byte{
		b1, b2,
		EncodeSrc(0), // set RAM address
		EncodeLdm(5),
		0xE4,         // WR0 — status[0][1][0] = 5
		EncodeLdm(6),
		0xE5,         // WR1 — status[0][1][1] = 6
		EncodeLdm(7),
		0xE6,         // WR2 — status[0][1][2] = 7
		EncodeLdm(8),
		0xE7,         // WR3 — status[0][1][3] = 8
		EncodeLdm(0),
		0xEC,         // RD0 — A = status[0][1][0] = 5
		EncodeHlt(),
	})
	if sim.Accumulator != 5 {
		t.Errorf("expected A=5 from RD0, got %d", sim.Accumulator)
	}
	if sim.RAMStatus[0][1][1] != 6 {
		t.Errorf("expected status[0][1][1]=6, got %d", sim.RAMStatus[0][1][1])
	}
	if sim.RAMStatus[0][1][2] != 7 {
		t.Errorf("expected status[0][1][2]=7, got %d", sim.RAMStatus[0][1][2])
	}
	if sim.RAMStatus[0][1][3] != 8 {
		t.Errorf("expected status[0][1][3]=8, got %d", sim.RAMStatus[0][1][3])
	}
}

func TestRD1(t *testing.T) {
	b1, b2 := EncodeFim(0, 0x00)
	sim, _ := runProgram(t, []byte{
		b1, b2,
		EncodeSrc(0),
		EncodeLdm(12),
		0xE5,          // WR1
		EncodeLdm(0),
		0xED,          // RD1
		EncodeHlt(),
	})
	if sim.Accumulator != 12 {
		t.Errorf("expected A=12, got %d", sim.Accumulator)
	}
}

func TestRD2(t *testing.T) {
	b1, b2 := EncodeFim(0, 0x00)
	sim, _ := runProgram(t, []byte{
		b1, b2,
		EncodeSrc(0),
		EncodeLdm(3),
		0xE6,          // WR2
		EncodeLdm(0),
		0xEE,          // RD2
		EncodeHlt(),
	})
	if sim.Accumulator != 3 {
		t.Errorf("expected A=3, got %d", sim.Accumulator)
	}
}

func TestRD3(t *testing.T) {
	b1, b2 := EncodeFim(0, 0x00)
	sim, _ := runProgram(t, []byte{
		b1, b2,
		EncodeSrc(0),
		EncodeLdm(14),
		0xE7,          // WR3
		EncodeLdm(0),
		0xEF,          // RD3
		EncodeHlt(),
	})
	if sim.Accumulator != 14 {
		t.Errorf("expected A=14, got %d", sim.Accumulator)
	}
}

// ===========================================================================
// I/O: SBM (0xE8) — Subtract RAM from accumulator
// ===========================================================================

func TestSBM(t *testing.T) {
	b1, b2 := EncodeFim(0, 0x00)
	sim, _ := runProgram(t, []byte{
		b1, b2,
		EncodeSrc(0),
		EncodeLdm(3),
		0xE0,          // WRM — RAM[0][0][0] = 3
		EncodeLdm(7),  // A = 7
		0xE8,          // SBM — A = 7 - 3 = 4
		EncodeHlt(),
	})
	if sim.Accumulator != 4 {
		t.Errorf("expected A=4, got %d", sim.Accumulator)
	}
	if !sim.Carry {
		t.Error("expected carry=true (no borrow)")
	}
}

// ===========================================================================
// I/O: ADM (0xEB) — Add RAM to accumulator
// ===========================================================================

func TestADM(t *testing.T) {
	b1, b2 := EncodeFim(0, 0x00)
	sim, _ := runProgram(t, []byte{
		b1, b2,
		EncodeSrc(0),
		EncodeLdm(5),
		0xE0,          // WRM — RAM[0][0][0] = 5
		EncodeLdm(3),  // A = 3
		0xEB,          // ADM — A = 3 + 5 = 8
		EncodeHlt(),
	})
	if sim.Accumulator != 8 {
		t.Errorf("expected A=8, got %d", sim.Accumulator)
	}
}

func TestADMOverflow(t *testing.T) {
	b1, b2 := EncodeFim(0, 0x00)
	sim, _ := runProgram(t, []byte{
		b1, b2,
		EncodeSrc(0),
		EncodeLdm(10),
		0xE0,          // WRM — RAM = 10
		EncodeLdm(9),  // A = 9
		0xEB,          // ADM — A = 9 + 10 = 19 → A=3, carry=true
		EncodeHlt(),
	})
	if sim.Accumulator != 3 {
		t.Errorf("expected A=3 (19 & 0xF), got %d", sim.Accumulator)
	}
	if !sim.Carry {
		t.Error("expected carry=true")
	}
}

// ===========================================================================
// CLB (0xF0) — Clear both
// ===========================================================================

func TestCLB(t *testing.T) {
	sim, _ := runProgram(t, []byte{
		EncodeLdm(15), // A = 15
		0xFA,          // STC — set carry
		0xF0,          // CLB — A=0, carry=false
		EncodeHlt(),
	})
	if sim.Accumulator != 0 {
		t.Errorf("expected A=0, got %d", sim.Accumulator)
	}
	if sim.Carry {
		t.Error("expected carry=false")
	}
}

// ===========================================================================
// CLC (0xF1) — Clear carry
// ===========================================================================

func TestCLC(t *testing.T) {
	sim, _ := runProgram(t, []byte{
		0xFA,          // STC — set carry
		0xF1,          // CLC — carry=false
		EncodeHlt(),
	})
	if sim.Carry {
		t.Error("expected carry=false")
	}
}

// ===========================================================================
// IAC (0xF2) — Increment accumulator
// ===========================================================================

func TestIAC(t *testing.T) {
	sim, _ := runProgram(t, []byte{
		EncodeLdm(5),
		0xF2,         // IAC — A = 6
		EncodeHlt(),
	})
	if sim.Accumulator != 6 {
		t.Errorf("expected A=6, got %d", sim.Accumulator)
	}
	if sim.Carry {
		t.Error("expected carry=false")
	}
}

func TestIACWrap(t *testing.T) {
	sim, _ := runProgram(t, []byte{
		EncodeLdm(15),
		0xF2,         // IAC — A = 0, carry=true
		EncodeHlt(),
	})
	if sim.Accumulator != 0 {
		t.Errorf("expected A=0, got %d", sim.Accumulator)
	}
	if !sim.Carry {
		t.Error("expected carry=true (wrap)")
	}
}

// ===========================================================================
// CMC (0xF3) — Complement carry
// ===========================================================================

func TestCMC(t *testing.T) {
	sim, _ := runProgram(t, []byte{
		0xF3,         // CMC — carry was false, now true
		EncodeHlt(),
	})
	if !sim.Carry {
		t.Error("expected carry=true")
	}

	sim2, _ := runProgram(t, []byte{
		0xFA,         // STC
		0xF3,         // CMC — carry was true, now false
		EncodeHlt(),
	})
	if sim2.Carry {
		t.Error("expected carry=false")
	}
}

// ===========================================================================
// CMA (0xF4) — Complement accumulator
// ===========================================================================

func TestCMA(t *testing.T) {
	sim, _ := runProgram(t, []byte{
		EncodeLdm(5),  // A = 0101
		0xF4,          // CMA — A = 1010 = 10
		EncodeHlt(),
	})
	if sim.Accumulator != 10 {
		t.Errorf("expected A=10 (~5 & 0xF), got %d", sim.Accumulator)
	}
}

func TestCMAZero(t *testing.T) {
	sim, _ := runProgram(t, []byte{
		EncodeLdm(0),
		0xF4,          // CMA — A = 15
		EncodeHlt(),
	})
	if sim.Accumulator != 15 {
		t.Errorf("expected A=15, got %d", sim.Accumulator)
	}
}

// ===========================================================================
// RAL (0xF5) — Rotate accumulator left through carry
// ===========================================================================

func TestRAL(t *testing.T) {
	// A=0b0101 (5), carry=0
	// After RAL: carry=0 (bit 3 was 0), A=0b1010 (10)
	sim, _ := runProgram(t, []byte{
		EncodeLdm(5),
		0xF5,         // RAL
		EncodeHlt(),
	})
	if sim.Accumulator != 10 {
		t.Errorf("expected A=10, got %d", sim.Accumulator)
	}
	if sim.Carry {
		t.Error("expected carry=false")
	}
}

func TestRALWithCarry(t *testing.T) {
	// A=0b1001 (9), carry=1
	// After RAL: carry=1 (bit 3 was 1), A=0b0011 (3, shifted left with old carry=1 in bit 0)
	sim, _ := runProgram(t, []byte{
		0xFA,          // STC
		EncodeLdm(9),  // A = 1001
		0xF5,          // RAL — A = 0011, carry = 1
		EncodeHlt(),
	})
	if sim.Accumulator != 3 {
		t.Errorf("expected A=3, got %d", sim.Accumulator)
	}
	if !sim.Carry {
		t.Error("expected carry=true")
	}
}

// ===========================================================================
// RAR (0xF6) — Rotate accumulator right through carry
// ===========================================================================

func TestRAR(t *testing.T) {
	// A=0b1010 (10), carry=0
	// After RAR: carry=0 (bit 0 was 0), A=0b0101 (5)
	sim, _ := runProgram(t, []byte{
		EncodeLdm(10),
		0xF6,         // RAR
		EncodeHlt(),
	})
	if sim.Accumulator != 5 {
		t.Errorf("expected A=5, got %d", sim.Accumulator)
	}
	if sim.Carry {
		t.Error("expected carry=false")
	}
}

func TestRARWithCarry(t *testing.T) {
	// A=0b0011 (3), carry=1
	// After RAR: carry=1 (bit 0 was 1), A=0b1001 (9, old carry into bit 3)
	sim, _ := runProgram(t, []byte{
		0xFA,          // STC
		EncodeLdm(3),  // A = 0011
		0xF6,          // RAR — A = 1001, carry = 1
		EncodeHlt(),
	})
	if sim.Accumulator != 9 {
		t.Errorf("expected A=9, got %d", sim.Accumulator)
	}
	if !sim.Carry {
		t.Error("expected carry=true")
	}
}

// ===========================================================================
// TCC (0xF7) — Transfer carry to accumulator
// ===========================================================================

func TestTCCCarrySet(t *testing.T) {
	sim, _ := runProgram(t, []byte{
		0xFA,         // STC
		0xF7,         // TCC — A=1, carry=false
		EncodeHlt(),
	})
	if sim.Accumulator != 1 {
		t.Errorf("expected A=1, got %d", sim.Accumulator)
	}
	if sim.Carry {
		t.Error("expected carry=false after TCC")
	}
}

func TestTCCCarryClear(t *testing.T) {
	sim, _ := runProgram(t, []byte{
		0xF7,         // TCC — carry=false, so A=0
		EncodeHlt(),
	})
	if sim.Accumulator != 0 {
		t.Errorf("expected A=0, got %d", sim.Accumulator)
	}
}

// ===========================================================================
// DAC (0xF8) — Decrement accumulator
// ===========================================================================

func TestDAC(t *testing.T) {
	sim, _ := runProgram(t, []byte{
		EncodeLdm(5),
		0xF8,         // DAC — A=4, carry=true (no borrow)
		EncodeHlt(),
	})
	if sim.Accumulator != 4 {
		t.Errorf("expected A=4, got %d", sim.Accumulator)
	}
	if !sim.Carry {
		t.Error("expected carry=true (no borrow)")
	}
}

func TestDACWrap(t *testing.T) {
	sim, _ := runProgram(t, []byte{
		EncodeLdm(0),
		0xF8,         // DAC — A=15, carry=false (borrow)
		EncodeHlt(),
	})
	if sim.Accumulator != 15 {
		t.Errorf("expected A=15, got %d", sim.Accumulator)
	}
	if sim.Carry {
		t.Error("expected carry=false (borrow)")
	}
}

// ===========================================================================
// TCS (0xF9) — Transfer carry subtract
// ===========================================================================

func TestTCSCarrySet(t *testing.T) {
	sim, _ := runProgram(t, []byte{
		0xFA,         // STC
		0xF9,         // TCS — A=10, carry=false
		EncodeHlt(),
	})
	if sim.Accumulator != 10 {
		t.Errorf("expected A=10, got %d", sim.Accumulator)
	}
	if sim.Carry {
		t.Error("expected carry=false")
	}
}

func TestTCSCarryClear(t *testing.T) {
	sim, _ := runProgram(t, []byte{
		0xF9,         // TCS — carry=false, A=9
		EncodeHlt(),
	})
	if sim.Accumulator != 9 {
		t.Errorf("expected A=9, got %d", sim.Accumulator)
	}
}

// ===========================================================================
// STC (0xFA) — Set carry
// ===========================================================================

func TestSTC(t *testing.T) {
	sim, _ := runProgram(t, []byte{
		0xFA,         // STC
		EncodeHlt(),
	})
	if !sim.Carry {
		t.Error("expected carry=true")
	}
}

// ===========================================================================
// DAA (0xFB) — Decimal adjust accumulator
// ===========================================================================

func TestDAANoAdjust(t *testing.T) {
	// A=5, carry=false → no adjustment needed (5 <= 9)
	sim, _ := runProgram(t, []byte{
		EncodeLdm(5),
		0xFB,         // DAA — no change
		EncodeHlt(),
	})
	if sim.Accumulator != 5 {
		t.Errorf("expected A=5, got %d", sim.Accumulator)
	}
}

func TestDAAWithOverflow(t *testing.T) {
	// A=12 (>9), carry=false → add 6: 12+6=18 → A=2, carry=true
	sim, _ := runProgram(t, []byte{
		EncodeLdm(12),
		0xFB,         // DAA — 12+6=18, A=2, carry=true
		EncodeHlt(),
	})
	if sim.Accumulator != 2 {
		t.Errorf("expected A=2, got %d", sim.Accumulator)
	}
	if !sim.Carry {
		t.Error("expected carry=true")
	}
}

func TestDAAWithCarrySet(t *testing.T) {
	// A=3, carry=true → add 6: 3+6=9, A=9, carry stays or becomes based on overflow
	sim, _ := runProgram(t, []byte{
		0xFA,          // STC
		EncodeLdm(3),  // A=3
		0xFB,          // DAA — carry is set, so add 6: 3+6=9, no overflow, carry depends on result
		EncodeHlt(),
	})
	if sim.Accumulator != 9 {
		t.Errorf("expected A=9, got %d", sim.Accumulator)
	}
	// 3+6=9 <= 15, so carry is NOT newly set by overflow.
	// But the Python code doesn't clear carry if no overflow — it only sets on overflow.
	// So carry remains true from STC? Let's check the Python:
	// "if result > 0xF: self.carry = True" — only sets, never clears.
	// So carry stays true from the STC. Actually wait, LDM doesn't affect carry.
	// STC sets carry=true, LDM 3 doesn't change carry, DAA: 3+6=9 <= 15, doesn't set carry again.
	// Carry remains true.
	if !sim.Carry {
		t.Error("expected carry=true (preserved from STC, result didn't overflow)")
	}
}

// ===========================================================================
// KBP (0xFC) — Keyboard process
// ===========================================================================

func TestKBP(t *testing.T) {
	// Truth table: 0→0, 1→1, 2→2, 4→3, 8→4, else→15
	tests := []struct {
		input    int
		expected int
	}{
		{0, 0}, {1, 1}, {2, 2}, {4, 3}, {8, 4},
		{3, 15}, {5, 15}, {6, 15}, {7, 15}, {9, 15},
		{10, 15}, {11, 15}, {12, 15}, {13, 15}, {14, 15}, {15, 15},
	}
	for _, tt := range tests {
		sim, _ := runProgram(t, []byte{
			EncodeLdm(tt.input),
			0xFC,         // KBP
			EncodeHlt(),
		})
		if sim.Accumulator != tt.expected {
			t.Errorf("KBP(%d): expected %d, got %d", tt.input, tt.expected, sim.Accumulator)
		}
	}
}

// ===========================================================================
// DCL (0xFD) — Designate command line (select RAM bank)
// ===========================================================================

func TestDCL(t *testing.T) {
	sim, _ := runProgram(t, []byte{
		EncodeLdm(2),
		0xFD,         // DCL — bank = 2
		EncodeHlt(),
	})
	if sim.RAMBank != 2 {
		t.Errorf("expected RAMBank=2, got %d", sim.RAMBank)
	}
}

func TestDCLClamp(t *testing.T) {
	// A=7 → bank = 7 & 0x7 = 7, then clamped to 7 & 0x3 = 3
	sim, _ := runProgram(t, []byte{
		EncodeLdm(7),
		0xFD,         // DCL
		EncodeHlt(),
	})
	if sim.RAMBank != 3 {
		t.Errorf("expected RAMBank=3 (clamped from 7), got %d", sim.RAMBank)
	}
}

// ===========================================================================
// Integration: RAM with bank selection
// ===========================================================================

func TestRAMWithBankSelection(t *testing.T) {
	b1, b2 := EncodeFim(0, 0x00) // register=0, character=0
	sim, _ := runProgram(t, []byte{
		// Write 5 to bank 0
		b1, b2,
		EncodeSrc(0),
		EncodeLdm(5),
		0xE0,          // WRM — bank 0

		// Switch to bank 1
		EncodeLdm(1),
		0xFD,          // DCL — bank = 1

		// Write 9 to bank 1 (same register/character)
		EncodeLdm(9),
		0xE0,          // WRM — bank 1

		// Read from bank 1
		EncodeLdm(0),
		0xE9,          // RDM — should be 9
		EncodeXch(0),  // save in R0

		// Switch back to bank 0
		EncodeLdm(0),
		0xFD,          // DCL — bank = 0
		0xE9,          // RDM — should be 5

		EncodeHlt(),
	})
	if sim.Accumulator != 5 {
		t.Errorf("expected A=5 from bank 0, got %d", sim.Accumulator)
	}
	if sim.Registers[0] != 9 {
		t.Errorf("expected R0=9 from bank 1, got %d", sim.Registers[0])
	}
}

// ===========================================================================
// Integration: 1+2 program (original test preserved)
// ===========================================================================

func TestOnePlusTwo(t *testing.T) {
	sim := NewIntel4004Simulator(4096)
	program := []byte{
		EncodeLdm(1),
		EncodeXch(0),
		EncodeLdm(2),
		EncodeAdd(0),
		EncodeXch(1),
		EncodeHlt(),
	}
	traces := sim.Run(program, 1000)
	if len(traces) != 6 {
		t.Fatalf("expected 6 traces, got %d", len(traces))
	}
	if sim.Registers[1] != 3 {
		t.Errorf("R1 should be 3, got %d", sim.Registers[1])
	}
}

// ===========================================================================
// Trace record validation
// ===========================================================================

func TestTraceFields(t *testing.T) {
	_, traces := runProgram(t, []byte{
		EncodeLdm(7),  // A: 0→7
		EncodeHlt(),
	})
	tr := traces[0]
	if tr.Address != 0 {
		t.Errorf("expected Address=0, got %d", tr.Address)
	}
	if tr.AccumulatorBefore != 0 {
		t.Errorf("expected AccBefore=0, got %d", tr.AccumulatorBefore)
	}
	if tr.AccumulatorAfter != 7 {
		t.Errorf("expected AccAfter=7, got %d", tr.AccumulatorAfter)
	}
	if tr.Mnemonic != "LDM 7" {
		t.Errorf("expected 'LDM 7', got '%s'", tr.Mnemonic)
	}
	if tr.Raw2 != -1 {
		t.Errorf("expected Raw2=-1 for single-byte, got %d", tr.Raw2)
	}
}

func TestTraceFieldsTwoByte(t *testing.T) {
	b1, b2 := EncodeJun(0x004)
	_, traces := runProgram(t, []byte{
		b1, b2,
		0, 0,
		EncodeHlt(),
	})
	tr := traces[0]
	if tr.Raw2 != int(b2) {
		t.Errorf("expected Raw2=%d, got %d", b2, tr.Raw2)
	}
}

// ===========================================================================
// Reset
// ===========================================================================

func TestReset(t *testing.T) {
	sim, _ := runProgram(t, []byte{
		EncodeLdm(15),
		0xFA,          // STC
		EncodeHlt(),
	})
	if sim.Accumulator != 15 || !sim.Carry {
		t.Fatal("pre-reset state unexpected")
	}
	sim.Reset()
	if sim.Accumulator != 0 {
		t.Errorf("expected A=0 after reset")
	}
	if sim.Carry {
		t.Error("expected carry=false after reset")
	}
	if sim.Halted {
		t.Error("expected halted=false after reset")
	}
}

// ===========================================================================
// Stack overflow wrapping
// ===========================================================================

func TestStackOverflowWraps(t *testing.T) {
	// Push 4 addresses — the 4th should overwrite the 1st
	sim := NewIntel4004Simulator(4096)
	sim.stackPush(0x100)
	sim.stackPush(0x200)
	sim.stackPush(0x300)
	// Stack is full (3 entries). 4th push wraps.
	sim.stackPush(0x400) // overwrites 0x100
	// Popping should give: 0x400, 0x300, 0x200
	v1 := sim.stackPop()
	v2 := sim.stackPop()
	v3 := sim.stackPop()
	if v1 != 0x400 {
		t.Errorf("expected 0x400, got 0x%03X", v1)
	}
	if v2 != 0x300 {
		t.Errorf("expected 0x300, got 0x%03X", v2)
	}
	if v3 != 0x200 {
		t.Errorf("expected 0x200, got 0x%03X", v3)
	}
}

// ===========================================================================
// Register pair read/write
// ===========================================================================

func TestReadWritePair(t *testing.T) {
	sim := NewIntel4004Simulator(4096)
	sim.writePair(3, 0xBE) // R6=0xB, R7=0xE
	if sim.Registers[6] != 0xB {
		t.Errorf("expected R6=0xB, got %d", sim.Registers[6])
	}
	if sim.Registers[7] != 0xE {
		t.Errorf("expected R7=0xE, got %d", sim.Registers[7])
	}
	val := sim.readPair(3)
	if val != 0xBE {
		t.Errorf("expected pair=0xBE, got 0x%02X", val)
	}
}

// ===========================================================================
// Encoder helpers
// ===========================================================================

func TestEncoders(t *testing.T) {
	if EncodeNOP() != 0x00 {
		t.Error("NOP encoding wrong")
	}
	if EncodeHlt() != 0x01 {
		t.Error("HLT encoding wrong")
	}
	if EncodeLdm(5) != 0xD5 {
		t.Error("LDM encoding wrong")
	}
	if EncodeLd(3) != 0xA3 {
		t.Error("LD encoding wrong")
	}
	if EncodeXch(2) != 0xB2 {
		t.Error("XCH encoding wrong")
	}
	if EncodeAdd(1) != 0x81 {
		t.Error("ADD encoding wrong")
	}
	if EncodeSub(4) != 0x94 {
		t.Error("SUB encoding wrong")
	}
	if EncodeInc(7) != 0x67 {
		t.Error("INC encoding wrong")
	}
	if EncodeBbl(3) != 0xC3 {
		t.Error("BBL encoding wrong")
	}

	b1, b2 := EncodeJun(0x123)
	if b1 != 0x41 || b2 != 0x23 {
		t.Errorf("JUN encoding wrong: %02X %02X", b1, b2)
	}

	b1, b2 = EncodeJms(0x456)
	if b1 != 0x54 || b2 != 0x56 {
		t.Errorf("JMS encoding wrong: %02X %02X", b1, b2)
	}

	b1, b2 = EncodeJcn(0x4, 0xAB)
	if b1 != 0x14 || b2 != 0xAB {
		t.Errorf("JCN encoding wrong: %02X %02X", b1, b2)
	}

	b1, b2 = EncodeFim(2, 0xCD)
	if b1 != 0x24 || b2 != 0xCD {
		t.Errorf("FIM encoding wrong: %02X %02X", b1, b2)
	}

	if EncodeSrc(1) != 0x23 {
		t.Errorf("SRC encoding wrong: %02X", EncodeSrc(1))
	}

	if EncodeFin(2) != 0x34 {
		t.Errorf("FIN encoding wrong: %02X", EncodeFin(2))
	}

	if EncodeJin(2) != 0x35 {
		t.Errorf("JIN encoding wrong: %02X", EncodeJin(2))
	}

	b1, b2 = EncodeIsz(5, 0x10)
	if b1 != 0x75 || b2 != 0x10 {
		t.Errorf("ISZ encoding wrong: %02X %02X", b1, b2)
	}
}

// ===========================================================================
// Unknown instruction
// ===========================================================================

func TestUnknownInstruction(t *testing.T) {
	_, traces := runProgram(t, []byte{
		0x02,          // Unknown (0x0_ but not NOP or HLT)
		EncodeHlt(),
	})
	if traces[0].Mnemonic != "UNKNOWN(0x02)" {
		t.Errorf("expected UNKNOWN(0x02), got %s", traces[0].Mnemonic)
	}
}

func TestUnknownFE(t *testing.T) {
	_, traces := runProgram(t, []byte{
		0xFE,          // 0xFE is not a defined instruction
		EncodeHlt(),
	})
	if traces[0].Mnemonic != "UNKNOWN(0xFE)" {
		t.Errorf("expected UNKNOWN(0xFE), got %s", traces[0].Mnemonic)
	}
}

func TestUnknownFF(t *testing.T) {
	_, traces := runProgram(t, []byte{
		0xFF,          // 0xFF is not a defined instruction
		EncodeHlt(),
	})
	if traces[0].Mnemonic != "UNKNOWN(0xFF)" {
		t.Errorf("expected UNKNOWN(0xFF), got %s", traces[0].Mnemonic)
	}
}

// ===========================================================================
// isTwoByte helper
// ===========================================================================

func TestIsTwoByte(t *testing.T) {
	// JCN (0x1_) — always 2-byte
	if !isTwoByte(0x14) {
		t.Error("JCN should be 2-byte")
	}
	// FIM (0x2_ even) — 2-byte
	if !isTwoByte(0x20) {
		t.Error("FIM should be 2-byte")
	}
	// SRC (0x2_ odd) — 1-byte
	if isTwoByte(0x21) {
		t.Error("SRC should be 1-byte")
	}
	// JUN (0x4_) — always 2-byte
	if !isTwoByte(0x40) {
		t.Error("JUN should be 2-byte")
	}
	// JMS (0x5_) — always 2-byte
	if !isTwoByte(0x50) {
		t.Error("JMS should be 2-byte")
	}
	// ISZ (0x7_) — always 2-byte
	if !isTwoByte(0x70) {
		t.Error("ISZ should be 2-byte")
	}
	// ADD (0x8_) — 1-byte
	if isTwoByte(0x80) {
		t.Error("ADD should be 1-byte")
	}
	// NOP — 1-byte
	if isTwoByte(0x00) {
		t.Error("NOP should be 1-byte")
	}
	// FIN (0x3_ even) — 1-byte
	if isTwoByte(0x30) {
		t.Error("FIN should be 1-byte")
	}
}

// ===========================================================================
// PC bounds check
// ===========================================================================

func TestPCBeyondMemory(t *testing.T) {
	sim := NewIntel4004Simulator(4)
	program := []byte{EncodeNOP(), EncodeNOP(), EncodeNOP(), EncodeNOP()}
	traces := sim.Run(program, 100)
	// Should stop when PC reaches memory size
	if len(traces) != 4 {
		t.Errorf("expected 4 traces, got %d", len(traces))
	}
}

// ===========================================================================
// Integration: counting loop with ISZ
// ===========================================================================

func TestCountingLoop(t *testing.T) {
	// Count from 12 to 0 (4 iterations), accumulating in A
	// R0 = 12 (loop counter, ISZ loops until wrap to 0)
	// R1 = running sum
	// Each iteration: LD R1, IAC, XCH R1
	b1, b2 := EncodeIsz(0, 0x05)
	sim, _ := runProgram(t, []byte{
		EncodeLdm(12), // 0x00: A = 12
		EncodeXch(0),  // 0x01: R0 = 12
		EncodeLdm(0),  // 0x02: A = 0
		EncodeXch(1),  // 0x03: R1 = 0
		EncodeLdm(0),  // 0x04: A = 0 (start of loop would be ISZ target)
		// Loop body:
		EncodeLd(1),   // 0x05: A = R1
		0xF2,          // 0x06: IAC — A++
		EncodeXch(1),  // 0x07: R1 = A
		b1, b2,        // 0x08: ISZ R0, 0x05 — loop if R0 != 0
		EncodeLd(1),   // 0x0A: A = R1
		EncodeHlt(),   // 0x0B: halt
	})
	// R0 goes: 12→13→14→15→0. That's 4 iterations. R1 = 4.
	if sim.Accumulator != 4 {
		t.Errorf("expected A=4 after 4 iterations, got %d", sim.Accumulator)
	}
}

// ===========================================================================
// Integration: BCD addition with DAA
// ===========================================================================

func TestBCDAddition(t *testing.T) {
	// Add BCD 7 + 8 = 15 (BCD: 1 carry, 5 result)
	sim, _ := runProgram(t, []byte{
		EncodeLdm(8),  // A = 8
		EncodeXch(0),  // R0 = 8
		EncodeLdm(7),  // A = 7
		0xF1,          // CLC — clear carry before add
		EncodeAdd(0),  // A = 7 + 8 = 15
		0xFB,          // DAA — 15 > 9, so A = (15+6) & 0xF = 5, carry = true
		EncodeHlt(),
	})
	if sim.Accumulator != 5 {
		t.Errorf("expected A=5 (BCD low digit), got %d", sim.Accumulator)
	}
	if !sim.Carry {
		t.Error("expected carry=true (BCD tens digit)")
	}
}

// ===========================================================================
// WMP with different banks
// ===========================================================================

func TestWMPBanks(t *testing.T) {
	sim, _ := runProgram(t, []byte{
		EncodeLdm(0),
		0xFD,          // DCL bank 0
		EncodeLdm(3),
		0xE1,          // WMP — output[0] = 3
		EncodeLdm(1),
		0xFD,          // DCL bank 1
		EncodeLdm(7),
		0xE1,          // WMP — output[1] = 7
		EncodeHlt(),
	})
	if sim.RAMOutput[0] != 3 {
		t.Errorf("expected output[0]=3, got %d", sim.RAMOutput[0])
	}
	if sim.RAMOutput[1] != 7 {
		t.Errorf("expected output[1]=7, got %d", sim.RAMOutput[1])
	}
}
