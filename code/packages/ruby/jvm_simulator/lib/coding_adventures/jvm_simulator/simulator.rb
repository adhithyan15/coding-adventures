# frozen_string_literal: true

# === Java Virtual Machine (JVM) Bytecode Simulator ===
#
# The JVM was introduced by Sun Microsystems in 1995 alongside Java. Its
# revolutionary promise: "write once, run anywhere." Compile source to
# platform-independent bytecode, and any machine with a JVM can run it.
#
# Today the JVM is the most widely deployed virtual machine in history,
# running Java, Kotlin, Scala, Clojure, Groovy, and JRuby.
#
# === Stack machine with typed opcodes ===
#
# Like WASM, the JVM is stack-based. But JVM opcodes are *typed*:
#   iadd = integer add, ladd = long add, fadd = float add, dadd = double add
#
# We implement only the "i" (integer, 32-bit signed) variants.
#
# === Variable-width bytecode ===
#
#   iconst_0..5   1 byte  (constant baked into opcode)
#   bipush V      2 bytes (opcode + signed byte)
#   ldc #N        2 bytes (opcode + pool index)
#   iload N       2 bytes (opcode + local index)
#   iload_0..3    1 byte  (shortcut)
#   istore N      2 bytes (opcode + local index)
#   istore_0..3   1 byte  (shortcut)
#   iadd/isub/imul/idiv  1 byte
#   goto +N       3 bytes (opcode + 2-byte signed offset)
#   if_icmpeq +N  3 bytes
#   if_icmpgt +N  3 bytes
#   ireturn       1 byte
#   return        1 byte
#
# === Branch offsets ===
#
# JVM branch offsets are relative to the start of the branch instruction
# itself. "goto +3" skips the goto's own 3 bytes to the next instruction.

module CodingAdventures
  module JvmSimulator
    # Real JVM opcode values from the JVM specification
    ICONST_0  = 0x03
    ICONST_1  = 0x04
    ICONST_2  = 0x05
    ICONST_3  = 0x06
    ICONST_4  = 0x07
    ICONST_5  = 0x08
    BIPUSH    = 0x10
    LDC       = 0x12
    ILOAD     = 0x15
    ILOAD_0   = 0x1A
    ILOAD_1   = 0x1B
    ILOAD_2   = 0x1C
    ILOAD_3   = 0x1D
    ISTORE    = 0x36
    ISTORE_0  = 0x3B
    ISTORE_1  = 0x3C
    ISTORE_2  = 0x3D
    ISTORE_3  = 0x3E
    IADD      = 0x60
    ISUB      = 0x64
    IMUL      = 0x68
    IDIV      = 0x6C
    IF_ICMPEQ = 0x9F
    IF_ICMPGT = 0xA3
    GOTO      = 0xA7
    IRETURN   = 0xAC
    RETURN    = 0xB1

    # Trace of one JVM instruction execution
    JVMTrace = Data.define(:pc, :opcode, :stack_before, :stack_after,
      :locals_snapshot, :description)

    # -------------------------------------------------------------------
    # Simulator
    # -------------------------------------------------------------------
    class JVMSimulator
      attr_reader :stack, :locals, :halted, :return_value
      attr_accessor :pc

      def initialize
        @stack = []
        @locals = Array.new(16)
        @constants = []
        @pc = 0
        @halted = false
        @return_value = nil
        @bytecode = "".b
      end

      def load(bytecode, constants: [], num_locals: 16)
        @bytecode = bytecode.b
        @constants = constants
        @stack = []
        @locals = Array.new(num_locals)
        @pc = 0
        @halted = false
        @return_value = nil
      end

      def step
        raise RuntimeError, "JVM simulator has halted" if @halted
        if @pc >= @bytecode.bytesize
          raise RuntimeError, "PC (#{@pc}) is past end of bytecode (#{@bytecode.bytesize} bytes)"
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

      def to_i32(value)
        value = value & 0xFFFFFFFF
        value -= 0x100000000 if value >= 0x80000000
        value
      end

      def execute(opcode_byte, stack_before)
        pc = @pc

        # iconst_0 through iconst_5
        if opcode_byte >= ICONST_0 && opcode_byte <= ICONST_5
          value = opcode_byte - ICONST_0
          @stack.push(value)
          @pc += 1
          return make_trace(pc, "iconst_#{value}", stack_before, "push #{value}")
        end

        case opcode_byte
        when BIPUSH
          raw = @bytecode.getbyte(@pc + 1)
          value = raw < 128 ? raw : raw - 256
          @stack.push(value)
          @pc += 2
          make_trace(pc, "bipush", stack_before, "push #{value}")

        when LDC
          index = @bytecode.getbyte(@pc + 1)
          raise RuntimeError, "Constant pool index #{index} out of range" if index >= @constants.size
          value = @constants[index]
          raise RuntimeError, "ldc: constant pool entry #{index} is not an integer" unless value.is_a?(Integer)
          @stack.push(value)
          @pc += 2
          make_trace(pc, "ldc", stack_before, "push constant[#{index}] = #{value}")

        when ILOAD_0, ILOAD_1, ILOAD_2, ILOAD_3
          slot = opcode_byte - ILOAD_0
          do_iload(pc, slot, "iload_#{slot}", stack_before)

        when ILOAD
          slot = @bytecode.getbyte(@pc + 1)
          @pc += 1
          do_iload(pc, slot, "iload", stack_before)

        when ISTORE_0, ISTORE_1, ISTORE_2, ISTORE_3
          slot = opcode_byte - ISTORE_0
          do_istore(pc, slot, "istore_#{slot}", stack_before)

        when ISTORE
          slot = @bytecode.getbyte(@pc + 1)
          @pc += 1
          do_istore(pc, slot, "istore", stack_before)

        when IADD
          do_binary_op(pc, "iadd", stack_before) { |a, b| a + b }

        when ISUB
          do_binary_op(pc, "isub", stack_before) { |a, b| a - b }

        when IMUL
          do_binary_op(pc, "imul", stack_before) { |a, b| a * b }

        when IDIV
          raise RuntimeError, "Stack underflow: idiv requires 2 operands" if @stack.size < 2
          raise RuntimeError, "ArithmeticException: division by zero" if @stack.last == 0
          do_binary_op(pc, "idiv", stack_before) { |a, b| (a.to_f / b).truncate }

        when GOTO
          offset = @bytecode.byteslice(@pc + 1, 2).unpack1("s>")
          target = @pc + offset
          @pc = target
          make_trace(pc, "goto", stack_before, "jump to PC=#{target} (offset #{format("%+d", offset)})")

        when IF_ICMPEQ
          do_if_icmp(pc, "if_icmpeq", stack_before) { |a, b| a == b }

        when IF_ICMPGT
          do_if_icmp(pc, "if_icmpgt", stack_before) { |a, b| a > b }

        when IRETURN
          raise RuntimeError, "Stack underflow: ireturn requires 1 operand" if @stack.empty?
          @return_value = @stack.pop
          @halted = true
          @pc += 1
          make_trace(pc, "ireturn", stack_before, "return #{@return_value}")

        when RETURN
          @halted = true
          @pc += 1
          make_trace(pc, "return", stack_before, "return void")

        else
          raise RuntimeError, "Unknown JVM opcode: 0x#{format("%02X", opcode_byte)} at PC=#{@pc}"
        end
      end

      def do_iload(pc, slot, mnemonic, stack_before)
        value = @locals[slot]
        raise RuntimeError, "Local variable #{slot} has not been initialized" if value.nil?
        @stack.push(value)
        @pc += 1
        make_trace(pc, mnemonic, stack_before, "push locals[#{slot}] = #{value}")
      end

      def do_istore(pc, slot, mnemonic, stack_before)
        raise RuntimeError, "Stack underflow: #{mnemonic} requires 1 operand" if @stack.empty?
        value = @stack.pop
        @locals[slot] = value
        @pc += 1
        make_trace(pc, mnemonic, stack_before, "pop #{value}, store in locals[#{slot}]")
      end

      def do_binary_op(pc, mnemonic, stack_before)
        raise RuntimeError, "Stack underflow: #{mnemonic} requires 2 operands" if @stack.size < 2
        b = @stack.pop
        a = @stack.pop
        result = to_i32(yield(a, b))
        @stack.push(result)
        @pc += 1
        make_trace(pc, mnemonic, stack_before, "pop #{b} and #{a}, push #{result}")
      end

      def do_if_icmp(pc, mnemonic, stack_before)
        raise RuntimeError, "Stack underflow: #{mnemonic} requires 2 operands" if @stack.size < 2
        offset = @bytecode.byteslice(@pc + 1, 2).unpack1("s>")
        b = @stack.pop
        a = @stack.pop
        taken = yield(a, b)

        if taken
          target = pc + offset
          @pc = target
          op = mnemonic.include?("eq") ? "==" : ">"
          desc = "pop #{b} and #{a}, #{a} #{op} #{b} is true, jump to PC=#{target}"
        else
          @pc = pc + 3
          op = mnemonic.include?("eq") ? "==" : ">"
          desc = "pop #{b} and #{a}, #{a} #{op} #{b} is false, fall through"
        end

        make_trace(pc, mnemonic, stack_before, desc)
      end

      def make_trace(pc, opcode, stack_before, description)
        JVMTrace.new(pc: pc, opcode: opcode, stack_before: stack_before,
          stack_after: @stack.dup, locals_snapshot: @locals.dup,
          description: description)
      end
    end

    # -------------------------------------------------------------------
    # Encoding helpers
    # -------------------------------------------------------------------

    def self.encode_iconst(n)
      if n >= 0 && n <= 5
        [ICONST_0 + n].pack("C")
      elsif n >= -128 && n <= 127
        raw = n >= 0 ? n : n + 256
        [BIPUSH, raw].pack("CC")
      else
        raise ArgumentError, "encode_iconst: value #{n} outside signed byte range"
      end
    end

    def self.encode_istore(slot)
      if slot >= 0 && slot <= 3
        [ISTORE_0 + slot].pack("C")
      else
        [ISTORE, slot].pack("CC")
      end
    end

    def self.encode_iload(slot)
      if slot >= 0 && slot <= 3
        [ILOAD_0 + slot].pack("C")
      else
        [ILOAD, slot].pack("CC")
      end
    end

    # Assemble JVM bytecode from instruction tuples.
    # Each instruction is [opcode] or [opcode, operand].
    def self.assemble_jvm(*instructions)
      # Single-byte opcodes
      one_byte = [
        ICONST_0, ICONST_1, ICONST_2, ICONST_3, ICONST_4, ICONST_5,
        ILOAD_0, ILOAD_1, ILOAD_2, ILOAD_3,
        ISTORE_0, ISTORE_1, ISTORE_2, ISTORE_3,
        IADD, ISUB, IMUL, IDIV, IRETURN, RETURN
      ].freeze
      two_byte = [BIPUSH, LDC, ILOAD, ISTORE].freeze
      three_byte = [GOTO, IF_ICMPEQ, IF_ICMPGT].freeze

      result = []
      instructions.each do |instr|
        op = instr[0]
        if one_byte.include?(op)
          result << [op].pack("C")
        elsif two_byte.include?(op)
          raise ArgumentError, "Opcode 0x#{format("%02X", op)} requires an operand" if instr.size < 2
          operand = instr[1]
          if op == BIPUSH
            raw = operand >= 0 ? operand : operand + 256
            result << [op, raw & 0xFF].pack("CC")
          else
            result << [op, operand & 0xFF].pack("CC")
          end
        elsif three_byte.include?(op)
          raise ArgumentError, "Opcode 0x#{format("%02X", op)} requires an offset" if instr.size < 2
          offset = instr[1]
          result << [op].pack("C") + [offset].pack("s>")
        else
          raise ArgumentError, "Unknown opcode in assemble_jvm: 0x#{format("%02X", op)}"
        end
      end
      result.join.b
    end
  end
end
