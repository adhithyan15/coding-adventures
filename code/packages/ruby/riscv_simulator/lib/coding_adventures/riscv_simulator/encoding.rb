# frozen_string_literal: true

# === Encoding helpers for constructing machine code in tests ===

module CodingAdventures
  module RiscvSimulator
    # I-type helper
    def self.encode_i_type(rd, rs1, imm, funct3, opcode)
      ((imm & 0xFFF) << 20) | (rs1 << 15) | (funct3 << 12) | (rd << 7) | opcode
    end

    # I-type arithmetic encoders
    def self.encode_addi(rd, rs1, imm)  = encode_i_type(rd, rs1, imm, FUNCT3_ADDI, OPCODE_OP_IMM)
    def self.encode_slti(rd, rs1, imm)  = encode_i_type(rd, rs1, imm, FUNCT3_SLTI, OPCODE_OP_IMM)
    def self.encode_sltiu(rd, rs1, imm) = encode_i_type(rd, rs1, imm, FUNCT3_SLTIU, OPCODE_OP_IMM)
    def self.encode_xori(rd, rs1, imm)  = encode_i_type(rd, rs1, imm, FUNCT3_XORI, OPCODE_OP_IMM)
    def self.encode_ori(rd, rs1, imm)   = encode_i_type(rd, rs1, imm, FUNCT3_ORI, OPCODE_OP_IMM)
    def self.encode_andi(rd, rs1, imm)  = encode_i_type(rd, rs1, imm, FUNCT3_ANDI, OPCODE_OP_IMM)

    def self.encode_slli(rd, rs1, shamt)
      (FUNCT7_NORMAL << 25) | ((shamt & 0x1F) << 20) | (rs1 << 15) | (FUNCT3_SLLI << 12) | (rd << 7) | OPCODE_OP_IMM
    end

    def self.encode_srli(rd, rs1, shamt)
      (FUNCT7_NORMAL << 25) | ((shamt & 0x1F) << 20) | (rs1 << 15) | (FUNCT3_SRLI << 12) | (rd << 7) | OPCODE_OP_IMM
    end

    def self.encode_srai(rd, rs1, shamt)
      (FUNCT7_ALT << 25) | ((shamt & 0x1F) << 20) | (rs1 << 15) | (FUNCT3_SRLI << 12) | (rd << 7) | OPCODE_OP_IMM
    end

    # R-type helper
    def self.encode_r_type(rd, rs1, rs2, funct3, funct7)
      (funct7 << 25) | (rs2 << 20) | (rs1 << 15) | (funct3 << 12) | (rd << 7) | OPCODE_OP
    end

    def self.encode_add(rd, rs1, rs2)  = encode_r_type(rd, rs1, rs2, FUNCT3_ADD, FUNCT7_NORMAL)
    def self.encode_sub(rd, rs1, rs2)  = encode_r_type(rd, rs1, rs2, FUNCT3_ADD, FUNCT7_ALT)
    def self.encode_sll(rd, rs1, rs2)  = encode_r_type(rd, rs1, rs2, FUNCT3_SLL, FUNCT7_NORMAL)
    def self.encode_slt(rd, rs1, rs2)  = encode_r_type(rd, rs1, rs2, FUNCT3_SLT, FUNCT7_NORMAL)
    def self.encode_sltu(rd, rs1, rs2) = encode_r_type(rd, rs1, rs2, FUNCT3_SLTU, FUNCT7_NORMAL)
    def self.encode_xor(rd, rs1, rs2)  = encode_r_type(rd, rs1, rs2, FUNCT3_XOR, FUNCT7_NORMAL)
    def self.encode_srl(rd, rs1, rs2)  = encode_r_type(rd, rs1, rs2, FUNCT3_SRL, FUNCT7_NORMAL)
    def self.encode_sra(rd, rs1, rs2)  = encode_r_type(rd, rs1, rs2, FUNCT3_SRL, FUNCT7_ALT)
    def self.encode_or(rd, rs1, rs2)   = encode_r_type(rd, rs1, rs2, FUNCT3_OR, FUNCT7_NORMAL)
    def self.encode_and(rd, rs1, rs2)  = encode_r_type(rd, rs1, rs2, FUNCT3_AND, FUNCT7_NORMAL)

    # Load encoders (I-type)
    def self.encode_lb(rd, rs1, imm)  = encode_i_type(rd, rs1, imm, FUNCT3_LB, OPCODE_LOAD)
    def self.encode_lh(rd, rs1, imm)  = encode_i_type(rd, rs1, imm, FUNCT3_LH, OPCODE_LOAD)
    def self.encode_lw(rd, rs1, imm)  = encode_i_type(rd, rs1, imm, FUNCT3_LW, OPCODE_LOAD)
    def self.encode_lbu(rd, rs1, imm) = encode_i_type(rd, rs1, imm, FUNCT3_LBU, OPCODE_LOAD)
    def self.encode_lhu(rd, rs1, imm) = encode_i_type(rd, rs1, imm, FUNCT3_LHU, OPCODE_LOAD)

    # Store encoders (S-type)
    def self.encode_s_type(rs2, rs1, imm, funct3)
      imm_val = imm & 0xFFF
      imm_low = imm_val & 0x1F
      imm_high = (imm_val >> 5) & 0x7F
      (imm_high << 25) | (rs2 << 20) | (rs1 << 15) | (funct3 << 12) | (imm_low << 7) | OPCODE_STORE
    end

    def self.encode_sb(rs2, rs1, imm) = encode_s_type(rs2, rs1, imm, FUNCT3_SB)
    def self.encode_sh(rs2, rs1, imm) = encode_s_type(rs2, rs1, imm, FUNCT3_SH)
    def self.encode_sw(rs2, rs1, imm) = encode_s_type(rs2, rs1, imm, FUNCT3_SW)

    # Branch encoders (B-type)
    def self.encode_b_type(rs1, rs2, offset, funct3)
      imm = offset & 0x1FFE
      bit12 = (imm >> 12) & 0x1
      bit11 = (imm >> 11) & 0x1
      bits10_5 = (imm >> 5) & 0x3F
      bits4_1 = (imm >> 1) & 0xF
      (bit12 << 31) | (bits10_5 << 25) | (rs2 << 20) | (rs1 << 15) | (funct3 << 12) | (bits4_1 << 8) | (bit11 << 7) | OPCODE_BRANCH
    end

    def self.encode_beq(rs1, rs2, offset)  = encode_b_type(rs1, rs2, offset, FUNCT3_BEQ)
    def self.encode_bne(rs1, rs2, offset)  = encode_b_type(rs1, rs2, offset, FUNCT3_BNE)
    def self.encode_blt(rs1, rs2, offset)  = encode_b_type(rs1, rs2, offset, FUNCT3_BLT)
    def self.encode_bge(rs1, rs2, offset)  = encode_b_type(rs1, rs2, offset, FUNCT3_BGE)
    def self.encode_bltu(rs1, rs2, offset) = encode_b_type(rs1, rs2, offset, FUNCT3_BLTU)
    def self.encode_bgeu(rs1, rs2, offset) = encode_b_type(rs1, rs2, offset, FUNCT3_BGEU)

    # JAL encoder (J-type)
    def self.encode_jal(rd, offset)
      imm = offset & 0x1FFFFE
      bit20 = (imm >> 20) & 0x1
      bits10_1 = (imm >> 1) & 0x3FF
      bit11 = (imm >> 11) & 0x1
      bits19_12 = (imm >> 12) & 0xFF
      (bit20 << 31) | (bits10_1 << 21) | (bit11 << 20) | (bits19_12 << 12) | (rd << 7) | OPCODE_JAL
    end

    # JALR encoder (I-type)
    def self.encode_jalr(rd, rs1, imm) = encode_i_type(rd, rs1, imm, 0, OPCODE_JALR)

    # U-type encoders
    def self.encode_lui(rd, imm)   = ((imm & 0xFFFFF) << 12) | (rd << 7) | OPCODE_LUI
    def self.encode_auipc(rd, imm) = ((imm & 0xFFFFF) << 12) | (rd << 7) | OPCODE_AUIPC

    # System instruction encoders
    def self.encode_ecall = OPCODE_SYSTEM

    def self.encode_mret
      (FUNCT7_MRET << 25) | (0b00010 << 20) | OPCODE_SYSTEM
    end

    def self.encode_csr(rd, csr, rs1, funct3)
      ((csr & 0xFFF) << 20) | (rs1 << 15) | (funct3 << 12) | (rd << 7) | OPCODE_SYSTEM
    end

    def self.encode_csrrw(rd, csr, rs1) = encode_csr(rd, csr, rs1, FUNCT3_CSRRW)
    def self.encode_csrrs(rd, csr, rs1) = encode_csr(rd, csr, rs1, FUNCT3_CSRRS)
    def self.encode_csrrc(rd, csr, rs1) = encode_csr(rd, csr, rs1, FUNCT3_CSRRC)

    # Assemble
    def self.assemble(instructions)
      instructions.map { |i| [i & 0xFFFFFFFF].pack("V") }.join.b
    end
  end
end
