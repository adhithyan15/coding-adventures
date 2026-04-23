package riscvsimulator

import (
	"testing"
)

// =============================================================================
// Helper: create a simulator, run a program, return the simulator for inspection
// =============================================================================

func runProgram(t *testing.T, instructions []uint32) *RiscVSimulator {
	t.Helper()
	sim := NewRiscVSimulator(65536)
	program := Assemble(instructions)
	sim.Run(program)
	return sim
}

func expectReg(t *testing.T, sim *RiscVSimulator, reg int, expected uint32) {
	t.Helper()
	got := sim.CPU.Registers.Read(reg)
	if got != expected {
		t.Errorf("x%d: expected %d (0x%08x), got %d (0x%08x)", reg, expected, expected, got, got)
	}
}

func expectRegSigned(t *testing.T, sim *RiscVSimulator, reg int, expected int32) {
	t.Helper()
	got := int32(sim.CPU.Registers.Read(reg))
	if got != expected {
		t.Errorf("x%d: expected %d, got %d", reg, expected, got)
	}
}

func TestHostSyscallsReadWriteAndExit(t *testing.T) {
	host := NewHostIO([]byte{'A'})
	sim := NewRiscVSimulatorWithHost(65536, host)
	program := Assemble([]uint32{
		EncodeAddi(17, 0, SyscallReadByte),  // a7 = read
		EncodeEcall(),                       // a0 = 'A'
		EncodeAddi(17, 0, SyscallWriteByte), // a7 = write
		EncodeEcall(),                       // stdout += a0
		EncodeAddi(10, 0, 0),                // a0 = exit code
		EncodeAddi(17, 0, SyscallExit),      // a7 = exit
		EncodeEcall(),
	})

	sim.Run(program)

	if !sim.CPU.Halted {
		t.Fatal("expected simulator to halt after host exit")
	}
	if !host.Exited {
		t.Fatal("expected host to record exit")
	}
	if host.ExitCode != 0 {
		t.Fatalf("expected exit code 0, got %d", host.ExitCode)
	}
	if got := host.OutputString(); got != "A" {
		t.Fatalf("expected host output %q, got %q", "A", got)
	}
}

// =============================================================================
// I-type arithmetic instructions
// =============================================================================

func TestAddi(t *testing.T) {
	sim := runProgram(t, []uint32{
		EncodeAddi(1, 0, 42), // x1 = 42
		EncodeAddi(2, 1, 10), // x2 = 42 + 10 = 52
		EncodeAddi(3, 0, -5), // x3 = -5
		EncodeAddi(4, 3, 3),  // x4 = -5 + 3 = -2
		EncodeEcall(),
	})
	expectReg(t, sim, 1, 42)
	expectReg(t, sim, 2, 52)
	expectRegSigned(t, sim, 3, -5)
	expectRegSigned(t, sim, 4, -2)
}

func TestSlti(t *testing.T) {
	sim := runProgram(t, []uint32{
		EncodeAddi(1, 0, 5),  // x1 = 5
		EncodeSlti(2, 1, 10), // x2 = (5 < 10) = 1
		EncodeSlti(3, 1, 3),  // x3 = (5 < 3) = 0
		EncodeSlti(4, 1, 5),  // x4 = (5 < 5) = 0
		EncodeAddi(5, 0, -1), // x5 = -1
		EncodeSlti(6, 5, 0),  // x6 = (-1 < 0) = 1  (signed comparison)
		EncodeEcall(),
	})
	expectReg(t, sim, 2, 1)
	expectReg(t, sim, 3, 0)
	expectReg(t, sim, 4, 0)
	expectReg(t, sim, 6, 1)
}

func TestSltiu(t *testing.T) {
	sim := runProgram(t, []uint32{
		EncodeAddi(1, 0, 5),   // x1 = 5
		EncodeSltiu(2, 1, 10), // x2 = (5 <u 10) = 1
		EncodeSltiu(3, 1, 3),  // x3 = (5 <u 3) = 0
		EncodeAddi(4, 0, -1),  // x4 = 0xFFFFFFFF
		EncodeSltiu(5, 4, 1),  // x5 = (0xFFFFFFFF <u 1) = 0 (unsigned: huge > 1)
		EncodeEcall(),
	})
	expectReg(t, sim, 2, 1)
	expectReg(t, sim, 3, 0)
	expectReg(t, sim, 5, 0)
}

func TestXori(t *testing.T) {
	sim := runProgram(t, []uint32{
		EncodeAddi(1, 0, 0xFF), // x1 = 0xFF
		EncodeXori(2, 1, 0x0F), // x2 = 0xFF ^ 0x0F = 0xF0
		EncodeEcall(),
	})
	expectReg(t, sim, 2, 0xF0)
}

func TestOri(t *testing.T) {
	sim := runProgram(t, []uint32{
		EncodeAddi(1, 0, 0x50), // x1 = 0x50
		EncodeOri(2, 1, 0x0F),  // x2 = 0x50 | 0x0F = 0x5F
		EncodeEcall(),
	})
	expectReg(t, sim, 2, 0x5F)
}

func TestAndi(t *testing.T) {
	sim := runProgram(t, []uint32{
		EncodeAddi(1, 0, 0xFF), // x1 = 0xFF
		EncodeAndi(2, 1, 0x0F), // x2 = 0xFF & 0x0F = 0x0F
		EncodeEcall(),
	})
	expectReg(t, sim, 2, 0x0F)
}

func TestSlli(t *testing.T) {
	sim := runProgram(t, []uint32{
		EncodeAddi(1, 0, 1),  // x1 = 1
		EncodeSlli(2, 1, 4),  // x2 = 1 << 4 = 16
		EncodeSlli(3, 1, 31), // x3 = 1 << 31 = 0x80000000
		EncodeEcall(),
	})
	expectReg(t, sim, 2, 16)
	expectReg(t, sim, 3, 0x80000000)
}

func TestSrli(t *testing.T) {
	sim := runProgram(t, []uint32{
		EncodeAddi(1, 0, -1), // x1 = 0xFFFFFFFF
		EncodeSrli(2, 1, 4),  // x2 = 0xFFFFFFFF >>> 4 = 0x0FFFFFFF (logical)
		EncodeSrli(3, 1, 31), // x3 = 0xFFFFFFFF >>> 31 = 1
		EncodeEcall(),
	})
	expectReg(t, sim, 2, 0x0FFFFFFF)
	expectReg(t, sim, 3, 1)
}

func TestSrai(t *testing.T) {
	sim := runProgram(t, []uint32{
		EncodeAddi(1, 0, -16), // x1 = 0xFFFFFFF0 (-16)
		EncodeSrai(2, 1, 2),   // x2 = -16 >> 2 = -4 (arithmetic, preserves sign)
		EncodeAddi(3, 0, 16),  // x3 = 16
		EncodeSrai(4, 3, 2),   // x4 = 16 >> 2 = 4  (positive, same as logical)
		EncodeEcall(),
	})
	expectRegSigned(t, sim, 2, -4)
	expectReg(t, sim, 4, 4)
}

// =============================================================================
// R-type arithmetic instructions
// =============================================================================

func TestAddSub(t *testing.T) {
	sim := runProgram(t, []uint32{
		EncodeAddi(1, 0, 10), // x1 = 10
		EncodeAddi(2, 0, 20), // x2 = 20
		EncodeAdd(3, 1, 2),   // x3 = 10 + 20 = 30
		EncodeSub(4, 1, 2),   // x4 = 10 - 20 = -10
		EncodeEcall(),
	})
	expectReg(t, sim, 3, 30)
	expectRegSigned(t, sim, 4, -10)
}

func TestSll(t *testing.T) {
	sim := runProgram(t, []uint32{
		EncodeAddi(1, 0, 1), // x1 = 1
		EncodeAddi(2, 0, 8), // x2 = 8
		EncodeSll(3, 1, 2),  // x3 = 1 << 8 = 256
		EncodeEcall(),
	})
	expectReg(t, sim, 3, 256)
}

func TestSlt(t *testing.T) {
	sim := runProgram(t, []uint32{
		EncodeAddi(1, 0, -5), // x1 = -5
		EncodeAddi(2, 0, 3),  // x2 = 3
		EncodeSlt(3, 1, 2),   // x3 = (-5 < 3) = 1 (signed)
		EncodeSlt(4, 2, 1),   // x4 = (3 < -5) = 0
		EncodeEcall(),
	})
	expectReg(t, sim, 3, 1)
	expectReg(t, sim, 4, 0)
}

func TestSltu(t *testing.T) {
	sim := runProgram(t, []uint32{
		EncodeAddi(1, 0, -1), // x1 = 0xFFFFFFFF
		EncodeAddi(2, 0, 1),  // x2 = 1
		EncodeSltu(3, 2, 1),  // x3 = (1 <u 0xFFFFFFFF) = 1
		EncodeSltu(4, 1, 2),  // x4 = (0xFFFFFFFF <u 1) = 0
		EncodeEcall(),
	})
	expectReg(t, sim, 3, 1)
	expectReg(t, sim, 4, 0)
}

func TestXor(t *testing.T) {
	sim := runProgram(t, []uint32{
		EncodeAddi(1, 0, 0xFF),
		EncodeAddi(2, 0, 0x0F),
		EncodeXor(3, 1, 2), // x3 = 0xFF ^ 0x0F = 0xF0
		EncodeEcall(),
	})
	expectReg(t, sim, 3, 0xF0)
}

func TestSrl(t *testing.T) {
	sim := runProgram(t, []uint32{
		EncodeAddi(1, 0, -1), // x1 = 0xFFFFFFFF
		EncodeAddi(2, 0, 4),  // x2 = 4
		EncodeSrl(3, 1, 2),   // x3 = 0xFFFFFFFF >>> 4 = 0x0FFFFFFF
		EncodeEcall(),
	})
	expectReg(t, sim, 3, 0x0FFFFFFF)
}

func TestSra(t *testing.T) {
	sim := runProgram(t, []uint32{
		EncodeAddi(1, 0, -16), // x1 = -16
		EncodeAddi(2, 0, 2),   // x2 = 2
		EncodeSra(3, 1, 2),    // x3 = -16 >> 2 = -4
		EncodeEcall(),
	})
	expectRegSigned(t, sim, 3, -4)
}

func TestOr(t *testing.T) {
	sim := runProgram(t, []uint32{
		EncodeAddi(1, 0, 0x50),
		EncodeAddi(2, 0, 0x0F),
		EncodeOr(3, 1, 2), // x3 = 0x50 | 0x0F = 0x5F
		EncodeEcall(),
	})
	expectReg(t, sim, 3, 0x5F)
}

func TestAnd(t *testing.T) {
	sim := runProgram(t, []uint32{
		EncodeAddi(1, 0, 0xFF),
		EncodeAddi(2, 0, 0x0F),
		EncodeAnd(3, 1, 2), // x3 = 0xFF & 0x0F = 0x0F
		EncodeEcall(),
	})
	expectReg(t, sim, 3, 0x0F)
}

// =============================================================================
// Load and store instructions
// =============================================================================

func TestStoreWordLoadWord(t *testing.T) {
	// Store a word to memory, then load it back
	sim := runProgram(t, []uint32{
		EncodeAddi(1, 0, 0x100), // x1 = 0x100 (base address)
		EncodeAddi(2, 0, 0x42),  // x2 = 0x42
		EncodeSw(2, 1, 0),       // mem[0x100] = 0x42
		EncodeLw(3, 1, 0),       // x3 = mem[0x100]
		EncodeEcall(),
	})
	expectReg(t, sim, 3, 0x42)
}

func TestStoreByteLoadByte(t *testing.T) {
	sim := runProgram(t, []uint32{
		EncodeAddi(1, 0, 0x200), // x1 = base address
		EncodeAddi(2, 0, 0xAB),  // x2 = 0xAB
		EncodeSb(2, 1, 0),       // mem[0x200] = 0xAB (byte)
		EncodeLbu(3, 1, 0),      // x3 = zero-extend(mem[0x200]) = 0xAB
		EncodeEcall(),
	})
	expectReg(t, sim, 3, 0xAB)
}

func TestLoadByteSignExtend(t *testing.T) {
	// Store 0xFF (which is -1 as a signed byte), then load with sign extension
	sim := runProgram(t, []uint32{
		EncodeAddi(1, 0, 0x200), // x1 = base address
		EncodeAddi(2, 0, 0xFF),  // x2 = 0xFF
		EncodeSb(2, 1, 0),       // mem[0x200] = 0xFF
		EncodeLb(3, 1, 0),       // x3 = sign-extend(0xFF) = 0xFFFFFFFF = -1
		EncodeLbu(4, 1, 0),      // x4 = zero-extend(0xFF) = 0x000000FF = 255
		EncodeEcall(),
	})
	expectRegSigned(t, sim, 3, -1) // lb sign-extends
	expectReg(t, sim, 4, 0xFF)     // lbu zero-extends
}

func TestStoreHalfLoadHalf(t *testing.T) {
	sim := runProgram(t, []uint32{
		EncodeAddi(1, 0, 0x200), // x1 = base address
		EncodeLui(2, 0),         // x2 = 0
		EncodeAddi(2, 0, 0x1FF), // x2 = 0x1FF (511)
		EncodeSh(2, 1, 0),       // mem[0x200] = 0x01FF (halfword)
		EncodeLhu(3, 1, 0),      // x3 = zero-extend(0x01FF) = 511
		EncodeEcall(),
	})
	expectReg(t, sim, 3, 0x1FF)
}

func TestLoadHalfSignExtend(t *testing.T) {
	// Store 0xFFFF as a halfword, then load with sign extension
	sim := runProgram(t, []uint32{
		EncodeAddi(1, 0, 0x200),
		EncodeAddi(2, 0, -1), // x2 = 0xFFFFFFFF
		EncodeSh(2, 1, 0),    // mem[0x200] = 0xFFFF (low 16 bits)
		EncodeLh(3, 1, 0),    // x3 = sign-extend(0xFFFF) = -1
		EncodeLhu(4, 1, 0),   // x4 = zero-extend(0xFFFF) = 65535
		EncodeEcall(),
	})
	expectRegSigned(t, sim, 3, -1)
	expectReg(t, sim, 4, 0xFFFF)
}

func TestStoreLoadWithOffset(t *testing.T) {
	// Use non-zero offset in store/load
	sim := runProgram(t, []uint32{
		EncodeAddi(1, 0, 0x200), // x1 = 0x200
		EncodeAddi(2, 0, 99),    // x2 = 99
		EncodeSw(2, 1, 4),       // mem[0x204] = 99
		EncodeLw(3, 1, 4),       // x3 = mem[0x204] = 99
		EncodeEcall(),
	})
	expectReg(t, sim, 3, 99)
}

// =============================================================================
// Branch instructions
// =============================================================================

func TestBeqTaken(t *testing.T) {
	// beq: if x1 == x2, skip next instruction
	sim := runProgram(t, []uint32{
		EncodeAddi(1, 0, 5),   // x1 = 5        (PC=0)
		EncodeAddi(2, 0, 5),   // x2 = 5        (PC=4)
		EncodeBeq(1, 2, 8),    // beq: skip 2 instructions (PC=8, target=16)
		EncodeAddi(3, 0, 999), // SKIPPED        (PC=12)
		EncodeAddi(4, 0, 42),  // x4 = 42       (PC=16, target)
		EncodeEcall(),
	})
	expectReg(t, sim, 3, 0)  // should be skipped
	expectReg(t, sim, 4, 42) // should execute
}

func TestBeqNotTaken(t *testing.T) {
	sim := runProgram(t, []uint32{
		EncodeAddi(1, 0, 5),
		EncodeAddi(2, 0, 10),
		EncodeBeq(1, 2, 8),   // not taken (5 != 10)
		EncodeAddi(3, 0, 42), // should execute
		EncodeEcall(),
	})
	expectReg(t, sim, 3, 42)
}

func TestBne(t *testing.T) {
	sim := runProgram(t, []uint32{
		EncodeAddi(1, 0, 5),
		EncodeAddi(2, 0, 10),
		EncodeBne(1, 2, 8),    // taken (5 != 10), skip to PC=16
		EncodeAddi(3, 0, 999), // SKIPPED
		EncodeAddi(4, 0, 42),  // should execute
		EncodeEcall(),
	})
	expectReg(t, sim, 3, 0)
	expectReg(t, sim, 4, 42)
}

func TestBlt(t *testing.T) {
	sim := runProgram(t, []uint32{
		EncodeAddi(1, 0, -5),  // x1 = -5 (signed)
		EncodeAddi(2, 0, 3),   // x2 = 3
		EncodeBlt(1, 2, 8),    // taken: -5 < 3 (signed), jump +8 from PC=8 -> PC=16
		EncodeAddi(3, 0, 999), // SKIPPED
		EncodeAddi(4, 0, 42),  // should execute
		EncodeEcall(),
	})
	expectReg(t, sim, 3, 0)
	expectReg(t, sim, 4, 42)
}

func TestBge(t *testing.T) {
	sim := runProgram(t, []uint32{
		EncodeAddi(1, 0, 5),
		EncodeAddi(2, 0, 5),
		EncodeBge(1, 2, 8),    // taken: 5 >= 5
		EncodeAddi(3, 0, 999), // SKIPPED
		EncodeAddi(4, 0, 42),
		EncodeEcall(),
	})
	expectReg(t, sim, 3, 0)
	expectReg(t, sim, 4, 42)
}

func TestBltu(t *testing.T) {
	// In unsigned comparison, -1 (0xFFFFFFFF) is the largest value
	sim := runProgram(t, []uint32{
		EncodeAddi(1, 0, 1),
		EncodeAddi(2, 0, -1),  // x2 = 0xFFFFFFFF
		EncodeBltu(1, 2, 8),   // taken: 1 <u 0xFFFFFFFF
		EncodeAddi(3, 0, 999), // SKIPPED
		EncodeAddi(4, 0, 42),
		EncodeEcall(),
	})
	expectReg(t, sim, 3, 0)
	expectReg(t, sim, 4, 42)
}

func TestBgeu(t *testing.T) {
	sim := runProgram(t, []uint32{
		EncodeAddi(1, 0, -1), // x1 = 0xFFFFFFFF
		EncodeAddi(2, 0, 1),
		EncodeBgeu(1, 2, 8),   // taken: 0xFFFFFFFF >=u 1
		EncodeAddi(3, 0, 999), // SKIPPED
		EncodeAddi(4, 0, 42),
		EncodeEcall(),
	})
	expectReg(t, sim, 3, 0)
	expectReg(t, sim, 4, 42)
}

func TestBranchBackward(t *testing.T) {
	// A simple loop: count from 0 to 3
	// x1 = counter, x2 = limit (3)
	sim := runProgram(t, []uint32{
		EncodeAddi(1, 0, 0), // x1 = 0          (PC=0)
		EncodeAddi(2, 0, 3), // x2 = 3          (PC=4)
		EncodeAddi(1, 1, 1), // x1++            (PC=8, loop target)
		EncodeBne(1, 2, -4), // if x1 != 3, jump back to PC=8 (offset=-4)
		EncodeEcall(),       //                 (PC=16)
	})
	expectReg(t, sim, 1, 3)
}

// =============================================================================
// Jump instructions
// =============================================================================

func TestJal(t *testing.T) {
	// jal: jump forward, saving return address
	sim := runProgram(t, []uint32{
		EncodeJal(1, 8),       // x1 = PC+4 = 4, jump to PC+8 = 8  (PC=0)
		EncodeAddi(2, 0, 999), // SKIPPED                           (PC=4)
		EncodeAddi(3, 0, 42),  // should execute                    (PC=8)
		EncodeEcall(),
	})
	expectReg(t, sim, 1, 4)  // return address = old PC + 4
	expectReg(t, sim, 2, 0)  // skipped
	expectReg(t, sim, 3, 42) // executed
}

func TestJalr(t *testing.T) {
	// jalr: indirect jump through register
	sim := runProgram(t, []uint32{
		EncodeAddi(5, 0, 12),  // x5 = 12 (target address)   (PC=0)
		EncodeJalr(1, 5, 0),   // x1 = PC+4 = 8, jump to 12 (PC=4)
		EncodeAddi(2, 0, 999), // SKIPPED                    (PC=8)
		EncodeAddi(3, 0, 42),  // should execute             (PC=12)
		EncodeEcall(),
	})
	expectReg(t, sim, 1, 8)
	expectReg(t, sim, 2, 0)
	expectReg(t, sim, 3, 42)
}

func TestJalrWithOffset(t *testing.T) {
	sim := runProgram(t, []uint32{
		EncodeAddi(5, 0, 8),   // x5 = 8
		EncodeJalr(1, 5, 4),   // jump to (8+4)=12
		EncodeAddi(2, 0, 999), // SKIPPED
		EncodeAddi(3, 0, 42),  // executed at PC=12
		EncodeEcall(),
	})
	expectReg(t, sim, 1, 8)
	expectReg(t, sim, 2, 0)
	expectReg(t, sim, 3, 42)
}

func TestCallAndReturn(t *testing.T) {
	// A proper function call pattern:
	//   PC=0:  jal x1, 12       (call function at PC=12, save return addr PC+4=4 in x1)
	//   PC=4:  addi x11, 0, 99  (runs after return: x11 = 99)
	//   PC=8:  ecall
	//   PC=12: addi x10, 0, 42  (function body: x10 = 42)
	//   PC=16: jalr x0, x1, 0   (return to x1=4, discard link)
	sim := runProgram(t, []uint32{
		EncodeJal(1, 12),      // call function at PC=12
		EncodeAddi(11, 0, 99), // x11 = 99 (after return)
		EncodeEcall(),
		EncodeAddi(10, 0, 42), // function body
		EncodeJalr(0, 1, 0),   // return (jalr x0, x1, 0)
	})
	expectReg(t, sim, 1, 4)   // return address
	expectReg(t, sim, 10, 42) // function result
	expectReg(t, sim, 11, 99) // executed after return
}

// =============================================================================
// LUI and AUIPC
// =============================================================================

func TestLui(t *testing.T) {
	sim := runProgram(t, []uint32{
		EncodeLui(1, 0x12345), // x1 = 0x12345 << 12 = 0x12345000
		EncodeEcall(),
	})
	expectReg(t, sim, 1, 0x12345000)
}

func TestLuiPlusAddi(t *testing.T) {
	// Construct a full 32-bit constant: 0x12345678
	sim := runProgram(t, []uint32{
		EncodeLui(1, 0x12345),   // x1 = 0x12345000
		EncodeAddi(1, 1, 0x678), // x1 = 0x12345000 + 0x678 = 0x12345678
		EncodeEcall(),
	})
	expectReg(t, sim, 1, 0x12345678)
}

func TestAuipc(t *testing.T) {
	// auipc at PC=0: x1 = PC + (imm << 12) = 0 + (1 << 12) = 0x1000
	sim := runProgram(t, []uint32{
		EncodeAuipc(1, 1), // x1 = PC(0) + 1<<12 = 0x1000
		EncodeEcall(),
	})
	expectReg(t, sim, 1, 0x1000)
}

func TestAuipcNonZeroPC(t *testing.T) {
	// auipc at PC=4: x1 = 4 + (2 << 12) = 4 + 0x2000 = 0x2004
	sim := runProgram(t, []uint32{
		EncodeAddi(0, 0, 0), // nop (PC=0)
		EncodeAuipc(1, 2),   // x1 = PC(4) + 2<<12 = 0x2004 (PC=4)
		EncodeEcall(),
	})
	expectReg(t, sim, 1, 0x2004)
}

// =============================================================================
// Register x0 hardwired to zero
// =============================================================================

func TestRegisterZeroHardwired(t *testing.T) {
	sim := runProgram(t, []uint32{
		EncodeAddi(0, 0, 42), // try to write 42 to x0
		EncodeEcall(),
	})
	expectReg(t, sim, 0, 0) // x0 must remain 0
}

func TestRegisterZeroOnRType(t *testing.T) {
	sim := runProgram(t, []uint32{
		EncodeAddi(1, 0, 5),
		EncodeAddi(2, 0, 10),
		EncodeAdd(0, 1, 2), // try to write to x0
		EncodeEcall(),
	})
	expectReg(t, sim, 0, 0)
}

// =============================================================================
// CSR operations
// =============================================================================

func TestCsrrw(t *testing.T) {
	sim := NewRiscVSimulator(65536)
	// Write 0x100 to mscratch, then read it back
	program := Assemble([]uint32{
		EncodeAddi(1, 0, 0x100),        // x1 = 0x100
		EncodeCsrrw(2, CSRMscratch, 1), // x2 = old mscratch (0), mscratch = 0x100
		EncodeCsrrw(3, CSRMscratch, 0), // x3 = mscratch (0x100), mscratch = x0 (0)
		EncodeEcall(),
	})
	sim.Run(program)
	expectReg(t, sim, 2, 0)     // mscratch was 0
	expectReg(t, sim, 3, 0x100) // mscratch was 0x100 after first write
}

func TestCsrrs(t *testing.T) {
	sim := NewRiscVSimulator(65536)
	// Set bits in mstatus
	program := Assemble([]uint32{
		EncodeAddi(1, 0, 8),           // x1 = 8 (MIE bit)
		EncodeCsrrs(2, CSRMstatus, 1), // x2 = old mstatus (0), mstatus |= 8
		EncodeCsrrs(3, CSRMstatus, 0), // x3 = mstatus (8), no bits set (x0=0)
		EncodeEcall(),
	})
	sim.Run(program)
	expectReg(t, sim, 2, 0)
	expectReg(t, sim, 3, 8)
}

func TestCsrrc(t *testing.T) {
	sim := NewRiscVSimulator(65536)
	// Set bits then clear some
	program := Assemble([]uint32{
		EncodeAddi(1, 0, 0xFF),         // x1 = 0xFF
		EncodeCsrrw(0, CSRMscratch, 1), // mscratch = 0xFF
		EncodeAddi(2, 0, 0x0F),         // x2 = 0x0F
		EncodeCsrrc(3, CSRMscratch, 2), // x3 = mscratch (0xFF), mscratch &^= 0x0F -> 0xF0
		EncodeCsrrs(4, CSRMscratch, 0), // x4 = mscratch (0xF0)
		EncodeEcall(),
	})
	sim.Run(program)
	expectReg(t, sim, 3, 0xFF) // old value
	expectReg(t, sim, 4, 0xF0) // after clearing low nibble
}

// =============================================================================
// ecall trap behavior
// =============================================================================

func TestEcallHaltWhenNoTrapHandler(t *testing.T) {
	// When mtvec=0 (default), ecall should halt
	sim := runProgram(t, []uint32{
		EncodeAddi(1, 0, 42),
		EncodeEcall(),
	})
	if !sim.CPU.Halted {
		t.Error("CPU should be halted after ecall with mtvec=0")
	}
	expectReg(t, sim, 1, 42)
}

func TestEcallTrapWithHandler(t *testing.T) {
	sim := NewRiscVSimulator(65536)

	// Set up a trap handler at address 0x100.
	// The handler writes 99 to x10 then halts (ecall with mtvec=0 will halt since
	// the handler clears mtvec before its own ecall... or we can use mret).
	//
	// Layout:
	//   PC=0x000: set mtvec to 0x100
	//   PC=0x004: ecall (triggers trap -> jumps to 0x100)
	//   PC=0x008: x11 = 77 (should execute after mret)
	//   PC=0x00C: ecall (halt, mtvec=0)
	//   ...
	//   PC=0x100: trap handler — write 99 to x10, advance mepc, mret

	// Main program at address 0
	mainCode := []uint32{
		EncodeAddi(1, 0, 0x100),     // x1 = 0x100 (trap handler addr)  (PC=0)
		EncodeCsrrw(0, CSRMtvec, 1), // mtvec = 0x100                   (PC=4)
		EncodeEcall(),               // trigger trap -> jump to 0x100   (PC=8)
		EncodeAddi(11, 0, 77),       // x11 = 77 (after return)         (PC=12)
		// Clear mtvec so next ecall halts
		EncodeCsrrw(0, CSRMtvec, 0), // mtvec = 0                       (PC=16)
		EncodeEcall(),               // halt                            (PC=20)
	}

	// Trap handler at address 0x100 (= 64 instructions * 4 bytes each)
	// We need to pad from mainCode end to 0x100
	padCount := (0x100 / 4) - len(mainCode)
	paddedMain := make([]uint32, 0, len(mainCode)+padCount+4)
	paddedMain = append(paddedMain, mainCode...)
	for i := 0; i < padCount; i++ {
		paddedMain = append(paddedMain, EncodeAddi(0, 0, 0)) // nop padding
	}

	// Trap handler code at 0x100:
	//   Read mepc, add 4 (skip past the ecall), write back, then mret
	trapHandler := []uint32{
		EncodeAddi(10, 0, 99),       // x10 = 99                       (PC=0x100)
		EncodeCsrrs(20, CSRMepc, 0), // x20 = mepc (read mepc)         (PC=0x104)
		EncodeAddi(20, 20, 4),       // x20 = mepc + 4                 (PC=0x108)
		EncodeCsrrw(0, CSRMepc, 20), // mepc = mepc + 4                (PC=0x10C)
		EncodeMret(),                // return from trap               (PC=0x110)
	}
	paddedMain = append(paddedMain, trapHandler...)

	program := Assemble(paddedMain)
	sim.Run(program)

	expectReg(t, sim, 10, 99) // trap handler wrote this
	expectReg(t, sim, 11, 77) // executed after mret
	if !sim.CPU.Halted {
		t.Error("CPU should be halted after second ecall")
	}
}

func TestEcallSetsCSRs(t *testing.T) {
	sim := NewRiscVSimulator(65536)

	// Enable interrupts, set mtvec, then ecall.
	// The trap handler at 0x200 clears mtvec first (so its own ecall halts),
	// then we can inspect the CSR values that were set by the original ecall.
	mainCode := []uint32{
		EncodeAddi(1, 0, 0x200),       // x1 = trap handler addr     (PC=0)
		EncodeCsrrw(0, CSRMtvec, 1),   // mtvec = 0x200              (PC=4)
		EncodeAddi(2, 0, 8),           // x2 = MIE bit               (PC=8)
		EncodeCsrrs(0, CSRMstatus, 2), // mstatus |= MIE             (PC=12)
		EncodeEcall(),                 // PC=16: triggers trap
	}

	// Pad to 0x200
	padCount := (0x200 / 4) - len(mainCode)
	padded := make([]uint32, 0, len(mainCode)+padCount+4)
	padded = append(padded, mainCode...)
	for i := 0; i < padCount; i++ {
		padded = append(padded, EncodeAddi(0, 0, 0)) // nop
	}

	// Trap handler at 0x200: save mepc and mcause to registers,
	// then clear mtvec and halt.
	trapHandler := []uint32{
		EncodeCsrrs(20, CSRMepc, 0),    // x20 = mepc                 (PC=0x200)
		EncodeCsrrs(21, CSRMcause, 0),  // x21 = mcause               (PC=0x204)
		EncodeCsrrs(22, CSRMstatus, 0), // x22 = mstatus              (PC=0x208)
		EncodeCsrrw(0, CSRMtvec, 0),    // mtvec = 0 (so next ecall halts) (PC=0x20C)
		EncodeEcall(),                  // halt                       (PC=0x210)
	}
	padded = append(padded, trapHandler...)

	program := Assemble(padded)
	sim.Run(program)

	// The trap handler saved the values into registers before any could be overwritten
	mepc := sim.CPU.Registers.Read(20)
	mcause := sim.CPU.Registers.Read(21)
	mstatus := sim.CPU.Registers.Read(22)

	if mepc != 16 {
		t.Errorf("mepc should be 16 (PC of ecall), got %d", mepc)
	}
	if mcause != CauseEcallMMode {
		t.Errorf("mcause should be %d, got %d", CauseEcallMMode, mcause)
	}
	// MIE should be cleared (interrupts disabled during trap)
	if mstatus&MIE != 0 {
		t.Errorf("MIE bit should be cleared during trap, mstatus=0x%x", mstatus)
	}
}

// =============================================================================
// mret
// =============================================================================

func TestMret(t *testing.T) {
	sim := NewRiscVSimulator(65536)
	// Manually set mepc and then execute mret
	sim.CSR.Write(CSRMepc, 12) // return to PC=12

	program := Assemble([]uint32{
		EncodeMret(),          // jump to mepc=12     (PC=0)
		EncodeAddi(1, 0, 999), // SKIPPED             (PC=4)
		EncodeAddi(2, 0, 999), // SKIPPED             (PC=8)
		EncodeAddi(3, 0, 42),  // should execute      (PC=12)
		EncodeEcall(),
	})
	sim.Run(program)

	expectReg(t, sim, 1, 0)
	expectReg(t, sim, 2, 0)
	expectReg(t, sim, 3, 42)
}

func TestMretReenablesInterrupts(t *testing.T) {
	sim := NewRiscVSimulator(65536)
	// Clear MIE (simulate being in trap handler)
	sim.CSR.Write(CSRMstatus, 0)
	sim.CSR.Write(CSRMepc, 4) // return to PC=4

	program := Assemble([]uint32{
		EncodeMret(),  // should re-enable MIE
		EncodeEcall(), // halt
	})
	sim.Run(program)

	mstatus := sim.CSR.Read(CSRMstatus)
	if mstatus&MIE == 0 {
		t.Errorf("MIE should be re-enabled after mret, mstatus=0x%x", mstatus)
	}
}

// =============================================================================
// Unknown instruction handling
// =============================================================================

func TestUnknownInstruction(t *testing.T) {
	sim := runProgram(t, []uint32{
		0xFFFFFFFF,
		EncodeEcall(),
	})
	if sim.CPU.Registers.Read(1) != 0 {
		t.Error("Unknown instruction should not modify registers")
	}
}

// =============================================================================
// Negative immediate decoding
// =============================================================================

func TestNegativeImmediate(t *testing.T) {
	sim := runProgram(t, []uint32{
		EncodeAddi(1, 0, -5),
		EncodeEcall(),
	})
	expectRegSigned(t, sim, 1, -5)
}

// =============================================================================
// Integration test: a small multi-instruction program
// =============================================================================

func TestFibonacci(t *testing.T) {
	// Compute the 10th Fibonacci number (fib(10) = 55)
	// Using 0-indexed: fib(0)=0, fib(1)=1, fib(2)=1, ..., fib(10)=55
	// x1 = fib(n-2), x2 = fib(n-1), x3 = fib(n)
	// x4 = counter (starts at 2, increments each iteration)
	// x5 = limit (11, because we want counter to reach 10 inclusive,
	//       and the loop exits when counter == limit, so limit = 10+1)
	sim := runProgram(t, []uint32{
		EncodeAddi(1, 0, 0),  // x1 = 0 (fib[0])       (PC=0)
		EncodeAddi(2, 0, 1),  // x2 = 1 (fib[1])       (PC=4)
		EncodeAddi(4, 0, 2),  // x4 = 2 (counter)      (PC=8)
		EncodeAddi(5, 0, 11), // x5 = 11 (limit)       (PC=12)
		// Loop body:                                            (PC=16)
		EncodeAdd(3, 1, 2),   // x3 = x1 + x2          (PC=16)
		EncodeAddi(1, 2, 0),  // x1 = x2               (PC=20)  (mv x1, x2)
		EncodeAddi(2, 3, 0),  // x2 = x3               (PC=24)  (mv x2, x3)
		EncodeAddi(4, 4, 1),  // x4++                   (PC=28)
		EncodeBne(4, 5, -16), // if x4 != 11, loop (PC=32, offset=-16 -> PC=16)
		EncodeEcall(),        //                        (PC=36)
	})
	// After 9 iterations (counter 2..10): x2 holds fib(10) = 55
	expectReg(t, sim, 2, 55)
}

func TestMemcpy(t *testing.T) {
	// Copy 4 bytes from address 0x200 to 0x300 using load/store
	sim := NewRiscVSimulator(65536)

	// Pre-store data at 0x200
	sim.CPU.Memory.WriteByte(0x200, 0xDE)
	sim.CPU.Memory.WriteByte(0x201, 0xAD)
	sim.CPU.Memory.WriteByte(0x202, 0xBE)
	sim.CPU.Memory.WriteByte(0x203, 0xEF)

	program := Assemble([]uint32{
		EncodeAddi(1, 0, 0x200), // x1 = src
		EncodeAddi(2, 0, 0x300), // x2 = dst
		EncodeLw(3, 1, 0),       // x3 = mem[src] (load word)
		EncodeSw(3, 2, 0),       // mem[dst] = x3 (store word)
		EncodeEcall(),
	})
	sim.Run(program)

	// Verify the copy
	for i := 0; i < 4; i++ {
		src := sim.CPU.Memory.ReadByte(0x200 + i)
		dst := sim.CPU.Memory.ReadByte(0x300 + i)
		if src != dst {
			t.Errorf("Byte %d: src=0x%02x, dst=0x%02x", i, src, dst)
		}
	}
}

func TestStackOperations(t *testing.T) {
	// Simulate push/pop using a stack pointer (x2 = sp)
	// Stack grows downward (conventional RISC-V behavior)
	sim := runProgram(t, []uint32{
		EncodeAddi(2, 0, 0x400), // sp = 0x400 (top of stack)    (PC=0)
		EncodeAddi(10, 0, 42),   // x10 = 42                     (PC=4)
		EncodeAddi(11, 0, 99),   // x11 = 99                     (PC=8)
		// Push x10
		EncodeAddi(2, 2, -4), // sp -= 4                      (PC=12)
		EncodeSw(10, 2, 0),   // mem[sp] = x10                (PC=16)
		// Push x11
		EncodeAddi(2, 2, -4), // sp -= 4                      (PC=20)
		EncodeSw(11, 2, 0),   // mem[sp] = x11                (PC=24)
		// Pop into x12 (should get x11's value = 99)
		EncodeLw(12, 2, 0),  // x12 = mem[sp]                (PC=28)
		EncodeAddi(2, 2, 4), // sp += 4                      (PC=32)
		// Pop into x13 (should get x10's value = 42)
		EncodeLw(13, 2, 0),  // x13 = mem[sp]                (PC=36)
		EncodeAddi(2, 2, 4), // sp += 4                      (PC=40)
		EncodeEcall(),
	})
	expectReg(t, sim, 12, 99)   // last pushed, first popped
	expectReg(t, sim, 13, 42)   // first pushed, last popped
	expectReg(t, sim, 2, 0x400) // sp restored
}

// =============================================================================
// Step-by-step execution
// =============================================================================

func TestStep(t *testing.T) {
	sim := NewRiscVSimulator(65536)
	program := Assemble([]uint32{
		EncodeAddi(1, 0, 1),
		EncodeAddi(2, 0, 2),
		EncodeEcall(),
	})
	sim.CPU.LoadProgram(program, 0)

	trace1 := sim.Step()
	if trace1.Decode.Mnemonic != "addi" {
		t.Errorf("Expected addi, got %s", trace1.Decode.Mnemonic)
	}
	expectReg(t, sim, 1, 1)

	trace2 := sim.Step()
	if trace2.Decode.Mnemonic != "addi" {
		t.Errorf("Expected addi, got %s", trace2.Decode.Mnemonic)
	}
	expectReg(t, sim, 2, 2)
}

// =============================================================================
// Encoding round-trip tests (encode then decode)
// =============================================================================

func TestEncodeDecodeRoundTrip(t *testing.T) {
	decoder := &RiscVDecoder{}

	tests := []struct {
		name     string
		encoded  uint32
		mnemonic string
	}{
		{"addi", EncodeAddi(1, 2, 42), "addi"},
		{"slti", EncodeSlti(1, 2, -5), "slti"},
		{"sltiu", EncodeSltiu(1, 2, 5), "sltiu"},
		{"xori", EncodeXori(1, 2, 0xFF), "xori"},
		{"ori", EncodeOri(1, 2, 0xFF), "ori"},
		{"andi", EncodeAndi(1, 2, 0xFF), "andi"},
		{"slli", EncodeSlli(1, 2, 5), "slli"},
		{"srli", EncodeSrli(1, 2, 5), "srli"},
		{"srai", EncodeSrai(1, 2, 5), "srai"},
		{"add", EncodeAdd(1, 2, 3), "add"},
		{"sub", EncodeSub(1, 2, 3), "sub"},
		{"sll", EncodeSll(1, 2, 3), "sll"},
		{"slt", EncodeSlt(1, 2, 3), "slt"},
		{"sltu", EncodeSltu(1, 2, 3), "sltu"},
		{"xor", EncodeXor(1, 2, 3), "xor"},
		{"srl", EncodeSrl(1, 2, 3), "srl"},
		{"sra", EncodeSra(1, 2, 3), "sra"},
		{"or", EncodeOr(1, 2, 3), "or"},
		{"and", EncodeAnd(1, 2, 3), "and"},
		{"lb", EncodeLb(1, 2, 4), "lb"},
		{"lh", EncodeLh(1, 2, 4), "lh"},
		{"lw", EncodeLw(1, 2, 4), "lw"},
		{"lbu", EncodeLbu(1, 2, 4), "lbu"},
		{"lhu", EncodeLhu(1, 2, 4), "lhu"},
		{"sb", EncodeSb(3, 2, 4), "sb"},
		{"sh", EncodeSh(3, 2, 4), "sh"},
		{"sw", EncodeSw(3, 2, 4), "sw"},
		{"beq", EncodeBeq(1, 2, 8), "beq"},
		{"bne", EncodeBne(1, 2, 8), "bne"},
		{"blt", EncodeBlt(1, 2, 8), "blt"},
		{"bge", EncodeBge(1, 2, 8), "bge"},
		{"bltu", EncodeBltu(1, 2, 8), "bltu"},
		{"bgeu", EncodeBgeu(1, 2, 8), "bgeu"},
		{"jal", EncodeJal(1, 8), "jal"},
		{"jalr", EncodeJalr(1, 2, 4), "jalr"},
		{"lui", EncodeLui(1, 0x12345), "lui"},
		{"auipc", EncodeAuipc(1, 0x12345), "auipc"},
		{"ecall", EncodeEcall(), "ecall"},
		{"mret", EncodeMret(), "mret"},
		{"csrrw", EncodeCsrrw(1, 0x300, 2), "csrrw"},
		{"csrrs", EncodeCsrrs(1, 0x300, 2), "csrrs"},
		{"csrrc", EncodeCsrrc(1, 0x300, 2), "csrrc"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := decoder.Decode(tt.encoded, 0)
			if result.Mnemonic != tt.mnemonic {
				t.Errorf("Decode(%s): expected mnemonic %q, got %q", tt.name, tt.mnemonic, result.Mnemonic)
			}
		})
	}
}

// =============================================================================
// CSR file unit tests
// =============================================================================

func TestCSRFileReadWrite(t *testing.T) {
	csr := NewCSRFile()

	// Uninitialized read returns 0
	if csr.Read(CSRMstatus) != 0 {
		t.Error("Uninitialized CSR should read 0")
	}

	csr.Write(CSRMstatus, 0x1234)
	if csr.Read(CSRMstatus) != 0x1234 {
		t.Errorf("Expected 0x1234, got 0x%x", csr.Read(CSRMstatus))
	}
}

func TestCSRFileReadWriteAtomic(t *testing.T) {
	csr := NewCSRFile()
	csr.Write(CSRMscratch, 42)

	old := csr.ReadWrite(CSRMscratch, 99)
	if old != 42 {
		t.Errorf("ReadWrite should return old value 42, got %d", old)
	}
	if csr.Read(CSRMscratch) != 99 {
		t.Errorf("ReadWrite should set new value 99, got %d", csr.Read(CSRMscratch))
	}
}

func TestCSRFileReadSet(t *testing.T) {
	csr := NewCSRFile()
	csr.Write(CSRMstatus, 0xF0)

	old := csr.ReadSet(CSRMstatus, 0x0F)
	if old != 0xF0 {
		t.Errorf("ReadSet should return old value 0xF0, got 0x%x", old)
	}
	if csr.Read(CSRMstatus) != 0xFF {
		t.Errorf("ReadSet should OR bits, expected 0xFF, got 0x%x", csr.Read(CSRMstatus))
	}
}

func TestCSRFileReadClear(t *testing.T) {
	csr := NewCSRFile()
	csr.Write(CSRMstatus, 0xFF)

	old := csr.ReadClear(CSRMstatus, 0x0F)
	if old != 0xFF {
		t.Errorf("ReadClear should return old value 0xFF, got 0x%x", old)
	}
	if csr.Read(CSRMstatus) != 0xF0 {
		t.Errorf("ReadClear should AND NOT bits, expected 0xF0, got 0x%x", csr.Read(CSRMstatus))
	}
}

// =============================================================================
// Edge case: shift amount uses only lower 5 bits of rs2
// =============================================================================

func TestShiftAmountMasking(t *testing.T) {
	sim := runProgram(t, []uint32{
		EncodeAddi(1, 0, 1),  // x1 = 1
		EncodeAddi(2, 0, 33), // x2 = 33 (but shift uses only 33 & 0x1F = 1)
		EncodeSll(3, 1, 2),   // x3 = 1 << 1 = 2
		EncodeEcall(),
	})
	expectReg(t, sim, 3, 2)
}

// =============================================================================
// Assemble helper test
// =============================================================================

func TestAssemble(t *testing.T) {
	bytes := Assemble([]uint32{0x12345678})
	if len(bytes) != 4 {
		t.Fatalf("Expected 4 bytes, got %d", len(bytes))
	}
	// Little-endian: lowest byte first
	if bytes[0] != 0x78 || bytes[1] != 0x56 || bytes[2] != 0x34 || bytes[3] != 0x12 {
		t.Errorf("Little-endian encoding wrong: %v", bytes)
	}
}
