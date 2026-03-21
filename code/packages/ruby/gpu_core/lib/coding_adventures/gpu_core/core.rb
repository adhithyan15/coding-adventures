# frozen_string_literal: true

# ---------------------------------------------------------------------------
# GPUCore -- the generic, pluggable accelerator processing element.
# ---------------------------------------------------------------------------
#
# === What is a GPU Core? ===
#
# A GPU core is the smallest independently programmable compute unit on a GPU.
# It's like a tiny, simplified CPU that does one thing well: floating-point math.
#
#     CPU Core (complex):                    GPU Core (simple):
#     +------------------------+             +----------------------+
#     | Branch predictor       |             |                      |
#     | Out-of-order engine    |             | In-order execution   |
#     | Large cache hierarchy  |             | Small register file  |
#     | Integer + FP ALUs      |             | FP ALU only          |
#     | Complex decoder        |             | Simple fetch-execute  |
#     | Speculative execution  |             | No speculation       |
#     +------------------------+             +----------------------+
#
# A single GPU core is MUCH simpler than a CPU core. GPUs achieve performance
# not through per-core complexity, but through massive parallelism: thousands
# of these simple cores running in parallel.
#
# === How This Core is Pluggable ===
#
# The GPUCore takes an ISA (Instruction Set Architecture) as a constructor
# parameter. This ISA object handles all the vendor-specific decode and
# execute logic:
#
#     # Generic educational ISA (this package)
#     core = GPUCore.new(isa: GenericISA.new)
#
#     # NVIDIA PTX (future package)
#     core = GPUCore.new(isa: PTXISA.new, num_registers: 255)
#
#     # AMD GCN (future package)
#     core = GPUCore.new(isa: GCNISA.new, num_registers: 256)
#
# The core itself (fetch loop, registers, memory, tracing) stays the same.
# Only the ISA changes.
#
# === Execution Model ===
#
# The GPU core uses a simple fetch-execute loop (no separate decode stage):
#
#     +------------------------------------------+
#     |              GPU Core                     |
#     |                                          |
#     |  +---------+    +------------------+     |
#     |  | Program  |--->|   Fetch          |     |
#     |  | Memory   |    |   instruction    |     |
#     |  +---------+    |   at PC          |     |
#     |                  +--------+---------+     |
#     |                           |               |
#     |                  +--------v---------+     |
#     |  +-----------+  |   ISA.execute()  |     |
#     |  | Register  |<-|   (pluggable!)   |---->| Trace
#     |  | File      |->|                  |     |
#     |  +-----------+  +--------+---------+     |
#     |                           |               |
#     |  +-----------+  +--------v---------+     |
#     |  |  Local    |<-|  Update PC       |     |
#     |  |  Memory   |  +------------------+     |
#     |  +-----------+                           |
#     +------------------------------------------+
#
# Each step():
# 1. Fetch: read instruction at program[PC]
# 2. Execute: call isa.execute(instruction, registers, memory)
# 3. Update PC: advance based on ExecuteResult (branch or +1)
# 4. Return trace: GPUCoreTrace with full execution details

module CodingAdventures
  module GpuCore
    # A generic GPU processing element with a pluggable instruction set.
    #
    # This is the central class of the package. It simulates a single
    # processing element -- one CUDA core, one AMD stream processor, one
    # Intel vector engine, or one ARM Mali execution engine -- depending
    # on which ISA you plug in.
    #
    # @param isa [#name, #execute] The instruction set to use (default: GenericISA).
    # @param fmt [FloatFormat] Floating-point format for registers (default: FP32).
    # @param num_registers [Integer] Number of FP registers (default: 32, max: 256).
    # @param memory_size [Integer] Local memory size in bytes (default: 4096).
    #
    # Example:
    #     core = GPUCore.new(isa: GenericISA.new)
    #     core.load_program([
    #       GpuCore.limm(0, 3.0),
    #       GpuCore.limm(1, 4.0),
    #       GpuCore.fmul(2, 0, 1),
    #       GpuCore.halt,
    #     ])
    #     traces = core.run
    #     core.registers.read_float(2)  # => 12.0
    class GPUCore
      attr_reader :isa, :fmt, :registers, :memory
      attr_accessor :pc, :cycle

      def initialize(isa: nil, fmt: FpArithmetic::FP32, num_registers: 32, memory_size: 4096)
        @isa = isa || GenericISA.new
        @fmt = fmt
        @registers = FPRegisterFile.new(num_registers: num_registers, fmt: fmt)
        @memory = LocalMemory.new(size: memory_size)
        @pc = 0
        @cycle = 0
        @halted = false
        @program = []
      end

      # True if the core has executed a HALT instruction.
      def halted?
        @halted
      end

      # Load a program (list of instructions) into the core.
      #
      # This replaces any previously loaded program and resets the PC to 0,
      # but does NOT reset registers or memory. Call reset for a full reset.
      #
      # @param program [Array<Instruction>] A list of Instruction objects to execute.
      def load_program(program)
        @program = program.dup
        @pc = 0
        @halted = false
        @cycle = 0
      end

      # Execute one instruction and return a trace of what happened.
      #
      # This is the core fetch-execute loop:
      # 1. Check if halted or PC out of range
      # 2. Fetch instruction at PC
      # 3. Call ISA.execute to perform the operation
      # 4. Update PC based on the result
      # 5. Build and return a trace record
      #
      # @return [GPUCoreTrace] describing what this instruction did.
      # @raise [RuntimeError] If the core is halted or PC is out of range.
      def step
        if @halted
          raise RuntimeError, "Cannot step: core is halted"
        end

        if @pc < 0 || @pc >= @program.length
          raise RuntimeError, "PC=#{@pc} out of program range [0, #{@program.length})"
        end

        # Fetch
        instruction = @program[@pc]
        current_pc = @pc
        @cycle += 1

        # Execute (delegated to the pluggable ISA)
        result = @isa.execute(instruction, @registers, @memory)

        # Update PC
        if result.halted
          @halted = true
          next_pc = current_pc  # PC doesn't advance on halt
        elsif result.absolute_jump
          next_pc = result.next_pc_offset
        else
          next_pc = current_pc + result.next_pc_offset
        end
        @pc = next_pc

        # Build trace
        GPUCoreTrace.new(
          cycle: @cycle,
          pc: current_pc,
          instruction: instruction,
          description: result.description,
          next_pc: next_pc,
          halted: result.halted,
          registers_changed: result.registers_changed || {},
          memory_changed: result.memory_changed || {}
        )
      end

      # Execute the program until HALT or max_steps reached.
      #
      # This repeatedly calls step until the core halts or the step
      # limit is reached (preventing infinite loops from hanging).
      #
      # @param max_steps [Integer] Maximum number of instructions to execute.
      # @return [Array<GPUCoreTrace>] A list of trace records, one per instruction.
      # @raise [RuntimeError] If max_steps is exceeded (likely an infinite loop).
      def run(max_steps: 10_000)
        traces = []
        steps = 0

        while !@halted && steps < max_steps
          traces << step
          steps += 1
        end

        if !@halted && steps >= max_steps
          raise RuntimeError,
            "Execution limit reached (#{max_steps} steps). " \
            "Possible infinite loop. Last PC=#{@pc}"
        end

        traces
      end

      # Reset the core to its initial state.
      #
      # Clears registers, memory, PC, and cycle count. The loaded program
      # is preserved -- call load_program to change it.
      def reset
        @registers = FPRegisterFile.new(
          num_registers: @registers.num_registers,
          fmt: @fmt
        )
        @memory = LocalMemory.new(size: @memory.size)
        @pc = 0
        @cycle = 0
        @halted = false
      end

      # Human-readable representation of the core state.
      def to_s
        status = @halted ? "halted" : "running at PC=#{@pc}"
        "GPUCore(isa=#{@isa.name}, regs=#{@registers.num_registers}, fmt=#{@fmt.name}, #{status})"
      end

      def inspect
        to_s
      end
    end
  end
end
