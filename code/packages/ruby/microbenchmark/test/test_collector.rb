# frozen_string_literal: true

require_relative "test_helper"

class MicrobenchmarkCollectorTest < Minitest::Test
  MB = CodingAdventures::Microbenchmark

  def test_collects_balanced_seeded_pairs_and_repeats_order
    config = config(pairs: 7, target_ns: 1, max_batch: 8, seed: -71)
    first = collect_with_workload(config)
    second = collect_with_workload(config)

    assert_equal 1, first.batch_size
    assert_equal first.pairs.map(&:order), second.pairs.map(&:order)
    assert_equal 4, first.pairs.count { |pair| pair.order == "baseline-first" }
    assert_equal 3, first.pairs.count { |pair| pair.order == "candidate-first" }
    assert first.pairs.all? { |pair| pair.baseline_ns == 4 && pair.candidate_ns == 3 }
  end

  def test_calibrates_a_common_batch_up_to_a_configured_limit_over_65536
    tick = 0
    batch = ->(operations) {
      tick += operations
      operations
    }
    report = MB.collect(
      baseline: batch,
      candidate: batch,
      expected_checksum: 1,
      config: config(pairs: 1, target_ns: 65_537, max_batch: MB::MAX_I64),
      clock: -> { tick }
    )
    assert_equal 131_072, report.batch_size
    assert_equal 1, report.pairs.length
  end

  def test_full_signed_i64_values_stay_exact_through_json_and_replay
    maximum = MB::MAX_I64
    minimum = MB::MIN_I64
    config = config(pairs: 3, seed: minimum, target_ns: 1, max_batch: maximum)
    time = maximum - 1_000
    report = MB.collect(
      baseline: ->(_operations) { maximum },
      candidate: ->(_operations) { maximum },
      expected_checksum: maximum,
      config: config,
      clock: -> { time += 10 }
    )
    serialized = JSON.generate(report.to_h)
    assert_includes serialized, '"seed":-9223372036854775808'
    assert_includes serialized, '"baseline_ns":10'
    assert_equal report, MB.replay_json(serialized)
  end

  def test_negative_full_width_checksum_is_valid_when_batch_is_one
    minimum = MB::MIN_I64
    time = 0
    report = MB.collect(
      baseline: ->(_operations) { minimum },
      candidate: ->(_operations) { minimum },
      expected_checksum: minimum,
      config: config(pairs: 1, seed: minimum, target_ns: 1, max_batch: 1),
      clock: -> { time += 1 }
    )
    assert_equal 1, report.pairs.first.baseline_ns
    assert_equal 1, report.pairs.first.candidate_ns
  end

  def test_rejects_config_errors_before_running_workloads
    calls = 0
    workload = ->(_operations) {
      calls += 1
      1
    }
    invalid = [
      config(pairs: 0), config(pairs: 10_001), config(confidence_percent: 0),
      config(minimum_factor: 1.0), config(target_ns: 0), config(max_batch: 0),
      config(seed: MB::MAX_I64 + 1), config(max_batch: MB::MAX_I64 + 1)
    ]
    invalid.each do |bad_config|
      assert_raises(ArgumentError) do
        MB.collect(baseline: workload, candidate: workload, expected_checksum: 1, config: bad_config, clock: -> { 1 })
      end
    end
    assert_equal 0, calls
  end

  def test_rejects_checksum_overflow_and_mismatch
    time = 0
    assert_raises(ArgumentError) do
      MB.collect(
        baseline: ->(_operations) { MB::MAX_I64 },
        candidate: ->(_operations) { MB::MAX_I64 },
        expected_checksum: MB::MAX_I64,
        config: config(pairs: 1, target_ns: MB::MAX_I64, max_batch: 2),
        clock: -> { time += 1 }
      )
    end
    assert_raises(ArgumentError) do
      MB.collect(baseline: ->(_operations) { 2 }, candidate: ->(_operations) { 1 }, expected_checksum: 1,
        config: config(pairs: 1, target_ns: 1), clock: -> { 1 })
    end
    assert_raises(ArgumentError) do
      MB.collect(baseline: ->(_operations) { MB::MAX_I64 + 1 }, candidate: ->(_operations) { 1 }, expected_checksum: 1,
        config: config(pairs: 1), clock: -> { 1 })
    end
  end

  def test_rejects_workload_errors_invalid_clocks_zero_measurements_and_exhaustion
    assert_raises(RuntimeError) do
      MB.collect(baseline: ->(_operations) { raise "workload failed" }, candidate: ->(_operations) { 1 },
        expected_checksum: 1, config: config(pairs: 1), clock: -> { 1 })
    end

    [-1, MB::MAX_I64 + 1, 1.5, true].each do |tick|
      assert_raises(ArgumentError) do
        MB.collect(baseline: ->(operations) { operations }, candidate: ->(operations) { operations },
          expected_checksum: 1, config: config(pairs: 1, target_ns: 1), clock: -> { tick })
      end
    end

    ticks = [2, 1]
    assert_raises(ArgumentError) do
      MB.collect(baseline: ->(operations) { operations }, candidate: ->(operations) { operations },
        expected_checksum: 1, config: config(pairs: 1, target_ns: 1), clock: -> { ticks.shift })
    end

    ticks = [0, 1, 1, 2, 2, 2, 2, 2]
    assert_raises(ArgumentError) do
      MB.collect(baseline: ->(operations) { operations }, candidate: ->(operations) { operations },
        expected_checksum: 1, config: config(pairs: 1, target_ns: 1, max_batch: 1), clock: -> { ticks.shift })
    end

    time = 0
    assert_raises(ArgumentError) do
      MB.collect(baseline: ->(operations) { operations }, candidate: ->(operations) { operations },
        expected_checksum: 1, config: config(pairs: 1, target_ns: 2, max_batch: 1), clock: -> { time += 1 })
    end
  end

  private

  def config(**overrides)
    values = {
      pairs: 5,
      confidence_percent: 95,
      minimum_factor: 1.05,
      seed: 0,
      target_ns: 1,
      max_batch: 16
    }.merge(overrides)
    MB::Config.new(**values)
  end

  def collect_with_workload(configuration)
    tick = 0
    MB.collect(
      baseline: ->(operations) {
        tick += 4 * operations
        10 * operations
      },
      candidate: ->(operations) {
        tick += 3 * operations
        10 * operations
      },
      expected_checksum: 10,
      config: configuration,
      clock: -> { tick }
    )
  end
end
