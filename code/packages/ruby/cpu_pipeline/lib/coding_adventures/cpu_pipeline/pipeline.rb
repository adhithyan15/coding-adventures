# frozen_string_literal: true

# The Pipeline class -- the main simulation engine.
#
# = How it Works
#
# The pipeline is an array of "slots", one per stage. Each slot holds a
# PipelineToken (or nil if the stage is empty). On each clock cycle (call
# to step):
#
#   1. Check for hazards (via hazard_fn callback)
#   2. If stalled: freeze stages before the stall point, insert bubble
#   3. If flushing: replace speculative stages with bubbles
#   4. Otherwise: shift all tokens one stage forward
#   5. Execute stage callbacks (fetch, decode, execute, memory, writeback)
#   6. Record a snapshot for tracing
#
# All transitions happen "simultaneously" -- we compute the next state
# from the current state, then swap. This models the behavior of
# edge-triggered flip-flops in real pipeline registers.
#
# = Pipeline Registers
#
# In real hardware, pipeline registers sit BETWEEN stages and latch
# data on the clock edge. In our model, we represent this by computing
# the new state of all stages before committing any changes.

module CodingAdventures
  module CpuPipeline
    class Pipeline
      # Returns the current program counter.
      attr_reader :pc

      # Returns the current cycle number.
      attr_reader :cycle

      # Creates a new pipeline with the given configuration and callbacks.
      #
      # The configuration is validated before use. All five stage callbacks
      # are required; hazard and predict callbacks are optional (set them
      # later via set_hazard_fn and set_predict_fn).
      #
      # Params:
      #   config      - PipelineConfig defining the pipeline stages
      #   fetch_fn    - Proc(pc) -> raw instruction bits
      #   decode_fn   - Proc(raw, token) -> decoded token
      #   execute_fn  - Proc(token) -> token with ALU result
      #   memory_fn   - Proc(token) -> token with memory data
      #   writeback_fn - Proc(token) -> void (writes register file)
      #
      # Raises ArgumentError if the configuration is invalid.
      def initialize(config:, fetch_fn:, decode_fn:, execute_fn:, memory_fn:, writeback_fn:)
        config.validate!

        @config = config
        @stages = Array.new(config.num_stages)
        @pc = 0
        @cycle = 0
        @halted = false
        @stats = PipelineStats.new
        @history = []

        @fetch_fn = fetch_fn
        @decode_fn = decode_fn
        @execute_fn = execute_fn
        @memory_fn = memory_fn
        @writeback_fn = writeback_fn

        @hazard_fn = nil
        @predict_fn = nil
      end

      # Sets the optional hazard detection callback.
      #
      # The hazard function is called at the beginning of each cycle to
      # determine if the pipeline needs to stall or flush.
      #
      # Signature: Proc(stages_array) -> HazardResponse
      def set_hazard_fn(fn)
        @hazard_fn = fn
      end

      # Sets the optional branch prediction callback.
      #
      # The predict function is called during the fetch stage to determine
      # the next PC to fetch from (speculatively, before branch resolution).
      #
      # Signature: Proc(pc) -> predicted next PC
      def set_predict_fn(fn)
        @predict_fn = fn
      end

      # Sets the program counter.
      def set_pc(new_pc)
        @pc = new_pc
      end

      # Returns true if a halt instruction has reached the last stage.
      def halted?
        @halted
      end

      # Returns a copy of the current execution statistics.
      def stats
        @stats
      end

      # Returns the pipeline configuration.
      def config
        @config
      end

      # Returns the token currently occupying the given stage.
      #
      # Returns nil if the stage is empty or the stage name is invalid.
      def stage_contents(stage_name)
        @config.stages.each_with_index do |s, i|
          return @stages[i] if s.name == stage_name
        end
        nil
      end

      # Returns the complete history of pipeline snapshots.
      #
      # The trace includes one snapshot per cycle, in chronological order.
      def trace
        @history.dup
      end

      # Returns the current pipeline state without advancing the clock.
      def snapshot
        take_snapshot
      end

      # Advances the pipeline by one clock cycle.
      #
      # This is the heart of the pipeline simulator. Each call to step
      # corresponds to one rising clock edge in hardware.
      #
      # == Step Algorithm
      #
      #   1. If halted, return the current snapshot (do nothing).
      #   2. Increment the cycle counter.
      #   3. Check for hazards by calling hazard_fn (if set).
      #   4. Handle the hazard response (flush, stall, forward, or none).
      #   5. Advance tokens through stages.
      #   6. Execute stage callbacks on each token.
      #   7. Update statistics.
      #   8. Record a snapshot and return it.
      def step
        return take_snapshot if @halted

        @cycle += 1
        @stats.total_cycles += 1
        num_stages = @config.num_stages

        # --- Phase 1: Check for hazards ---
        #
        # The hazard function examines the CURRENT pipeline state (before any
        # advancement) and returns a verdict: stall, flush, forward, or proceed.
        hazard = HazardResponse.new
        if @hazard_fn
          stages_copy = @stages.dup
          hazard = @hazard_fn.call(stages_copy)
        end

        # --- Phase 2: Compute next state ---
        #
        # We build the next state in a new array, then swap it in at the end.
        # This ensures all transitions are "simultaneous".
        next_stages = Array.new(num_stages)
        stalled = false
        flushing = false

        case hazard.action

        when HazardAction::FLUSH
          # FLUSH: Replace speculative stages with bubbles.
          #
          # A flush happens when a branch misprediction is detected. The
          # instructions fetched after the branch (which were fetched
          # speculatively based on the wrong prediction) must be discarded.
          flushing = true
          @stats.flush_cycles += 1

          # Determine how many stages to flush (from the front).
          flush_count = hazard.flush_count
          if flush_count <= 0
            @config.stages.each_with_index do |s, i|
              if s.category == StageCategory::EXECUTE
                flush_count = i
                break
              end
            end
            flush_count = 1 if flush_count <= 0
          end
          flush_count = num_stages if flush_count > num_stages

          # Shift non-flushed stages forward (from back to front).
          (num_stages - 1).downto(flush_count) do |i|
            if i > 0 && (i - 1) >= flush_count
              next_stages[i] = @stages[i - 1]
            elsif i > 0
              next_stages[i] = CpuPipeline.new_bubble
              next_stages[i].stage_entered[@config.stages[i].name] = @cycle
            else
              next_stages[i] = @stages[i]
            end
          end

          # Replace flushed stages with bubbles.
          (0...flush_count).each do |i|
            next_stages[i] = CpuPipeline.new_bubble
            next_stages[i].stage_entered[@config.stages[i].name] = @cycle
          end

          # Redirect PC and fetch from the correct target.
          @pc = hazard.redirect_pc
          tok = fetch_new_instruction
          next_stages[0] = tok

        when HazardAction::STALL
          # STALL: Freeze earlier stages and insert a bubble.
          #
          # A stall happens when a data hazard cannot be resolved by
          # forwarding -- typically a load-use hazard.
          stalled = true
          @stats.stall_cycles += 1

          # Find the stall insertion point.
          stall_point = hazard.stall_stages
          if stall_point <= 0
            @config.stages.each_with_index do |s, i|
              if s.category == StageCategory::EXECUTE
                stall_point = i
                break
              end
            end
            stall_point = 1 if stall_point <= 0
          end
          stall_point = num_stages - 1 if stall_point >= num_stages

          # Stages AFTER the stall point advance normally.
          (num_stages - 1).downto(stall_point + 1) do |i|
            next_stages[i] = @stages[i - 1]
          end

          # Insert bubble at the stall point.
          next_stages[stall_point] = CpuPipeline.new_bubble
          next_stages[stall_point].stage_entered[@config.stages[stall_point].name] = @cycle

          # Stages BEFORE the stall point are frozen.
          (0...stall_point).each do |i|
            next_stages[i] = @stages[i]
          end

          # PC does NOT advance during a stall.

        else
          # NONE or FORWARD: Normal advancement.
          #
          # Every token moves one stage forward.

          # Handle forwarding if needed.
          if hazard.action == HazardAction::FORWARD_FROM_EX ||
              hazard.action == HazardAction::FORWARD_FROM_MEM
            @config.stages.each_with_index do |s, i|
              if s.category == StageCategory::DECODE &&
                  @stages[i] && !@stages[i].is_bubble
                @stages[i].alu_result = hazard.forward_value
                @stages[i].forwarded_from = hazard.forward_source
                break
              end
            end
          end

          # Shift tokens forward (from back to front).
          (num_stages - 1).downto(1) do |i|
            next_stages[i] = @stages[i - 1]
          end

          # Fetch new instruction into IF stage.
          tok = fetch_new_instruction
          next_stages[0] = tok
        end

        # --- Phase 3: Commit the new state ---
        @stages = next_stages

        # --- Phase 4: Execute stage callbacks ---
        #
        # Now that all tokens are in their new positions, run the
        # stage-specific callbacks. We iterate from LAST to FIRST.
        (num_stages - 1).downto(0) do |i|
          tok = @stages[i]
          next if tok.nil? || tok.is_bubble

          stage = @config.stages[i]

          # Record when this token entered this stage.
          tok.stage_entered[stage.name] = @cycle unless tok.stage_entered.key?(stage.name)

          case stage.category
          when StageCategory::FETCH
            # Already handled by fetch_new_instruction.

          when StageCategory::DECODE
            if tok.opcode.empty?
              @stages[i] = @decode_fn.call(tok.raw_instruction, tok)
            end

          when StageCategory::EXECUTE
            if tok.stage_entered[stage.name] == @cycle
              @stages[i] = @execute_fn.call(tok)
            end

          when StageCategory::MEMORY
            if tok.stage_entered[stage.name] == @cycle
              @stages[i] = @memory_fn.call(tok)
            end

          when StageCategory::WRITEBACK
            # Writeback is handled in Phase 5 (retirement).
          end
        end

        # --- Phase 5: Retire the instruction in the last stage ---
        #
        # The token that is NOW in the last stage (after advancement) gets
        # its writeback callback called.
        last_tok = @stages[num_stages - 1]
        if last_tok && !last_tok.is_bubble
          @writeback_fn.call(last_tok)
          @stats.instructions_completed += 1
          @halted = true if last_tok.is_halt
        end

        # Count bubbles across all stages.
        @stages.each do |t|
          @stats.bubble_cycles += 1 if t && t.is_bubble
        end

        # --- Phase 6: Take snapshot ---
        snap = PipelineSnapshot.new(
          cycle: @cycle,
          stages: {},
          stalled: stalled,
          flushing: flushing,
          pc: @pc
        )
        @config.stages.each_with_index do |stage, i|
          snap.stages[stage.name] = @stages[i].clone if @stages[i]
        end
        @history << snap

        snap
      end

      # Runs the pipeline until a halt instruction is encountered or
      # the maximum cycle count is reached.
      #
      # Returns the final execution statistics.
      def run(max_cycles)
        while @cycle < max_cycles && !@halted
          step
        end
        @stats
      end

      private

      # Creates a new token by calling the fetch callback.
      #
      # This is called at the start of each cycle to fetch the instruction
      # at the current PC. The PC is then advanced (either by the branch
      # predictor's prediction or by the default PC+4).
      def fetch_new_instruction
        tok = CpuPipeline.new_token
        tok.pc = @pc
        tok.raw_instruction = @fetch_fn.call(@pc)
        tok.stage_entered[@config.stages[0].name] = @cycle

        # Advance PC: use branch predictor if available, otherwise PC+4.
        if @predict_fn
          @pc = @predict_fn.call(@pc)
        else
          @pc += 4
        end

        tok
      end

      # Creates a snapshot of the current pipeline state.
      def take_snapshot
        snap = PipelineSnapshot.new(
          cycle: @cycle,
          stages: {},
          pc: @pc
        )
        @config.stages.each_with_index do |stage, i|
          snap.stages[stage.name] = @stages[i].clone if @stages[i]
        end
        snap
      end
    end
  end
end
