# frozen_string_literal: true

require_relative "test_helper"

# Edge-case tests to improve branch coverage.
class TestEdgeCases < Minitest::Test
  def test_memory_controller_negative_address_read
    mem = Array.new(64, 0)
    mc = CodingAdventures::Core::MemoryController.new(mem, 1)
    assert_equal 0, mc.read_word(-1)
  end

  def test_memory_controller_negative_address_write
    mem = Array.new(64, 0)
    mc = CodingAdventures::Core::MemoryController.new(mem, 1)
    mc.write_word(-1, 42) # should not raise
  end

  def test_memory_controller_negative_load_program
    mem = Array.new(64, 0)
    mc = CodingAdventures::Core::MemoryController.new(mem, 1)
    mc.load_program([1, 2], -1) # should not raise
  end

  def test_memory_controller_read_memory_oob
    mem = Array.new(64, 0)
    mc = CodingAdventures::Core::MemoryController.new(mem, 2)
    mc.request_read(1000, 4, 0) # out-of-bounds address
    mc.tick
    result = mc.tick
    assert_equal 1, result.length
    assert_equal 4, result[0].data.length
  end

  def test_memory_controller_write_memory_oob
    mem = Array.new(64, 0)
    mc = CodingAdventures::Core::MemoryController.new(mem, 1)
    mc.request_write(1000, [0xAB], 0) # out-of-bounds
    mc.tick # should not raise
  end

  def test_register_file_64_bit_width
    cfg = CodingAdventures::Core::RegisterFileConfig.new(count: 4, width: 64, zero_register: false)
    rf = CodingAdventures::Core::RegisterFile.new(cfg)
    rf.write(1, 0x7FFFFFFFFFFFFFFF)
    assert_equal 0x7FFFFFFFFFFFFFFF, rf.read(1)
  end

  def test_core_stats_to_s_with_cache_stats
    s = CodingAdventures::Core::CoreStats.new
    s.instructions_completed = 5
    s.total_cycles = 10
    s.cache_stats = {"L1D" => CodingAdventures::Cache::CacheStats.new}
    str = s.to_s
    assert_includes str, "Cache Performance"
  end

  def test_core_step_when_halted
    config = CodingAdventures::Core.simple_config
    decoder = CodingAdventures::Core::MockDecoder.new
    c = CodingAdventures::Core::Core.new(config, decoder)
    program = CodingAdventures::Core.encode_program(CodingAdventures::Core.encode_halt)
    c.load_program(program, 0)
    c.run(100)
    assert c.halted?

    # Stepping when halted should return a snapshot without advancing.
    snap = c.step
    refute_nil snap
  end

  def test_core_config_with_zero_memory
    config = CodingAdventures::Core.simple_config
    config.memory_size = 0
    config.memory_latency = 0
    decoder = CodingAdventures::Core::MockDecoder.new
    c = CodingAdventures::Core::Core.new(config, decoder)
    # Should use defaults.
    refute_nil c.mem_ctrl
  end

  def test_core_config_with_zero_btb
    config = CodingAdventures::Core.simple_config
    config.btb_size = 0
    decoder = CodingAdventures::Core::MockDecoder.new
    c = CodingAdventures::Core::Core.new(config, decoder)
    refute_nil c
  end

  def test_core_config_with_empty_pipeline
    config = CodingAdventures::Core.default_core_config
    config.pipeline = nil
    decoder = CodingAdventures::Core::MockDecoder.new
    c = CodingAdventures::Core::Core.new(config, decoder)
    refute_nil c.pipeline
  end

  def test_decoder_store_execute
    d = CodingAdventures::Core::MockDecoder.new
    rf = CodingAdventures::Core::RegisterFile.new
    rf.write(1, 100)
    rf.write(2, 42)

    token = CodingAdventures::CpuPipeline.new_token
    d.decode(CodingAdventures::Core.encode_store(1, 2, 8), token)
    d.execute(token, rf)
    assert_equal 108, token.alu_result # effective address
    assert_equal 42, token.write_data  # data to store
  end

  def test_decoder_nop_execute
    d = CodingAdventures::Core::MockDecoder.new
    rf = CodingAdventures::Core::RegisterFile.new
    token = CodingAdventures::CpuPipeline.new_token
    d.decode(CodingAdventures::Core.encode_nop, token)
    d.execute(token, rf)
    assert_equal 0, token.alu_result
  end

  def test_decoder_halt_execute
    d = CodingAdventures::Core::MockDecoder.new
    rf = CodingAdventures::Core::RegisterFile.new
    token = CodingAdventures::CpuPipeline.new_token
    d.decode(CodingAdventures::Core.encode_halt, token)
    d.execute(token, rf)
    assert_equal 0, token.alu_result
  end

  def test_decoder_unknown_execute
    d = CodingAdventures::Core::MockDecoder.new
    rf = CodingAdventures::Core::RegisterFile.new
    token = CodingAdventures::CpuPipeline.new_token
    d.decode(0xFF << 24, token) # unknown opcode -> NOP
    d.execute(token, rf)
    assert_equal 0, token.alu_result
  end

  def test_decoder_negative_immediate_sign_extension
    d = CodingAdventures::Core::MockDecoder.new
    # Encode ADDI with immediate that has bit 11 set (negative)
    raw = (0x06 << 24) | (1 << 20) | (0 << 16) | 0xFFF # imm = 0xFFF -> -1
    token = CodingAdventures::CpuPipeline.new_token
    d.decode(raw, token)
    assert_equal(-1, token.immediate)
  end

  def test_multi_core_config_zero_num_cores
    config = CodingAdventures::Core::MultiCoreConfig.new(
      num_cores: 0,
      core_config: CodingAdventures::Core.simple_config,
      memory_size: 65536,
      memory_latency: 100
    )
    decoders = [CodingAdventures::Core::MockDecoder.new]
    mc = CodingAdventures::Core::MultiCoreCPU.new(config, decoders)
    assert_equal 1, mc.cores.length
  end

  def test_multi_core_config_zero_memory
    config = CodingAdventures::Core::MultiCoreConfig.new(
      num_cores: 1,
      core_config: CodingAdventures::Core.simple_config,
      memory_size: 0,
      memory_latency: 0
    )
    decoders = [CodingAdventures::Core::MockDecoder.new]
    mc = CodingAdventures::Core::MultiCoreCPU.new(config, decoders)
    refute_nil mc.shared_memory_controller
  end

  def test_all_halted_when_not_halted
    config = CodingAdventures::Core.default_multi_core_config
    decoders = [CodingAdventures::Core::MockDecoder.new, CodingAdventures::Core::MockDecoder.new]
    mc = CodingAdventures::Core::MultiCoreCPU.new(config, decoders)
    refute mc.all_halted?
  end

  def test_interrupt_controller_multiple_pending
    ic = CodingAdventures::Core::InterruptController.new(4)
    ic.raise_interrupt(1, 0)
    ic.raise_interrupt(2, 0)
    ic.raise_interrupt(3, 1)
    assert_equal 3, ic.pending_count
    assert_equal 2, ic.pending_for_core(0).length
    assert_equal 1, ic.pending_for_core(1).length
  end

  def test_register_file_to_s_with_no_nonzero
    rf = CodingAdventures::Core::RegisterFile.new
    s = rf.to_s
    assert_includes s, "RegisterFile(16x32):"
    # No register values should be printed since all are 0.
    refute_includes s, "R1="
  end

  def test_core_with_l2_cache
    config = CodingAdventures::Core.cortex_a78_like_config
    decoder = CodingAdventures::Core::MockDecoder.new
    c = CodingAdventures::Core::Core.new(config, decoder)
    program = CodingAdventures::Core.encode_program(
      CodingAdventures::Core.encode_addi(1, 0, 1),
      CodingAdventures::Core.encode_halt
    )
    c.load_program(program, 0)
    stats = c.run(200)
    assert stats.cache_stats.key?("L2")
  end
end
