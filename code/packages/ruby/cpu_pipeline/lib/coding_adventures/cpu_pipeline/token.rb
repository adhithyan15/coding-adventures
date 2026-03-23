# frozen_string_literal: true

# Token, stage, and configuration types for the CPU pipeline.
#
# = The Pipeline: a CPU's Assembly Line
#
# A CPU pipeline is the central execution engine of a processor core. Instead
# of completing one instruction fully before starting the next (like a
# single-cycle CPU), a pipelined CPU overlaps instruction execution -- while
# one instruction is being executed, the next is being decoded, and the one
# after that is being fetched.
#
# This is the same principle as a factory assembly line:
#
#   Single-cycle (no pipeline):
#   Instr 1: [IF][ID][EX][MEM][WB]
#   Instr 2:                       [IF][ID][EX][MEM][WB]
#   Throughput: 1 instruction every 5 cycles
#
#   Pipelined:
#   Instr 1: [IF][ID][EX][MEM][WB]
#   Instr 2:     [IF][ID][EX][MEM][WB]
#   Instr 3:         [IF][ID][EX][MEM][WB]
#   Throughput: 1 instruction every 1 cycle (after filling)

module CodingAdventures
  module CpuPipeline
    # =====================================================================
    # StageCategory -- classifies pipeline stages by their function
    # =====================================================================
    #
    # Every stage in a pipeline does one of these five jobs, regardless of
    # how many stages the pipeline has. A 5-stage pipeline has one stage per
    # category. A 13-stage pipeline might have 2 fetch stages, 2 decode
    # stages, 3 execute stages, etc.
    #
    # This classification is used for:
    #   - Determining which callback to invoke for each stage
    #   - Knowing where to insert stall bubbles
    #   - Knowing which stages to flush on a misprediction
    module StageCategory
      FETCH     = :fetch
      DECODE    = :decode
      EXECUTE   = :execute
      MEMORY    = :memory
      WRITEBACK = :writeback

      # All valid categories, in pipeline order.
      ALL = [FETCH, DECODE, EXECUTE, MEMORY, WRITEBACK].freeze

      # Returns a human-readable name for the category.
      def self.to_s(category)
        category.to_s
      end
    end

    # =====================================================================
    # PipelineStage -- definition of a single stage in the pipeline
    # =====================================================================
    #
    # A stage has a short name (used in diagrams), a description (for humans),
    # and a category (for the pipeline to know what callback to invoke).
    #
    # Example stages:
    #   PipelineStage.new(name: "IF",  description: "Instruction Fetch", category: :fetch)
    #   PipelineStage.new(name: "EX1", description: "Execute - ALU",     category: :execute)
    class PipelineStage
      attr_reader :name, :description, :category

      def initialize(name:, description:, category:)
        @name = name
        @description = description
        @category = category
      end

      # Returns the stage name for display in diagrams.
      def to_s
        @name
      end
    end

    # =====================================================================
    # PipelineToken -- a unit of work flowing through the pipeline
    # =====================================================================
    #
    # Think of it as a tray on an assembly line. The tray starts empty at the
    # IF stage, gets filled with decoded information at ID, gets computed
    # results at EX, gets memory data at MEM, and delivers results at WB.
    #
    # The token is ISA-independent. The ISA decoder fills in the fields via
    # callbacks. The pipeline itself never looks at instruction semantics --
    # it only moves tokens between stages and handles stalls/flushes.
    #
    # == Token Lifecycle
    #
    #   IF stage:  fetch_fn fills in pc and raw_instruction
    #   ID stage:  decode_fn fills in opcode, registers, control signals
    #   EX stage:  execute_fn fills in alu_result, branch_taken, branch_target
    #   MEM stage: memory_fn fills in mem_data (for loads)
    #   WB stage:  writeback_fn uses write_data to update register file
    #
    # == Bubbles
    #
    # A "bubble" is a special token that represents NO instruction. Bubbles
    # are inserted when the pipeline stalls (to fill the gap left by frozen
    # stages) or when the pipeline flushes (to replace discarded speculative
    # instructions). A bubble flows through the pipeline like a normal token
    # but does nothing at each stage.
    #
    # In hardware, a bubble is a NOP (no-operation) instruction. In our
    # simulator, it is a token with is_bubble = true.
    class PipelineToken
      # --- Instruction identity ---
      attr_accessor :pc,              # Program counter (memory address of this instruction)
        :raw_instruction, # Raw instruction bits as fetched from memory
        :opcode           # Decoded instruction name (e.g., "ADD", "LDR", "BEQ")

      # --- Decoded operands (set by ID stage callback) ---
      attr_accessor :rs1,       # First source register number (-1 means unused)
        :rs2,       # Second source register number (-1 means unused)
        :rd,        # Destination register number (-1 means unused)
        :immediate  # Sign-extended immediate value from the instruction

      # --- Control signals (set by ID stage callback) ---
      #
      # Truth table for control signals:
      #   ADD  R1, R2, R3  -> reg_write=true,  mem_read=false, mem_write=false
      #   STR  R1, [R2]    -> reg_write=false, mem_read=false, mem_write=true
      #   BEQ  R1, R2, L   -> reg_write=false, mem_read=false, mem_write=false, is_branch=true
      #   LDR  R1, [R2]    -> reg_write=true,  mem_read=true,  mem_write=false
      attr_accessor :reg_write,  # True if this instruction writes a register
        :mem_read,   # True if this instruction reads from data memory
        :mem_write,  # True if this instruction writes to data memory
        :is_branch,  # True if this instruction is a branch
        :is_halt     # True if this is a halt/stop instruction

      # --- Computed values (filled during execution) ---
      attr_accessor :alu_result,    # Output of the ALU in the EX stage
        :mem_data,      # Data read from memory in the MEM stage
        :write_data,    # Final value to write to the destination register
        :branch_taken,  # True if the branch was actually taken
        :branch_target  # Actual branch target address

      # --- Pipeline metadata ---
      attr_accessor :is_bubble,      # True if this token is a NOP/bubble
        :stage_entered,  # Hash mapping stage name -> cycle number
        :forwarded_from  # Stage that provided a forwarded value (or empty string)

      def initialize(is_bubble: false)
        @pc = 0
        @raw_instruction = 0
        @opcode = ""
        @rs1 = -1
        @rs2 = -1
        @rd = -1
        @immediate = 0
        @reg_write = false
        @mem_read = false
        @mem_write = false
        @is_branch = false
        @is_halt = false
        @alu_result = 0
        @mem_data = 0
        @write_data = 0
        @branch_taken = false
        @branch_target = 0
        @is_bubble = is_bubble
        @stage_entered = {}
        @forwarded_from = ""
      end

      # Returns a human-readable representation of the token.
      #
      # For debugging and pipeline diagrams:
      #   - Bubbles display as "---" (like empty slots on the assembly line)
      #   - Normal tokens display their opcode and PC
      def to_s
        return "---" if @is_bubble
        return "#{@opcode}@#{@pc}" unless @opcode.empty?

        "instr@#{@pc}"
      end

      # Returns a deep copy of the token.
      #
      # This is necessary because tokens are passed between pipeline stages
      # via pipeline registers. Each register holds its own copy so that
      # modifying a token in one stage does not affect the copy in the
      # pipeline register.
      def clone
        copy = super
        copy.instance_variable_set(:@stage_entered, @stage_entered.dup)
        copy
      end
    end

    # Creates a new bubble token.
    #
    # A bubble is a "do nothing" instruction that occupies a pipeline stage
    # without performing any useful work. It is the pipeline equivalent of
    # a "no-op" on an assembly line.
    def self.new_bubble
      PipelineToken.new(is_bubble: true)
    end

    # Creates a new empty token with default register values.
    #
    # The token starts with all register fields set to -1 (unused) and
    # all control signals set to false.
    def self.new_token
      PipelineToken.new
    end

    # =====================================================================
    # PipelineConfig -- configuration for the pipeline
    # =====================================================================
    #
    # The key insight: a pipeline's behavior is determined entirely by its
    # stage configuration and execution width. Everything else (instruction
    # semantics, hazard handling) is injected via callbacks.
    class PipelineConfig
      attr_reader :stages, :execution_width

      def initialize(stages:, execution_width: 1)
        @stages = stages
        @execution_width = execution_width
      end

      # Returns the number of stages in the pipeline.
      def num_stages
        @stages.length
      end

      # Validates that the configuration is well-formed.
      #
      # Rules:
      #   - Must have at least 2 stages (a 1-stage "pipeline" is not a pipeline)
      #   - Execution width must be at least 1
      #   - All stage names must be unique
      #   - There must be at least one fetch stage and one writeback stage
      #
      # Returns nil if valid, or an error message string if invalid.
      def validate
        if @stages.length < 2
          return "pipeline must have at least 2 stages, got #{@stages.length}"
        end

        if @execution_width < 1
          return "execution width must be at least 1, got #{@execution_width}"
        end

        # Check for unique stage names.
        seen = {}
        @stages.each do |s|
          if seen[s.name]
            return "duplicate stage name: #{s.name.inspect}"
          end
          seen[s.name] = true
        end

        # Check for required categories.
        has_fetch = @stages.any? { |s| s.category == StageCategory::FETCH }
        has_writeback = @stages.any? { |s| s.category == StageCategory::WRITEBACK }

        return "pipeline must have at least one fetch stage" unless has_fetch
        return "pipeline must have at least one writeback stage" unless has_writeback

        nil
      end

      # Raises an error if the configuration is invalid.
      def validate!
        error = validate
        raise ArgumentError, error if error
      end
    end

    # Returns the standard 5-stage RISC pipeline configuration.
    #
    # This is the pipeline described in every computer architecture textbook:
    #
    #   IF -> ID -> EX -> MEM -> WB
    #
    # It matches the MIPS R2000 (1985) and is the foundation for understanding
    # all modern CPU pipelines.
    def self.classic_5_stage
      PipelineConfig.new(
        stages: [
          PipelineStage.new(name: "IF", description: "Instruction Fetch", category: StageCategory::FETCH),
          PipelineStage.new(name: "ID", description: "Instruction Decode", category: StageCategory::DECODE),
          PipelineStage.new(name: "EX", description: "Execute", category: StageCategory::EXECUTE),
          PipelineStage.new(name: "MEM", description: "Memory Access", category: StageCategory::MEMORY),
          PipelineStage.new(name: "WB", description: "Write Back", category: StageCategory::WRITEBACK)
        ],
        execution_width: 1
      )
    end

    # Returns a 13-stage pipeline inspired by ARM Cortex-A78.
    #
    # Modern high-performance CPUs split the classic 5 stages into many
    # sub-stages to enable higher clock frequencies. Each sub-stage does
    # less work, so it completes faster, allowing a faster clock.
    #
    # The tradeoff: a branch misprediction now costs 10+ cycles instead of 2.
    def self.deep_13_stage
      PipelineConfig.new(
        stages: [
          PipelineStage.new(name: "IF1", description: "Fetch 1 - TLB lookup", category: StageCategory::FETCH),
          PipelineStage.new(name: "IF2", description: "Fetch 2 - cache read", category: StageCategory::FETCH),
          PipelineStage.new(name: "IF3", description: "Fetch 3 - align/buffer", category: StageCategory::FETCH),
          PipelineStage.new(name: "ID1", description: "Decode 1 - pre-decode", category: StageCategory::DECODE),
          PipelineStage.new(name: "ID2", description: "Decode 2 - full decode", category: StageCategory::DECODE),
          PipelineStage.new(name: "ID3", description: "Decode 3 - register read", category: StageCategory::DECODE),
          PipelineStage.new(name: "EX1", description: "Execute 1 - ALU", category: StageCategory::EXECUTE),
          PipelineStage.new(name: "EX2", description: "Execute 2 - shift/multiply", category: StageCategory::EXECUTE),
          PipelineStage.new(name: "EX3", description: "Execute 3 - result select", category: StageCategory::EXECUTE),
          PipelineStage.new(name: "MEM1", description: "Memory 1 - address calc", category: StageCategory::MEMORY),
          PipelineStage.new(name: "MEM2", description: "Memory 2 - cache access", category: StageCategory::MEMORY),
          PipelineStage.new(name: "MEM3", description: "Memory 3 - data align", category: StageCategory::MEMORY),
          PipelineStage.new(name: "WB", description: "Write Back", category: StageCategory::WRITEBACK)
        ],
        execution_width: 1
      )
    end
  end
end
