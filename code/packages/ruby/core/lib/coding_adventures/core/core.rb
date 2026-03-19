# frozen_string_literal: true

# Core -- a configurable processor core that composes all D-series
# sub-components.
#
# = What the Core Does
#
# The Core wires together:
#   - Pipeline (D04): manages instruction flow through stages
#   - Branch Predictor (D02): speculative fetch direction
#   - Hazard Unit (D03): data, control, and structural hazard detection
#   - Cache Hierarchy (D01): L1I + L1D + optional L2
#   - Register File: fast operand storage
#   - Clock: cycle-accurate timing
#   - Memory Controller: access to backing memory
#
# The Core provides callback functions to the pipeline. When the pipeline
# needs to fetch an instruction, it calls the Core's fetch callback, which
# reads from the L1I cache. When it needs to decode, it calls the ISA
# decoder. And so on.
#
# = ISA Independence
#
# The Core does not know what instructions mean. The ISA decoder provides
# instruction semantics. The same Core can run ARM, RISC-V, or any custom
# ISA by swapping the decoder.
#
# = Usage
#
#   config = CodingAdventures::Core.simple_config
#   decoder = CodingAdventures::Core::MockDecoder.new
#   core = CodingAdventures::Core::Core.new(config, decoder)
#   program = CodingAdventures::Core.encode_program(
#     CodingAdventures::Core.encode_addi(1, 0, 42),
#     CodingAdventures::Core.encode_halt
#   )
#   core.load_program(program, 0)
#   stats = core.run(100)

module CodingAdventures
  module Core
    class Core
      # @return [CoreConfig] the core configuration.
      attr_reader :config

      # @return [RegisterFile] the general-purpose register file.
      attr_reader :reg_file

      # @return [MemoryController] the memory controller.
      attr_reader :mem_ctrl

      # @return [Integer] the current cycle number.
      attr_reader :cycle

      # Creates a fully-wired processor core from the given configuration
      # and ISA decoder.
      #
      # @param config [CoreConfig] core configuration.
      # @param decoder [Object] ISA decoder responding to decode, execute, instruction_size.
      def initialize(config, decoder)
        @config = config
        @decoder = decoder

        # --- 1. Register File ---
        @reg_file = RegisterFile.new(config.register_file)

        # --- 2. Memory ---
        mem_size = config.memory_size
        mem_size = 65536 if mem_size <= 0
        memory = Array.new(mem_size, 0)
        mem_latency = config.memory_latency
        mem_latency = 100 if mem_latency <= 0
        @mem_ctrl = MemoryController.new(memory, mem_latency)

        # --- 3. Cache Hierarchy ---
        @cache_hierarchy = build_cache_hierarchy(config, mem_latency)

        # --- 4. Branch Predictor + BTB ---
        @predictor = CodingAdventures::Core.create_branch_predictor(
          config.branch_predictor_type,
          config.branch_predictor_size
        )
        btb_size = config.btb_size
        btb_size = 64 if btb_size <= 0
        @btb = CodingAdventures::BranchPredictor::BranchTargetBuffer.new(size: btb_size)

        # --- 5. Hazard Unit ---
        num_fp_units = config.fp_unit ? 1 : 0
        @hazard_unit = CodingAdventures::HazardDetection::HazardUnit.new(
          num_alus: 1, num_fp_units: num_fp_units, split_caches: true
        )

        # --- 6. Pipeline ---
        pipeline_config = config.pipeline
        pipeline_config = CodingAdventures::CpuPipeline.classic_5_stage if pipeline_config.nil? || pipeline_config.stages.empty?

        @pipeline = CodingAdventures::CpuPipeline::Pipeline.new(
          config: pipeline_config,
          fetch_fn: method(:fetch_callback),
          decode_fn: method(:decode_callback),
          execute_fn: method(:execute_callback),
          memory_fn: method(:memory_callback),
          writeback_fn: method(:writeback_callback)
        )

        # Wire optional callbacks.
        if config.hazard_detection
          @pipeline.set_hazard_fn(method(:hazard_callback))
        end
        @pipeline.set_predict_fn(method(:predict_callback))

        # --- 7. Clock ---
        @clk = CodingAdventures::Clock::ClockGenerator.new

        # --- Tracking ---
        @halted = false
        @cycle = 0
        @instructions_completed = 0
        @forward_count = 0
        @stall_count = 0
        @flush_count = 0
      end

      # Loads machine code into memory starting at the given address.
      #
      # @param program [Array<Integer>] byte array of program data.
      # @param start_address [Integer] memory address to load at.
      def load_program(program, start_address)
        @mem_ctrl.load_program(program, start_address)
        @pipeline.set_pc(start_address)
      end

      # Executes one clock cycle.
      #
      # Returns the pipeline snapshot for this cycle.
      #
      # @return [CodingAdventures::CpuPipeline::PipelineSnapshot] snapshot.
      def step
        return @pipeline.snapshot if @halted

        @cycle += 1
        snap = @pipeline.step

        # Check if the pipeline halted this cycle.
        @halted = true if @pipeline.halted?

        # Track completed instructions.
        @instructions_completed = @pipeline.stats.instructions_completed

        snap
      end

      # Runs the core until it halts or max_cycles is reached.
      #
      # Returns aggregate statistics for the entire run.
      #
      # @param max_cycles [Integer] maximum number of cycles to run.
      # @return [CoreStats] aggregate statistics.
      def run(max_cycles)
        while @cycle < max_cycles && !@halted
          step
        end
        stats
      end

      # Collects aggregate statistics from all sub-components.
      #
      # @return [CoreStats] aggregate statistics.
      def stats
        p_stats = @pipeline.stats

        s = CoreStats.new
        s.instructions_completed = p_stats.instructions_completed
        s.total_cycles = p_stats.total_cycles
        s.pipeline_stats = p_stats
        s.predictor_stats = @predictor.stats
        s.cache_stats = {}
        s.forward_count = @forward_count
        s.stall_count = @stall_count
        s.flush_count = @flush_count

        # Collect cache stats.
        s.cache_stats["L1I"] = @cache_hierarchy.l1i.stats if @cache_hierarchy.l1i
        s.cache_stats["L1D"] = @cache_hierarchy.l1d.stats if @cache_hierarchy.l1d
        s.cache_stats["L2"] = @cache_hierarchy.l2.stats if @cache_hierarchy.l2

        s
      end

      # Returns true if a halt instruction has completed.
      #
      # @return [Boolean] whether the core is halted.
      def halted?
        @halted
      end

      # Reads a general-purpose register.
      #
      # @param index [Integer] register index.
      # @return [Integer] register value.
      def read_register(index)
        @reg_file.read(index)
      end

      # Writes a general-purpose register.
      #
      # @param index [Integer] register index.
      # @param value [Integer] value to write.
      def write_register(index, value)
        @reg_file.write(index, value)
      end

      # Returns the underlying pipeline (for advanced inspection).
      #
      # @return [CodingAdventures::CpuPipeline::Pipeline] pipeline.
      def pipeline
        @pipeline
      end

      # Returns the branch predictor (for inspection).
      def predictor
        @predictor
      end

      # Returns the cache hierarchy (for inspection).
      def cache_hierarchy
        @cache_hierarchy
      end

      private

      # Builds the cache hierarchy from config.
      def build_cache_hierarchy(config, mem_latency)
        # Default L1I: 4KB direct-mapped, 64B lines, 1-cycle latency.
        l1i_cfg = config.l1i_cache || CodingAdventures::Cache::CacheConfig.new(
          name: "L1I", total_size: 4096, line_size: 64,
          associativity: 1, access_latency: 1, write_policy: "write-back"
        )
        l1i = CodingAdventures::Cache::CacheSimulator.new(l1i_cfg)

        # Default L1D: 4KB direct-mapped, 64B lines, 1-cycle latency.
        l1d_cfg = config.l1d_cache || CodingAdventures::Cache::CacheConfig.new(
          name: "L1D", total_size: 4096, line_size: 64,
          associativity: 1, access_latency: 1, write_policy: "write-back"
        )
        l1d = CodingAdventures::Cache::CacheSimulator.new(l1d_cfg)

        # Optional L2.
        l2 = config.l2_cache ? CodingAdventures::Cache::CacheSimulator.new(config.l2_cache) : nil

        CodingAdventures::Cache::CacheHierarchy.new(
          l1i: l1i, l1d: l1d, l2: l2, l3: nil,
          main_memory_latency: mem_latency
        )
      end

      # === Pipeline Callbacks ===

      # Called by the pipeline's IF stage. Reads raw instruction bits from
      # memory at the given PC.
      def fetch_callback(pc)
        # Read from instruction cache hierarchy for statistics.
        @cache_hierarchy.read(address: pc, is_instruction: true, cycle: @cycle)
        # Read the actual instruction bits from memory.
        @mem_ctrl.read_word(pc)
      end

      # Called by the pipeline's ID stage. Delegates to the ISA decoder.
      def decode_callback(raw, token)
        @decoder.decode(raw, token)
      end

      # Called by the pipeline's EX stage. Computes ALU results, resolves
      # branches, and updates the branch predictor.
      def execute_callback(token)
        result = @decoder.execute(token, @reg_file)

        # Update branch predictor with actual outcome.
        if result.is_branch
          @predictor.update(pc: result.pc, taken: result.branch_taken, target: result.branch_target)
          if result.branch_taken
            @btb.update(pc: result.pc, target: result.branch_target, branch_type: "conditional")
          end
        end

        result
      end

      # Called by the pipeline's MEM stage. Handles loads and stores.
      def memory_callback(token)
        if token.mem_read
          # Load: read from data cache hierarchy.
          @cache_hierarchy.read(address: token.alu_result, is_instruction: false, cycle: @cycle)
          # Read the actual word from memory.
          token.mem_data = @mem_ctrl.read_word(token.alu_result)
          token.write_data = token.mem_data
        elsif token.mem_write
          # Store: write to data cache hierarchy.
          @cache_hierarchy.write(address: token.alu_result, data: [token.write_data & 0xFF], cycle: @cycle)
          # Write the actual word to memory.
          @mem_ctrl.write_word(token.alu_result, token.write_data)
        end
        token
      end

      # Called by the pipeline's WB stage. Writes results to registers.
      def writeback_callback(token)
        if token.reg_write && token.rd >= 0
          @reg_file.write(token.rd, token.write_data)
        end
      end

      # Called at the start of each cycle to check for hazards.
      def hazard_callback(stages)
        num_stages = stages.length
        pipeline_cfg = @config.pipeline
        pipeline_cfg = CodingAdventures::CpuPipeline.classic_5_stage if pipeline_cfg.nil? || pipeline_cfg.stages.empty?

        # Find the IF, ID, EX, MEM stages by category.
        if_tok = nil
        id_tok = nil
        ex_tok = nil
        mem_tok = nil

        pipeline_cfg.stages.each_with_index do |stage, i|
          break if i >= num_stages
          tok = stages[i]
          case stage.category
          when CodingAdventures::CpuPipeline::StageCategory::FETCH
            if_tok ||= tok
          when CodingAdventures::CpuPipeline::StageCategory::DECODE
            # Use the LAST decode stage (closest to EX).
            id_tok = tok
          when CodingAdventures::CpuPipeline::StageCategory::EXECUTE
            ex_tok ||= tok
          when CodingAdventures::CpuPipeline::StageCategory::MEMORY
            mem_tok ||= tok
          end
        end

        # Convert PipelineTokens to PipelineSlots for the hazard unit.
        if_slot = token_to_slot(if_tok)
        id_slot = token_to_slot(id_tok)
        ex_slot = token_to_slot(ex_tok)
        mem_slot = token_to_slot(mem_tok)

        # Run hazard detection.
        result = @hazard_unit.check(if_slot, id_slot, ex_slot, mem_slot)

        # Convert HazardResult to HazardResponse.
        response = CodingAdventures::CpuPipeline::HazardResponse.new

        case result.action
        when CodingAdventures::HazardDetection::HazardAction::STALL
          response.action = CodingAdventures::CpuPipeline::HazardAction::STALL
          response.stall_stages = result.stall_cycles
          @stall_count += 1

        when CodingAdventures::HazardDetection::HazardAction::FLUSH
          response.action = CodingAdventures::CpuPipeline::HazardAction::FLUSH
          response.flush_count = result.flush_count
          # Redirect PC to the correct target.
          if ex_tok && ex_tok.is_branch
            if ex_tok.branch_taken
              response.redirect_pc = ex_tok.branch_target
            else
              response.redirect_pc = ex_tok.pc + @decoder.instruction_size
            end
          end
          @flush_count += 1

        when CodingAdventures::HazardDetection::HazardAction::FORWARD_FROM_EX
          response.action = CodingAdventures::CpuPipeline::HazardAction::FORWARD_FROM_EX
          response.forward_value = result.forwarded_value || 0
          response.forward_source = result.forwarded_from
          @forward_count += 1

        when CodingAdventures::HazardDetection::HazardAction::FORWARD_FROM_MEM
          response.action = CodingAdventures::CpuPipeline::HazardAction::FORWARD_FROM_MEM
          response.forward_value = result.forwarded_value || 0
          response.forward_source = result.forwarded_from
          @forward_count += 1
        end

        response
      end

      # Called by the pipeline's IF stage to predict the next PC.
      def predict_callback(pc)
        prediction = @predictor.predict(pc: pc)
        instr_size = @decoder.instruction_size

        if prediction.taken
          target = @btb.lookup(pc: pc)
          return target if target
        end

        # Default: sequential fetch.
        pc + instr_size
      end

      # Converts a PipelineToken to a hazard-detection PipelineSlot.
      def token_to_slot(tok)
        if tok.nil? || tok.is_bubble
          return CodingAdventures::HazardDetection::PipelineSlot.new(valid: false)
        end

        source_regs = []
        source_regs << tok.rs1 if tok.rs1 >= 0
        source_regs << tok.rs2 if tok.rs2 >= 0

        dest_reg = nil
        dest_value = nil
        if tok.rd >= 0 && tok.reg_write
          dest_reg = tok.rd
          if tok.alu_result != 0 || tok.write_data != 0
            dest_value = (tok.write_data != 0) ? tok.write_data : tok.alu_result
          end
        end

        CodingAdventures::HazardDetection::PipelineSlot.new(
          valid: true,
          pc: tok.pc,
          source_regs: source_regs,
          dest_reg: dest_reg,
          dest_value: dest_value,
          is_branch: tok.is_branch,
          branch_taken: tok.branch_taken,
          branch_predicted_taken: false,
          mem_read: tok.mem_read,
          mem_write: tok.mem_write,
          uses_alu: true
        )
      end
    end
  end
end
