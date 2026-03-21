# frozen_string_literal: true

require "test_helper"

# =========================================================================
# Test helpers -- simple instruction memory and callbacks
# =========================================================================
#
# For testing, we create a tiny "instruction memory" -- just an array of
# integers. Each integer represents one instruction's raw bits. The fetch
# callback reads from this array using PC/4 as the index.
#
# The decode callback creates simple instructions:
#   - opcode 0x01 = ADD (register write)
#   - opcode 0x02 = LDR (load from memory, register write)
#   - opcode 0x03 = STR (store to memory)
#   - opcode 0x04 = BEQ (branch if equal)
#   - opcode 0xFF = HALT
#   - opcode 0x00 = NOP
#
# Encoding: raw = (opcode << 24) | (rd << 16) | (rs1 << 8) | rs2
#
# This encoding is deliberately simple -- the focus is on testing the
# pipeline mechanics, not instruction decoding.

module TestHelpers
  # Test opcode constants.
  OP_NOP  = 0x00
  OP_ADD  = 0x01
  OP_LDR  = 0x02
  OP_STR  = 0x03
  OP_BEQ  = 0x04
  OP_HALT = 0xFF

  # Encodes a test instruction.
  #
  #   opcode: 8 bits (bits 31-24)
  #   rd:     8 bits (bits 23-16)
  #   rs1:    8 bits (bits 15-8)
  #   rs2:    8 bits (bits 7-0)
  def self.make_instruction(opcode, rd, rs1, rs2)
    (opcode << 24) | (rd << 16) | (rs1 << 8) | rs2
  end

  # Returns a fetch proc that reads from the given instruction memory.
  #
  # If the PC is out of bounds, returns a NOP. This prevents crashes when
  # the pipeline fetches past the end of the program.
  def self.simple_fetch(instrs)
    ->(pc) {
      idx = pc / 4
      if idx < 0 || idx >= instrs.length
        make_instruction(OP_NOP, 0, 0, 0)
      else
        instrs[idx]
      end
    }
  end

  # Returns a decode proc that parses our test encoding.
  def self.simple_decode
    ->(raw, tok) {
      opcode = (raw >> 24) & 0xFF
      rd     = (raw >> 16) & 0xFF
      rs1    = (raw >> 8) & 0xFF
      rs2    = raw & 0xFF

      case opcode
      when OP_ADD
        tok.opcode = "ADD"
        tok.rd = rd
        tok.rs1 = rs1
        tok.rs2 = rs2
        tok.reg_write = true
      when OP_LDR
        tok.opcode = "LDR"
        tok.rd = rd
        tok.rs1 = rs1
        tok.mem_read = true
        tok.reg_write = true
      when OP_STR
        tok.opcode = "STR"
        tok.rs1 = rs1
        tok.rs2 = rs2
        tok.mem_write = true
      when OP_BEQ
        tok.opcode = "BEQ"
        tok.rs1 = rs1
        tok.rs2 = rs2
        tok.is_branch = true
      when OP_HALT
        tok.opcode = "HALT"
        tok.is_halt = true
      else
        tok.opcode = "NOP"
      end
      tok
    }
  end

  # Returns an execute callback that sets alu_result.
  def self.simple_execute
    ->(tok) {
      case tok.opcode
      when "ADD"
        tok.alu_result = tok.rs1 + tok.rs2 # Simplified: use register numbers as values
      when "LDR"
        tok.alu_result = tok.rs1 + tok.immediate # Address calculation
      when "STR"
        tok.alu_result = tok.rs1 + tok.immediate
      when "BEQ"
        tok.branch_target = tok.pc + tok.immediate
      end
      tok
    }
  end

  # Returns a memory callback that handles loads.
  def self.simple_memory
    ->(tok) {
      if tok.mem_read
        tok.mem_data = 42 # Return a fixed value for testing
        tok.write_data = tok.mem_data
      else
        tok.write_data = tok.alu_result
      end
      tok
    }
  end

  # Returns a writeback callback that records completed instructions.
  def self.simple_writeback(completed)
    ->(tok) {
      completed << tok.pc if completed
    }
  end

  # Creates a 5-stage pipeline with simple test callbacks.
  def self.new_test_pipeline(instrs, completed)
    config = CodingAdventures::CpuPipeline.classic_5_stage
    CodingAdventures::CpuPipeline::Pipeline.new(
      config: config,
      fetch_fn: simple_fetch(instrs),
      decode_fn: simple_decode,
      execute_fn: simple_execute,
      memory_fn: simple_memory,
      writeback_fn: simple_writeback(completed)
    )
  end
end

# =========================================================================
# Token tests
# =========================================================================

class TestPipelineToken < Minitest::Test
  def test_new_token
    tok = CodingAdventures::CpuPipeline.new_token
    assert_equal(-1, tok.rs1)
    assert_equal(-1, tok.rs2)
    assert_equal(-1, tok.rd)
    refute tok.is_bubble
    assert_instance_of Hash, tok.stage_entered
  end

  def test_new_bubble
    b = CodingAdventures::CpuPipeline.new_bubble
    assert b.is_bubble
    assert_equal "---", b.to_s
  end

  def test_token_string_with_opcode
    tok = CodingAdventures::CpuPipeline.new_token
    tok.opcode = "ADD"
    tok.pc = 100
    assert_equal "ADD@100", tok.to_s
  end

  def test_token_string_without_opcode
    tok = CodingAdventures::CpuPipeline.new_token
    tok.pc = 200
    assert_equal "instr@200", tok.to_s
  end

  def test_token_clone
    tok = CodingAdventures::CpuPipeline.new_token
    tok.pc = 100
    tok.opcode = "ADD"
    tok.stage_entered["IF"] = 1
    tok.stage_entered["ID"] = 2

    copy = tok.clone
    assert_equal 100, copy.pc
    assert_equal "ADD", copy.opcode

    # Mutating the clone should not affect the original.
    copy.stage_entered["EX"] = 3
    refute tok.stage_entered.key?("EX")
  end

  def test_token_clone_nil
    # Cloning nil should return nil (Ruby's nil.clone returns nil in 3.4+,
    # but we test our PipelineToken's clone).
    tok = CodingAdventures::CpuPipeline.new_token
    copy = tok.clone
    refute_nil copy
  end
end

# =========================================================================
# PipelineConfig tests
# =========================================================================

class TestPipelineConfig < Minitest::Test
  def test_classic_5_stage
    config = CodingAdventures::CpuPipeline.classic_5_stage
    assert_equal 5, config.num_stages
    assert_nil config.validate
    assert_equal "IF", config.stages[0].name
    assert_equal "WB", config.stages[4].name
  end

  def test_deep_13_stage
    config = CodingAdventures::CpuPipeline.deep_13_stage
    assert_equal 13, config.num_stages
    assert_nil config.validate
  end

  def test_config_validation_too_few_stages
    cfg = CodingAdventures::CpuPipeline::PipelineConfig.new(
      stages: [
        CodingAdventures::CpuPipeline::PipelineStage.new(
          name: "IF",
          description: "Fetch",
          category: CodingAdventures::CpuPipeline::StageCategory::FETCH
        )
      ],
      execution_width: 1
    )
    refute_nil cfg.validate
  end

  def test_config_validation_zero_execution_width
    cfg = CodingAdventures::CpuPipeline::PipelineConfig.new(
      stages: [
        CodingAdventures::CpuPipeline::PipelineStage.new(
          name: "IF", description: "Fetch",
          category: CodingAdventures::CpuPipeline::StageCategory::FETCH
        ),
        CodingAdventures::CpuPipeline::PipelineStage.new(
          name: "WB", description: "Writeback",
          category: CodingAdventures::CpuPipeline::StageCategory::WRITEBACK
        )
      ],
      execution_width: 0
    )
    refute_nil cfg.validate
  end

  def test_config_validation_duplicate_stage_names
    cfg = CodingAdventures::CpuPipeline::PipelineConfig.new(
      stages: [
        CodingAdventures::CpuPipeline::PipelineStage.new(
          name: "IF", description: "Fetch",
          category: CodingAdventures::CpuPipeline::StageCategory::FETCH
        ),
        CodingAdventures::CpuPipeline::PipelineStage.new(
          name: "IF", description: "Writeback",
          category: CodingAdventures::CpuPipeline::StageCategory::WRITEBACK
        )
      ],
      execution_width: 1
    )
    refute_nil cfg.validate
  end

  def test_config_validation_no_fetch_stage
    cfg = CodingAdventures::CpuPipeline::PipelineConfig.new(
      stages: [
        CodingAdventures::CpuPipeline::PipelineStage.new(
          name: "EX", description: "Execute",
          category: CodingAdventures::CpuPipeline::StageCategory::EXECUTE
        ),
        CodingAdventures::CpuPipeline::PipelineStage.new(
          name: "WB", description: "Writeback",
          category: CodingAdventures::CpuPipeline::StageCategory::WRITEBACK
        )
      ],
      execution_width: 1
    )
    refute_nil cfg.validate
  end

  def test_config_validation_no_writeback_stage
    cfg = CodingAdventures::CpuPipeline::PipelineConfig.new(
      stages: [
        CodingAdventures::CpuPipeline::PipelineStage.new(
          name: "IF", description: "Fetch",
          category: CodingAdventures::CpuPipeline::StageCategory::FETCH
        ),
        CodingAdventures::CpuPipeline::PipelineStage.new(
          name: "EX", description: "Execute",
          category: CodingAdventures::CpuPipeline::StageCategory::EXECUTE
        )
      ],
      execution_width: 1
    )
    refute_nil cfg.validate
  end

  def test_config_validation_valid_2_stage
    cfg = CodingAdventures::CpuPipeline::PipelineConfig.new(
      stages: [
        CodingAdventures::CpuPipeline::PipelineStage.new(
          name: "IF", description: "Fetch",
          category: CodingAdventures::CpuPipeline::StageCategory::FETCH
        ),
        CodingAdventures::CpuPipeline::PipelineStage.new(
          name: "WB", description: "Writeback",
          category: CodingAdventures::CpuPipeline::StageCategory::WRITEBACK
        )
      ],
      execution_width: 1
    )
    assert_nil cfg.validate
  end

  def test_validate_bang_raises_on_invalid
    cfg = CodingAdventures::CpuPipeline::PipelineConfig.new(
      stages: [
        CodingAdventures::CpuPipeline::PipelineStage.new(
          name: "IF", description: "Fetch",
          category: CodingAdventures::CpuPipeline::StageCategory::FETCH
        )
      ],
      execution_width: 1
    )
    assert_raises(ArgumentError) { cfg.validate! }
  end
end

# =========================================================================
# Basic Pipeline tests
# =========================================================================

class TestPipelineBasic < Minitest::Test
  include TestHelpers

  def test_new_pipeline
    instrs = [TestHelpers.make_instruction(TestHelpers::OP_ADD, 1, 2, 3)]
    p = TestHelpers.new_test_pipeline(instrs, nil)

    refute p.halted?
    assert_equal 0, p.cycle
    assert_equal 0, p.pc
  end

  def test_new_pipeline_invalid_config
    cfg = CodingAdventures::CpuPipeline::PipelineConfig.new(
      stages: [
        CodingAdventures::CpuPipeline::PipelineStage.new(
          name: "IF", description: "Fetch",
          category: CodingAdventures::CpuPipeline::StageCategory::FETCH
        )
      ],
      execution_width: 1
    )
    assert_raises(ArgumentError) do
      CodingAdventures::CpuPipeline::Pipeline.new(
        config: cfg,
        fetch_fn: ->(_pc) { 0 },
        decode_fn: ->(_raw, tok) { tok },
        execute_fn: ->(tok) { tok },
        memory_fn: ->(tok) { tok },
        writeback_fn: ->(_tok) {}
      )
    end
  end

  # Verifies that a single instruction progresses through all 5 stages
  # in 5 cycles.
  #
  # Timeline:
  #   Cycle 1: ADD enters IF
  #   Cycle 2: ADD enters ID
  #   Cycle 3: ADD enters EX
  #   Cycle 4: ADD enters MEM
  #   Cycle 5: ADD enters WB and retires
  def test_single_instruction_flows_through_5_stages
    instrs = [
      TestHelpers.make_instruction(TestHelpers::OP_ADD, 1, 2, 3),
      TestHelpers.make_instruction(TestHelpers::OP_NOP, 0, 0, 0),
      TestHelpers.make_instruction(TestHelpers::OP_NOP, 0, 0, 0),
      TestHelpers.make_instruction(TestHelpers::OP_NOP, 0, 0, 0),
      TestHelpers.make_instruction(TestHelpers::OP_NOP, 0, 0, 0)
    ]

    completed = []
    p = TestHelpers.new_test_pipeline(instrs, completed)

    5.times { p.step }

    assert completed.length >= 1, "expected at least one instruction to complete after 5 cycles"
    assert_equal 0, completed[0], "expected first completed instruction at PC=0"
  end

  # Verifies that the first instruction completes at exactly cycle 5
  # (for a 5-stage pipeline), and subsequent instructions complete one
  # per cycle.
  def test_pipeline_fill_timing
    instrs = Array.new(20) { TestHelpers.make_instruction(TestHelpers::OP_ADD, 1, 2, 3) }

    completed = []
    p = TestHelpers.new_test_pipeline(instrs, completed)

    # After 4 cycles, nothing should have completed yet.
    4.times { p.step }
    assert_equal 0, completed.length, "expected 0 completions after 4 cycles"

    # After cycle 5, exactly 1 instruction should have completed.
    p.step
    assert_equal 1, completed.length, "expected 1 completion after 5 cycles"

    # After cycle 6, 2 completions. After cycle 7, 3 completions.
    p.step
    assert_equal 2, completed.length, "expected 2 completions after 6 cycles"

    p.step
    assert_equal 3, completed.length, "expected 3 completions after 7 cycles"
  end

  # Verifies that after the pipeline fills, the IPC approaches 1.0 for
  # a stream of independent instructions.
  def test_steady_state_ipc
    instrs = Array.new(100) { TestHelpers.make_instruction(TestHelpers::OP_ADD, 1, 2, 3) }

    p = TestHelpers.new_test_pipeline(instrs, nil)
    50.times { p.step }

    stats = p.stats
    expected_completed = 50 - 5 + 1 # 46
    assert_equal expected_completed, stats.instructions_completed

    ipc = stats.ipc
    assert ipc > 0.85 && ipc <= 1.01, "expected IPC near 1.0, got #{ipc}"
  end

  # Verifies that a HALT instruction eventually reaches the WB stage
  # and stops the pipeline.
  def test_halt_propagation
    instrs = [
      TestHelpers.make_instruction(TestHelpers::OP_ADD, 1, 2, 3),
      TestHelpers.make_instruction(TestHelpers::OP_ADD, 4, 5, 6),
      TestHelpers.make_instruction(TestHelpers::OP_HALT, 0, 0, 0),
      TestHelpers.make_instruction(TestHelpers::OP_NOP, 0, 0, 0),
      TestHelpers.make_instruction(TestHelpers::OP_NOP, 0, 0, 0)
    ]

    completed = []
    p = TestHelpers.new_test_pipeline(instrs, completed)
    stats = p.run(100)

    assert p.halted?
    assert_equal 7, p.cycle, "expected halt at cycle 7"
    assert_equal 3, stats.instructions_completed, "expected 3 completions (2 ADD + 1 HALT)"
  end

  # Verifies that stepping an empty pipeline (no program) works.
  def test_empty_pipeline
    instrs = []
    p = TestHelpers.new_test_pipeline(instrs, nil)

    snap = p.step
    assert_equal 1, snap.cycle
  end
end

# =========================================================================
# Stall tests
# =========================================================================

class TestPipelineStall < Minitest::Test
  # Verifies that during a stall, the IF and ID stages are frozen (contain
  # the same tokens) and a bubble is inserted at EX.
  def test_stall_freezes_earlier_stages
    instrs = [
      TestHelpers.make_instruction(TestHelpers::OP_LDR, 1, 2, 0),
      TestHelpers.make_instruction(TestHelpers::OP_ADD, 3, 1, 4),
      TestHelpers.make_instruction(TestHelpers::OP_ADD, 5, 6, 7),
      TestHelpers.make_instruction(TestHelpers::OP_NOP, 0, 0, 0),
      TestHelpers.make_instruction(TestHelpers::OP_NOP, 0, 0, 0),
      TestHelpers.make_instruction(TestHelpers::OP_NOP, 0, 0, 0),
      TestHelpers.make_instruction(TestHelpers::OP_NOP, 0, 0, 0),
      TestHelpers.make_instruction(TestHelpers::OP_NOP, 0, 0, 0)
    ]

    completed = []
    p = TestHelpers.new_test_pipeline(instrs, completed)

    stall_injected = false
    p.set_hazard_fn(->(stages) {
      if !stall_injected && stages.length >= 3
        ex_tok = stages[2]
        id_tok = stages[1]
        if ex_tok && !ex_tok.is_bubble && ex_tok.opcode == "LDR" &&
            id_tok && !id_tok.is_bubble && id_tok.opcode == "ADD"
          stall_injected = true
          return CodingAdventures::CpuPipeline::HazardResponse.new(
            action: CodingAdventures::CpuPipeline::HazardAction::STALL,
            stall_stages: 2
          )
        end
      end
      CodingAdventures::CpuPipeline::HazardResponse.new
    })

    p.step # cycle 1
    p.step # cycle 2
    p.step # cycle 3
    snap = p.step # cycle 4 -- stall should occur

    assert snap.stalled, "expected pipeline to be stalled at cycle 4"

    ex_tok = p.stage_contents("EX")
    assert ex_tok && ex_tok.is_bubble, "expected bubble in EX stage after stall"

    id_tok = p.stage_contents("ID")
    assert id_tok && id_tok.opcode == "ADD", "expected ADD to remain in ID stage (frozen)"

    assert_equal 1, p.stats.stall_cycles
  end

  # Verifies that a bubble is inserted into the correct stage during a stall.
  def test_stall_bubble_insertion
    instrs = Array.new(10) { TestHelpers.make_instruction(TestHelpers::OP_ADD, 1, 2, 3) }

    p = TestHelpers.new_test_pipeline(instrs, nil)

    stall_count = 0
    p.set_hazard_fn(->(stages) {
      stall_count += 1
      if stall_count == 3
        return CodingAdventures::CpuPipeline::HazardResponse.new(
          action: CodingAdventures::CpuPipeline::HazardAction::STALL,
          stall_stages: 2
        )
      end
      CodingAdventures::CpuPipeline::HazardResponse.new
    })

    3.times { p.step }

    ex_tok = p.stage_contents("EX")
    assert ex_tok && ex_tok.is_bubble, "expected bubble in EX after stall"
  end
end

# =========================================================================
# Flush tests
# =========================================================================

class TestPipelineFlush < Minitest::Test
  # Verifies that a flush replaces speculative stages with bubbles and
  # redirects the PC.
  def test_flush_replaces_with_bubbles
    instrs = [
      TestHelpers.make_instruction(TestHelpers::OP_BEQ, 0, 1, 2),
      TestHelpers.make_instruction(TestHelpers::OP_ADD, 1, 2, 3),
      TestHelpers.make_instruction(TestHelpers::OP_ADD, 4, 5, 6),
      TestHelpers.make_instruction(TestHelpers::OP_NOP, 0, 0, 0),
      TestHelpers.make_instruction(TestHelpers::OP_NOP, 0, 0, 0),
      TestHelpers.make_instruction(TestHelpers::OP_ADD, 7, 8, 9), # PC=20
      TestHelpers.make_instruction(TestHelpers::OP_NOP, 0, 0, 0),
      TestHelpers.make_instruction(TestHelpers::OP_NOP, 0, 0, 0)
    ]

    p = TestHelpers.new_test_pipeline(instrs, nil)

    flushed = false
    p.set_hazard_fn(->(stages) {
      if !flushed && stages.length >= 3
        ex_tok = stages[2]
        if ex_tok && !ex_tok.is_bubble && ex_tok.is_branch
          flushed = true
          return CodingAdventures::CpuPipeline::HazardResponse.new(
            action: CodingAdventures::CpuPipeline::HazardAction::FLUSH,
            flush_count: 2,
            redirect_pc: 20
          )
        end
      end
      CodingAdventures::CpuPipeline::HazardResponse.new
    })

    p.step # cycle 1
    p.step # cycle 2
    p.step # cycle 3
    snap = p.step # cycle 4 -- flush

    assert snap.flushing, "expected flush at cycle 4"
    assert_equal 24, p.pc, "expected PC=24 after flush (20 + 4)"
    assert_equal 1, p.stats.flush_cycles
  end
end

# =========================================================================
# Forwarding integration tests
# =========================================================================

class TestPipelineForwarding < Minitest::Test
  # Verifies that the forwarding callback updates the token with the
  # forwarded value and records the source.
  def test_forwarding_from_ex
    instrs = Array.new(10) { TestHelpers.make_instruction(TestHelpers::OP_ADD, 1, 2, 3) }

    p = TestHelpers.new_test_pipeline(instrs, nil)

    forward_cycle = 0
    p.set_hazard_fn(->(stages) {
      forward_cycle += 1
      if forward_cycle == 4
        return CodingAdventures::CpuPipeline::HazardResponse.new(
          action: CodingAdventures::CpuPipeline::HazardAction::FORWARD_FROM_EX,
          forward_value: 99,
          forward_source: "EX"
        )
      end
      CodingAdventures::CpuPipeline::HazardResponse.new
    })

    4.times { p.step }

    ex_tok = p.stage_contents("EX")
    refute_nil ex_tok, "expected token in EX stage"
    assert_equal "EX", ex_tok.forwarded_from
  end

  def test_forwarding_from_mem
    instrs = Array.new(10) { TestHelpers.make_instruction(TestHelpers::OP_ADD, 1, 2, 3) }

    p = TestHelpers.new_test_pipeline(instrs, nil)

    forward_cycle = 0
    p.set_hazard_fn(->(stages) {
      forward_cycle += 1
      if forward_cycle == 4
        return CodingAdventures::CpuPipeline::HazardResponse.new(
          action: CodingAdventures::CpuPipeline::HazardAction::FORWARD_FROM_MEM,
          forward_value: 77,
          forward_source: "MEM"
        )
      end
      CodingAdventures::CpuPipeline::HazardResponse.new
    })

    4.times { p.step }

    ex_tok = p.stage_contents("EX")
    refute_nil ex_tok, "expected token in EX stage"
    assert_equal "MEM", ex_tok.forwarded_from
  end
end

# =========================================================================
# Statistics tests
# =========================================================================

class TestPipelineStats < Minitest::Test
  def test_ipc_calculation
    stats = CodingAdventures::CpuPipeline::PipelineStats.new
    stats.total_cycles = 100
    stats.instructions_completed = 80
    assert_in_delta 0.8, stats.ipc, 0.001
  end

  def test_cpi_calculation
    stats = CodingAdventures::CpuPipeline::PipelineStats.new
    stats.total_cycles = 120
    stats.instructions_completed = 100
    assert_in_delta 1.2, stats.cpi, 0.001
  end

  def test_ipc_zero_cycles
    stats = CodingAdventures::CpuPipeline::PipelineStats.new
    assert_equal 0.0, stats.ipc
  end

  def test_cpi_zero_instructions
    stats = CodingAdventures::CpuPipeline::PipelineStats.new
    stats.total_cycles = 10
    assert_equal 0.0, stats.cpi
  end

  def test_stats_string
    stats = CodingAdventures::CpuPipeline::PipelineStats.new
    stats.total_cycles = 100
    stats.instructions_completed = 80
    stats.stall_cycles = 5
    stats.flush_cycles = 3
    stats.bubble_cycles = 10
    refute_empty stats.to_s
  end

  # Verifies that stalls reduce the IPC below 1.0.
  def test_stall_reduces_ipc
    instrs = Array.new(50) { TestHelpers.make_instruction(TestHelpers::OP_ADD, 1, 2, 3) }

    p = TestHelpers.new_test_pipeline(instrs, nil)

    cycle_count = 0
    p.set_hazard_fn(->(stages) {
      cycle_count += 1
      if cycle_count % 5 == 0
        return CodingAdventures::CpuPipeline::HazardResponse.new(
          action: CodingAdventures::CpuPipeline::HazardAction::STALL,
          stall_stages: 2
        )
      end
      CodingAdventures::CpuPipeline::HazardResponse.new
    })

    30.times { p.step }

    stats = p.stats
    assert stats.ipc < 1.0, "expected IPC < 1.0 with stalls, got #{stats.ipc}"
    assert stats.stall_cycles > 0, "expected nonzero stall cycles"
  end
end

# =========================================================================
# Trace and Snapshot tests
# =========================================================================

class TestPipelineSnapshot < Minitest::Test
  # Verifies that snapshots correctly reflect pipeline contents.
  def test_snapshot_accuracy
    instrs = [
      TestHelpers.make_instruction(TestHelpers::OP_ADD, 1, 2, 3),
      TestHelpers.make_instruction(TestHelpers::OP_ADD, 4, 5, 6),
      TestHelpers.make_instruction(TestHelpers::OP_NOP, 0, 0, 0)
    ]

    p = TestHelpers.new_test_pipeline(instrs, nil)

    snap1 = p.step
    assert_equal 1, snap1.cycle
    if_tok = snap1.stages["IF"]
    refute_nil if_tok, "expected token in IF stage at cycle 1"
    assert_equal 0, if_tok.pc

    snap2 = p.step
    assert_equal 2, snap2.cycle
    id_tok = snap2.stages["ID"]
    refute_nil id_tok, "expected token in ID stage at cycle 2"
    assert_equal 0, id_tok.pc
  end

  # Verifies that trace records every cycle's state.
  def test_trace_completeness
    instrs = Array.new(10) { TestHelpers.make_instruction(TestHelpers::OP_ADD, 1, 2, 3) }

    p = TestHelpers.new_test_pipeline(instrs, nil)
    7.times { p.step }

    trace = p.trace
    assert_equal 7, trace.length

    trace.each_with_index do |snap, i|
      assert_equal i + 1, snap.cycle
    end
  end

  # Verifies that taking a snapshot does not modify the pipeline state.
  def test_snapshot_does_not_advance
    instrs = [TestHelpers.make_instruction(TestHelpers::OP_ADD, 1, 2, 3)]
    p = TestHelpers.new_test_pipeline(instrs, nil)

    p.step
    snap1 = p.snapshot
    snap2 = p.snapshot

    assert_equal snap1.cycle, snap2.cycle
  end

  def test_snapshot_string
    snap = CodingAdventures::CpuPipeline::PipelineSnapshot.new(cycle: 7, pc: 28, stalled: true)
    refute_empty snap.to_s
  end
end

# =========================================================================
# Configuration preset tests
# =========================================================================

class TestPipelineConfigurations < Minitest::Test
  # Verifies that a deeper pipeline takes more cycles to fill.
  def test_deep_pipeline_longer_fill_time
    config = CodingAdventures::CpuPipeline.deep_13_stage
    instrs = Array.new(30) { TestHelpers.make_instruction(TestHelpers::OP_ADD, 1, 2, 3) }

    p = CodingAdventures::CpuPipeline::Pipeline.new(
      config: config,
      fetch_fn: TestHelpers.simple_fetch(instrs),
      decode_fn: TestHelpers.simple_decode,
      execute_fn: TestHelpers.simple_execute,
      memory_fn: TestHelpers.simple_memory,
      writeback_fn: TestHelpers.simple_writeback(nil)
    )

    12.times { p.step }
    assert_equal 0, p.stats.instructions_completed,
      "expected 0 completions after 12 cycles in 13-stage pipeline"

    p.step # cycle 13
    assert_equal 1, p.stats.instructions_completed,
      "expected 1 completion after 13 cycles in 13-stage pipeline"
  end

  # Verifies that custom 3-stage pipeline works.
  def test_custom_stage_configuration
    config = CodingAdventures::CpuPipeline::PipelineConfig.new(
      stages: [
        CodingAdventures::CpuPipeline::PipelineStage.new(
          name: "IF", description: "Fetch",
          category: CodingAdventures::CpuPipeline::StageCategory::FETCH
        ),
        CodingAdventures::CpuPipeline::PipelineStage.new(
          name: "EX", description: "Execute",
          category: CodingAdventures::CpuPipeline::StageCategory::EXECUTE
        ),
        CodingAdventures::CpuPipeline::PipelineStage.new(
          name: "WB", description: "Writeback",
          category: CodingAdventures::CpuPipeline::StageCategory::WRITEBACK
        )
      ],
      execution_width: 1
    )

    instrs = Array.new(10) { TestHelpers.make_instruction(TestHelpers::OP_ADD, 1, 2, 3) }
    completed = []

    p = CodingAdventures::CpuPipeline::Pipeline.new(
      config: config,
      fetch_fn: TestHelpers.simple_fetch(instrs),
      decode_fn: TestHelpers.simple_decode,
      execute_fn: TestHelpers.simple_execute,
      memory_fn: TestHelpers.simple_memory,
      writeback_fn: TestHelpers.simple_writeback(completed)
    )

    2.times { p.step }
    assert_equal 0, completed.length, "expected 0 completions after 2 cycles in 3-stage pipeline"

    p.step # cycle 3
    assert_equal 1, completed.length, "expected 1 completion after 3 cycles in 3-stage pipeline"
  end

  # Verifies that a 2-stage pipeline works.
  def test_two_stage_pipeline
    config = CodingAdventures::CpuPipeline::PipelineConfig.new(
      stages: [
        CodingAdventures::CpuPipeline::PipelineStage.new(
          name: "IF", description: "Fetch",
          category: CodingAdventures::CpuPipeline::StageCategory::FETCH
        ),
        CodingAdventures::CpuPipeline::PipelineStage.new(
          name: "WB", description: "Writeback",
          category: CodingAdventures::CpuPipeline::StageCategory::WRITEBACK
        )
      ],
      execution_width: 1
    )

    instrs = Array.new(10) { TestHelpers.make_instruction(TestHelpers::OP_ADD, 1, 2, 3) }
    completed = []

    p = CodingAdventures::CpuPipeline::Pipeline.new(
      config: config,
      fetch_fn: TestHelpers.simple_fetch(instrs),
      decode_fn: TestHelpers.simple_decode,
      execute_fn: TestHelpers.simple_execute,
      memory_fn: TestHelpers.simple_memory,
      writeback_fn: TestHelpers.simple_writeback(completed)
    )

    p.step # cycle 1
    assert_equal 0, completed.length

    p.step # cycle 2
    assert_equal 1, completed.length
  end
end

# =========================================================================
# Branch prediction integration tests
# =========================================================================

class TestBranchPrediction < Minitest::Test
  def test_branch_predictor_integration
    instrs = Array.new(100) { TestHelpers.make_instruction(TestHelpers::OP_ADD, 1, 2, 3) }

    p = TestHelpers.new_test_pipeline(instrs, nil)

    # Set a predictor that always predicts PC+8 (skip one instruction).
    p.set_predict_fn(->(pc) { pc + 8 })

    p.step # cycle 1: fetches PC=0, predicts next=8
    assert_equal 8, p.pc, "expected PC=8 after prediction"

    p.step # cycle 2: fetches PC=8, predicts next=16
    assert_equal 16, p.pc, "expected PC=16 after second prediction"
  end
end

# =========================================================================
# SetPC test
# =========================================================================

class TestSetPC < Minitest::Test
  def test_set_pc
    instrs = Array.new(10) { TestHelpers.make_instruction(TestHelpers::OP_ADD, 1, 2, 3) }
    p = TestHelpers.new_test_pipeline(instrs, nil)
    p.set_pc(100)
    assert_equal 100, p.pc
  end
end

# =========================================================================
# Halted pipeline test
# =========================================================================

class TestHaltedPipeline < Minitest::Test
  def test_halted_pipeline_does_not_advance
    instrs = [
      TestHelpers.make_instruction(TestHelpers::OP_HALT, 0, 0, 0),
      TestHelpers.make_instruction(TestHelpers::OP_NOP, 0, 0, 0),
      TestHelpers.make_instruction(TestHelpers::OP_NOP, 0, 0, 0),
      TestHelpers.make_instruction(TestHelpers::OP_NOP, 0, 0, 0),
      TestHelpers.make_instruction(TestHelpers::OP_NOP, 0, 0, 0)
    ]

    p = TestHelpers.new_test_pipeline(instrs, nil)
    p.run(100)

    cycle_at_halt = p.cycle
    completed_at_halt = p.stats.instructions_completed

    p.step
    p.step

    assert_equal cycle_at_halt, p.cycle
    assert_equal completed_at_halt, p.stats.instructions_completed
  end
end

# =========================================================================
# StageCategory tests
# =========================================================================

class TestStageCategory < Minitest::Test
  def test_stage_category_to_s
    assert_equal "fetch", CodingAdventures::CpuPipeline::StageCategory.to_s(:fetch)
    assert_equal "decode", CodingAdventures::CpuPipeline::StageCategory.to_s(:decode)
    assert_equal "execute", CodingAdventures::CpuPipeline::StageCategory.to_s(:execute)
    assert_equal "memory", CodingAdventures::CpuPipeline::StageCategory.to_s(:memory)
    assert_equal "writeback", CodingAdventures::CpuPipeline::StageCategory.to_s(:writeback)
  end
end

# =========================================================================
# HazardAction tests
# =========================================================================

class TestHazardAction < Minitest::Test
  def test_hazard_action_to_s
    assert_equal "NONE", CodingAdventures::CpuPipeline::HazardAction.to_s(:none)
    assert_equal "FORWARD_FROM_EX", CodingAdventures::CpuPipeline::HazardAction.to_s(:forward_from_ex)
    assert_equal "FORWARD_FROM_MEM", CodingAdventures::CpuPipeline::HazardAction.to_s(:forward_from_mem)
    assert_equal "STALL", CodingAdventures::CpuPipeline::HazardAction.to_s(:stall)
    assert_equal "FLUSH", CodingAdventures::CpuPipeline::HazardAction.to_s(:flush)
    assert_equal "UNKNOWN", CodingAdventures::CpuPipeline::HazardAction.to_s(:bogus)
  end
end

# =========================================================================
# PipelineStage String test
# =========================================================================

class TestPipelineStage < Minitest::Test
  def test_pipeline_stage_to_s
    stage = CodingAdventures::CpuPipeline::PipelineStage.new(
      name: "IF",
      description: "Instruction Fetch",
      category: CodingAdventures::CpuPipeline::StageCategory::FETCH
    )
    assert_equal "IF", stage.to_s
  end
end

# =========================================================================
# Config returns test
# =========================================================================

class TestPipelineConfigAccess < Minitest::Test
  def test_pipeline_config
    instrs = [TestHelpers.make_instruction(TestHelpers::OP_NOP, 0, 0, 0)]
    p = TestHelpers.new_test_pipeline(instrs, nil)
    cfg = p.config
    assert_equal 5, cfg.num_stages
  end
end

# =========================================================================
# Multiple stall and flush cycles test
# =========================================================================

class TestMultipleStallsAndFlushes < Minitest::Test
  def test_multiple_stalls_and_flushes
    instrs = Array.new(50) { TestHelpers.make_instruction(TestHelpers::OP_ADD, 1, 2, 3) }

    p = TestHelpers.new_test_pipeline(instrs, nil)

    cycle_counter = 0
    p.set_hazard_fn(->(stages) {
      cycle_counter += 1
      if cycle_counter == 5 || cycle_counter == 10
        return CodingAdventures::CpuPipeline::HazardResponse.new(
          action: CodingAdventures::CpuPipeline::HazardAction::STALL,
          stall_stages: 2
        )
      end
      if cycle_counter == 15
        return CodingAdventures::CpuPipeline::HazardResponse.new(
          action: CodingAdventures::CpuPipeline::HazardAction::FLUSH,
          flush_count: 2,
          redirect_pc: 0
        )
      end
      CodingAdventures::CpuPipeline::HazardResponse.new
    })

    20.times { p.step }

    stats = p.stats
    assert_equal 2, stats.stall_cycles
    assert_equal 1, stats.flush_cycles
  end
end

# =========================================================================
# Run with max cycles test
# =========================================================================

class TestRunMaxCycles < Minitest::Test
  def test_run_max_cycles
    instrs = Array.new(100) { TestHelpers.make_instruction(TestHelpers::OP_ADD, 1, 2, 3) }

    p = TestHelpers.new_test_pipeline(instrs, nil)
    stats = p.run(10)

    assert_equal 10, stats.total_cycles
    refute p.halted?
  end
end

# =========================================================================
# StageContents test
# =========================================================================

class TestStageContents < Minitest::Test
  def test_stage_contents_invalid_name
    instrs = [TestHelpers.make_instruction(TestHelpers::OP_NOP, 0, 0, 0)]
    p = TestHelpers.new_test_pipeline(instrs, nil)
    p.step

    tok = p.stage_contents("NONEXISTENT")
    assert_nil tok
  end
end

# =========================================================================
# Flush with default flush count test
# =========================================================================

class TestFlushDefaultFlushCount < Minitest::Test
  def test_flush_default_flush_count
    instrs = Array.new(20) { TestHelpers.make_instruction(TestHelpers::OP_ADD, 1, 2, 3) }

    p = TestHelpers.new_test_pipeline(instrs, nil)

    flushed = false
    p.set_hazard_fn(->(stages) {
      if !flushed && stages.length >= 3 && stages[2] && !stages[2].is_bubble
        flushed = true
        return CodingAdventures::CpuPipeline::HazardResponse.new(
          action: CodingAdventures::CpuPipeline::HazardAction::FLUSH,
          flush_count: 0, # Use default
          redirect_pc: 100
        )
      end
      CodingAdventures::CpuPipeline::HazardResponse.new
    })

    5.times { p.step }
    assert_equal 1, p.stats.flush_cycles
  end
end

# =========================================================================
# Stall with default stall point test
# =========================================================================

class TestStallDefaultStallPoint < Minitest::Test
  def test_stall_default_stall_point
    instrs = Array.new(20) { TestHelpers.make_instruction(TestHelpers::OP_ADD, 1, 2, 3) }

    p = TestHelpers.new_test_pipeline(instrs, nil)

    stall_count = 0
    p.set_hazard_fn(->(stages) {
      stall_count += 1
      if stall_count == 3
        return CodingAdventures::CpuPipeline::HazardResponse.new(
          action: CodingAdventures::CpuPipeline::HazardAction::STALL,
          stall_stages: 0 # Use default (first execute stage)
        )
      end
      CodingAdventures::CpuPipeline::HazardResponse.new
    })

    5.times { p.step }
    assert_equal 1, p.stats.stall_cycles
  end
end

# =========================================================================
# Edge cases
# =========================================================================

class TestEdgeCases < Minitest::Test
  # Verifies that flush count larger than pipeline is clamped.
  def test_flush_count_larger_than_pipeline
    instrs = Array.new(20) { TestHelpers.make_instruction(TestHelpers::OP_ADD, 1, 2, 3) }

    p = TestHelpers.new_test_pipeline(instrs, nil)

    flushed = false
    p.set_hazard_fn(->(stages) {
      if !flushed && stages[2] && !stages[2].is_bubble
        flushed = true
        return CodingAdventures::CpuPipeline::HazardResponse.new(
          action: CodingAdventures::CpuPipeline::HazardAction::FLUSH,
          flush_count: 100, # Way too many -- should be clamped
          redirect_pc: 0
        )
      end
      CodingAdventures::CpuPipeline::HazardResponse.new
    })

    # Should not raise.
    10.times { p.step }
  end

  # Verifies that stall point larger than pipeline is clamped.
  def test_stall_point_larger_than_pipeline
    instrs = Array.new(20) { TestHelpers.make_instruction(TestHelpers::OP_ADD, 1, 2, 3) }

    p = TestHelpers.new_test_pipeline(instrs, nil)

    stall_count = 0
    p.set_hazard_fn(->(stages) {
      stall_count += 1
      if stall_count == 3
        return CodingAdventures::CpuPipeline::HazardResponse.new(
          action: CodingAdventures::CpuPipeline::HazardAction::STALL,
          stall_stages: 100 # Way too large -- should be clamped
        )
      end
      CodingAdventures::CpuPipeline::HazardResponse.new
    })

    # Should not raise.
    10.times { p.step }
  end

  # Verifies normal operation without hazard detection.
  def test_pipeline_with_no_hazard_func
    instrs = Array.new(20) { TestHelpers.make_instruction(TestHelpers::OP_ADD, 1, 2, 3) }

    p = TestHelpers.new_test_pipeline(instrs, nil)
    10.times { p.step }

    stats = p.stats
    assert_equal 0, stats.stall_cycles
    assert_equal 0, stats.flush_cycles
  end
end

# =========================================================================
# Decode stage test
# =========================================================================

class TestDecodeStage < Minitest::Test
  def test_decode_stage
    instrs = [
      TestHelpers.make_instruction(TestHelpers::OP_LDR, 5, 3, 0),
      TestHelpers.make_instruction(TestHelpers::OP_NOP, 0, 0, 0)
    ]

    p = TestHelpers.new_test_pipeline(instrs, nil)
    p.step # cycle 1: LDR enters IF
    p.step # cycle 2: LDR moves to ID, gets decoded

    id_tok = p.stage_contents("ID")
    refute_nil id_tok
    assert_equal "LDR", id_tok.opcode
    assert_equal 5, id_tok.rd
    assert id_tok.mem_read
    assert id_tok.reg_write
  end
end

# =========================================================================
# Instruction count verification
# =========================================================================

class TestInstructionCount < Minitest::Test
  def test_instruction_count_matches_completions
    instrs = [
      TestHelpers.make_instruction(TestHelpers::OP_ADD, 1, 2, 3),
      TestHelpers.make_instruction(TestHelpers::OP_ADD, 4, 5, 6),
      TestHelpers.make_instruction(TestHelpers::OP_ADD, 7, 8, 9),
      TestHelpers.make_instruction(TestHelpers::OP_HALT, 0, 0, 0),
      TestHelpers.make_instruction(TestHelpers::OP_NOP, 0, 0, 0),
      TestHelpers.make_instruction(TestHelpers::OP_NOP, 0, 0, 0),
      TestHelpers.make_instruction(TestHelpers::OP_NOP, 0, 0, 0),
      TestHelpers.make_instruction(TestHelpers::OP_NOP, 0, 0, 0)
    ]

    completed = []
    p = TestHelpers.new_test_pipeline(instrs, completed)
    stats = p.run(100)

    assert_equal stats.instructions_completed, completed.length
    assert_equal 4, stats.instructions_completed, "expected 4 completed instructions"
  end
end
