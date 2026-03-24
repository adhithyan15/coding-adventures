# frozen_string_literal: true

# === CPU Simulator — the generic processor core ===
#
# Every computer, from the Intel 4004 (1971) to Apple's M-series chips, works
# the same way at the highest level: it runs a loop called the
# **fetch-decode-execute cycle**:
#
#     1. FETCH:   Read the next instruction from memory at the Program Counter (PC)
#     2. DECODE:  Figure out what those bits mean (which operation? which registers?)
#     3. EXECUTE: Do the operation (add, subtract, move data, branch)
#     4. Repeat.
#
# This module provides a *generic* CPU that knows how to do the cycle but does
# NOT know any specific instruction set. You plug in a decoder and executor for
# the ISA you want (ARM, RISC-V, etc.) and the CPU drives the pipeline.
#
# === Design: Protocols via duck typing ===
#
# In Python we used Protocol classes. In Ruby we use duck typing — the decoder
# must respond to `decode(raw_instruction, pc)` and the executor must respond
# to `execute(decoded, registers, memory, pc)`. No formal interface needed;
# Ruby trusts that if it quacks like a duck, it is a duck.
#
# === Why Data.define for traces? ===
#
# Ruby 3.2 introduced Data.define, which creates immutable value objects —
# perfect for trace records that capture a moment in time and should never be
# mutated. This is the Ruby equivalent of Python's @dataclass(frozen=True) or
# Haskell's data types.

module CodingAdventures
  module CpuSimulator
    # -----------------------------------------------------------------------
    # Pipeline stage records — immutable snapshots of each pipeline phase
    # -----------------------------------------------------------------------

    # FetchResult captures what the FETCH stage produced: the PC where we
    # fetched from, and the raw 32-bit instruction word we read from memory.
    #
    # Example:
    #   FetchResult.new(pc: 0, raw_instruction: 0x00100093)
    FetchResult = Data.define(:pc, :raw_instruction)

    # DecodeResult captures what the DECODE stage figured out: the mnemonic
    # (human-readable name like "add" or "mov"), the decoded fields (register
    # numbers, immediates, etc.), and the raw instruction for display.
    #
    # Example:
    #   DecodeResult.new(mnemonic: "addi", fields: {rd: 1, rs1: 0, imm: 1},
    #                    raw_instruction: 0x00100093)
    DecodeResult = Data.define(:mnemonic, :fields, :raw_instruction)

    # ExecuteResult captures what the EXECUTE stage did: a human-readable
    # description, which registers and memory locations changed, the next PC,
    # and whether the CPU should halt.
    #
    # Example:
    #   ExecuteResult.new(description: "x1 = 0 + 1 = 1",
    #                     registers_changed: {"x1" => 1},
    #                     memory_changed: {},
    #                     next_pc: 4,
    #                     halted: false)
    ExecuteResult = Data.define(:description, :registers_changed, :memory_changed,
      :next_pc, :halted) do
      def initialize(description:, registers_changed:, memory_changed:, next_pc:, halted: false)
        super
      end
    end

    # PipelineTrace is the complete record of one instruction's journey through
    # all three pipeline stages. It combines the fetch, decode, and execute
    # results along with a snapshot of all register values after execution.
    #
    # This is the main data structure for visualization — you can print a
    # trace to see exactly what happened at each stage.
    PipelineTrace = Data.define(:cycle, :fetch, :decode, :execute, :register_snapshot) do
      def initialize(cycle:, fetch:, decode:, execute:, register_snapshot: {})
        super
      end

      # Format this trace as a visual pipeline diagram showing all three
      # stages side by side, like a real hardware pipeline visualization.
      def format_pipeline
        fetch_lines = [
          "FETCH",
          format("PC: 0x%04X", fetch.pc),
          format("-> 0x%08X", fetch.raw_instruction)
        ]
        decode_lines = [
          "DECODE",
          decode.mnemonic,
          decode.fields.map { |k, v| "#{k}=#{v}" }.join(" ")
        ]
        execute_lines = [
          "EXECUTE",
          execute.description,
          "PC -> #{execute.next_pc}"
        ]

        max_lines = [fetch_lines.size, decode_lines.size, execute_lines.size].max
        [fetch_lines, decode_lines, execute_lines].each do |lines|
          lines << "" while lines.size < max_lines
        end

        col_width = 20
        result = ["--- Cycle #{cycle} ---"]
        max_lines.times do |i|
          f = fetch_lines[i].ljust(col_width)
          d = decode_lines[i].ljust(col_width)
          e = execute_lines[i].ljust(col_width)
          result << "  #{f} | #{d} | #{e}"
        end
        result.join("\n")
      end
    end

    # -----------------------------------------------------------------------
    # RegisterFile — the CPU's fast, small storage
    # -----------------------------------------------------------------------
    #
    # Registers are the fastest storage in a computer. They sit inside the CPU
    # itself and can be read or written in a single clock cycle. A typical CPU
    # has between 8 and 32 registers, each holding one "word" of data.
    #
    # Think of registers like the small whiteboard on your desk — you can
    # glance at it instantly (fast), but it only holds a few things. Memory
    # (RAM) is like a filing cabinet across the room — much more capacity
    # but slower to access.
    class RegisterFile
      attr_reader :num_registers, :bit_width

      def initialize(num_registers: 16, bit_width: 32)
        raise ArgumentError, "num_registers must be positive" if num_registers < 1
        raise ArgumentError, "bit_width must be positive" if bit_width < 1

        @num_registers = num_registers
        @bit_width = bit_width
        @values = Array.new(num_registers, 0)
        @max_value = (1 << bit_width) - 1
      end

      # Read the value stored in the register at the given index.
      def read(index)
        unless (0...@num_registers).include?(index)
          raise IndexError, "Register index #{index} out of range (0-#{@num_registers - 1})"
        end
        @values[index]
      end

      # Write a value to the register at the given index.
      # Values are masked to the register's bit width.
      def write(index, value)
        unless (0...@num_registers).include?(index)
          raise IndexError, "Register index #{index} out of range (0-#{@num_registers - 1})"
        end
        @values[index] = value & @max_value
      end

      # Return all register values as a hash for inspection.
      # Keys are "R0", "R1", etc.
      def dump
        @values.each_with_index.to_h { |v, i| ["R#{i}", v] }
      end
    end

    # -----------------------------------------------------------------------
    # Memory — the CPU's large, slow storage
    # -----------------------------------------------------------------------
    #
    # Memory (RAM) is a large array of bytes that the CPU can read from and
    # write to. Every byte has an "address" — a number that identifies its
    # location, like a house number on a street.
    #
    # We simulate memory as a Ruby Array of integers (each 0-255). Multi-byte
    # values (like 32-bit integers) are stored in consecutive bytes using
    # little-endian byte order (least significant byte at the lowest address).
    class Memory
      attr_reader :size

      def initialize(size: 65536)
        raise ArgumentError, "Memory size must be at least 1 byte" if size < 1

        @size = size
        @data = Hash.new(0)
      end

      # Read a single byte (0-255) from the given address.
      def read_byte(address)
        check_address(address)
        @data[address]
      end

      # Write a single byte to the given address. Value is masked to 0-255.
      def write_byte(address, value)
        check_address(address)
        @data[address] = value & 0xFF
      end

      # Read a 32-bit word (4 bytes) from memory, little-endian.
      # Little-endian means the least significant byte is at the lowest address.
      def read_word(address)
        check_address(address, 4)
        @data[address] |
          (@data[address + 1] << 8) |
          (@data[address + 2] << 16) |
          (@data[address + 3] << 24)
      end

      # Write a 32-bit word to memory, little-endian.
      def write_word(address, value)
        check_address(address, 4)
        value = value & 0xFFFFFFFF
        @data[address] = value & 0xFF
        @data[address + 1] = (value >> 8) & 0xFF
        @data[address + 2] = (value >> 16) & 0xFF
        @data[address + 3] = (value >> 24) & 0xFF
      end

      # Load a sequence of bytes into memory starting at the given address.
      # This is how programs are loaded into the computer.
      def load_bytes(address, data)
        check_address(address, data.size)
        data.each_byte.with_index do |byte, i|
          @data[address + i] = byte
        end
      end

      # Return a slice of memory as an array of byte values.
      def dump(start = 0, length = 16)
        check_address(start, length)
        Array.new(length) { |i| @data[start + i] }
      end

      private

      def check_address(address, num_bytes = 1)
        if address < 0 || address + num_bytes > @size
          raise IndexError,
            "Memory access out of bounds: address #{address}, " \
            "size #{num_bytes}, memory size #{@size}"
        end
      end
    end

    # -----------------------------------------------------------------------
    # CPU — the central processing unit
    # -----------------------------------------------------------------------
    #
    # The CPU ties everything together: registers, memory, program counter,
    # and the fetch-decode-execute cycle. It does NOT know how to decode
    # specific instructions — that's ISA-specific. Instead, it accepts a
    # decoder and executor via dependency injection (duck typing).
    #
    # This separation means the same CPU can run RISC-V, ARM, WASM, or 4004
    # instructions — you just plug in a different decoder and executor.
    class CPU
      attr_reader :registers, :memory, :halted, :cycle
      attr_accessor :pc

      def initialize(decoder:, executor:, num_registers: 16, bit_width: 32, memory_size: 65536)
        @registers = RegisterFile.new(num_registers: num_registers, bit_width: bit_width)
        @memory = Memory.new(size: memory_size)
        @pc = 0
        @halted = false
        @cycle = 0
        @decoder = decoder
        @executor = executor
      end

      # Capture the current CPU state as an immutable snapshot.
      def state
        {pc: @pc, registers: @registers.dump, halted: @halted, cycle: @cycle}
      end

      # Load machine code bytes into memory and set the PC to the start address.
      def load_program(program, start_address: 0)
        @memory.load_bytes(start_address, program)
        @pc = start_address
      end

      # Execute ONE instruction through the full fetch-decode-execute pipeline.
      # Returns a PipelineTrace showing what happened at each stage.
      def step
        raise RuntimeError, "CPU has halted — no more instructions to execute" if @halted

        # === STAGE 1: FETCH ===
        raw_instruction = @memory.read_word(@pc)
        fetch_result = FetchResult.new(pc: @pc, raw_instruction: raw_instruction)

        # === STAGE 2: DECODE ===
        decode_result = @decoder.decode(raw_instruction, @pc)

        # === STAGE 3: EXECUTE ===
        execute_result = @executor.execute(decode_result, @registers, @memory, @pc)

        # === UPDATE CPU STATE ===
        @pc = execute_result.next_pc
        @halted = execute_result.halted

        trace = PipelineTrace.new(
          cycle: @cycle,
          fetch: fetch_result,
          decode: decode_result,
          execute: execute_result,
          register_snapshot: @registers.dump
        )

        @cycle += 1
        trace
      end

      # Run the CPU until it halts or hits the step limit.
      # Returns a list of PipelineTrace objects.
      def run(max_steps: 10_000)
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
