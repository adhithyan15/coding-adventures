# frozen_string_literal: true

require_relative "test_helper"

# Tests for CoreConfig, presets, and the branch predictor factory.
class TestRegisterFileConfig < Minitest::Test
  def test_default_register_file_config
    cfg = CodingAdventures::Core.default_register_file_config
    assert_equal 16, cfg.count
    assert_equal 32, cfg.width
    assert_equal true, cfg.zero_register
  end

  def test_custom_register_file_config
    cfg = CodingAdventures::Core::RegisterFileConfig.new(count: 31, width: 64, zero_register: false)
    assert_equal 31, cfg.count
    assert_equal 64, cfg.width
    assert_equal false, cfg.zero_register
  end
end

class TestFPUnitConfig < Minitest::Test
  def test_fp_unit_config
    cfg = CodingAdventures::Core::FPUnitConfig.new(formats: ["fp32", "fp64"], pipeline_depth: 4)
    assert_equal ["fp32", "fp64"], cfg.formats
    assert_equal 4, cfg.pipeline_depth
  end
end

class TestCoreConfig < Minitest::Test
  def test_default_core_config
    cfg = CodingAdventures::Core.default_core_config
    assert_equal "Default", cfg.name
    assert cfg.hazard_detection
    assert cfg.forwarding
    assert_nil cfg.register_file
    assert_nil cfg.fp_unit
    assert_nil cfg.l1i_cache
    assert_nil cfg.l1d_cache
    assert_nil cfg.l2_cache
    assert_equal 65536, cfg.memory_size
    assert_equal 100, cfg.memory_latency
  end

  def test_simple_config_fields
    cfg = CodingAdventures::Core.simple_config
    assert_equal "Simple", cfg.name
    assert_equal 5, cfg.pipeline.stages.length
    assert_equal "static_always_not_taken", cfg.branch_predictor_type
    refute_nil cfg.register_file
    assert_equal 16, cfg.register_file.count
    assert_nil cfg.fp_unit
    refute_nil cfg.l1i_cache
    assert_equal 4096, cfg.l1i_cache.total_size
    refute_nil cfg.l1d_cache
    assert_equal 4096, cfg.l1d_cache.total_size
    assert_nil cfg.l2_cache
  end

  def test_cortex_a78_like_config_fields
    cfg = CodingAdventures::Core.cortex_a78_like_config
    assert_equal "CortexA78Like", cfg.name
    assert_equal 13, cfg.pipeline.stages.length
    assert_equal "two_bit", cfg.branch_predictor_type
    assert_equal 4096, cfg.branch_predictor_size
    refute_nil cfg.register_file
    assert_equal 31, cfg.register_file.count
    refute_nil cfg.fp_unit
    refute_nil cfg.l1i_cache
    assert_equal 65536, cfg.l1i_cache.total_size
    refute_nil cfg.l2_cache
    assert_equal 262144, cfg.l2_cache.total_size
  end
end

class TestMultiCoreConfig < Minitest::Test
  def test_default_multi_core_config
    cfg = CodingAdventures::Core.default_multi_core_config
    assert_equal 2, cfg.num_cores
    assert_equal 1048576, cfg.memory_size
    assert_equal 100, cfg.memory_latency
    refute_nil cfg.core_config
  end
end

class TestCreateBranchPredictor < Minitest::Test
  def test_all_predictor_types
    types = %w[
      static_always_taken
      static_always_not_taken
      static_btfnt
      one_bit
      two_bit
      unknown_type
    ]

    types.each do |type_name|
      predictor = CodingAdventures::Core.create_branch_predictor(type_name, 256)
      refute_nil predictor, "createBranchPredictor(#{type_name.inspect}) returned nil"
      # Should respond to the predictor interface.
      assert_respond_to predictor, :predict
      assert_respond_to predictor, :update
      assert_respond_to predictor, :stats
    end
  end
end
