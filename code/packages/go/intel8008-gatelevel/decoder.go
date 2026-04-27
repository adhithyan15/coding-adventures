package intel8008gatelevel

// Instruction decoder — combinational logic that maps 8008 opcodes to control signals.
//
// # How instruction decoding works in hardware
//
// The decoder takes an 8-bit opcode and produces control signals that drive the
// datapath for one instruction. It is purely combinational — no state, no clock.
// Given the same 8 input bits, it always produces the same output signals.
//
// The real 8008 decoder was a network of AND/OR/NOT gates arranged in a priority
// tree. We implement the same logic using the logic-gates package functions.
//
// # The 8008's group structure
//
// Instructions are grouped by the top 2 bits (bits 7-6):
//
//	group 00 (bits 7-6 = 00): HLT(0x00), MVI, INR, DCR, rotates, OUT
//	group 01 (bits 7-6 = 01): MOV, IN, JMP, CAL, conditional JMP/CAL, HLT(0x76)
//	group 10 (bits 7-6 = 10): ALU register ops (ADD/ADC/SUB/SBB/ANA/XRA/ORA/CMP)
//	group 11 (bits 7-6 = 11): ALU immediate, RST, RET, HLT(0xFF)
//
// # Encoding conflicts
//
// The 8008 has intentional encoding conflicts that the decoder must resolve:
//
//	0x76 (group 01, DDD=M, SSS=M) → HLT, not MOV M,M
//	0xFF                           → HLT, not RST 7
//	0x7E (group 01)                → CAL unconditional (3 bytes), not MOV A,M
//	0x7C (group 01)                → JMP unconditional (3 bytes), not MOV A,H
//	SSS=001 in group 01            → IN instruction, not MOV D,C
//	0x40,0x42,... (even, SSS=0/2)  → conditional JMP/CAL, not MOV
//
// See the Intel MCS-8 User Manual for the complete encoding table.

import (
	logicgates "github.com/adhithyan15/coding-adventures/code/packages/go/logic-gates"
)

// DecodedInstruction holds all control signals produced by the 8008 decoder.
//
// Every integer field represents a gate output: 0 or 1.
// The instruction-type fields (IsHLT, IsMOV, etc.) are mutually exclusive — exactly
// one will be 1 for any valid opcode.
type DecodedInstruction struct {
	// Original opcode byte and decoded bit fields
	Raw      int // original opcode
	Group    int // bits 7-6: 0, 1, 2, or 3
	DDD      int // bits 5-3: destination register (or ALU op code, or condition)
	SSS      int // bits 2-0: source register (or other field)

	// Instruction family (exactly one will be 1)
	IsHLT     int // halt
	IsMOV     int // register-to-register copy (group 01, excluding conflicts)
	IsIN      int // input from port (group 01, SSS=001)
	IsJMP     int // unconditional jump (0x7C)
	IsCAL     int // unconditional call (0x7E)
	IsJcond   int // conditional jump (group 01, SSS=000)
	IsCcond   int // conditional call (group 01, SSS=010)
	IsRcond   int // conditional return (group 00, SSS=011)
	IsRET     int // unconditional return (group 11, opcode 0xC7)
	IsRST     int // restart (group 11, SSS=101)
	IsMVI     int // move immediate (group 00, SSS=110)
	IsINR     int // increment register (group 00, SSS=000)
	IsDCR     int // decrement register (group 00, SSS=001)
	IsALUreg  int // ALU op with register source (group 10)
	IsALUimm  int // ALU op with immediate source (group 11, SSS=100)
	IsRLC     int // rotate left circular (opcode 0x02)
	IsRRC     int // rotate right circular (opcode 0x0A)
	IsRAL     int // rotate left through carry (opcode 0x12)
	IsRAR     int // rotate right through carry (opcode 0x1A)
	IsOUT     int // output to port (group 00, SSS=010, DDD>=4)

	// Instruction length (1, 2, or 3 bytes)
	InstrLen int

	// Decoded operands
	RegDst   int // destination register index (0-7, excluding 6=M)
	RegSrc   int // source register index (0-7, excluding 6=M)
	ALUOp    int // ALU operation code (0=ADD, 1=ADC, 2=SUB, 3=SBB, 4=ANA, 5=XRA, 6=ORA, 7=CMP)
	CondCode int // condition code (0=FC, 1=FZ, 2=FS, 3=FP, 4=TC, 5=TZ, 6=TS, 7=TP)
	RSTVec   int // RST vector address (0, 8, 16, 24, 32, 40, 48, 56)
	PortNum  int // I/O port number
}

// ALU operation codes — match DDD field in group 10 (bits 5-3)
const (
	ALUOpADD = 0 // A = A + S
	ALUOpADC = 1 // A = A + S + CY
	ALUOpSUB = 2 // A = A - S
	ALUOpSBB = 3 // A = A - S - CY
	ALUOpANA = 4 // A = A AND S
	ALUOpXRA = 5 // A = A XOR S
	ALUOpORA = 6 // A = A OR S
	ALUOpCMP = 7 // flags only: A - S
)

// Condition codes — match bits 5-3 in conditional instructions
// Bit 5 = T/F sense: 1 = jump-if-true, 0 = jump-if-false
// Bits 4-3 = which flag: 00=CY, 01=Z, 10=S, 11=P
const (
	CondFC = 0 // Carry false
	CondFZ = 1 // Zero false
	CondFS = 2 // Sign false
	CondFP = 3 // Parity false
	CondTC = 4 // Carry true
	CondTZ = 5 // Zero true
	CondTS = 6 // Sign true
	CondTP = 7 // Parity true
)

// Decode decodes an 8-bit opcode into control signals using gate logic.
//
// Input: 8-bit opcode integer.
// Output: DecodedInstruction with all control signals set.
//
// The decoder is organized as a hierarchy of AND/OR/NOT gates that match
// the 8008's instruction encoding structure (see MCS-8 manual, Table 2).
func Decode(opcode int) DecodedInstruction {
	result, _ := StartNew[DecodedInstruction]("intel8008-gatelevel.Decode", DecodedInstruction{},
		func(op *Operation[DecodedInstruction], rf *ResultFactory[DecodedInstruction]) *OperationResult[DecodedInstruction] {
			op.AddProperty("opcode", opcode)

			// Extract bit fields using AND gates
			//   bit7 = opcode >> 7 & 1
			//   bit6 = opcode >> 6 & 1
			//   ...
			bits := IntToBits(opcode, 8)
			b7, b6, b5, b4, b3, b2, b1, b0 := bits[7], bits[6], bits[5], bits[4], bits[3], bits[2], bits[1], bits[0]

			// Level 1: Decode group from bits 7-6
			//   group_00 = AND(NOT(b7), NOT(b6))
			//   group_01 = AND(NOT(b7), b6)
			//   group_10 = AND(b7, NOT(b6))
			//   group_11 = AND(b7, b6)
			g00 := logicgates.AND(logicgates.NOT(b7), logicgates.NOT(b6))
			g01 := logicgates.AND(logicgates.NOT(b7), b6)
			g10 := logicgates.AND(b7, logicgates.NOT(b6))
			g11 := logicgates.AND(b7, b6)

			group := (b7 << 1) | b6 // 0, 1, 2, or 3
			ddd := (b5 << 2) | (b4 << 1) | b3 // bits 5-3
			sss := (b2 << 2) | (b1 << 1) | b0 // bits 2-0

			d := DecodedInstruction{
				Raw:   opcode,
				Group: group,
				DDD:   ddd,
				SSS:   sss,
			}

			// ─── Handle HLT (special cases first) ───────────────────────────
			// HLT has 3 encodings:
			//   0x00: group_00 + all-zero DDD and SSS
			//   0x76: group_01 + DDD=110 + SSS=110 (would be MOV M,M)
			//   0xFF: group_11 + all-ones

			// is_0x00 = AND(g00, NOT(b5), NOT(b4), NOT(b3), NOT(b2), NOT(b1), NOT(b0))
			is0x00 := logicgates.AND(g00, logicgates.AND(
				logicgates.AND(logicgates.NOT(b5), logicgates.NOT(b4)),
				logicgates.AND(logicgates.AND(logicgates.NOT(b3), logicgates.NOT(b2)),
					logicgates.AND(logicgates.NOT(b1), logicgates.NOT(b0)))))

			// is_0x76 = AND(g01, b5, b4, NOT(b3), b2, b1, NOT(b0))  → ddd=110 (b5=1,b4=1,b3=0), sss=110 (b2=1,b1=1,b0=0)
			is0x76 := logicgates.AND(g01,
				logicgates.AND(
					logicgates.AND(logicgates.AND(b5, b4), logicgates.AND(logicgates.NOT(b3), b2)),
					logicgates.AND(b1, logicgates.NOT(b0))))

			// is_0xFF = AND(g11, b5, b4, b3, b2, b1, b0)
			is0xFF := logicgates.AND(g11,
				logicgates.AND(
					logicgates.AND(logicgates.AND(b5, b4), logicgates.AND(b3, b2)),
					logicgates.AND(b1, b0)))

			isHLT := logicgates.OR(logicgates.OR(is0x00, is0x76), is0xFF)

			if isHLT == 1 {
				d.IsHLT = 1
				d.InstrLen = 1
				return rf.Generate(true, false, d)
			}

			// ─── Group 00 ──────────────────────────────────────────────────
			if g00 == 1 {
				// Rotates: sss=010, ddd=0/1/2/3 (ddd < 4)
				//   RLC = 0x02 (ddd=000, sss=010)
				//   RRC = 0x0A (ddd=001, sss=010)
				//   RAL = 0x12 (ddd=010, sss=010)
				//   RAR = 0x1A (ddd=011, sss=010)
				// is_sss010 = AND(NOT(b2), b1, NOT(b0))
				isSss010 := logicgates.AND(logicgates.NOT(b2), logicgates.AND(b1, logicgates.NOT(b0)))
				// ddd < 4 means bit3=0 (b3=0 after group extraction means b5=0 here)
				// Actually ddd is bits 5-3, and ddd<4 means b5=0
				isDddLt4 := logicgates.NOT(b5)
				isRotate := logicgates.AND(isSss010, isDddLt4)

				if isRotate == 1 {
					d.InstrLen = 1
					switch opcode {
					case 0x02:
						d.IsRLC = 1
					case 0x0A:
						d.IsRRC = 1
					case 0x12:
						d.IsRAL = 1
					case 0x1A:
						d.IsRAR = 1
					default:
						// Other ddd values with sss=010 and ddd<4 shouldn't exist
						// but if they do, treat as rotate
						d.IsRLC = 1
					}
					return rf.Generate(true, false, d)
				}

				// OUT: sss=010, ddd >= 4
				//   OUT port: opcode = 00DDD010 where DDD >= 4
				//   port number = (opcode >> 1) & 0x1F
				isOut := logicgates.AND(isSss010, b5) // b5=1 means ddd >= 4
				if isOut == 1 {
					d.IsOUT = 1
					d.InstrLen = 1
					d.PortNum = (opcode >> 1) & 0x1F
					return rf.Generate(true, false, d)
				}

				// MVI: sss=110 (source=M used to signal immediate mode)
				//   opcode = 00DDD110
				isSss110 := logicgates.AND(b2, logicgates.AND(b1, logicgates.NOT(b0)))
				if isSss110 == 1 {
					d.IsMVI = 1
					d.InstrLen = 2
					d.RegDst = ddd
					return rf.Generate(true, false, d)
				}

				// INR: sss=000
				//   opcode = 00DDD000
				isSss000 := logicgates.AND(logicgates.NOT(b2), logicgates.AND(logicgates.NOT(b1), logicgates.NOT(b0)))
				if isSss000 == 1 {
					d.IsINR = 1
					d.InstrLen = 1
					d.RegDst = ddd
					return rf.Generate(true, false, d)
				}

				// DCR: sss=001
				//   opcode = 00DDD001
				isSss001 := logicgates.AND(logicgates.NOT(b2), logicgates.AND(logicgates.NOT(b1), b0))
				if isSss001 == 1 {
					d.IsDCR = 1
					d.InstrLen = 1
					d.RegDst = ddd
					return rf.Generate(true, false, d)
				}

				// Rcond: sss=011 (conditional return)
				//   opcode = 00CCC011 where CCC = DDD field = condition code
				isSss011 := logicgates.AND(logicgates.NOT(b2), logicgates.AND(b1, b0))
				if isSss011 == 1 {
					d.IsRcond = 1
					d.InstrLen = 1
					d.CondCode = ddd
					return rf.Generate(true, false, d)
				}

				// Unknown group 00 opcode — skip
				d.InstrLen = 1
				return rf.Generate(true, false, d)
			}

			// ─── Group 01 ──────────────────────────────────────────────────
			if g01 == 1 {
				// Check specific conflicting opcodes first

				// JMP (unconditional): opcode 0x7C
				if opcode == 0x7C {
					d.IsJMP = 1
					d.InstrLen = 3
					return rf.Generate(true, false, d)
				}

				// CAL (unconditional): opcode 0x7E
				if opcode == 0x7E {
					d.IsCAL = 1
					d.InstrLen = 3
					return rf.Generate(true, false, d)
				}

				// Conditional JMP: opcode = 01CCC000 (sss=000)
				// Encoded opcodes: 0x40,0x48,0x50,0x58 (TC/TZ/TS/TP sense=0)
				//                  0x44,0x4C,0x54,0x5C (sense=1)
				// sss=000 means bits 2-0 = 000
				isSss000 := logicgates.AND(logicgates.NOT(b2), logicgates.AND(logicgates.NOT(b1), logicgates.NOT(b0)))
				if isSss000 == 1 {
					d.IsJcond = 1
					d.InstrLen = 3
					d.CondCode = ddd // DDD = condition code (0-7)
					return rf.Generate(true, false, d)
				}

				// Conditional CAL: opcode = 01CCC010 (sss=010)
				isSss010 := logicgates.AND(logicgates.NOT(b2), logicgates.AND(b1, logicgates.NOT(b0)))
				if isSss010 == 1 {
					d.IsCcond = 1
					d.InstrLen = 3
					d.CondCode = ddd
					return rf.Generate(true, false, d)
				}

				// IN: sss=001 (replaces MOV D,C for all DDD values)
				isSss001 := logicgates.AND(logicgates.NOT(b2), logicgates.AND(logicgates.NOT(b1), b0))
				if isSss001 == 1 {
					d.IsIN = 1
					d.InstrLen = 1
					// Port number from DDD bits
					d.PortNum = ddd
					return rf.Generate(true, false, d)
				}

				// MOV: sss=111, 100, 101 (or sss=110 with ddd != 110)
				// All remaining group-01 encodings are MOV instructions
				d.IsMOV = 1
				d.InstrLen = 1
				d.RegDst = ddd
				d.RegSrc = sss
				return rf.Generate(true, false, d)
			}

			// ─── Group 10 — ALU register ops ───────────────────────────────
			if g10 == 1 {
				// opcode = 10OOO SSS where OOO = ALU op (DDD field), SSS = source register
				d.IsALUreg = 1
				d.InstrLen = 1
				d.ALUOp = ddd // DDD = ALU operation code
				d.RegSrc = sss // SSS = source register
				return rf.Generate(true, false, d)
			}

			// ─── Group 11 — ALU immediate, RST, RET ─────────────────────────
			// g11 == 1 here (after HLT 0xFF was handled above)

			// RET (unconditional return): opcode 0xC7
			if opcode == 0xC7 {
				d.IsRET = 1
				d.InstrLen = 1
				return rf.Generate(true, false, d)
			}

			// RST n: opcode = 11NNN101 (sss=101)
			//   Calls to address 8*NNN
			isSss101 := logicgates.AND(b2, logicgates.AND(logicgates.NOT(b1), b0))
			if isSss101 == 1 {
				d.IsRST = 1
				d.InstrLen = 1
				d.RSTVec = ddd * 8 // NNN * 8
				return rf.Generate(true, false, d)
			}

			// ALU immediate: sss=100 (SSS=100 flags immediate mode)
			//   opcode = 11OOO100
			//   OOO encodes: 000=ADI, 001=SUI, 100=ANI, 101=XRI, 110=ORI, 111=CPI
			//   Note: 010 and 011 are not defined for immediate ops (no ADC/SBB imm)
			isSss100 := logicgates.AND(b2, logicgates.AND(logicgates.NOT(b1), logicgates.NOT(b0)))
			if isSss100 == 1 {
				d.IsALUimm = 1
				d.InstrLen = 2
				d.ALUOp = ddd
				return rf.Generate(true, false, d)
			}

			// Unknown group 11 — treat as 1-byte NOP
			d.InstrLen = 1
			return rf.Generate(true, false, d)
		}).GetResult()
	return result
}
