# frozen_string_literal: true

require_relative "test_helper"

# Tests for MultiCoreCPU -- multi-core processor.
class TestMultiCoreConstruction < Minitest::Test
  def test_construction
    config = CodingAdventures::Core.default_multi_core_config
    decoders = [CodingAdventures::Core::MockDecoder.new, CodingAdventures::Core::MockDecoder.new]

    mc = CodingAdventures::Core::MultiCoreCPU.new(config, decoders)
    assert_equal 2, mc.cores.length
    refute_nil mc.interrupt_controller
    refute_nil mc.shared_memory_controller
  end
end

class TestMultiCoreExecution < Minitest::Test
  def make_multi_core
    config = CodingAdventures::Core.default_multi_core_config
    decoders = [CodingAdventures::Core::MockDecoder.new, CodingAdventures::Core::MockDecoder.new]
    CodingAdventures::Core::MultiCoreCPU.new(config, decoders)
  end

  def test_independent_programs
    mc = make_multi_core

    # Core 0: R1 = 10
    prog0 = CodingAdventures::Core.encode_program(
      CodingAdventures::Core.encode_addi(1, 0, 10),
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_halt
    )
    # Core 1: R1 = 20, loaded at a different address
    prog1 = CodingAdventures::Core.encode_program(
      CodingAdventures::Core.encode_addi(1, 0, 20),
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_nop,
      CodingAdventures::Core.encode_halt
    )

    mc.load_program(0, prog0, 0)
    mc.load_program(1, prog1, 4096)
    mc.run(200)

    assert mc.all_halted?
    assert_equal 10, mc.cores[0].read_register(1)
    assert_equal 20, mc.cores[1].read_register(1)
  end

  def test_shared_memory
    mc = make_multi_core

    # Write a value to shared memory.
    mc.shared_memory_controller.write_word(512, 0xCAFE)

    # Core 0 loads from that address.
    prog0 = CodingAdventures::Core.encode_program(
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
    mc.load_program(0, prog0, 0)

    # Core 1 just halts.
    prog1 = CodingAdventures::Core.encode_program(CodingAdventures::Core.encode_halt)
    mc.load_program(1, prog1, 4096)

    mc.run(200)
    assert_equal 0xCAFE, mc.cores[0].read_register(2)
  end

  def test_per_core_stats
    mc = make_multi_core

    prog = CodingAdventures::Core.encode_program(
      CodingAdventures::Core.encode_addi(1, 0, 1),
      CodingAdventures::Core.encode_halt
    )
    mc.load_program(0, prog, 0)
    mc.load_program(1, prog, 4096)

    stats = mc.run(200)
    assert_equal 2, stats.length
    stats.each_with_index do |s, i|
      assert s.total_cycles > 0, "core #{i} should have cycles"
    end
  end

  def test_step_returns_snapshots
    mc = make_multi_core
    prog = CodingAdventures::Core.encode_program(CodingAdventures::Core.encode_halt)
    mc.load_program(0, prog, 0)
    mc.load_program(1, prog, 4096)

    snapshots = mc.step
    assert_equal 2, snapshots.length
  end

  def test_cycle_tracking
    mc = make_multi_core
    prog = CodingAdventures::Core.encode_program(CodingAdventures::Core.encode_halt)
    mc.load_program(0, prog, 0)
    mc.load_program(1, prog, 4096)

    assert_equal 0, mc.cycle
    mc.step
    assert_equal 1, mc.cycle
  end

  def test_load_program_invalid_core
    mc = make_multi_core
    prog = CodingAdventures::Core.encode_program(CodingAdventures::Core.encode_halt)
    # Should not raise for invalid core IDs.
    mc.load_program(-1, prog, 0)
    mc.load_program(99, prog, 0)
  end
end
