# frozen_string_literal: true

# === Instruction decoder for all RV32I formats ===
#
# The decoder takes a raw 32-bit instruction and breaks it into meaningful
# fields. Step 1: read bits [6:0] for opcode. Step 2: dispatch to the
# appropriate format decoder. Step 3: extract registers, immediates, etc.

module CodingAdventures
  module RiscvSimulator
    class RiscVDecoder
      def decode(raw, pc)
        opcode = raw & 0x7F

        case opcode
        when OPCODE_OP_IMM  then decode_op_imm(raw)
        when OPCODE_OP      then decode_r_type(raw)
        when OPCODE_LOAD    then decode_load(raw)
        when OPCODE_STORE   then decode_s_type(raw)
        when OPCODE_BRANCH  then decode_b_type(raw)
        when OPCODE_JAL     then decode_j_type(raw, pc)
        when OPCODE_JALR    then decode_jalr(raw)
        when OPCODE_LUI     then decode_u_type(raw, "lui")
        when OPCODE_AUIPC   then decode_u_type(raw, "auipc")
        when OPCODE_SYSTEM  then decode_system(raw)
        else
          CpuSimulator::DecodeResult.new(
            mnemonic: "UNKNOWN(0x#{format("%02x", opcode)})",
            fields: {opcode: opcode}, raw_instruction: raw
          )
        end
      end

      private

      def decode_op_imm(raw)
        rd     = (raw >> 7) & 0x1F
        funct3 = (raw >> 12) & 0x7
        rs1    = (raw >> 15) & 0x1F
        imm    = (raw >> 20) & 0xFFF
        imm -= 0x1000 if imm & 0x800 != 0

        mnemonic = case funct3
        when FUNCT3_ADDI  then "addi"
        when FUNCT3_SLTI  then "slti"
        when FUNCT3_SLTIU then "sltiu"
        when FUNCT3_XORI  then "xori"
        when FUNCT3_ORI   then "ori"
        when FUNCT3_ANDI  then "andi"
        when FUNCT3_SLLI
          imm = imm & 0x1F
          "slli"
        when FUNCT3_SRLI
          funct7 = (raw >> 25) & 0x7F
          imm = imm & 0x1F
          funct7 == FUNCT7_ALT ? "srai" : "srli"
        else
          "opimm(f3=#{funct3})"
        end

        CpuSimulator::DecodeResult.new(
          mnemonic: mnemonic,
          fields: {rd: rd, rs1: rs1, imm: imm, funct3: funct3},
          raw_instruction: raw
        )
      end

      def decode_r_type(raw)
        rd     = (raw >> 7) & 0x1F
        funct3 = (raw >> 12) & 0x7
        rs1    = (raw >> 15) & 0x1F
        rs2    = (raw >> 20) & 0x1F
        funct7 = (raw >> 25) & 0x7F

        mnemonic = case [funct3, funct7]
        when [FUNCT3_ADD, FUNCT7_NORMAL] then "add"
        when [FUNCT3_ADD, FUNCT7_ALT]    then "sub"
        when [FUNCT3_SLL, FUNCT7_NORMAL] then "sll"
        when [FUNCT3_SLT, FUNCT7_NORMAL] then "slt"
        when [FUNCT3_SLTU, FUNCT7_NORMAL] then "sltu"
        when [FUNCT3_XOR, FUNCT7_NORMAL] then "xor"
        when [FUNCT3_SRL, FUNCT7_NORMAL] then "srl"
        when [FUNCT3_SRL, FUNCT7_ALT]    then "sra"
        when [FUNCT3_OR, FUNCT7_NORMAL]  then "or"
        when [FUNCT3_AND, FUNCT7_NORMAL] then "and"
        else "r_op(f3=#{funct3},f7=#{funct7})"
        end

        CpuSimulator::DecodeResult.new(
          mnemonic: mnemonic,
          fields: {rd: rd, rs1: rs1, rs2: rs2, funct3: funct3, funct7: funct7},
          raw_instruction: raw
        )
      end

      def decode_load(raw)
        rd     = (raw >> 7) & 0x1F
        funct3 = (raw >> 12) & 0x7
        rs1    = (raw >> 15) & 0x1F
        imm    = (raw >> 20) & 0xFFF
        imm -= 0x1000 if imm & 0x800 != 0

        mnemonic = case funct3
        when FUNCT3_LB  then "lb"
        when FUNCT3_LH  then "lh"
        when FUNCT3_LW  then "lw"
        when FUNCT3_LBU then "lbu"
        when FUNCT3_LHU then "lhu"
        else "load(f3=#{funct3})"
        end

        CpuSimulator::DecodeResult.new(
          mnemonic: mnemonic,
          fields: {rd: rd, rs1: rs1, imm: imm, funct3: funct3},
          raw_instruction: raw
        )
      end

      def decode_s_type(raw)
        funct3  = (raw >> 12) & 0x7
        rs1     = (raw >> 15) & 0x1F
        rs2     = (raw >> 20) & 0x1F
        imm_low  = (raw >> 7) & 0x1F
        imm_high = (raw >> 25) & 0x7F
        imm = (imm_high << 5) | imm_low
        imm -= 0x1000 if imm & 0x800 != 0

        mnemonic = case funct3
        when FUNCT3_SB then "sb"
        when FUNCT3_SH then "sh"
        when FUNCT3_SW then "sw"
        else "store(f3=#{funct3})"
        end

        CpuSimulator::DecodeResult.new(
          mnemonic: mnemonic,
          fields: {rs1: rs1, rs2: rs2, imm: imm, funct3: funct3},
          raw_instruction: raw
        )
      end

      def decode_b_type(raw)
        funct3  = (raw >> 12) & 0x7
        rs1     = (raw >> 15) & 0x1F
        rs2     = (raw >> 20) & 0x1F
        imm12   = (raw >> 31) & 0x1
        imm11   = (raw >> 7) & 0x1
        imm10_5 = (raw >> 25) & 0x3F
        imm4_1  = (raw >> 8) & 0xF
        imm = (imm12 << 12) | (imm11 << 11) | (imm10_5 << 5) | (imm4_1 << 1)
        imm -= 0x2000 if imm & 0x1000 != 0

        mnemonic = case funct3
        when FUNCT3_BEQ  then "beq"
        when FUNCT3_BNE  then "bne"
        when FUNCT3_BLT  then "blt"
        when FUNCT3_BGE  then "bge"
        when FUNCT3_BLTU then "bltu"
        when FUNCT3_BGEU then "bgeu"
        else "branch(f3=#{funct3})"
        end

        CpuSimulator::DecodeResult.new(
          mnemonic: mnemonic,
          fields: {rs1: rs1, rs2: rs2, imm: imm, funct3: funct3},
          raw_instruction: raw
        )
      end

      def decode_j_type(raw, _pc)
        rd       = (raw >> 7) & 0x1F
        imm20    = (raw >> 31) & 0x1
        imm10_1  = (raw >> 21) & 0x3FF
        imm11    = (raw >> 20) & 0x1
        imm19_12 = (raw >> 12) & 0xFF
        imm = (imm20 << 20) | (imm19_12 << 12) | (imm11 << 11) | (imm10_1 << 1)
        imm -= 0x200000 if imm & 0x100000 != 0

        CpuSimulator::DecodeResult.new(
          mnemonic: "jal",
          fields: {rd: rd, imm: imm},
          raw_instruction: raw
        )
      end

      def decode_jalr(raw)
        rd  = (raw >> 7) & 0x1F
        rs1 = (raw >> 15) & 0x1F
        imm = (raw >> 20) & 0xFFF
        imm -= 0x1000 if imm & 0x800 != 0

        CpuSimulator::DecodeResult.new(
          mnemonic: "jalr",
          fields: {rd: rd, rs1: rs1, imm: imm},
          raw_instruction: raw
        )
      end

      def decode_u_type(raw, mnemonic)
        rd = (raw >> 7) & 0x1F
        imm = raw >> 12
        imm -= 0x100000 if imm & 0x80000 != 0

        CpuSimulator::DecodeResult.new(
          mnemonic: mnemonic,
          fields: {rd: rd, imm: imm},
          raw_instruction: raw
        )
      end

      def decode_system(raw)
        funct3 = (raw >> 12) & 0x7

        if funct3 == FUNCT3_PRIV
          funct7 = (raw >> 25) & 0x7F
          if funct7 == FUNCT7_MRET
            return CpuSimulator::DecodeResult.new(
              mnemonic: "mret", fields: {funct7: funct7}, raw_instruction: raw
            )
          end
          return CpuSimulator::DecodeResult.new(
            mnemonic: "ecall", fields: {funct7: funct7}, raw_instruction: raw
          )
        end

        rd  = (raw >> 7) & 0x1F
        rs1 = (raw >> 15) & 0x1F
        csr = (raw >> 20) & 0xFFF

        mnemonic = case funct3
        when FUNCT3_CSRRW then "csrrw"
        when FUNCT3_CSRRS then "csrrs"
        when FUNCT3_CSRRC then "csrrc"
        else "system(f3=#{funct3})"
        end

        CpuSimulator::DecodeResult.new(
          mnemonic: mnemonic,
          fields: {rd: rd, rs1: rs1, csr: csr, funct3: funct3},
          raw_instruction: raw
        )
      end
    end
  end
end
