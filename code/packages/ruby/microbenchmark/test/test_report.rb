# frozen_string_literal: true

require_relative "test_helper"

class MicrobenchmarkReportTest < Minitest::Test
  MB = CodingAdventures::Microbenchmark

  def test_json_round_trip_preserves_signed_integer_tokens_and_analysis
    tick = 0
    report = MB.collect(
      baseline: ->(operations) { operations * 7 },
      candidate: ->(operations) { operations * 7 },
      expected_checksum: 7,
      config: MB::Config.new(pairs: 3, confidence_percent: 95, minimum_factor: 1.05, seed: MB::MIN_I64,
        target_ns: 1, max_batch: MB::MAX_I64),
      clock: -> { tick += 17 }
    )
    text = JSON.generate(report.to_h)
    assert_equal report, MB.replay_json(text)
    assert_includes text, '"seed":-9223372036854775808'
    assert_includes text, '"batch_size":1'

    large = text.sub('"baseline_ns":17', '"baseline_ns":9007199254740993')
    parsed = JSON.parse(large)
    assert_instance_of Integer, parsed.fetch("pairs").first.fetch("baseline_ns")
    assert_equal 9_007_199_254_740_993, parsed.fetch("pairs").first.fetch("baseline_ns")
    assert_equal 9_007_199_254_740_993, MB.replay_json(large).pairs.first.baseline_ns
  end

  def test_replay_rejects_forged_analysis_and_unbalanced_collection_order
    report = simple_report
    forged_analysis = deep_copy(report.to_h)
    forged_analysis.fetch("analysis")["verdict"] = "faster"
    assert_raises(ArgumentError) { MB.replay_report(forged_analysis) }

    forged_order = deep_copy(report.to_h)
    forged_order.fetch("pairs").each { |pair| pair["order"] = "candidate-first" }
    assert_raises(ArgumentError) { MB.replay_report(forged_order) }
  end

  def test_replay_rejects_invalid_shape_ranges_and_order
    report = simple_report.to_h
    [
      report.merge("batch_size" => 0),
      report.merge("batch_size" => MB::MAX_I64 + 1),
      report.merge("pairs" => []),
      report.merge("analysis" => nil)
    ].each do |invalid|
      assert_raises(ArgumentError) { MB.replay_report(invalid) }
    end

    invalid_pair = deep_copy(report)
    invalid_pair.fetch("pairs").first["candidate_ns"] = 0
    assert_raises(ArgumentError) { MB.replay_report(invalid_pair) }
    invalid_order = deep_copy(report)
    invalid_order.fetch("pairs").first["order"] = "baseline"
    assert_raises(ArgumentError) { MB.replay_report(invalid_order) }

    assert_raises(ArgumentError) { MB.replay_json("not JSON") }
    assert_raises(ArgumentError) { MB.replay_json('{"config":{"seed":9223372036854775808}}') }
    assert_raises(TypeError) { MB.replay_json(nil) }
  end

  def test_cli_metadata_survives_as_json_while_replay_checks_report
    raw = simple_report.to_h.merge("workload" => "example", "revision" => "abc")
    parsed = MB.replay_json(JSON.generate(raw))
    assert_equal simple_report.pairs, parsed.pairs
  end

  def test_workload_metadata_is_validated_before_core_projection
    base = JSON.generate(simple_report.to_h).sub(/\}\z/, "")
    accepted = [
      '"dependent-link-order"', '"\\u2003"', '"\\ud83d\\ude00"',
      '"\\ufffd"', '"\\\\uD800"'
    ]
    accepted.each do |token|
      assert_equal simple_report.pairs, MB.replay_json("#{base},\"workload\":#{token}}").pairs
    end

    rejected = [
      '""', '" \\t\\n\\r\\f\\u000b"', '"\\u0000"', '"x\\u0000y"',
      "null", "17", "true", "[]", '"\\ud800"', '"\\udfff"'
    ]
    rejected.each do |token|
      assert_raises(ArgumentError) { MB.replay_json("#{base},\"workload\":#{token}}") }
    end
  end

  def test_json_ingress_rejects_duplicate_members_after_escape_decoding
    raw = simple_report.to_h.merge("revision" => "shared", "runtime" => "shared")
    text = JSON.generate(raw)
    assert_equal simple_report.pairs, MB.replay_json(text).pairs

    invalid = [
      text.sub('"pairs":5', '"pairs":5,"pairs":5'),
      text.sub('"baseline_ns":1', '"baseline_ns":1,"baseline_ns":1'),
      text.sub('"verdict":"inconclusive"', '"verdict":"inconclusive","verdict":"inconclusive"'),
      text.sub('"revision":"shared"', '"revision":"shared","revision":"shared"'),
      text.sub('"pairs":5', '"pairs":5,"pa\\u0069rs":5')
    ]
    invalid.each do |duplicate|
      assert_raises(ArgumentError) { MB.replay_json(duplicate) }
    end
    assert_raises(ArgumentError) { MB.replay_json(text.sub(/\}\z/, ",}")) }

    utf16 = text.encode(Encoding::UTF_16LE)
    assert_equal simple_report.pairs, MB.replay_json(utf16).pairs
  end

  def test_json_ingress_preserves_full_i64_and_accepts_maximum_pair_count
    maximum = MB::MAX_I64
    minimum = MB::MIN_I64
    config = MB::Config.new(
      pairs: 1, confidence_percent: 95, minimum_factor: 1.05,
      seed: minimum, target_ns: maximum, max_batch: 1
    )
    pair = MB::Pair.new(baseline_ns: maximum, candidate_ns: maximum, order: "baseline-first")
    analysis = MB.analyze([pair], confidence_percent: config.confidence_percent,
      minimum_factor: config.minimum_factor)
    report = MB::Report.new(config: config, batch_size: 1, pairs: [pair], analysis: analysis)
    replayed_extreme = MB.replay_json(JSON.generate(report.to_h))
    assert_equal minimum, replayed_extreme.config.seed
    assert_equal maximum, replayed_extreme.config.target_ns
    assert_equal maximum, replayed_extreme.pairs.first.baseline_ns

    tick = 0
    large = MB.collect(
      baseline: ->(operations) { operations },
      candidate: ->(operations) { operations },
      expected_checksum: 1,
      config: MB::Config.new(pairs: 10_000, confidence_percent: 95, minimum_factor: 1.05,
        seed: 0, target_ns: 1, max_batch: 1),
      clock: -> { tick += 1 }
    )
    assert_equal 10_000, MB.replay_json(JSON.generate(large.to_h)).pairs.length
  end

  private

  def simple_report
    time = 0
    MB.collect(
      baseline: ->(operations) { operations },
      candidate: ->(operations) { operations },
      expected_checksum: 1,
      config: MB::Config.new(pairs: 5, confidence_percent: 95, minimum_factor: 1.05, seed: 12,
        target_ns: 1, max_batch: 8),
      clock: -> { time += 1 }
    )
  end

  def deep_copy(value)
    JSON.parse(JSON.generate(value))
  end
end
