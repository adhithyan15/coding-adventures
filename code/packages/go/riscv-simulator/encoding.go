// Package riscvsimulator — encoding helpers for constructing machine code.
//
// === Why encoding helpers? ===
//
// When testing a CPU simulator, we need to feed it real machine code bytes.
// Writing raw hex is error-prone and unreadable. These helpers let us write:
//
//   EncodeAddi(1, 0, 42)   // "addi x1, x0, 42" — much clearer than 0x02A00093
//
// Each helper constructs the 32-bit instruction word by placing fields in
// their correct bit positions according to the RISC-V encoding format.
//
// === Bit manipulation patterns ===
//
// All encoders use the same basic technique: shift each field to its bit
// position and OR them together. For example, an I-type instruction:
//
//   [imm[11:0] | rs1[4:0] | funct3[2:0] | rd[4:0] | opcode[6:0]]
//    bits 31:20  bits 19:15  bits 14:12    bits 11:7  bits 6:0
//
//   encoded = (imm << 20) | (rs1 << 15) | (funct3 << 12) | (rd << 7) | opcode
package riscvsimulator

// === I-type encoders ===
//
// I-type format: [imm[11:0] | rs1 | funct3 | rd | opcode]

// EncodeAddi encodes: addi rd, rs1, imm
func EncodeAddi(rd, rs1, imm int) uint32 {
	result, _ := StartNew[uint32]("riscv-simulator.EncodeAddi", 0,
		func(op *Operation[uint32], rf *ResultFactory[uint32]) *OperationResult[uint32] {
			return rf.Generate(true, false, encodeIType(rd, rs1, imm, Funct3ADDI, OpcodeOpImm))
		}).GetResult()
	return result
}

// EncodeSlti encodes: slti rd, rs1, imm  (set less than immediate, signed)
func EncodeSlti(rd, rs1, imm int) uint32 {
	result, _ := StartNew[uint32]("riscv-simulator.EncodeSlti", 0,
		func(op *Operation[uint32], rf *ResultFactory[uint32]) *OperationResult[uint32] {
			return rf.Generate(true, false, encodeIType(rd, rs1, imm, Funct3SLTI, OpcodeOpImm))
		}).GetResult()
	return result
}

// EncodeSltiu encodes: sltiu rd, rs1, imm  (set less than immediate, unsigned)
func EncodeSltiu(rd, rs1, imm int) uint32 {
	result, _ := StartNew[uint32]("riscv-simulator.EncodeSltiu", 0,
		func(op *Operation[uint32], rf *ResultFactory[uint32]) *OperationResult[uint32] {
			return rf.Generate(true, false, encodeIType(rd, rs1, imm, Funct3SLTIU, OpcodeOpImm))
		}).GetResult()
	return result
}

// EncodeXori encodes: xori rd, rs1, imm
func EncodeXori(rd, rs1, imm int) uint32 {
	result, _ := StartNew[uint32]("riscv-simulator.EncodeXori", 0,
		func(op *Operation[uint32], rf *ResultFactory[uint32]) *OperationResult[uint32] {
			return rf.Generate(true, false, encodeIType(rd, rs1, imm, Funct3XORI, OpcodeOpImm))
		}).GetResult()
	return result
}

// EncodeOri encodes: ori rd, rs1, imm
func EncodeOri(rd, rs1, imm int) uint32 {
	result, _ := StartNew[uint32]("riscv-simulator.EncodeOri", 0,
		func(op *Operation[uint32], rf *ResultFactory[uint32]) *OperationResult[uint32] {
			return rf.Generate(true, false, encodeIType(rd, rs1, imm, Funct3ORI, OpcodeOpImm))
		}).GetResult()
	return result
}

// EncodeAndi encodes: andi rd, rs1, imm
func EncodeAndi(rd, rs1, imm int) uint32 {
	result, _ := StartNew[uint32]("riscv-simulator.EncodeAndi", 0,
		func(op *Operation[uint32], rf *ResultFactory[uint32]) *OperationResult[uint32] {
			return rf.Generate(true, false, encodeIType(rd, rs1, imm, Funct3ANDI, OpcodeOpImm))
		}).GetResult()
	return result
}

// EncodeSlli encodes: slli rd, rs1, shamt  (shift left logical immediate)
// The shift amount occupies the lower 5 bits of the immediate field.
func EncodeSlli(rd, rs1, shamt int) uint32 {
	result, _ := StartNew[uint32]("riscv-simulator.EncodeSlli", 0,
		func(op *Operation[uint32], rf *ResultFactory[uint32]) *OperationResult[uint32] {
			return rf.Generate(true, false, uint32((Funct7Normal<<25)|((shamt&0x1F)<<20)|(rs1<<15)|(Funct3SLLI<<12)|(rd<<7)|OpcodeOpImm))
		}).GetResult()
	return result
}

// EncodeSrli encodes: srli rd, rs1, shamt  (shift right logical immediate)
func EncodeSrli(rd, rs1, shamt int) uint32 {
	result, _ := StartNew[uint32]("riscv-simulator.EncodeSrli", 0,
		func(op *Operation[uint32], rf *ResultFactory[uint32]) *OperationResult[uint32] {
			return rf.Generate(true, false, uint32((Funct7Normal<<25)|((shamt&0x1F)<<20)|(rs1<<15)|(Funct3SRLI<<12)|(rd<<7)|OpcodeOpImm))
		}).GetResult()
	return result
}

// EncodeSrai encodes: srai rd, rs1, shamt  (shift right arithmetic immediate)
// Distinguished from srli by funct7 = 0x20.
func EncodeSrai(rd, rs1, shamt int) uint32 {
	result, _ := StartNew[uint32]("riscv-simulator.EncodeSrai", 0,
		func(op *Operation[uint32], rf *ResultFactory[uint32]) *OperationResult[uint32] {
			return rf.Generate(true, false, uint32((Funct7Alt<<25)|((shamt&0x1F)<<20)|(rs1<<15)|(Funct3SRLI<<12)|(rd<<7)|OpcodeOpImm))
		}).GetResult()
	return result
}

// encodeIType is the shared helper for I-type instructions.
func encodeIType(rd, rs1, imm, funct3, opcode int) uint32 {
	return uint32(((imm & 0xFFF) << 20) | (rs1 << 15) | (funct3 << 12) | (rd << 7) | opcode)
}

// === R-type encoders ===
//
// R-type format: [funct7 | rs2 | rs1 | funct3 | rd | opcode]

// EncodeAdd encodes: add rd, rs1, rs2
func EncodeAdd(rd, rs1, rs2 int) uint32 {
	result, _ := StartNew[uint32]("riscv-simulator.EncodeAdd", 0,
		func(op *Operation[uint32], rf *ResultFactory[uint32]) *OperationResult[uint32] {
			return rf.Generate(true, false, encodeRType(rd, rs1, rs2, Funct3ADD, Funct7Normal))
		}).GetResult()
	return result
}

// EncodeSub encodes: sub rd, rs1, rs2
func EncodeSub(rd, rs1, rs2 int) uint32 {
	result, _ := StartNew[uint32]("riscv-simulator.EncodeSub", 0,
		func(op *Operation[uint32], rf *ResultFactory[uint32]) *OperationResult[uint32] {
			return rf.Generate(true, false, encodeRType(rd, rs1, rs2, Funct3ADD, Funct7Alt))
		}).GetResult()
	return result
}

// EncodeSll encodes: sll rd, rs1, rs2  (shift left logical)
func EncodeSll(rd, rs1, rs2 int) uint32 {
	result, _ := StartNew[uint32]("riscv-simulator.EncodeSll", 0,
		func(op *Operation[uint32], rf *ResultFactory[uint32]) *OperationResult[uint32] {
			return rf.Generate(true, false, encodeRType(rd, rs1, rs2, Funct3SLL, Funct7Normal))
		}).GetResult()
	return result
}

// EncodeSlt encodes: slt rd, rs1, rs2  (set less than, signed)
func EncodeSlt(rd, rs1, rs2 int) uint32 {
	result, _ := StartNew[uint32]("riscv-simulator.EncodeSlt", 0,
		func(op *Operation[uint32], rf *ResultFactory[uint32]) *OperationResult[uint32] {
			return rf.Generate(true, false, encodeRType(rd, rs1, rs2, Funct3SLT, Funct7Normal))
		}).GetResult()
	return result
}

// EncodeSltu encodes: sltu rd, rs1, rs2  (set less than, unsigned)
func EncodeSltu(rd, rs1, rs2 int) uint32 {
	result, _ := StartNew[uint32]("riscv-simulator.EncodeSltu", 0,
		func(op *Operation[uint32], rf *ResultFactory[uint32]) *OperationResult[uint32] {
			return rf.Generate(true, false, encodeRType(rd, rs1, rs2, Funct3SLTU, Funct7Normal))
		}).GetResult()
	return result
}

// EncodeXor encodes: xor rd, rs1, rs2
func EncodeXor(rd, rs1, rs2 int) uint32 {
	result, _ := StartNew[uint32]("riscv-simulator.EncodeXor", 0,
		func(op *Operation[uint32], rf *ResultFactory[uint32]) *OperationResult[uint32] {
			return rf.Generate(true, false, encodeRType(rd, rs1, rs2, Funct3XOR, Funct7Normal))
		}).GetResult()
	return result
}

// EncodeSrl encodes: srl rd, rs1, rs2  (shift right logical)
func EncodeSrl(rd, rs1, rs2 int) uint32 {
	result, _ := StartNew[uint32]("riscv-simulator.EncodeSrl", 0,
		func(op *Operation[uint32], rf *ResultFactory[uint32]) *OperationResult[uint32] {
			return rf.Generate(true, false, encodeRType(rd, rs1, rs2, Funct3SRL, Funct7Normal))
		}).GetResult()
	return result
}

// EncodeSra encodes: sra rd, rs1, rs2  (shift right arithmetic)
func EncodeSra(rd, rs1, rs2 int) uint32 {
	result, _ := StartNew[uint32]("riscv-simulator.EncodeSra", 0,
		func(op *Operation[uint32], rf *ResultFactory[uint32]) *OperationResult[uint32] {
			return rf.Generate(true, false, encodeRType(rd, rs1, rs2, Funct3SRL, Funct7Alt))
		}).GetResult()
	return result
}

// EncodeOr encodes: or rd, rs1, rs2
func EncodeOr(rd, rs1, rs2 int) uint32 {
	result, _ := StartNew[uint32]("riscv-simulator.EncodeOr", 0,
		func(op *Operation[uint32], rf *ResultFactory[uint32]) *OperationResult[uint32] {
			return rf.Generate(true, false, encodeRType(rd, rs1, rs2, Funct3OR, Funct7Normal))
		}).GetResult()
	return result
}

// EncodeAnd encodes: and rd, rs1, rs2
func EncodeAnd(rd, rs1, rs2 int) uint32 {
	result, _ := StartNew[uint32]("riscv-simulator.EncodeAnd", 0,
		func(op *Operation[uint32], rf *ResultFactory[uint32]) *OperationResult[uint32] {
			return rf.Generate(true, false, encodeRType(rd, rs1, rs2, Funct3AND, Funct7Normal))
		}).GetResult()
	return result
}

// encodeRType is the shared helper for R-type instructions.
func encodeRType(rd, rs1, rs2, funct3, funct7 int) uint32 {
	return uint32((funct7 << 25) | (rs2 << 20) | (rs1 << 15) | (funct3 << 12) | (rd << 7) | OpcodeOp)
}

// === Load encoders (I-type format) ===

// EncodeLb encodes: lb rd, imm(rs1)  (load byte, sign-extend)
func EncodeLb(rd, rs1, imm int) uint32 {
	result, _ := StartNew[uint32]("riscv-simulator.EncodeLb", 0,
		func(op *Operation[uint32], rf *ResultFactory[uint32]) *OperationResult[uint32] {
			return rf.Generate(true, false, encodeIType(rd, rs1, imm, Funct3LB, OpcodeLoad))
		}).GetResult()
	return result
}

// EncodeLh encodes: lh rd, imm(rs1)  (load halfword, sign-extend)
func EncodeLh(rd, rs1, imm int) uint32 {
	result, _ := StartNew[uint32]("riscv-simulator.EncodeLh", 0,
		func(op *Operation[uint32], rf *ResultFactory[uint32]) *OperationResult[uint32] {
			return rf.Generate(true, false, encodeIType(rd, rs1, imm, Funct3LH, OpcodeLoad))
		}).GetResult()
	return result
}

// EncodeLw encodes: lw rd, imm(rs1)  (load word)
func EncodeLw(rd, rs1, imm int) uint32 {
	result, _ := StartNew[uint32]("riscv-simulator.EncodeLw", 0,
		func(op *Operation[uint32], rf *ResultFactory[uint32]) *OperationResult[uint32] {
			return rf.Generate(true, false, encodeIType(rd, rs1, imm, Funct3LW, OpcodeLoad))
		}).GetResult()
	return result
}

// EncodeLbu encodes: lbu rd, imm(rs1)  (load byte, zero-extend)
func EncodeLbu(rd, rs1, imm int) uint32 {
	result, _ := StartNew[uint32]("riscv-simulator.EncodeLbu", 0,
		func(op *Operation[uint32], rf *ResultFactory[uint32]) *OperationResult[uint32] {
			return rf.Generate(true, false, encodeIType(rd, rs1, imm, Funct3LBU, OpcodeLoad))
		}).GetResult()
	return result
}

// EncodeLhu encodes: lhu rd, imm(rs1)  (load halfword, zero-extend)
func EncodeLhu(rd, rs1, imm int) uint32 {
	result, _ := StartNew[uint32]("riscv-simulator.EncodeLhu", 0,
		func(op *Operation[uint32], rf *ResultFactory[uint32]) *OperationResult[uint32] {
			return rf.Generate(true, false, encodeIType(rd, rs1, imm, Funct3LHU, OpcodeLoad))
		}).GetResult()
	return result
}

// === Store encoders (S-type format) ===
//
// S-type format: [imm[11:5] | rs2 | rs1 | funct3 | imm[4:0] | opcode]
//
// The immediate is split: lower 5 bits go to bits [11:7] of the instruction,
// upper 7 bits go to bits [31:25].

// EncodeSb encodes: sb rs2, imm(rs1)  (store byte)
func EncodeSb(rs2, rs1, imm int) uint32 {
	result, _ := StartNew[uint32]("riscv-simulator.EncodeSb", 0,
		func(op *Operation[uint32], rf *ResultFactory[uint32]) *OperationResult[uint32] {
			return rf.Generate(true, false, encodeSType(rs2, rs1, imm, Funct3SB))
		}).GetResult()
	return result
}

// EncodeSh encodes: sh rs2, imm(rs1)  (store halfword)
func EncodeSh(rs2, rs1, imm int) uint32 {
	result, _ := StartNew[uint32]("riscv-simulator.EncodeSh", 0,
		func(op *Operation[uint32], rf *ResultFactory[uint32]) *OperationResult[uint32] {
			return rf.Generate(true, false, encodeSType(rs2, rs1, imm, Funct3SH))
		}).GetResult()
	return result
}

// EncodeSw encodes: sw rs2, imm(rs1)  (store word)
func EncodeSw(rs2, rs1, imm int) uint32 {
	result, _ := StartNew[uint32]("riscv-simulator.EncodeSw", 0,
		func(op *Operation[uint32], rf *ResultFactory[uint32]) *OperationResult[uint32] {
			return rf.Generate(true, false, encodeSType(rs2, rs1, imm, Funct3SW))
		}).GetResult()
	return result
}

// encodeSType is the shared helper for S-type instructions.
func encodeSType(rs2, rs1, imm, funct3 int) uint32 {
	immVal := imm & 0xFFF
	immLow := immVal & 0x1F        // bits [4:0]
	immHigh := (immVal >> 5) & 0x7F // bits [11:5]
	return uint32((immHigh << 25) | (rs2 << 20) | (rs1 << 15) | (funct3 << 12) | (immLow << 7) | OpcodeStore)
}

// === Branch encoders (B-type format) ===
//
// B-type format: [imm[12|10:5] | rs2 | rs1 | funct3 | imm[4:1|11] | opcode]
//
// The immediate bits are scrambled for hardware efficiency:
//   bit 31 = imm[12], bits 30:25 = imm[10:5]
//   bits 11:8 = imm[4:1], bit 7 = imm[11]
//
// Note: imm[0] is always 0 (2-byte alignment) and is not encoded.

// EncodeBeq encodes: beq rs1, rs2, offset
func EncodeBeq(rs1, rs2, offset int) uint32 {
	result, _ := StartNew[uint32]("riscv-simulator.EncodeBeq", 0,
		func(op *Operation[uint32], rf *ResultFactory[uint32]) *OperationResult[uint32] {
			return rf.Generate(true, false, encodeBType(rs1, rs2, offset, Funct3BEQ))
		}).GetResult()
	return result
}

// EncodeBne encodes: bne rs1, rs2, offset
func EncodeBne(rs1, rs2, offset int) uint32 {
	result, _ := StartNew[uint32]("riscv-simulator.EncodeBne", 0,
		func(op *Operation[uint32], rf *ResultFactory[uint32]) *OperationResult[uint32] {
			return rf.Generate(true, false, encodeBType(rs1, rs2, offset, Funct3BNE))
		}).GetResult()
	return result
}

// EncodeBlt encodes: blt rs1, rs2, offset
func EncodeBlt(rs1, rs2, offset int) uint32 {
	result, _ := StartNew[uint32]("riscv-simulator.EncodeBlt", 0,
		func(op *Operation[uint32], rf *ResultFactory[uint32]) *OperationResult[uint32] {
			return rf.Generate(true, false, encodeBType(rs1, rs2, offset, Funct3BLT))
		}).GetResult()
	return result
}

// EncodeBge encodes: bge rs1, rs2, offset
func EncodeBge(rs1, rs2, offset int) uint32 {
	result, _ := StartNew[uint32]("riscv-simulator.EncodeBge", 0,
		func(op *Operation[uint32], rf *ResultFactory[uint32]) *OperationResult[uint32] {
			return rf.Generate(true, false, encodeBType(rs1, rs2, offset, Funct3BGE))
		}).GetResult()
	return result
}

// EncodeBltu encodes: bltu rs1, rs2, offset
func EncodeBltu(rs1, rs2, offset int) uint32 {
	result, _ := StartNew[uint32]("riscv-simulator.EncodeBltu", 0,
		func(op *Operation[uint32], rf *ResultFactory[uint32]) *OperationResult[uint32] {
			return rf.Generate(true, false, encodeBType(rs1, rs2, offset, Funct3BLTU))
		}).GetResult()
	return result
}

// EncodeBgeu encodes: bgeu rs1, rs2, offset
func EncodeBgeu(rs1, rs2, offset int) uint32 {
	result, _ := StartNew[uint32]("riscv-simulator.EncodeBgeu", 0,
		func(op *Operation[uint32], rf *ResultFactory[uint32]) *OperationResult[uint32] {
			return rf.Generate(true, false, encodeBType(rs1, rs2, offset, Funct3BGEU))
		}).GetResult()
	return result
}

// encodeBType is the shared helper for B-type instructions.
func encodeBType(rs1, rs2, offset, funct3 int) uint32 {
	imm := offset & 0x1FFE // mask to 13 bits, bit 0 forced to 0
	// Extract the scattered bits:
	bit12 := (imm >> 12) & 0x1
	bit11 := (imm >> 11) & 0x1
	bits10_5 := (imm >> 5) & 0x3F
	bits4_1 := (imm >> 1) & 0xF

	return uint32((bit12 << 31) | (bits10_5 << 25) | (rs2 << 20) | (rs1 << 15) | (funct3 << 12) | (bits4_1 << 8) | (bit11 << 7) | OpcodeBranch)
}

// === JAL encoder (J-type format) ===
//
// J-type format: [imm[20|10:1|11|19:12] | rd | opcode]
//
// The 21-bit immediate (bit 0 implicit 0) is scrambled across the instruction:
//   bit 31 = imm[20], bits 30:21 = imm[10:1]
//   bit 20 = imm[11], bits 19:12 = imm[19:12]

// EncodeJal encodes: jal rd, offset
func EncodeJal(rd, offset int) uint32 {
	result, _ := StartNew[uint32]("riscv-simulator.EncodeJal", 0,
		func(op *Operation[uint32], rf *ResultFactory[uint32]) *OperationResult[uint32] {
			imm := offset & 0x1FFFFE // 21 bits, bit 0 forced to 0
			bit20 := (imm >> 20) & 0x1
			bits10_1 := (imm >> 1) & 0x3FF
			bit11 := (imm >> 11) & 0x1
			bits19_12 := (imm >> 12) & 0xFF
			return rf.Generate(true, false, uint32((bit20<<31)|(bits10_1<<21)|(bit11<<20)|(bits19_12<<12)|(rd<<7)|OpcodeJAL))
		}).GetResult()
	return result
}

// === JALR encoder (I-type format) ===

// EncodeJalr encodes: jalr rd, rs1, imm
func EncodeJalr(rd, rs1, imm int) uint32 {
	result, _ := StartNew[uint32]("riscv-simulator.EncodeJalr", 0,
		func(op *Operation[uint32], rf *ResultFactory[uint32]) *OperationResult[uint32] {
			return rf.Generate(true, false, encodeIType(rd, rs1, imm, 0, OpcodeJALR))
		}).GetResult()
	return result
}

// === U-type encoders ===
//
// U-type format: [imm[31:12] | rd | opcode]
//
// The immediate occupies the upper 20 bits. We pass the raw 20-bit value;
// the hardware shifts it left by 12 during execution.

// EncodeLui encodes: lui rd, imm  (imm is the upper 20-bit value)
func EncodeLui(rd, imm int) uint32 {
	result, _ := StartNew[uint32]("riscv-simulator.EncodeLui", 0,
		func(op *Operation[uint32], rf *ResultFactory[uint32]) *OperationResult[uint32] {
			return rf.Generate(true, false, uint32(((imm&0xFFFFF)<<12)|(rd<<7)|OpcodeLUI))
		}).GetResult()
	return result
}

// EncodeAuipc encodes: auipc rd, imm  (imm is the upper 20-bit value)
func EncodeAuipc(rd, imm int) uint32 {
	result, _ := StartNew[uint32]("riscv-simulator.EncodeAuipc", 0,
		func(op *Operation[uint32], rf *ResultFactory[uint32]) *OperationResult[uint32] {
			return rf.Generate(true, false, uint32(((imm&0xFFFFF)<<12)|(rd<<7)|OpcodeAUIPC))
		}).GetResult()
	return result
}

// === System instruction encoders ===

// EncodeEcall encodes: ecall (environment call — trigger trap)
func EncodeEcall() uint32 {
	result, _ := StartNew[uint32]("riscv-simulator.EncodeEcall", 0,
		func(op *Operation[uint32], rf *ResultFactory[uint32]) *OperationResult[uint32] {
			return rf.Generate(true, false, uint32(OpcodeSystem))
		}).GetResult()
	return result
}

// EncodeMret encodes: mret (return from machine-mode trap)
// mret has funct7=0x18 (0b0011000), rs2=0b00010, rs1=0, funct3=0, rd=0
func EncodeMret() uint32 {
	result, _ := StartNew[uint32]("riscv-simulator.EncodeMret", 0,
		func(op *Operation[uint32], rf *ResultFactory[uint32]) *OperationResult[uint32] {
			return rf.Generate(true, false, uint32((Funct7MRET<<25)|(0b00010<<20)|OpcodeSystem))
		}).GetResult()
	return result
}

// EncodeCsrrw encodes: csrrw rd, csr, rs1  (CSR read-write)
func EncodeCsrrw(rd, csr, rs1 int) uint32 {
	result, _ := StartNew[uint32]("riscv-simulator.EncodeCsrrw", 0,
		func(op *Operation[uint32], rf *ResultFactory[uint32]) *OperationResult[uint32] {
			return rf.Generate(true, false, encodeCSR(rd, csr, rs1, Funct3CSRRW))
		}).GetResult()
	return result
}

// EncodeCsrrs encodes: csrrs rd, csr, rs1  (CSR read-set)
func EncodeCsrrs(rd, csr, rs1 int) uint32 {
	result, _ := StartNew[uint32]("riscv-simulator.EncodeCsrrs", 0,
		func(op *Operation[uint32], rf *ResultFactory[uint32]) *OperationResult[uint32] {
			return rf.Generate(true, false, encodeCSR(rd, csr, rs1, Funct3CSRRS))
		}).GetResult()
	return result
}

// EncodeCsrrc encodes: csrrc rd, csr, rs1  (CSR read-clear)
func EncodeCsrrc(rd, csr, rs1 int) uint32 {
	result, _ := StartNew[uint32]("riscv-simulator.EncodeCsrrc", 0,
		func(op *Operation[uint32], rf *ResultFactory[uint32]) *OperationResult[uint32] {
			return rf.Generate(true, false, encodeCSR(rd, csr, rs1, Funct3CSRRC))
		}).GetResult()
	return result
}

// encodeCSR is the shared helper for CSR instructions (I-type format).
func encodeCSR(rd, csr, rs1, funct3 int) uint32 {
	return uint32(((csr & 0xFFF) << 20) | (rs1 << 15) | (funct3 << 12) | (rd << 7) | OpcodeSystem)
}

// Assemble converts a slice of 32-bit instruction words into a byte slice
// in little-endian order, ready to be loaded into simulated memory.
//
// RISC-V uses little-endian byte ordering for instructions:
//   instruction 0x12345678 becomes bytes [0x78, 0x56, 0x34, 0x12]
func Assemble(instructions []uint32) []byte {
	result, _ := StartNew[[]byte]("riscv-simulator.Assemble", nil,
		func(op *Operation[[]byte], rf *ResultFactory[[]byte]) *OperationResult[[]byte] {
			out := make([]byte, 0, len(instructions)*4)
			for _, inst := range instructions {
				out = append(out,
					byte(inst&0xFF),
					byte((inst>>8)&0xFF),
					byte((inst>>16)&0xFF),
					byte((inst>>24)&0xFF),
				)
			}
			return rf.Generate(true, false, out)
		}).GetResult()
	return result
}
