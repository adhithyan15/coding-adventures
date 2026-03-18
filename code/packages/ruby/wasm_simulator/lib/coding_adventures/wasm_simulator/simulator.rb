# frozen_string_literal: true

# === WebAssembly (WASM) Simulator — a modern stack-based virtual machine ===
#
# WebAssembly is a binary instruction format designed as a portable compilation
# target for the web. Standardized by the W3C in 2017, it runs in all major
# browsers. Languages like Rust, C++, and Go compile down to WASM.
#
# === Stack machines vs register machines ===
#
# RISC-V is a register machine: instructions name specific registers.
# WASM is a stack machine: operands live on an implicit operand stack.
#
#   Register machine (RISC-V):       Stack machine (WASM):
#     addi x1, x0, 1                   i32.const 1
#     addi x2, x0, 2                   i32.const 2
#     add  x3, x1, x2                  i32.add
#                                       local.set 0
#
# Both compute "x = 1 + 2", but the stack machine never names a destination.
#
# === Variable-width encoding ===
#
# Unlike RISC-V (fixed 32-bit), WASM instructions are variable-width:
#   i32.const V  = 0x41 + 4-byte LE value  (5 bytes)
#   i32.add      = 0x6A                     (1 byte)
#   i32.sub      = 0x6B                     (1 byte)
#   local.get N  = 0x20 N                   (2 bytes)
#   local.set N  = 0x21 N                   (2 bytes)
#   end          = 0x0B                     (1 byte)

module CodingAdventures
  module WasmSimulator
    # Opcode constants — real WASM opcode byte values
    OP_END       = 0x0B
    OP_LOCAL_GET = 0x20
    OP_LOCAL_SET = 0x21
    OP_I32_CONST = 0x41
    OP_I32_ADD   = 0x6A
    OP_I32_SUB   = 0x6B

    # Decoded WASM instruction — immutable record
    WasmInstruction = Data.define(:opcode, :mnemonic, :operand, :size)

    # Trace of one instruction execution — immutable record
    WasmStepTrace = Data.define(:pc, :instruction, :stack_before, :stack_after,
      :locals_snapshot, :description, :halted) do
      def initialize(pc:, instruction:, stack_before:, stack_after:,
        locals_snapshot:, description:, halted: false)
        super
      end
    end

    # -------------------------------------------------------------------
    # Decoder
    # -------------------------------------------------------------------
    class WasmDecoder
      def decode(bytecode, pc)
        opcode = bytecode.getbyte(pc)

        case opcode
        when OP_I32_CONST
          bytes = bytecode.byteslice(pc + 1, 4)
          value = bytes.unpack1("l<") # little-endian signed 32-bit
          WasmInstruction.new(opcode: opcode, mnemonic: "i32.const", operand: value, size: 5)
        when OP_I32_ADD
          WasmInstruction.new(opcode: opcode, mnemonic: "i32.add", operand: nil, size: 1)
        when OP_I32_SUB
          WasmInstruction.new(opcode: opcode, mnemonic: "i32.sub", operand: nil, size: 1)
        when OP_LOCAL_GET
          index = bytecode.getbyte(pc + 1)
          WasmInstruction.new(opcode: opcode, mnemonic: "local.get", operand: index, size: 2)
        when OP_LOCAL_SET
          index = bytecode.getbyte(pc + 1)
          WasmInstruction.new(opcode: opcode, mnemonic: "local.set", operand: index, size: 2)
        when OP_END
          WasmInstruction.new(opcode: opcode, mnemonic: "end", operand: nil, size: 1)
        else
          raise ArgumentError, "Unknown WASM opcode: 0x#{format("%02X", opcode)} at PC=#{pc}"
        end
      end
    end

    # -------------------------------------------------------------------
    # Executor
    # -------------------------------------------------------------------
    class WasmExecutor
      def execute(instruction, stack, locals, pc)
        stack_before = stack.dup

        case instruction.mnemonic
        when "i32.const"
          value = instruction.operand
          stack.push(value)
          WasmStepTrace.new(pc: pc, instruction: instruction,
            stack_before: stack_before, stack_after: stack.dup,
            locals_snapshot: locals.dup, description: "push #{value}")
        when "i32.add"
          b = stack.pop
          a = stack.pop
          result = (a + b) & 0xFFFFFFFF
          stack.push(result)
          WasmStepTrace.new(pc: pc, instruction: instruction,
            stack_before: stack_before, stack_after: stack.dup,
            locals_snapshot: locals.dup, description: "pop #{b} and #{a}, push #{result}")
        when "i32.sub"
          b = stack.pop
          a = stack.pop
          result = (a - b) & 0xFFFFFFFF
          stack.push(result)
          WasmStepTrace.new(pc: pc, instruction: instruction,
            stack_before: stack_before, stack_after: stack.dup,
            locals_snapshot: locals.dup, description: "pop #{b} and #{a}, push #{result}")
        when "local.get"
          index = instruction.operand
          value = locals[index]
          stack.push(value)
          WasmStepTrace.new(pc: pc, instruction: instruction,
            stack_before: stack_before, stack_after: stack.dup,
            locals_snapshot: locals.dup, description: "push locals[#{index}] = #{value}")
        when "local.set"
          index = instruction.operand
          value = stack.pop
          locals[index] = value
          WasmStepTrace.new(pc: pc, instruction: instruction,
            stack_before: stack_before, stack_after: stack.dup,
            locals_snapshot: locals.dup, description: "pop #{value}, store in locals[#{index}]")
        when "end"
          WasmStepTrace.new(pc: pc, instruction: instruction,
            stack_before: stack_before, stack_after: stack.dup,
            locals_snapshot: locals.dup, description: "halt", halted: true)
        else
          raise ArgumentError, "Cannot execute: #{instruction.mnemonic}"
        end
      end
    end

    # -------------------------------------------------------------------
    # Encoding helpers
    # -------------------------------------------------------------------

    def self.encode_i32_const(value)
      [OP_I32_CONST].pack("C") + [value].pack("l<")
    end

    def self.encode_i32_add
      [OP_I32_ADD].pack("C")
    end

    def self.encode_i32_sub
      [OP_I32_SUB].pack("C")
    end

    def self.encode_local_get(index)
      [OP_LOCAL_GET, index].pack("CC")
    end

    def self.encode_local_set(index)
      [OP_LOCAL_SET, index].pack("CC")
    end

    def self.encode_end
      [OP_END].pack("C")
    end

    def self.assemble_wasm(instructions)
      instructions.join.b
    end

    # -------------------------------------------------------------------
    # Simulator
    # -------------------------------------------------------------------
    class WasmSimulator
      attr_reader :stack, :locals, :halted, :cycle
      attr_accessor :pc

      def initialize(num_locals: 4)
        @stack = []
        @locals = Array.new(num_locals, 0)
        @pc = 0
        @bytecode = "".b
        @halted = false
        @cycle = 0
        @decoder = WasmDecoder.new
        @executor = WasmExecutor.new
      end

      def load(bytecode)
        @bytecode = bytecode.b
        @pc = 0
        @halted = false
        @cycle = 0
        @stack.clear
        @locals.fill(0)
      end

      def step
        raise RuntimeError, "WASM simulator has halted" if @halted

        instruction = @decoder.decode(@bytecode, @pc)
        trace = @executor.execute(instruction, @stack, @locals, @pc)
        @pc += instruction.size
        @halted = trace.halted
        @cycle += 1
        trace
      end

      def run(program, max_steps: 10_000)
        load(program)
        traces = []
        max_steps.times do
          break if @halted
          traces << step
        end
        traces
      end
    end
  end
end
