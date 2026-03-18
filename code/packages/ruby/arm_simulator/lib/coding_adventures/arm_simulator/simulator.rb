# frozen_string_literal: true

# === ARM Simulator — the architecture that powers your phone ===
#
# ARM (originally Acorn RISC Machine) was designed in 1985 by Sophie Wilson
# and Steve Furber at Acorn Computers in Cambridge, England. It was one of
# the first commercial RISC processors, inspired by the Berkeley RISC project.
#
# ARM's big insight was power efficiency. While Intel focused on raw speed,
# ARM optimized for low power consumption. This bet paid off spectacularly:
# today ARM processors are in virtually every smartphone, tablet, and
# embedded device. Apple's M-series chips are ARM.
#
# === ARM vs RISC-V ===
#
#   ARM:      16 registers (R0-R15). Condition codes on every instruction.
#             Complex encoding. Commercial (licensed by ARM Ltd).
#
#   RISC-V:   32 registers (x0-x31). No condition codes. Clean, regular
#             encoding. Open-source. The "clean slate" ISA.
#
# === Register conventions ===
#
# ARM has 16 registers, each 32 bits wide:
#   R0-R3   = function arguments and return values
#   R4-R11  = general purpose (callee-saved)
#   R12     = IP (intra-procedure scratch register)
#   R13     = SP (stack pointer)
#   R14     = LR (link register -- return address)
#   R15     = PC (program counter -- yes, it's a visible register!)
#
# === Instruction encoding ===
#
# Every ARM instruction is exactly 32 bits. The condition code is ALWAYS
# in bits [31:28] -- this enables conditional execution on every instruction.
#
# Data processing format:
#   [cond(4) | 00 | I(1) | opcode(4) | S(1) | Rn(4) | Rd(4) | operand2(12)]
#    31   28  27 26  25    24     21   20     19  16   15  12   11         0
#
# === MVP instruction set ===
#
#   MOV R0, #1      -> R0 = 1        (I=1, opcode=MOV)
#   ADD R2, R0, R1  -> R2 = R0 + R1  (I=0, opcode=ADD)
#   SUB R2, R0, R1  -> R2 = R0 - R1  (I=0, opcode=SUB)
#   HLT             -> halt           (custom encoding: 0xFFFFFFFF)

require "coding_adventures_cpu_simulator"

module CodingAdventures
  module ArmSimulator
    # Condition code: Always execute (the most common)
    COND_AL = 0b1110

    # ARM data processing opcodes (bits [24:21])
    OPCODE_MOV = 0b1101 # MOV Rd, operand2 (ignores Rn)
    OPCODE_ADD = 0b0100 # ADD Rd, Rn, operand2
    OPCODE_SUB = 0b0010 # SUB Rd, Rn, operand2

    # Our custom halt sentinel
    HLT_INSTRUCTION = 0xFFFFFFFF

    # -------------------------------------------------------------------
    # Decoder — extracts fields from 32-bit ARM instruction words
    # -------------------------------------------------------------------
    class ARMDecoder
      # Decode a 32-bit ARM instruction into a DecodeResult.
      # The decoder extracts the condition code, opcode, register numbers,
      # and immediate values from the raw instruction bits.
      def decode(raw, pc)
        if raw == HLT_INSTRUCTION
          return CpuSimulator::DecodeResult.new(
            mnemonic: "hlt", fields: {}, raw_instruction: raw
          )
        end

        decode_data_processing(raw)
      end

      private

      # Decode an ARM data processing instruction.
      # Handles both immediate (I=1) and register (I=0) forms.
      def decode_data_processing(raw)
        cond = (raw >> 28) & 0xF
        i_bit = (raw >> 25) & 0x1
        opcode = (raw >> 21) & 0xF
        s_bit = (raw >> 20) & 0x1
        rn = (raw >> 16) & 0xF
        rd = (raw >> 12) & 0xF
        operand2 = raw & 0xFFF

        mnemonic = case opcode
        when OPCODE_MOV then "mov"
        when OPCODE_ADD then "add"
        when OPCODE_SUB then "sub"
        else "dp_op(#{format("%#06b", opcode)})"
        end

        if i_bit == 1
          # Immediate: operand2 = [rotate(4) | imm8(8)]
          rotate = (operand2 >> 8) & 0xF
          imm8 = operand2 & 0xFF
          shift = rotate * 2
          imm_value = if shift > 0
            ((imm8 >> shift) | (imm8 << (32 - shift))) & 0xFFFFFFFF
          else
            imm8
          end

          fields = {cond: cond, i_bit: i_bit, opcode: opcode, s_bit: s_bit,
                    rn: rn, rd: rd, imm: imm_value}
        else
          # Register: lowest 4 bits of operand2 are Rm
          rm = operand2 & 0xF
          fields = {cond: cond, i_bit: i_bit, opcode: opcode, s_bit: s_bit,
                    rn: rn, rd: rd, rm: rm}
        end

        CpuSimulator::DecodeResult.new(
          mnemonic: mnemonic, fields: fields, raw_instruction: raw
        )
      end
    end

    # -------------------------------------------------------------------
    # Executor — performs decoded ARM instructions
    # -------------------------------------------------------------------
    class ARMExecutor
      def execute(decoded, registers, memory, pc)
        case decoded.mnemonic
        when "mov" then exec_mov(decoded, registers, pc)
        when "add" then exec_add(decoded, registers, pc)
        when "sub" then exec_sub(decoded, registers, pc)
        when "hlt"
          CpuSimulator::ExecuteResult.new(
            description: "Halt", registers_changed: {},
            memory_changed: {}, next_pc: pc, halted: true
          )
        else
          CpuSimulator::ExecuteResult.new(
            description: "Unknown instruction: #{decoded.mnemonic}",
            registers_changed: {}, memory_changed: {}, next_pc: pc + 4
          )
        end
      end

      private

      def exec_mov(decoded, registers, pc)
        rd = decoded.fields[:rd]
        imm = decoded.fields[:imm]
        result = imm & 0xFFFFFFFF
        registers.write(rd, result)
        CpuSimulator::ExecuteResult.new(
          description: "R#{rd} = #{result}",
          registers_changed: {"R#{rd}" => result},
          memory_changed: {}, next_pc: pc + 4
        )
      end

      def exec_add(decoded, registers, pc)
        rd = decoded.fields[:rd]
        rn = decoded.fields[:rn]
        rm = decoded.fields[:rm]
        rn_val = registers.read(rn)
        rm_val = registers.read(rm)
        result = (rn_val + rm_val) & 0xFFFFFFFF
        registers.write(rd, result)
        CpuSimulator::ExecuteResult.new(
          description: "R#{rd} = R#{rn}(#{rn_val}) + R#{rm}(#{rm_val}) = #{result}",
          registers_changed: {"R#{rd}" => result},
          memory_changed: {}, next_pc: pc + 4
        )
      end

      def exec_sub(decoded, registers, pc)
        rd = decoded.fields[:rd]
        rn = decoded.fields[:rn]
        rm = decoded.fields[:rm]
        rn_val = registers.read(rn)
        rm_val = registers.read(rm)
        result = (rn_val - rm_val) & 0xFFFFFFFF
        registers.write(rd, result)
        CpuSimulator::ExecuteResult.new(
          description: "R#{rd} = R#{rn}(#{rn_val}) - R#{rm}(#{rm_val}) = #{result}",
          registers_changed: {"R#{rd}" => result},
          memory_changed: {}, next_pc: pc + 4
        )
      end
    end

    # -------------------------------------------------------------------
    # Assembler helpers — encode ARM instructions to 32-bit words
    # -------------------------------------------------------------------

    def self.encode_mov_imm(rd, imm)
      (COND_AL << 28) | (0b00 << 26) | (1 << 25) | (OPCODE_MOV << 21) |
        (0 << 20) | (0 << 16) | (rd << 12) | (0 << 8) | (imm & 0xFF)
    end

    def self.encode_add(rd, rn, rm)
      (COND_AL << 28) | (0b00 << 26) | (0 << 25) | (OPCODE_ADD << 21) |
        (0 << 20) | (rn << 16) | (rd << 12) | rm
    end

    def self.encode_sub(rd, rn, rm)
      (COND_AL << 28) | (0b00 << 26) | (0 << 25) | (OPCODE_SUB << 21) |
        (0 << 20) | (rn << 16) | (rd << 12) | rm
    end

    def self.encode_hlt
      HLT_INSTRUCTION
    end

    # Convert a list of 32-bit instruction words to bytes (little-endian).
    def self.assemble(instructions)
      instructions.map { |i| [i & 0xFFFFFFFF].pack("V") }.join.b
    end

    # -------------------------------------------------------------------
    # High-level simulator — wraps CPU with ARM decoder/executor
    # -------------------------------------------------------------------
    class ARMSimulator
      attr_reader :cpu

      def initialize(memory_size: 65536)
        @decoder = ARMDecoder.new
        @executor = ARMExecutor.new
        @cpu = CpuSimulator::CPU.new(
          decoder: @decoder, executor: @executor,
          num_registers: 16, bit_width: 32, memory_size: memory_size
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
