# frozen_string_literal: true

# ISADecoder -- the interface between the Core and any instruction set.
#
# = Why a Separate Decoder?
#
# The Core knows how to move instructions through a pipeline, predict
# branches, detect hazards, and access caches. But it does NOT know what
# any instruction means. That is the ISA decoder's job.
#
# This separation mirrors real CPU design:
#   - ARM defines the decoder semantics (what ADD, LDR, BEQ mean)
#   - Apple/Qualcomm build the pipeline and caches
#   - The decoder plugs into the pipeline via a well-defined interface
#
# = The Two Methods
#
# The decoder has exactly two responsibilities:
#
#  1. Decode: turn raw instruction bits into a structured PipelineToken
#     (fill in opcode, registers, control signals, immediate value)
#
#  2. Execute: perform the actual computation (ALU operation, branch
#     resolution, effective address calculation)
#
# These map directly to the ID and EX stages of the pipeline:
#
#   IF stage:  fetch raw bits from memory
#   ID stage:  decoder.decode(raw, token) -> fills in decoded fields
#   EX stage:  decoder.execute(token, reg_file) -> computes ALU result
#   MEM stage: core handles cache access
#   WB stage:  core handles register writeback

module CodingAdventures
  module Core
    # MockDecoder is a minimal ISA decoder for testing purposes.
    #
    # It supports a handful of instructions encoded in a simple format:
    #
    #   Bits 31-24: opcode (0=NOP, 1=ADD, 2=LOAD, 3=STORE, 4=BRANCH, 5=HALT,
    #                        6=ADDI, 7=SUB)
    #   Bits 23-20: Rd  (destination register)
    #   Bits 19-16: Rs1 (first source register)
    #   Bits 15-12: Rs2 (second source register)
    #   Bits 11-0:  immediate (12-bit, sign-extended)
    #
    # This encoding does not match any real ISA. It exists solely to exercise
    # the Core's pipeline, hazard detection, branch prediction, and caches.
    #
    # = Instruction Reference
    #
    #   NOP    (0x00): Do nothing. Occupies a pipeline slot but has no effect.
    #   ADD    (0x01): Rd = Rs1 + Rs2
    #   LOAD   (0x02): Rd = Memory[Rs1 + imm]  (word load)
    #   STORE  (0x03): Memory[Rs1 + imm] = Rs2  (word store)
    #   BRANCH (0x04): If Rs1 == Rs2, PC = PC + imm (conditional branch)
    #   HALT   (0x05): Stop execution.
    #   ADDI   (0x06): Rd = Rs1 + imm
    #   SUB    (0x07): Rd = Rs1 - Rs2
    class MockDecoder
      # Returns the size of one instruction in bytes (always 4 for mock).
      #
      # @return [Integer] instruction size in bytes.
      def instruction_size
        4
      end

      # Decodes a raw 32-bit instruction into the given pipeline token.
      #
      # Encoding layout:
      #
      #     31      24 23    20 19    16 15    12 11           0
      #   +----------+--------+--------+--------+--------------+
      #   |  opcode  |   Rd   |  Rs1   |  Rs2   |  immediate   |
      #   +----------+--------+--------+--------+--------------+
      #
      # The immediate is sign-extended from 12 bits to a full integer.
      #
      # @param raw [Integer] raw instruction bits.
      # @param token [CodingAdventures::CpuPipeline::PipelineToken] token to fill.
      # @return [CodingAdventures::CpuPipeline::PipelineToken] the filled token.
      def decode(raw, token)
        # Extract fields using bit masking and shifting.
        opcode = (raw >> 24) & 0xFF
        rd = (raw >> 20) & 0x0F
        rs1 = (raw >> 16) & 0x0F
        rs2 = (raw >> 12) & 0x0F
        imm = raw & 0xFFF

        # Sign-extend the 12-bit immediate to a full integer.
        # If bit 11 is set, the value is negative.
        imm |= (-1 << 12) if (imm & 0x800) != 0

        # Fill in decoded fields based on opcode.
        case opcode
        when 0x00 # NOP
          token.opcode = "NOP"
          token.rd = -1
          token.rs1 = -1
          token.rs2 = -1

        when 0x01 # ADD Rd, Rs1, Rs2
          token.opcode = "ADD"
          token.rd = rd
          token.rs1 = rs1
          token.rs2 = rs2
          token.reg_write = true

        when 0x02 # LOAD Rd, [Rs1 + imm]
          token.opcode = "LOAD"
          token.rd = rd
          token.rs1 = rs1
          token.rs2 = -1
          token.immediate = imm
          token.reg_write = true
          token.mem_read = true

        when 0x03 # STORE [Rs1 + imm], Rs2
          token.opcode = "STORE"
          token.rd = -1
          token.rs1 = rs1
          token.rs2 = rs2
          token.immediate = imm
          token.mem_write = true

        when 0x04 # BRANCH Rs1, Rs2, imm (branch if Rs1 == Rs2)
          token.opcode = "BRANCH"
          token.rd = -1
          token.rs1 = rs1
          token.rs2 = rs2
          token.immediate = imm
          token.is_branch = true

        when 0x05 # HALT
          token.opcode = "HALT"
          token.rd = -1
          token.rs1 = -1
          token.rs2 = -1
          token.is_halt = true

        when 0x06 # ADDI Rd, Rs1, imm
          token.opcode = "ADDI"
          token.rd = rd
          token.rs1 = rs1
          token.rs2 = -1
          token.immediate = imm
          token.reg_write = true

        when 0x07 # SUB Rd, Rs1, Rs2
          token.opcode = "SUB"
          token.rd = rd
          token.rs1 = rs1
          token.rs2 = rs2
          token.reg_write = true

        else # Unknown opcode -- treat as NOP
          token.opcode = "NOP"
          token.rd = -1
          token.rs1 = -1
          token.rs2 = -1
        end

        token
      end

      # Performs the ALU operation for a decoded instruction.
      #
      # This reads register values, computes the result, and fills in
      # alu_result, branch_taken, branch_target, and write_data.
      #
      # @param token [CodingAdventures::CpuPipeline::PipelineToken] decoded token.
      # @param reg_file [RegisterFile] register file for reading source values.
      # @return [CodingAdventures::CpuPipeline::PipelineToken] token with results.
      def execute(token, reg_file)
        # Read source register values.
        rs1_val = (token.rs1 >= 0) ? reg_file.read(token.rs1) : 0
        rs2_val = (token.rs2 >= 0) ? reg_file.read(token.rs2) : 0

        case token.opcode
        when "ADD"
          token.alu_result = rs1_val + rs2_val
          token.write_data = token.alu_result

        when "SUB"
          token.alu_result = rs1_val - rs2_val
          token.write_data = token.alu_result

        when "ADDI"
          token.alu_result = rs1_val + token.immediate
          token.write_data = token.alu_result

        when "LOAD"
          # Effective address = Rs1 + immediate
          # The actual memory read happens in the MEM stage (handled by Core).
          token.alu_result = rs1_val + token.immediate

        when "STORE"
          # Effective address = Rs1 + immediate
          # The data to store comes from Rs2.
          token.alu_result = rs1_val + token.immediate
          token.write_data = rs2_val

        when "BRANCH"
          # Branch condition: Rs1 == Rs2
          # Branch target: PC + (immediate * instruction_size)
          taken = rs1_val == rs2_val
          token.branch_taken = taken
          target = token.pc + (token.immediate * 4)
          token.branch_target = target
          token.alu_result = taken ? target : (token.pc + 4)

        when "NOP", "HALT"
          # No computation needed.

        else
          # Unknown opcode -- no computation.
        end

        token
      end
    end

    # =========================================================================
    # Instruction encoding helpers for building test programs
    # =========================================================================

    # Returns the raw encoding for a NOP instruction.
    def self.encode_nop
      0x00 << 24
    end

    # Returns the raw encoding for ADD Rd, Rs1, Rs2.
    def self.encode_add(rd, rs1, rs2)
      (0x01 << 24) | (rd << 20) | (rs1 << 16) | (rs2 << 12)
    end

    # Returns the raw encoding for SUB Rd, Rs1, Rs2.
    def self.encode_sub(rd, rs1, rs2)
      (0x07 << 24) | (rd << 20) | (rs1 << 16) | (rs2 << 12)
    end

    # Returns the raw encoding for ADDI Rd, Rs1, imm.
    def self.encode_addi(rd, rs1, imm)
      (0x06 << 24) | (rd << 20) | (rs1 << 16) | (imm & 0xFFF)
    end

    # Returns the raw encoding for LOAD Rd, [Rs1 + imm].
    def self.encode_load(rd, rs1, imm)
      (0x02 << 24) | (rd << 20) | (rs1 << 16) | (imm & 0xFFF)
    end

    # Returns the raw encoding for STORE [Rs1 + imm], Rs2.
    def self.encode_store(rs1, rs2, imm)
      (0x03 << 24) | (rs1 << 16) | (rs2 << 12) | (imm & 0xFFF)
    end

    # Returns the raw encoding for BRANCH Rs1, Rs2, imm.
    # The branch is taken if Rs1 == Rs2, jumping to PC + imm*4.
    def self.encode_branch(rs1, rs2, imm)
      (0x04 << 24) | (rs1 << 16) | (rs2 << 12) | (imm & 0xFFF)
    end

    # Returns the raw encoding for a HALT instruction.
    def self.encode_halt
      0x05 << 24
    end

    # Converts a sequence of raw instruction integers into a byte string
    # suitable for load_program.
    #
    # Each instruction is encoded as 4 bytes in little-endian order.
    #
    # @param instructions [Array<Integer>] raw instruction integers.
    # @return [Array<Integer>] byte array.
    def self.encode_program(*instructions)
      result = Array.new(instructions.length * 4, 0)
      instructions.each_with_index do |instr, i|
        offset = i * 4
        result[offset] = instr & 0xFF
        result[offset + 1] = (instr >> 8) & 0xFF
        result[offset + 2] = (instr >> 16) & 0xFF
        result[offset + 3] = (instr >> 24) & 0xFF
      end
      result
    end
  end
end
