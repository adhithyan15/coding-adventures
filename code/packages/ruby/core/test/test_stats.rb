# frozen_string_literal: true

require_relative "test_helper"

# Tests for CoreStats -- aggregate performance statistics.
class TestCoreStats < Minitest::Test
  def test_ipc_zero_cycles
    s = CodingAdventures::Core::CoreStats.new
    assert_equal 0.0, s.ipc
  end

  def test_cpi_zero_instructions
    s = CodingAdventures::Core::CoreStats.new
    assert_equal 0.0, s.cpi
  end

  def test_ipc_calculation
    s = CodingAdventures::Core::CoreStats.new
    s.instructions_completed = 10
    s.total_cycles = 20
    assert_in_delta 0.5, s.ipc, 0.001
  end

  def test_cpi_calculation
    s = CodingAdventures::Core::CoreStats.new
    s.instructions_completed = 10
    s.total_cycles = 20
    assert_in_delta 2.0, s.cpi, 0.001
  end

  def test_to_s_not_empty
    s = CodingAdventures::Core::CoreStats.new
    s.instructions_completed = 5
    s.total_cycles = 10
    str = s.to_s
    refute_empty str
    assert_includes str, "Core Statistics"
  end

  def test_to_s_with_pipeline_stats
    s = CodingAdventures::Core::CoreStats.new
    s.instructions_completed = 5
    s.total_cycles = 10
    s.pipeline_stats = CodingAdventures::CpuPipeline::PipelineStats.new
    str = s.to_s
    assert_includes str, "Pipeline"
  end

  def test_to_s_with_predictor_stats
    s = CodingAdventures::Core::CoreStats.new
    s.instructions_completed = 5
    s.total_cycles = 10
    s.predictor_stats = CodingAdventures::BranchPredictor::PredictionStats.new
    str = s.to_s
    assert_includes str, "Branch Prediction"
  end
end
