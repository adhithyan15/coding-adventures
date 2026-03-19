# frozen_string_literal: true

# === Instruction executor for all RV32I + M-mode instructions ===
#
# The executor performs the actual computation after decoding. It reads
# register values, performs operations, writes results back, and determines
# the next PC.

module CodingAdventures
  module RiscvSimulator
    class RiscVExecutor
      attr_accessor :csr

      def initialize(csr: nil)
        @csr = csr
      end

      def execute(decoded, registers, memory, pc)
        case decoded.mnemonic
        # I-type arithmetic
        when "addi"  then exec_imm_arith(decoded, registers, pc) { |a, b| to_u32(a + b) }
        when "slti"  then exec_imm_arith(decoded, registers, pc) { |a, b| a < b ? 1 : 0 }
        when "sltiu" then exec_imm_arith(decoded, registers, pc) { |a, b| to_u32(a) < to_u32(b) ? 1 : 0 }
        when "xori"  then exec_imm_arith(decoded, registers, pc) { |a, b| to_u32(a) ^ to_u32(b) }
        when "ori"   then exec_imm_arith(decoded, registers, pc) { |a, b| to_u32(a) | to_u32(b) }
        when "andi"  then exec_imm_arith(decoded, registers, pc) { |a, b| to_u32(a) & to_u32(b) }
        # Shift immediate
        when "slli"  then exec_shift_imm(decoded, registers, pc) { |v, s| (v << s) & MASK32 }
        when "srli"  then exec_shift_imm(decoded, registers, pc) { |v, s| v >> s }
        when "srai"  then exec_shift_imm(decoded, registers, pc) { |v, s| to_u32(to_s32(v) >> s) }
        # R-type arithmetic
        when "add"   then exec_reg_arith(decoded, registers, pc) { |a, b| to_u32(to_s32(a) + to_s32(b)) }
        when "sub"   then exec_reg_arith(decoded, registers, pc) { |a, b| to_u32(to_s32(a) - to_s32(b)) }
        when "sll"   then exec_reg_arith(decoded, registers, pc) { |a, b| (a << (b & 0x1F)) & MASK32 }
        when "slt"   then exec_reg_arith(decoded, registers, pc) { |a, b| to_s32(a) < to_s32(b) ? 1 : 0 }
        when "sltu"  then exec_reg_arith(decoded, registers, pc) { |a, b| a < b ? 1 : 0 }
        when "xor"   then exec_reg_arith(decoded, registers, pc) { |a, b| a ^ b }
        when "srl"   then exec_reg_arith(decoded, registers, pc) { |a, b| a >> (b & 0x1F) }
        when "sra"   then exec_reg_arith(decoded, registers, pc) { |a, b| to_u32(to_s32(a) >> (b & 0x1F)) }
        when "or"    then exec_reg_arith(decoded, registers, pc) { |a, b| a | b }
        when "and"   then exec_reg_arith(decoded, registers, pc) { |a, b| a & b }
        # Loads
        when "lb", "lh", "lw", "lbu", "lhu" then exec_load(decoded, registers, memory, pc)
        # Stores
        when "sb", "sh", "sw" then exec_store(decoded, registers, memory, pc)
        # Branches
        when "beq"   then exec_branch(decoded, registers, pc) { |a, b| a == b }
        when "bne"   then exec_branch(decoded, registers, pc) { |a, b| a != b }
        when "blt"   then exec_branch(decoded, registers, pc) { |a, b| to_s32(a) < to_s32(b) }
        when "bge"   then exec_branch(decoded, registers, pc) { |a, b| to_s32(a) >= to_s32(b) }
        when "bltu"  then exec_branch(decoded, registers, pc) { |a, b| a < b }
        when "bgeu"  then exec_branch(decoded, registers, pc) { |a, b| a >= b }
        # Jumps
        when "jal"   then exec_jal(decoded, registers, pc)
        when "jalr"  then exec_jalr(decoded, registers, pc)
        # Upper immediates
        when "lui"   then exec_lui(decoded, registers, pc)
        when "auipc" then exec_auipc(decoded, registers, pc)
        # System
        when "ecall" then exec_ecall(decoded, registers, pc)
        when "mret"  then exec_mret(decoded, registers, pc)
        when "csrrw" then exec_csrrw(decoded, registers, pc)
        when "csrrs" then exec_csrrs(decoded, registers, pc)
        when "csrrc" then exec_csrrc(decoded, registers, pc)
        else
          CpuSimulator::ExecuteResult.new(
            description: "Unknown instruction: #{decoded.mnemonic}",
            registers_changed: {}, memory_changed: {},
            next_pc: pc + 4
          )
        end
      end

      private

      def to_s32(val)
        val = val & MASK32
        val >= 0x80000000 ? val - 0x100000000 : val
      end

      def to_u32(val)
        val & MASK32
      end

      def write_rd(registers, rd, value)
        value = to_u32(value)
        changes = {}
        if rd != 0
          registers.write(rd, value)
          changes["x#{rd}"] = value
        end
        changes
      end

      def exec_imm_arith(decoded, registers, pc)
        rd = decoded.fields[:rd]
        rs1 = decoded.fields[:rs1]
        imm = decoded.fields[:imm]
        rs1_val = to_s32(registers.read(rs1))
        result = to_u32(yield(rs1_val, to_s32(imm)))
        changes = write_rd(registers, rd, result)
        CpuSimulator::ExecuteResult.new(
          description: "#{decoded.mnemonic}: x#{rd} = #{result}",
          registers_changed: changes, memory_changed: {},
          next_pc: pc + 4
        )
      end

      def exec_shift_imm(decoded, registers, pc)
        rd = decoded.fields[:rd]
        rs1 = decoded.fields[:rs1]
        shamt = decoded.fields[:imm] & 0x1F
        rs1_val = registers.read(rs1)
        result = to_u32(yield(rs1_val, shamt))
        changes = write_rd(registers, rd, result)
        CpuSimulator::ExecuteResult.new(
          description: "#{decoded.mnemonic}: x#{rd} = #{result}",
          registers_changed: changes, memory_changed: {},
          next_pc: pc + 4
        )
      end

      def exec_reg_arith(decoded, registers, pc)
        rd = decoded.fields[:rd]
        rs1 = decoded.fields[:rs1]
        rs2 = decoded.fields[:rs2]
        rs1_val = registers.read(rs1)
        rs2_val = registers.read(rs2)
        result = to_u32(yield(rs1_val, rs2_val))
        changes = write_rd(registers, rd, result)
        CpuSimulator::ExecuteResult.new(
          description: "#{decoded.mnemonic}: x#{rd} = #{result}",
          registers_changed: changes, memory_changed: {},
          next_pc: pc + 4
        )
      end

      def exec_load(decoded, registers, memory, pc)
        rd = decoded.fields[:rd]
        rs1 = decoded.fields[:rs1]
        imm = decoded.fields[:imm]
        m = decoded.mnemonic

        addr = to_u32(to_s32(registers.read(rs1)) + to_s32(imm))

        result = case m
        when "lb"
          b = memory.read_byte(addr)
          to_u32((b & 0x80) != 0 ? b - 256 : b)
        when "lh"
          lo = memory.read_byte(addr)
          hi = memory.read_byte(addr + 1)
          half = lo | (hi << 8)
          to_u32((half & 0x8000) != 0 ? half - 0x10000 : half)
        when "lw"
          memory.read_word(addr)
        when "lbu"
          memory.read_byte(addr)
        when "lhu"
          lo = memory.read_byte(addr)
          hi = memory.read_byte(addr + 1)
          lo | (hi << 8)
        end

        changes = write_rd(registers, rd, result)
        CpuSimulator::ExecuteResult.new(
          description: "#{m}: x#{rd} = mem[#{addr}] = #{result}",
          registers_changed: changes, memory_changed: {},
          next_pc: pc + 4
        )
      end

      def exec_store(decoded, registers, memory, pc)
        rs1 = decoded.fields[:rs1]
        rs2 = decoded.fields[:rs2]
        imm = decoded.fields[:imm]
        m = decoded.mnemonic

        addr = to_u32(to_s32(registers.read(rs1)) + to_s32(imm))
        val = registers.read(rs2)
        mem_changes = {}

        case m
        when "sb"
          b = val & 0xFF
          memory.write_byte(addr, b)
          mem_changes[addr] = b
        when "sh"
          lo = val & 0xFF
          hi = (val >> 8) & 0xFF
          memory.write_byte(addr, lo)
          memory.write_byte(addr + 1, hi)
          mem_changes[addr] = lo
          mem_changes[addr + 1] = hi
        when "sw"
          memory.write_word(addr, val)
          mem_changes[addr] = val & 0xFF
          mem_changes[addr + 1] = (val >> 8) & 0xFF
          mem_changes[addr + 2] = (val >> 16) & 0xFF
          mem_changes[addr + 3] = (val >> 24) & 0xFF
        end

        CpuSimulator::ExecuteResult.new(
          description: "#{m}: mem[#{addr}] = #{val}",
          registers_changed: {}, memory_changed: mem_changes,
          next_pc: pc + 4
        )
      end

      def exec_branch(decoded, registers, pc)
        rs1 = decoded.fields[:rs1]
        rs2 = decoded.fields[:rs2]
        imm = decoded.fields[:imm]

        rs1_val = registers.read(rs1)
        rs2_val = registers.read(rs2)
        taken = yield(rs1_val, rs2_val)
        next_pc = taken ? pc + imm : pc + 4

        CpuSimulator::ExecuteResult.new(
          description: "#{decoded.mnemonic}: taken=#{taken}",
          registers_changed: {}, memory_changed: {},
          next_pc: next_pc
        )
      end

      def exec_jal(decoded, registers, pc)
        rd = decoded.fields[:rd]
        imm = decoded.fields[:imm]
        return_addr = to_u32(pc + 4)
        changes = write_rd(registers, rd, return_addr)
        CpuSimulator::ExecuteResult.new(
          description: "jal: x#{rd} = #{return_addr}, jump to #{pc + imm}",
          registers_changed: changes, memory_changed: {},
          next_pc: pc + imm
        )
      end

      def exec_jalr(decoded, registers, pc)
        rd = decoded.fields[:rd]
        rs1 = decoded.fields[:rs1]
        imm = decoded.fields[:imm]
        return_addr = to_u32(pc + 4)
        target = (to_s32(registers.read(rs1)) + to_s32(imm)) & ~1
        changes = write_rd(registers, rd, return_addr)
        CpuSimulator::ExecuteResult.new(
          description: "jalr: x#{rd} = #{return_addr}, jump to #{target}",
          registers_changed: changes, memory_changed: {},
          next_pc: target
        )
      end

      def exec_lui(decoded, registers, pc)
        rd = decoded.fields[:rd]
        imm = decoded.fields[:imm]
        result = to_u32(imm << 12)
        changes = write_rd(registers, rd, result)
        CpuSimulator::ExecuteResult.new(
          description: "lui: x#{rd} = #{result}",
          registers_changed: changes, memory_changed: {},
          next_pc: pc + 4
        )
      end

      def exec_auipc(decoded, registers, pc)
        rd = decoded.fields[:rd]
        imm = decoded.fields[:imm]
        result = to_u32(pc + (imm << 12))
        changes = write_rd(registers, rd, result)
        CpuSimulator::ExecuteResult.new(
          description: "auipc: x#{rd} = #{result}",
          registers_changed: changes, memory_changed: {},
          next_pc: pc + 4
        )
      end

      def exec_ecall(_decoded, _registers, pc)
        if @csr.nil?
          return CpuSimulator::ExecuteResult.new(
            description: "ecall: halt (no CSR file)",
            registers_changed: {}, memory_changed: {},
            next_pc: pc, halted: true
          )
        end

        mtvec = @csr.read(CSR_MTVEC)
        if mtvec == 0
          return CpuSimulator::ExecuteResult.new(
            description: "ecall: halt (mtvec=0)",
            registers_changed: {}, memory_changed: {},
            next_pc: pc, halted: true
          )
        end

        @csr.write(CSR_MEPC, pc)
        @csr.write(CSR_MCAUSE, CAUSE_ECALL_M_MODE)
        mstatus = @csr.read(CSR_MSTATUS)
        @csr.write(CSR_MSTATUS, mstatus & ~MIE)

        CpuSimulator::ExecuteResult.new(
          description: "ecall: trap to mtvec=0x#{format("%x", mtvec)}",
          registers_changed: {}, memory_changed: {},
          next_pc: mtvec
        )
      end

      def exec_mret(_decoded, _registers, pc)
        if @csr.nil?
          return CpuSimulator::ExecuteResult.new(
            description: "mret: no CSR file",
            registers_changed: {}, memory_changed: {},
            next_pc: pc + 4
          )
        end

        mepc = @csr.read(CSR_MEPC)
        mstatus = @csr.read(CSR_MSTATUS)
        @csr.write(CSR_MSTATUS, mstatus | MIE)

        CpuSimulator::ExecuteResult.new(
          description: "mret: return to mepc=0x#{format("%x", mepc)}",
          registers_changed: {}, memory_changed: {},
          next_pc: mepc
        )
      end

      def exec_csrrw(decoded, registers, pc)
        rd = decoded.fields[:rd]
        rs1 = decoded.fields[:rs1]
        csr_addr = decoded.fields[:csr]
        rs1_val = registers.read(rs1)
        old_csr = @csr.read_write(csr_addr, rs1_val)
        changes = write_rd(registers, rd, old_csr)
        CpuSimulator::ExecuteResult.new(
          description: "csrrw: x#{rd} = CSR[0x#{format("%x", csr_addr)}]",
          registers_changed: changes, memory_changed: {},
          next_pc: pc + 4
        )
      end

      def exec_csrrs(decoded, registers, pc)
        rd = decoded.fields[:rd]
        rs1 = decoded.fields[:rs1]
        csr_addr = decoded.fields[:csr]
        rs1_val = registers.read(rs1)
        old_csr = @csr.read_set(csr_addr, rs1_val)
        changes = write_rd(registers, rd, old_csr)
        CpuSimulator::ExecuteResult.new(
          description: "csrrs: x#{rd} = CSR[0x#{format("%x", csr_addr)}]",
          registers_changed: changes, memory_changed: {},
          next_pc: pc + 4
        )
      end

      def exec_csrrc(decoded, registers, pc)
        rd = decoded.fields[:rd]
        rs1 = decoded.fields[:rs1]
        csr_addr = decoded.fields[:csr]
        rs1_val = registers.read(rs1)
        old_csr = @csr.read_clear(csr_addr, rs1_val)
        changes = write_rd(registers, rd, old_csr)
        CpuSimulator::ExecuteResult.new(
          description: "csrrc: x#{rd} = CSR[0x#{format("%x", csr_addr)}]",
          registers_changed: changes, memory_changed: {},
          next_pc: pc + 4
        )
      end
    end
  end
end
