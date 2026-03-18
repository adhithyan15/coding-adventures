# frozen_string_literal: true

# === RISC-V RV32I Simulator — a clean, modern instruction set ===
#
# RISC-V (pronounced "risk-five") is an open-source instruction set architecture
# designed at UC Berkeley by Patterson and Hennessy in 2010. It was designed from
# scratch with no historical baggage, making it the cleanest ISA to learn.
#
# "RISC" stands for Reduced Instruction Set Computer — the philosophy that a
# CPU should have a small number of simple instructions rather than many complex
# ones. Each instruction does one thing well.
#
# === Register conventions ===
#
# RISC-V has 32 registers, each 32 bits wide:
#   x0  = always 0 (hardwired — writes are ignored)
#   x1  = ra (return address)
#   x2  = sp (stack pointer)
#   x10-x17 = a0-a7 (function arguments and return values)
#
# The x0 register is special: it's always 0. This simplifies many operations.
# To load an immediate: addi x1, x0, 42 -> x1 = 0 + 42 = 42
#
# === Instruction encoding ===
#
# Every RISC-V instruction is exactly 32 bits. The opcode is in bits [6:0].
#
# R-type (register-register):
#   [funct7(7) | rs2(5) | rs1(5) | funct3(3) | rd(5) | opcode(7)]
#
# I-type (immediate):
#   [imm[11:0](12) | rs1(5) | funct3(3) | rd(5) | opcode(7)]

require "coding_adventures_cpu_simulator"

module CodingAdventures
  module RiscvSimulator
    # Opcode constants — bits [6:0] of the 32-bit instruction
    OPCODE_OP_IMM = 0b0010011 # I-type arithmetic (addi)
    OPCODE_OP     = 0b0110011 # R-type arithmetic (add, sub)
    OPCODE_SYSTEM = 0b1110011 # System instructions (ecall)

    # -------------------------------------------------------------------
    # Decoder
    # -------------------------------------------------------------------
    class RiscVDecoder
      def decode(raw, pc)
        opcode = raw & 0x7F

        case opcode
        when OPCODE_OP_IMM then decode_i_type(raw, "addi")
        when OPCODE_OP     then decode_r_type(raw)
        when OPCODE_SYSTEM
          CpuSimulator::DecodeResult.new(
            mnemonic: "ecall", fields: {}, raw_instruction: raw
          )
        else
          CpuSimulator::DecodeResult.new(
            mnemonic: "UNKNOWN(0x#{format("%02x", opcode)})",
            fields: {opcode: opcode}, raw_instruction: raw
          )
        end
      end

      private

      def decode_r_type(raw)
        rd     = (raw >> 7) & 0x1F
        funct3 = (raw >> 12) & 0x7
        rs1    = (raw >> 15) & 0x1F
        rs2    = (raw >> 20) & 0x1F
        funct7 = (raw >> 25) & 0x7F

        mnemonic = if funct3 == 0 && funct7 == 0
          "add"
        elsif funct3 == 0 && funct7 == 0x20
          "sub"
        else
          "r_op(f3=#{funct3},f7=#{funct7})"
        end

        CpuSimulator::DecodeResult.new(
          mnemonic: mnemonic,
          fields: {rd: rd, rs1: rs1, rs2: rs2, funct3: funct3, funct7: funct7},
          raw_instruction: raw
        )
      end

      def decode_i_type(raw, default_mnemonic)
        rd     = (raw >> 7) & 0x1F
        funct3 = (raw >> 12) & 0x7
        rs1    = (raw >> 15) & 0x1F
        imm    = (raw >> 20) & 0xFFF

        # Sign-extend 12-bit immediate
        imm -= 0x1000 if imm & 0x800 != 0

        CpuSimulator::DecodeResult.new(
          mnemonic: default_mnemonic,
          fields: {rd: rd, rs1: rs1, imm: imm, funct3: funct3},
          raw_instruction: raw
        )
      end
    end

    # -------------------------------------------------------------------
    # Executor
    # -------------------------------------------------------------------
    class RiscVExecutor
      def execute(decoded, registers, memory, pc)
        case decoded.mnemonic
        when "addi" then exec_addi(decoded, registers, pc)
        when "add"  then exec_add(decoded, registers, pc)
        when "sub"  then exec_sub(decoded, registers, pc)
        when "ecall"
          CpuSimulator::ExecuteResult.new(
            description: "System call (halt)",
            registers_changed: {}, memory_changed: {},
            next_pc: pc, halted: true
          )
        else
          CpuSimulator::ExecuteResult.new(
            description: "Unknown instruction: #{decoded.mnemonic}",
            registers_changed: {}, memory_changed: {},
            next_pc: pc + 4
          )
        end
      end

      private

      def exec_addi(decoded, registers, pc)
        rd = decoded.fields[:rd]
        rs1 = decoded.fields[:rs1]
        imm = decoded.fields[:imm]
        rs1_val = registers.read(rs1)
        result = (rs1_val + imm) & 0xFFFFFFFF

        changes = {}
        if rd != 0
          registers.write(rd, result)
          changes["x#{rd}"] = result
        end

        CpuSimulator::ExecuteResult.new(
          description: "x#{rd} = x#{rs1}(#{rs1_val}) + #{imm} = #{result}",
          registers_changed: changes, memory_changed: {},
          next_pc: pc + 4
        )
      end

      def exec_add(decoded, registers, pc)
        rd = decoded.fields[:rd]
        rs1 = decoded.fields[:rs1]
        rs2 = decoded.fields[:rs2]
        rs1_val = registers.read(rs1)
        rs2_val = registers.read(rs2)
        result = (rs1_val + rs2_val) & 0xFFFFFFFF

        changes = {}
        if rd != 0
          registers.write(rd, result)
          changes["x#{rd}"] = result
        end

        CpuSimulator::ExecuteResult.new(
          description: "x#{rd} = x#{rs1}(#{rs1_val}) + x#{rs2}(#{rs2_val}) = #{result}",
          registers_changed: changes, memory_changed: {},
          next_pc: pc + 4
        )
      end

      def exec_sub(decoded, registers, pc)
        rd = decoded.fields[:rd]
        rs1 = decoded.fields[:rs1]
        rs2 = decoded.fields[:rs2]
        rs1_val = registers.read(rs1)
        rs2_val = registers.read(rs2)
        result = (rs1_val - rs2_val) & 0xFFFFFFFF

        changes = {}
        if rd != 0
          registers.write(rd, result)
          changes["x#{rd}"] = result
        end

        CpuSimulator::ExecuteResult.new(
          description: "x#{rd} = x#{rs1}(#{rs1_val}) - x#{rs2}(#{rs2_val}) = #{result}",
          registers_changed: changes, memory_changed: {},
          next_pc: pc + 4
        )
      end
    end

    # -------------------------------------------------------------------
    # Assembler helpers
    # -------------------------------------------------------------------

    def self.encode_addi(rd, rs1, imm)
      imm_bits = imm & 0xFFF
      (imm_bits << 20) | (rs1 << 15) | (0 << 12) | (rd << 7) | OPCODE_OP_IMM
    end

    def self.encode_add(rd, rs1, rs2)
      (0 << 25) | (rs2 << 20) | (rs1 << 15) | (0 << 12) | (rd << 7) | OPCODE_OP
    end

    def self.encode_sub(rd, rs1, rs2)
      (0x20 << 25) | (rs2 << 20) | (rs1 << 15) | (0 << 12) | (rd << 7) | OPCODE_OP
    end

    def self.encode_ecall
      OPCODE_SYSTEM
    end

    def self.assemble(instructions)
      instructions.map { |i| [i & 0xFFFFFFFF].pack("V") }.join.b
    end

    # -------------------------------------------------------------------
    # High-level simulator
    # -------------------------------------------------------------------
    class RiscVSimulator
      attr_reader :cpu

      def initialize(memory_size: 65536)
        @decoder = RiscVDecoder.new
        @executor = RiscVExecutor.new
        @cpu = CpuSimulator::CPU.new(
          decoder: @decoder, executor: @executor,
          num_registers: 32, bit_width: 32, memory_size: memory_size
        )
      end

      def run(program)
        @cpu.load_program(program)
        @cpu.run
      end

      def step
        @cpu.step
      end
    end
  end
end
