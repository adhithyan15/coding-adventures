# frozen_string_literal: true

# ---------------------------------------------------------------------------
# GenericISA -- a simplified, educational instruction set.
# ---------------------------------------------------------------------------
#
# === What is this? ===
#
# This is the default InstructionSet implementation -- a vendor-neutral ISA
# designed for teaching, not for matching any real hardware. It proves that
# the pluggable ISA design works: if you can implement GenericISA, you can
# implement NVIDIA PTX, AMD GCN, Intel Xe, or ARM Mali the same way.
#
# === How it works ===
#
# The GenericISA#execute method is a big case/when statement. For each
# opcode, it:
# 1. Reads source registers
# 2. Calls the appropriate fp_arithmetic function
# 3. Writes the result to the destination register
# 4. Returns an ExecuteResult describing what happened
#
#     FADD R2, R0, R1:
#         a = registers.read(R0)          # read 3.14
#         b = registers.read(R1)          # read 2.71
#         result = fp_add(a, b)           # 3.14 + 2.71 = 5.85
#         registers.write(R2, result)     # store in R2
#         return ExecuteResult("R2 = R0 + R1 = 3.14 + 2.71 = 5.85", ...)
#
# === Future ISAs follow the same pattern ===
#
#     class PTXISA
#       def execute(instruction, registers, memory)
#         case instruction.opcode
#         when :add_f32   # same as :fadd but with PTX naming
#         when :fma_rn_f32 # same as :ffma but with PTX naming
#
# The GPUCore doesn't care which ISA is plugged in -- it just calls
# isa.execute() and processes the ExecuteResult.
#
# === Ruby Duck Typing ===
#
# In Python, we use Protocol classes to define the ISA interface. In Ruby,
# we use duck typing: any object that responds to #name and
# #execute(instruction, registers, memory) can be used as an ISA. No
# explicit interface declaration needed -- if it quacks like an ISA, it's
# an ISA.

module CodingAdventures
  module GpuCore
    # A simplified, educational instruction set for GPU cores.
    #
    # This ISA is not tied to any vendor -- it's a teaching tool. It has
    # 16 opcodes covering arithmetic, memory, data movement, and control
    # flow. Any floating-point program can be expressed with these.
    #
    # To use a different ISA, create a class with the same #execute method
    # signature and pass it to GPUCore.new(isa: YourISA.new).
    class GenericISA
      # ISA identifier.
      def name
        "Generic"
      end

      # Execute a single instruction.
      #
      # This is the heart of the ISA -- a dispatch table that maps opcodes
      # to their implementations. Each case reads operands, performs the
      # operation, writes results, and returns a trace description.
      #
      # @param instruction [Instruction] The instruction to execute.
      # @param registers [FPRegisterFile] The core's floating-point register file.
      # @param memory [LocalMemory] The core's local scratchpad memory.
      # @return [ExecuteResult] describing what happened.
      def execute(instruction, registers, memory)
        case instruction.opcode
        # --- Floating-point arithmetic ---
        when :fadd then exec_fadd(instruction, registers)
        when :fsub then exec_fsub(instruction, registers)
        when :fmul then exec_fmul(instruction, registers)
        when :ffma then exec_ffma(instruction, registers)
        when :fneg then exec_fneg(instruction, registers)
        when :fabs then exec_fabs(instruction, registers)
        # --- Memory ---
        when :load then exec_load(instruction, registers, memory)
        when :store then exec_store(instruction, registers, memory)
        # --- Data movement ---
        when :mov then exec_mov(instruction, registers)
        when :limm then exec_limm(instruction, registers)
        # --- Control flow ---
        when :beq then exec_beq(instruction, registers)
        when :blt then exec_blt(instruction, registers)
        when :bne then exec_bne(instruction, registers)
        when :jmp then exec_jmp(instruction)
        when :nop then ExecuteResult.new(description: "No operation")
        when :halt then ExecuteResult.new(description: "Halted", halted: true)
        else
          raise ArgumentError, "Unknown opcode: #{instruction.opcode}"
        end
      end

      private

      # --- Arithmetic implementations ---

      # FADD Rd, Rs1, Rs2 -> Rd = Rs1 + Rs2.
      def exec_fadd(inst, regs)
        a = regs.read(inst.rs1)
        b = regs.read(inst.rs2)
        result = FpArithmetic.fp_add(a, b)
        regs.write(inst.rd, result)
        a_f = FpArithmetic.bits_to_float(a)
        b_f = FpArithmetic.bits_to_float(b)
        r_f = FpArithmetic.bits_to_float(result)
        ExecuteResult.new(
          description: "R#{inst.rd} = R#{inst.rs1} + R#{inst.rs2} = #{a_f} + #{b_f} = #{r_f}",
          registers_changed: {"R#{inst.rd}" => r_f}
        )
      end

      # FSUB Rd, Rs1, Rs2 -> Rd = Rs1 - Rs2.
      def exec_fsub(inst, regs)
        a = regs.read(inst.rs1)
        b = regs.read(inst.rs2)
        result = FpArithmetic.fp_sub(a, b)
        regs.write(inst.rd, result)
        a_f = FpArithmetic.bits_to_float(a)
        b_f = FpArithmetic.bits_to_float(b)
        r_f = FpArithmetic.bits_to_float(result)
        ExecuteResult.new(
          description: "R#{inst.rd} = R#{inst.rs1} - R#{inst.rs2} = #{a_f} - #{b_f} = #{r_f}",
          registers_changed: {"R#{inst.rd}" => r_f}
        )
      end

      # FMUL Rd, Rs1, Rs2 -> Rd = Rs1 * Rs2.
      def exec_fmul(inst, regs)
        a = regs.read(inst.rs1)
        b = regs.read(inst.rs2)
        result = FpArithmetic.fp_mul(a, b)
        regs.write(inst.rd, result)
        a_f = FpArithmetic.bits_to_float(a)
        b_f = FpArithmetic.bits_to_float(b)
        r_f = FpArithmetic.bits_to_float(result)
        ExecuteResult.new(
          description: "R#{inst.rd} = R#{inst.rs1} * R#{inst.rs2} = #{a_f} * #{b_f} = #{r_f}",
          registers_changed: {"R#{inst.rd}" => r_f}
        )
      end

      # FFMA Rd, Rs1, Rs2, Rs3 -> Rd = Rs1 * Rs2 + Rs3.
      def exec_ffma(inst, regs)
        a = regs.read(inst.rs1)
        b = regs.read(inst.rs2)
        c = regs.read(inst.rs3)
        result = FpArithmetic.fp_fma(a, b, c)
        regs.write(inst.rd, result)
        a_f = FpArithmetic.bits_to_float(a)
        b_f = FpArithmetic.bits_to_float(b)
        c_f = FpArithmetic.bits_to_float(c)
        r_f = FpArithmetic.bits_to_float(result)
        ExecuteResult.new(
          description: "R#{inst.rd} = R#{inst.rs1} * R#{inst.rs2} + R#{inst.rs3} = #{a_f} * #{b_f} + #{c_f} = #{r_f}",
          registers_changed: {"R#{inst.rd}" => r_f}
        )
      end

      # FNEG Rd, Rs1 -> Rd = -Rs1.
      def exec_fneg(inst, regs)
        a = regs.read(inst.rs1)
        result = FpArithmetic.fp_neg(a)
        regs.write(inst.rd, result)
        a_f = FpArithmetic.bits_to_float(a)
        r_f = FpArithmetic.bits_to_float(result)
        ExecuteResult.new(
          description: "R#{inst.rd} = -R#{inst.rs1} = -#{a_f} = #{r_f}",
          registers_changed: {"R#{inst.rd}" => r_f}
        )
      end

      # FABS Rd, Rs1 -> Rd = |Rs1|.
      def exec_fabs(inst, regs)
        a = regs.read(inst.rs1)
        result = FpArithmetic.fp_abs(a)
        regs.write(inst.rd, result)
        a_f = FpArithmetic.bits_to_float(a)
        r_f = FpArithmetic.bits_to_float(result)
        ExecuteResult.new(
          description: "R#{inst.rd} = |R#{inst.rs1}| = |#{a_f}| = #{r_f}",
          registers_changed: {"R#{inst.rd}" => r_f}
        )
      end

      # --- Memory implementations ---

      # LOAD Rd, [Rs1+imm] -> Rd = Mem[Rs1 + immediate].
      def exec_load(inst, regs, memory)
        base = FpArithmetic.bits_to_float(regs.read(inst.rs1))
        address = (base + inst.immediate).to_i
        value = memory.load_float(address, regs.fmt)
        regs.write(inst.rd, value)
        val_f = FpArithmetic.bits_to_float(value)
        ExecuteResult.new(
          description: "R#{inst.rd} = Mem[R#{inst.rs1}+#{inst.immediate}] = Mem[#{address}] = #{val_f}",
          registers_changed: {"R#{inst.rd}" => val_f}
        )
      end

      # STORE [Rs1+imm], Rs2 -> Mem[Rs1 + immediate] = Rs2.
      def exec_store(inst, regs, memory)
        base = FpArithmetic.bits_to_float(regs.read(inst.rs1))
        address = (base + inst.immediate).to_i
        value = regs.read(inst.rs2)
        memory.store_float(address, value)
        val_f = FpArithmetic.bits_to_float(value)
        ExecuteResult.new(
          description: "Mem[R#{inst.rs1}+#{inst.immediate}] = R#{inst.rs2} -> Mem[#{address}] = #{val_f}",
          memory_changed: {address => val_f}
        )
      end

      # --- Data movement implementations ---

      # MOV Rd, Rs1 -> Rd = Rs1.
      def exec_mov(inst, regs)
        value = regs.read(inst.rs1)
        regs.write(inst.rd, value)
        val_f = FpArithmetic.bits_to_float(value)
        ExecuteResult.new(
          description: "R#{inst.rd} = R#{inst.rs1} = #{val_f}",
          registers_changed: {"R#{inst.rd}" => val_f}
        )
      end

      # LIMM Rd, immediate -> Rd = float literal.
      def exec_limm(inst, regs)
        regs.write_float(inst.rd, inst.immediate)
        ExecuteResult.new(
          description: "R#{inst.rd} = #{inst.immediate}",
          registers_changed: {"R#{inst.rd}" => inst.immediate}
        )
      end

      # --- Control flow implementations ---

      # BEQ Rs1, Rs2, offset -> if Rs1 == Rs2: PC += offset.
      def exec_beq(inst, regs)
        cmp = FpArithmetic.fp_compare(regs.read(inst.rs1), regs.read(inst.rs2))
        taken = cmp == 0
        offset = taken ? inst.immediate.to_i : 1
        a_f = FpArithmetic.bits_to_float(regs.read(inst.rs1))
        b_f = FpArithmetic.bits_to_float(regs.read(inst.rs2))
        branch_msg = taken ? "Yes -> branch" : "No -> fall through"
        ExecuteResult.new(
          description: "BEQ R#{inst.rs1}(#{a_f}) == R#{inst.rs2}(#{b_f})? #{branch_msg}",
          next_pc_offset: offset
        )
      end

      # BLT Rs1, Rs2, offset -> if Rs1 < Rs2: PC += offset.
      def exec_blt(inst, regs)
        cmp = FpArithmetic.fp_compare(regs.read(inst.rs1), regs.read(inst.rs2))
        taken = cmp < 0
        offset = taken ? inst.immediate.to_i : 1
        a_f = FpArithmetic.bits_to_float(regs.read(inst.rs1))
        b_f = FpArithmetic.bits_to_float(regs.read(inst.rs2))
        branch_msg = taken ? "Yes -> branch" : "No -> fall through"
        ExecuteResult.new(
          description: "BLT R#{inst.rs1}(#{a_f}) < R#{inst.rs2}(#{b_f})? #{branch_msg}",
          next_pc_offset: offset
        )
      end

      # BNE Rs1, Rs2, offset -> if Rs1 != Rs2: PC += offset.
      def exec_bne(inst, regs)
        cmp = FpArithmetic.fp_compare(regs.read(inst.rs1), regs.read(inst.rs2))
        taken = cmp != 0
        offset = taken ? inst.immediate.to_i : 1
        a_f = FpArithmetic.bits_to_float(regs.read(inst.rs1))
        b_f = FpArithmetic.bits_to_float(regs.read(inst.rs2))
        branch_msg = taken ? "Yes -> branch" : "No -> fall through"
        ExecuteResult.new(
          description: "BNE R#{inst.rs1}(#{a_f}) != R#{inst.rs2}(#{b_f})? #{branch_msg}",
          next_pc_offset: offset
        )
      end

      # JMP target -> PC = target (absolute jump).
      def exec_jmp(inst)
        target = inst.immediate.to_i
        ExecuteResult.new(
          description: "Jump to PC=#{target}",
          next_pc_offset: target,
          absolute_jump: true
        )
      end
    end
  end
end
