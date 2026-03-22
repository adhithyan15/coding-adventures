# frozen_string_literal: true

require_relative "test_helper"

# Tests for the Core class -- the complete processor core.
class TestCoreConstruction < Minitest::Test
  def make_simple_core
    config = CodingAdventures::Core.simple_config
    decoder = CodingAdventures::Core::MockDecoder.new
    CodingAdventures::Core::Core.new(config, decoder)
  end

  def make_default_core
    config = CodingAdventures::Core.default_core_config
    decoder = CodingAdventures::Core::MockDecoder.new
    CodingAdventures::Core::Core.new(config, decoder)
  end

  def test_core_construction
    c = make_simple_core
    refute_nil c.pipeline
    refute_nil c.predictor
    refute_nil c.reg_file
    refute_nil c.mem_ctrl
    refute_nil c.cache_hierarchy
  end

  def test_simple_config_runs
    c = make_simple_core
    program = CodingAdventures::Core.encode_program(CodingAdventures::Core.encode_halt)
    c.load_program(program, 0)
    stats = c.run(100)
    assert stats.total_cycles > 0
  end

  def test_complex_config_runs
    config = CodingAdventures::Core.cortex_a78_like_config
    decoder = CodingAdventures::Core::MockDecoder.new
    c = CodingAdventures::Core::Core.new(config, decoder)

    program = CodingAdventures::Core.encode_program(CodingAdventures::Core.encode_halt)
    c.load_program(program, 0)
    stats = c.run(200)
    assert stats.total_cycles > 0
  end

  def test_missing_optional
    config = CodingAdventures::Core.simple_config
    config.l2_cache = nil
    config.fp_unit = nil

    c = CodingAdventures::Core::Core.new(config, CodingAdventures::Core::MockDecoder.new)
    program = CodingAdventures::Core.encode_program(
      CodingAdventures::Core.encode_addi(1, 0, 10),
      CodingAdventures::Core.encode_halt
    )
    c.load_program(program, 0)
    c.run(100)
    assert c.halted?
  end

  def test_default_core_config
    c = make_default_core
    program = CodingAdventures::Core.encode_program(
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_halt
    )
    c.load_program(program, 0)
    c.run(100)
    assert c.halted?
  end
end

class TestCoreSingleInstruction < Minitest::Test
  def make_core
    config = CodingAdventures::Core.simple_config
    decoder = CodingAdventures::Core::MockDecoder.new
    CodingAdventures::Core::Core.new(config, decoder)
  end

  def test_nop
    c = make_core
    program = CodingAdventures::Core.encode_program(
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_halt
    )
    c.load_program(program, 0)
    c.run(100)
    assert c.halted?

    # NOP should not modify any register.
    c.reg_file.count.times do |i|
      assert_equal 0, c.reg_file.read(i), "register R#{i} should be 0 after NOP"
    end
  end

  def test_addi
    c = make_core
    program = CodingAdventures::Core.encode_program(
      CodingAdventures::Core.encode_addi(1, 0, 42),
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_halt
    )
    c.load_program(program, 0)
    c.run(100)
    assert c.halted?
    assert_equal 42, c.read_register(1)
  end

  def test_add
    c = make_core
    program = CodingAdventures::Core.encode_program(
      CodingAdventures::Core.encode_addi(1, 0, 10),
      CodingAdventures::Core.encode_addi(2, 0, 20),
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_add(3, 1, 2),
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_halt
    )
    c.load_program(program, 0)
    c.run(200)
    assert c.halted?
    assert_equal 10, c.read_register(1)
    assert_equal 20, c.read_register(2)
    assert_equal 30, c.read_register(3)
  end

  def test_sub
    c = make_core
    program = CodingAdventures::Core.encode_program(
      CodingAdventures::Core.encode_addi(1, 0, 50),
      CodingAdventures::Core.encode_addi(2, 0, 20),
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_sub(3, 1, 2),
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_halt
    )
    c.load_program(program, 0)
    c.run(200)
    assert c.halted?
    assert_equal 30, c.read_register(3)
  end

  def test_halt
    c = make_core
    program = CodingAdventures::Core.encode_program(CodingAdventures::Core.encode_halt)
    c.load_program(program, 0)
    c.run(100)
    assert c.halted?
  end

  def test_load
    c = make_core
    c.mem_ctrl.write_word(512, 0xDEAD)
    program = CodingAdventures::Core.encode_program(
      CodingAdventures::Core.encode_addi(1, 0, 0),
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_load(2, 1, 512),
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_halt
    )
    c.load_program(program, 0)
    c.run(200)
    assert c.halted?
    assert_equal 0xDEAD, c.read_register(2)
  end

  def test_store
    c = make_core
    program = CodingAdventures::Core.encode_program(
      CodingAdventures::Core.encode_addi(1, 0, 0),
      CodingAdventures::Core.encode_addi(2, 0, 0x42),
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_store(1, 2, 512),
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_halt
    )
    c.load_program(program, 0)
    c.run(200)
    assert c.halted?
    assert_equal 0x42, c.mem_ctrl.read_word(512)
  end
end

class TestCoreProgramExecution < Minitest::Test
  def make_core
    config = CodingAdventures::Core.simple_config
    decoder = CodingAdventures::Core::MockDecoder.new
    CodingAdventures::Core::Core.new(config, decoder)
  end

  def test_simple_sequence
    c = make_core
    c.mem_ctrl.write_word(512, 100)

    program = CodingAdventures::Core.encode_program(
      CodingAdventures::Core.encode_addi(1, 0, 0),
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_load(2, 1, 512),
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_addi(3, 2, 50),
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_store(1, 3, 516),
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_halt
    )
    c.load_program(program, 0)
    c.run(500)

    assert c.halted?
    assert_equal 100, c.read_register(2)
    assert_equal 150, c.read_register(3)
    assert_equal 150, c.mem_ctrl.read_word(516)
  end

  def test_counting_program
    c = make_core
    program = CodingAdventures::Core.encode_program(
      CodingAdventures::Core.encode_addi(1, 0, 0),
      CodingAdventures::Core.encode_addi(2, 0, 1),
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_add(1, 1, 2),
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_add(1, 1, 2),
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_add(1, 1, 2),
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_add(1, 1, 2),
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_add(1, 1, 2),
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_halt
    )
    c.load_program(program, 0)
    c.run(500)

    assert c.halted?
    assert_equal 5, c.read_register(1)
  end
end

class TestCoreStatistics < Minitest::Test
  def make_core
    config = CodingAdventures::Core.simple_config
    decoder = CodingAdventures::Core::MockDecoder.new
    CodingAdventures::Core::Core.new(config, decoder)
  end

  def test_ipc_calculation
    c = make_core
    program = CodingAdventures::Core.encode_program(
      CodingAdventures::Core.encode_addi(1, 0, 1),
      CodingAdventures::Core.encode_addi(2, 0, 2),
      CodingAdventures::Core.encode_addi(3, 0, 3),
      CodingAdventures::Core.encode_halt
    )
    c.load_program(program, 0)
    stats = c.run(200)

    assert stats.instructions_completed > 0
    assert stats.total_cycles > 0

    expected_ipc = stats.instructions_completed.to_f / stats.total_cycles
    assert_in_delta expected_ipc, stats.ipc, 0.0001

    # CPI should be the inverse.
    expected_cpi = stats.total_cycles.to_f / stats.instructions_completed
    assert_in_delta expected_cpi, stats.cpi, 0.0001
  end

  def test_aggregate_stats
    c = make_core
    program = CodingAdventures::Core.encode_program(
      CodingAdventures::Core.encode_addi(1, 0, 10),
      CodingAdventures::Core.encode_addi(2, 0, 20),
      CodingAdventures::Core.encode_halt
    )
    c.load_program(program, 0)
    stats = c.run(200)

    # Pipeline stats should be populated.
    refute_nil stats.pipeline_stats
    assert stats.pipeline_stats.total_cycles > 0

    # Predictor stats should exist.
    refute_nil stats.predictor_stats

    # Cache stats should have L1I and L1D.
    assert stats.cache_stats.key?("L1I")
    assert stats.cache_stats.key?("L1D")

    # L1I should have been accessed (instruction fetches).
    assert stats.cache_stats["L1I"].total_accesses > 0
  end

  def test_stats_string
    c = make_core
    program = CodingAdventures::Core.encode_program(
      CodingAdventures::Core.encode_addi(1, 0, 1),
      CodingAdventures::Core.encode_halt
    )
    c.load_program(program, 0)
    stats = c.run(100)
    refute_empty stats.to_s
  end

  def test_step_returns_snapshot
    c = make_core
    program = CodingAdventures::Core.encode_program(CodingAdventures::Core.encode_halt)
    c.load_program(program, 0)
    snap = c.step
    refute_nil snap
  end

  def test_cycle_tracking
    c = make_core
    program = CodingAdventures::Core.encode_program(CodingAdventures::Core.encode_halt)
    c.load_program(program, 0)
    assert_equal 0, c.cycle
    c.step
    assert_equal 1, c.cycle
  end

  def test_write_register
    c = make_core
    c.write_register(5, 123)
    assert_equal 123, c.read_register(5)
  end

  def test_config_accessor
    c = make_core
    assert_equal "Simple", c.config.name
  end
end
