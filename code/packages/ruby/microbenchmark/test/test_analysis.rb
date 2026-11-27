# frozen_string_literal: true

require_relative "test_helper"

class MicrobenchmarkAnalysisTest < Minitest::Test
  MB = CodingAdventures::Microbenchmark

  def test_replays_every_shared_fixture_analysis_and_checks_copy
    package_fixture = File.read(File.expand_path("../testdata/microbenchmark-pairs-v1.json", __dir__))
    canonical = File.read(File.expand_path("../../../../specs/fixtures/microbenchmark-pairs-v1.json", __dir__))
    assert_equal canonical, package_fixture

    fixture = JSON.parse(package_fixture)
    fixture.fetch("cases").each do |example|
      pairs = example.fetch("candidate_ns").each_with_index.map do |candidate, index|
        MB::Pair.new(
          baseline_ns: example.fetch("baseline_ns"),
          candidate_ns: candidate,
          order: index.even? ? "baseline-first" : "candidate-first"
        )
      end
      expected = MB::Analysis.new(
        ratio: example.fetch("ratio"),
        interval: example.fetch("interval"),
        verdict: example.fetch("verdict")
      )
      assert_equal expected, MB.analyze(
        pairs,
        confidence_percent: fixture.fetch("confidence_percent"),
        minimum_factor: fixture.fetch("minimum_factor")
      ), example.fetch("name")
    end
  end

  def test_analysis_uses_strict_decision_boundaries
    faster_boundary = Array.new(6) { MB::Pair.new(baseline_ns: 5, candidate_ns: 4, order: "baseline-first") }
    slower_boundary = Array.new(6) { MB::Pair.new(baseline_ns: 4, candidate_ns: 5, order: "candidate-first") }
    assert_equal "equivalent", MB.analyze(faster_boundary, minimum_factor: 1.25).verdict
    assert_equal "equivalent", MB.analyze(slower_boundary, minimum_factor: 1.25).verdict
    assert_equal 1.0, MB.analyze([MB::Pair.new(baseline_ns: MB::MAX_I64, candidate_ns: MB::MAX_I64, order: "baseline-first")]).ratio
  end

  def test_analysis_accepts_unbalanced_arbitrary_observations
    pairs = Array.new(6) { MB::Pair.new(baseline_ns: 100, candidate_ns: 100, order: "baseline-first") }
    assert_equal "equivalent", MB.analyze(pairs).verdict
  end

  def test_analysis_bounds_pair_count_before_mapping_pair_members
    pair = MB::Pair.new(baseline_ns: 1, candidate_ns: 1, order: "baseline-first")
    assert_equal 1.0, MB.analyze(Array.new(MB::MAX_PAIRS, pair)).ratio
    error = assert_raises(ArgumentError) { MB.analyze(Array.new(MB::MAX_PAIRS + 1, :invalid)) }
    assert_match(/at most 10000/, error.message)
    confidence_error = assert_raises(ArgumentError) do
      MB.analyze(Array.new(MB::MAX_PAIRS + 1, :invalid), confidence_percent: 0)
    end
    assert_match(/confidence_percent/, confidence_error.message)
  end

  def test_analysis_rejects_invalid_pairs_and_options
    assert_raises(ArgumentError) { MB.analyze([]) }
    assert_raises(ArgumentError) { MB.analyze([:not_a_pair]) }
    assert_raises(ArgumentError) { MB.analyze([MB::Pair.new(baseline_ns: 0, candidate_ns: 1, order: "baseline-first")]) }
    assert_raises(ArgumentError) { MB.analyze([MB::Pair.new(baseline_ns: 1, candidate_ns: 1, order: "first")]) }
    [true, 0, 100, 1.5].each do |confidence|
      assert_raises(ArgumentError) do
        MB.analyze([MB::Pair.new(baseline_ns: 1, candidate_ns: 1, order: "baseline-first")], confidence_percent: confidence)
      end
    end
    [1.0, Float::INFINITY, Float::NAN, "1.1"].each do |factor|
      assert_raises(ArgumentError) do
        MB.analyze([MB::Pair.new(baseline_ns: 1, candidate_ns: 1, order: "baseline-first")], minimum_factor: factor)
      end
    end
    [0, MB::MAX_I64 + 1, 1.5, true].each do |duration|
      assert_raises(ArgumentError) do
        MB.analyze([MB::Pair.new(baseline_ns: duration, candidate_ns: 1, order: "baseline-first")])
      end
    end
  end
end
