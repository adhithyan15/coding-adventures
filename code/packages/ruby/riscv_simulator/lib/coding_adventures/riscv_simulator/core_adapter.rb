# frozen_string_literal: true

# === RISC-V Core Adapter ===
#
# Adapts the RISC-V decoder and executor to a Core ISADecoder interface.
# The adapter bridges between the RISC-V instruction world (DecodeResult,
# ExecuteResult) and the Core pipeline world (PipelineToken hashes).
#
# === Two Translation Points ===
#
#   1. Decode: RISC-V decoder fills a DecodeResult -> adapter copies fields
#      into the pipeline token (rs1, rs2, rd, control signals).
#
#   2. Execute: adapter reads register values, computes ALU results using
#      RISC-V semantics, fills token's alu_result, branch_taken, etc.
#
# The adapter must NOT read/write registers or access memory during Execute.
# It only computes ALU results. The Core handles memory and writeback.

module CodingAdventures
  module RiscvSimulator
    class RiscVISADecoder
      attr_reader :csr

      def initialize
        @decoder = RiscVDecoder.new
        @csr = CSRFile.new
      end

      # All RV32I instructions are 32 bits (4 bytes).
      def instruction_size
        4
      end

      # Decode a raw RISC-V instruction into a pipeline token hash.
      # Sets control signals based on the instruction mnemonic.
      def decode(raw_instruction, token)
        raw = raw_instruction & MASK32
        decoded = @decoder.decode(raw, token[:pc] || 0)

        token[:opcode] = decoded.mnemonic
        token[:rd] = decoded.fields.fetch(:rd, -1)
        token[:rs1] = decoded.fields.fetch(:rs1, -1)
        token[:rs2] = decoded.fields.fetch(:rs2, -1)
        token[:immediate] = decoded.fields.fetch(:imm, 0)

        case decoded.mnemonic
        when "add", "sub", "sll", "slt", "sltu", "xor", "srl", "sra", "or", "and"
          token[:reg_write] = true
        when "addi", "slti", "sltiu", "xori", "ori", "andi", "slli", "srli", "srai"
          token[:reg_write] = true
        when "lui", "auipc"
          token[:reg_write] = true
        when "lb", "lh", "lw", "lbu", "lhu"
          token[:reg_write] = true
          token[:mem_read] = true
        when "sb", "sh", "sw"
          token[:mem_write] = true
        when "beq", "bne", "blt", "bge", "bltu", "bgeu"
          token[:is_branch] = true
        when "jal", "jalr"
          token[:reg_write] = true
          token[:is_branch] = true
        when "ecall"
          token[:is_halt] = true if @csr.read(CSR_MTVEC) == 0
        when "csrrw", "csrrs", "csrrc"
          token[:reg_write] = true
        when "mret"
          token[:is_branch] = true
        end

        token
      end

      # Perform ALU computation for a decoded RISC-V instruction.
      # Does NOT access memory or write registers.
      def execute(token, reg_file) # rubocop:disable Metrics/CyclomaticComplexity, Metrics/MethodLength, Metrics/PerceivedComplexity, Metrics/AbcSize
        rs1_val = (token[:rs1] || -1) >= 0 ? reg_file.read(token[:rs1]) : 0
        rs2_val = (token[:rs2] || -1) >= 0 ? reg_file.read(token[:rs2]) : 0

        rs1_u = rs1_val & MASK32
        rs2_u = rs2_val & MASK32
        imm = token[:immediate] || 0
        pc = token[:pc] || 0
        opcode = token[:opcode] || ""

        case opcode
        # R-type arithmetic
        when "add"
          result = to_signed32((rs1_u + rs2_u) & MASK32)
          token[:alu_result] = result
          token[:write_data] = result
        when "sub"
          result = to_signed32((rs1_u - rs2_u) & MASK32)
          token[:alu_result] = result
          token[:write_data] = result
        when "sll"
          result = (rs1_u << (rs2_u & 0x1F)) & MASK32
          token[:alu_result] = result
          token[:write_data] = result
        when "slt"
          result = to_signed32(rs1_u) < to_signed32(rs2_u) ? 1 : 0
          token[:alu_result] = result
          token[:write_data] = result
        when "sltu"
          result = rs1_u < rs2_u ? 1 : 0
          token[:alu_result] = result
          token[:write_data] = result
        when "xor"
          result = rs1_u ^ rs2_u
          token[:alu_result] = result
          token[:write_data] = result
        when "srl"
          result = rs1_u >> (rs2_u & 0x1F)
          token[:alu_result] = result
          token[:write_data] = result
        when "sra"
          result = to_signed32(rs1_u) >> (rs2_u & 0x1F)
          token[:alu_result] = result
          token[:write_data] = result
        when "or"
          result = rs1_u | rs2_u
          token[:alu_result] = result
          token[:write_data] = result
        when "and"
          result = rs1_u & rs2_u
          token[:alu_result] = result
          token[:write_data] = result

        # I-type arithmetic
        when "addi"
          result = to_signed32((rs1_u + (imm & MASK32)) & MASK32)
          token[:alu_result] = result
          token[:write_data] = result
        when "slti"
          result = to_signed32(rs1_u) < to_signed32(imm) ? 1 : 0
          token[:alu_result] = result
          token[:write_data] = result
        when "sltiu"
          result = rs1_u < (imm & MASK32) ? 1 : 0
          token[:alu_result] = result
          token[:write_data] = result
        when "xori"
          result = rs1_u ^ (imm & MASK32)
          token[:alu_result] = result
          token[:write_data] = result
        when "ori"
          result = rs1_u | (imm & MASK32)
          token[:alu_result] = result
          token[:write_data] = result
        when "andi"
          result = rs1_u & (imm & MASK32)
          token[:alu_result] = result
          token[:write_data] = result
        when "slli"
          shamt = (imm & MASK32) & 0x1F
          result = (rs1_u << shamt) & MASK32
          token[:alu_result] = result
          token[:write_data] = result
        when "srli"
          shamt = (imm & MASK32) & 0x1F
          result = rs1_u >> shamt
          token[:alu_result] = result
          token[:write_data] = result
        when "srai"
          shamt = (imm & MASK32) & 0x1F
          result = to_signed32(rs1_u) >> shamt
          token[:alu_result] = result
          token[:write_data] = result

        # Upper immediate
        when "lui"
          result = (imm << 12) & MASK32
          token[:alu_result] = result
          token[:write_data] = result
        when "auipc"
          result = ((pc & MASK32) + ((imm << 12) & MASK32)) & MASK32
          token[:alu_result] = result
          token[:write_data] = result

        # Load instructions: compute effective address
        when "lb", "lh", "lw", "lbu", "lhu"
          addr = to_signed32(rs1_u) + to_signed32(imm)
          token[:alu_result] = addr

        # Store instructions: compute address and prepare data
        when "sb", "sh", "sw"
          addr = to_signed32(rs1_u) + to_signed32(imm)
          token[:alu_result] = addr
          token[:write_data] = rs2_val

        # Branch instructions
        when "beq"
          execute_branch(token, rs1_u == rs2_u, pc, imm)
        when "bne"
          execute_branch(token, rs1_u != rs2_u, pc, imm)
        when "blt"
          execute_branch(token, to_signed32(rs1_u) < to_signed32(rs2_u), pc, imm)
        when "bge"
          execute_branch(token, to_signed32(rs1_u) >= to_signed32(rs2_u), pc, imm)
        when "bltu"
          execute_branch(token, rs1_u < rs2_u, pc, imm)
        when "bgeu"
          execute_branch(token, rs1_u >= rs2_u, pc, imm)

        # Jump instructions
        when "jal"
          return_addr = pc + 4
          target = pc + imm
          token[:alu_result] = target
          token[:write_data] = return_addr
          token[:branch_taken] = true
          token[:branch_target] = target
        when "jalr"
          return_addr = pc + 4
          target = (to_signed32(rs1_u) + to_signed32(imm)) & ~1
          token[:alu_result] = target
          token[:write_data] = return_addr
          token[:branch_taken] = true
          token[:branch_target] = target

        # CSR instructions
        when "csrrw"
          csr_addr = ((token[:raw_instruction] || 0) >> 20) & 0xFFF
          old_val = @csr.read_write(csr_addr, rs1_u)
          token[:alu_result] = old_val
          token[:write_data] = old_val
        when "csrrs"
          csr_addr = ((token[:raw_instruction] || 0) >> 20) & 0xFFF
          old_val = @csr.read_set(csr_addr, rs1_u)
          token[:alu_result] = old_val
          token[:write_data] = old_val
        when "csrrc"
          csr_addr = ((token[:raw_instruction] || 0) >> 20) & 0xFFF
          old_val = @csr.read_clear(csr_addr, rs1_u)
          token[:alu_result] = old_val
          token[:write_data] = old_val

        # ecall
        when "ecall"
          mtvec = @csr.read(CSR_MTVEC)
          if mtvec != 0
            @csr.write(CSR_MEPC, pc)
            @csr.write(CSR_MCAUSE, CAUSE_ECALL_M_MODE)
            mstatus = @csr.read(CSR_MSTATUS)
            @csr.write(CSR_MSTATUS, mstatus & ~MIE)
            token[:branch_taken] = true
            token[:branch_target] = mtvec
            token[:alu_result] = mtvec
          end

        # mret
        when "mret"
          mepc = @csr.read(CSR_MEPC)
          mstatus = @csr.read(CSR_MSTATUS)
          @csr.write(CSR_MSTATUS, mstatus | MIE)
          token[:branch_taken] = true
          token[:branch_target] = mepc
          token[:alu_result] = mepc
        end

        token
      end

      private

      def to_signed32(val)
        val = val & MASK32
        val >= 0x80000000 ? val - 0x100000000 : val
      end

      def execute_branch(token, taken, pc, imm)
        target = pc + imm
        token[:branch_taken] = taken
        token[:branch_target] = target
        token[:alu_result] = taken ? target : pc + 4
      end
    end

    # Factory function to create a RISC-V ISA decoder.
    def self.new_riscv_core
      RiscVISADecoder.new
    end
  end
end
