# frozen_string_literal: true

# === CLR IL Simulator — Microsoft's answer to the JVM ===
#
# The Common Language Runtime (CLR) is the virtual machine at the heart of
# Microsoft's .NET framework, first released in 2002. C#, F#, VB.NET all
# compile to Common Intermediate Language (CIL/MSIL), which the CLR executes.
#
# === CLR vs JVM: two philosophies ===
#
# JVM: type in the opcode  (iadd = integer add, ladd = long add)
# CLR: type inferred from stack  (add = works for any numeric type)
#
# The CLR has ONE add opcode; the JVM has four (iadd, ladd, fadd, dadd).
#
# === Short encodings ===
#
# CLR: ldc.i4.0 through ldc.i4.8  (9 shortcuts, 0-8)
# JVM: iconst_0 through iconst_5   (6 shortcuts, 0-5)
#
# === Branch offset convention ===
#
# CLR branch offsets are relative to the NEXT instruction's PC.
# br.s at PC=10 (2 bytes) with offset +3 -> target = 12 + 3 = 15
#
# === Two-byte opcodes (0xFE prefix) ===
#
# The CLR uses 0xFE as a prefix byte for comparison instructions:
#   ceq = 0xFE 0x01, cgt = 0xFE 0x02, clt = 0xFE 0x04

module CodingAdventures
  module ClrSimulator
    # Real CLR IL opcode values from ECMA-335
    NOP       = 0x00
    LDNULL    = 0x01
    LDLOC_0   = 0x06
    LDLOC_1   = 0x07
    LDLOC_2   = 0x08
    LDLOC_3   = 0x09
    STLOC_0   = 0x0A
    STLOC_1   = 0x0B
    STLOC_2   = 0x0C
    STLOC_3   = 0x0D
    LDLOC_S   = 0x11
    STLOC_S   = 0x13
    LDC_I4_0  = 0x16
    LDC_I4_1  = 0x17
    LDC_I4_2  = 0x18
    LDC_I4_3  = 0x19
    LDC_I4_4  = 0x1A
    LDC_I4_5  = 0x1B
    LDC_I4_6  = 0x1C
    LDC_I4_7  = 0x1D
    LDC_I4_8  = 0x1E
    LDC_I4_S  = 0x1F
    LDC_I4    = 0x20
    RET       = 0x2A
    BR_S      = 0x2B
    BRFALSE_S = 0x2C
    BRTRUE_S  = 0x2D
    ADD       = 0x58
    SUB       = 0x59
    MUL       = 0x5A
    DIV       = 0x5B
    PREFIX_FE = 0xFE

    # Two-byte opcode second bytes
    CEQ_BYTE = 0x01
    CGT_BYTE = 0x02
    CLT_BYTE = 0x04

    # Trace of one CLR instruction execution
    CLRTrace = Data.define(:pc, :opcode, :stack_before, :stack_after,
      :locals_snapshot, :description)

    # -------------------------------------------------------------------
    # Simulator
    # -------------------------------------------------------------------
    class CLRSimulator
      attr_reader :stack, :locals, :halted
      attr_accessor :pc

      def initialize
        @stack = []
        @locals = Array.new(16)
        @pc = 0
        @bytecode = "".b
        @halted = false
      end

      def load(bytecode, num_locals: 16)
        @bytecode = bytecode.b
        @stack = []
        @locals = Array.new(num_locals)
        @pc = 0
        @halted = false
      end

      def step
        raise RuntimeError, "CLR simulator has halted" if @halted
        if @pc >= @bytecode.bytesize
          raise RuntimeError, "PC (#{@pc}) is beyond end of bytecode"
        end

        stack_before = @stack.dup
        opcode_byte = @bytecode.getbyte(@pc)
        execute(opcode_byte, stack_before)
      end

      def run(max_steps: 10_000)
        traces = []
        max_steps.times do
          break if @halted
          traces << step
        end
        traces
      end

      private

      def execute(opcode_byte, stack_before)
        orig_pc = @pc

        # Two-byte opcodes (0xFE prefix)
        if opcode_byte == PREFIX_FE
          return execute_two_byte(stack_before)
        end

        # NOP
        if opcode_byte == NOP
          @pc += 1
          return make_trace(orig_pc, "nop", stack_before, "no operation")
        end

        # LDNULL
        if opcode_byte == LDNULL
          @stack.push(nil)
          @pc += 1
          return make_trace(orig_pc, "ldnull", stack_before, "push null")
        end

        # LDC.I4.0 through LDC.I4.8
        if opcode_byte >= LDC_I4_0 && opcode_byte <= LDC_I4_8
          value = opcode_byte - LDC_I4_0
          @stack.push(value)
          @pc += 1
          return make_trace(orig_pc, "ldc.i4.#{value}", stack_before, "push #{value}")
        end

        # LDC.I4.S
        if opcode_byte == LDC_I4_S
          value = @bytecode.byteslice(@pc + 1, 1).unpack1("c")
          @stack.push(value)
          @pc += 2
          return make_trace(orig_pc, "ldc.i4.s", stack_before, "push #{value}")
        end

        # LDC.I4
        if opcode_byte == LDC_I4
          value = @bytecode.byteslice(@pc + 1, 4).unpack1("l<")
          @stack.push(value)
          @pc += 5
          return make_trace(orig_pc, "ldc.i4", stack_before, "push #{value}")
        end

        # LDLOC.0 through LDLOC.3
        if opcode_byte >= LDLOC_0 && opcode_byte <= LDLOC_3
          slot = opcode_byte - LDLOC_0
          value = @locals[slot]
          raise RuntimeError, "Local variable #{slot} is uninitialized" if value.nil?
          @stack.push(value)
          @pc += 1
          return make_trace(orig_pc, "ldloc.#{slot}", stack_before,
            "push locals[#{slot}] = #{value}")
        end

        # STLOC.0 through STLOC.3
        if opcode_byte >= STLOC_0 && opcode_byte <= STLOC_3
          slot = opcode_byte - STLOC_0
          value = @stack.pop
          @locals[slot] = value
          @pc += 1
          return make_trace(orig_pc, "stloc.#{slot}", stack_before,
            "pop #{value}, store in locals[#{slot}]")
        end

        # LDLOC.S
        if opcode_byte == LDLOC_S
          slot = @bytecode.getbyte(@pc + 1)
          value = @locals[slot]
          raise RuntimeError, "Local variable #{slot} is uninitialized" if value.nil?
          @stack.push(value)
          @pc += 2
          return make_trace(orig_pc, "ldloc.s", stack_before,
            "push locals[#{slot}] = #{value}")
        end

        # STLOC.S
        if opcode_byte == STLOC_S
          slot = @bytecode.getbyte(@pc + 1)
          value = @stack.pop
          @locals[slot] = value
          @pc += 2
          return make_trace(orig_pc, "stloc.s", stack_before,
            "pop #{value}, store in locals[#{slot}]")
        end

        # Arithmetic: ADD, SUB, MUL, DIV
        if opcode_byte == ADD
          return execute_arithmetic(stack_before, "add") { |a, b| a + b }
        end
        if opcode_byte == SUB
          return execute_arithmetic(stack_before, "sub") { |a, b| a - b }
        end
        if opcode_byte == MUL
          return execute_arithmetic(stack_before, "mul") { |a, b| a * b }
        end
        if opcode_byte == DIV
          return execute_div(stack_before)
        end

        # RET
        if opcode_byte == RET
          @pc += 1
          @halted = true
          return make_trace(orig_pc, "ret", stack_before, "return")
        end

        # BR.S
        if opcode_byte == BR_S
          offset = @bytecode.byteslice(@pc + 1, 1).unpack1("c")
          next_pc = @pc + 2
          target = next_pc + offset
          @pc = target
          return make_trace(orig_pc, "br.s", stack_before,
            "branch to PC=#{target} (offset #{format("%+d", offset)})")
        end

        # BRFALSE.S
        if opcode_byte == BRFALSE_S
          return execute_conditional_branch_s(stack_before, "brfalse.s", take_if_zero: true)
        end

        # BRTRUE.S
        if opcode_byte == BRTRUE_S
          return execute_conditional_branch_s(stack_before, "brtrue.s", take_if_zero: false)
        end

        raise ArgumentError, "Unknown CLR opcode: 0x#{format("%02X", opcode_byte)} at PC=#{@pc}"
      end

      def execute_arithmetic(stack_before, mnemonic)
        orig_pc = @pc
        b = @stack.pop
        a = @stack.pop
        result = yield(a, b)
        @stack.push(result)
        @pc += 1
        make_trace(orig_pc, mnemonic, stack_before, "pop #{b} and #{a}, push #{result}")
      end

      def execute_div(stack_before)
        orig_pc = @pc
        b = @stack.pop
        a = @stack.pop
        raise ZeroDivisionError, "System.DivideByZeroException: division by zero" if b == 0
        result = (a.to_f / b).truncate
        @stack.push(result)
        @pc += 1
        make_trace(orig_pc, "div", stack_before, "pop #{b} and #{a}, push #{result}")
      end

      def execute_two_byte(stack_before)
        orig_pc = @pc
        raise ArgumentError, "Incomplete two-byte opcode at PC=#{@pc}" if @pc + 1 >= @bytecode.bytesize

        second = @bytecode.getbyte(@pc + 1)

        case second
        when CEQ_BYTE
          b = @stack.pop
          a = @stack.pop
          result = a == b ? 1 : 0
          @stack.push(result)
          @pc += 2
          make_trace(orig_pc, "ceq", stack_before, "pop #{b} and #{a}, push #{result} (#{a} == #{b})")
        when CGT_BYTE
          b = @stack.pop
          a = @stack.pop
          result = a > b ? 1 : 0
          @stack.push(result)
          @pc += 2
          make_trace(orig_pc, "cgt", stack_before, "pop #{b} and #{a}, push #{result} (#{a} > #{b})")
        when CLT_BYTE
          b = @stack.pop
          a = @stack.pop
          result = a < b ? 1 : 0
          @stack.push(result)
          @pc += 2
          make_trace(orig_pc, "clt", stack_before, "pop #{b} and #{a}, push #{result} (#{a} < #{b})")
        else
          raise ArgumentError, "Unknown two-byte opcode: 0xFE 0x#{format("%02X", second)} at PC=#{@pc}"
        end
      end

      def execute_conditional_branch_s(stack_before, mnemonic, take_if_zero:)
        orig_pc = @pc
        offset = @bytecode.byteslice(@pc + 1, 1).unpack1("c")
        next_pc = @pc + 2
        target = next_pc + offset

        value = @stack.pop
        numeric_value = value.nil? ? 0 : value
        should_branch = take_if_zero ? (numeric_value == 0) : (numeric_value != 0)

        if should_branch
          @pc = target
          make_trace(orig_pc, mnemonic, stack_before, "pop #{value}, branch taken to PC=#{target}")
        else
          @pc = next_pc
          make_trace(orig_pc, mnemonic, stack_before, "pop #{value}, branch not taken")
        end
      end

      def make_trace(pc, opcode, stack_before, description)
        CLRTrace.new(pc: pc, opcode: opcode, stack_before: stack_before,
          stack_after: @stack.dup, locals_snapshot: @locals.dup,
          description: description)
      end
    end

    # -------------------------------------------------------------------
    # Encoding helpers
    # -------------------------------------------------------------------

    def self.encode_ldc_i4(n)
      if n >= 0 && n <= 8
        [LDC_I4_0 + n].pack("C")
      elsif n >= -128 && n <= 127
        [LDC_I4_S].pack("C") + [n].pack("c")
      else
        [LDC_I4].pack("C") + [n].pack("l<")
      end
    end

    def self.encode_stloc(slot)
      if slot >= 0 && slot <= 3
        [STLOC_0 + slot].pack("C")
      else
        [STLOC_S, slot].pack("CC")
      end
    end

    def self.encode_ldloc(slot)
      if slot >= 0 && slot <= 3
        [LDLOC_0 + slot].pack("C")
      else
        [LDLOC_S, slot].pack("CC")
      end
    end

    # Assemble CLR IL from instruction tuples or raw bytes.
    def self.assemble_clr(*instructions)
      result = []
      instructions.each do |instr|
        if instr.is_a?(String)
          result << instr
        else
          result << instr.pack("C*")
        end
      end
      result.join.b
    end
  end
end
