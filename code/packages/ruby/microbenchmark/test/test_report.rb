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
    raw = simple_report.to_h.merge(
      "workload" => "example",
      "invocation" => [],
      "revision" => "abc",
      "runtime" => "Ruby",
      "os" => "test",
      "arch" => "test"
    )
    parsed = MB.replay_json(JSON.generate(raw))
    assert_equal simple_report.pairs, parsed.pairs
  end

  def test_complete_metadata_group_is_validated_before_core_projection
    report = simple_report
    complete = report.to_h.merge(
      "workload" => "dependent-link-order",
      "invocation" => [],
      "revision" => "r",
      "runtime" => "Ruby",
      "os" => "test",
      "arch" => "test"
    )
    assert_equal report, MB.replay_report(complete)
    assert_equal report, MB.replay_json(JSON.generate(complete))
    assert_equal report, MB.replay_report(complete.merge("invocation" => [""]))
    assert_equal report, MB.replay_json(JSON.generate(complete.merge("invocation" => [""])))

    %w[workload invocation revision runtime os arch].each do |field|
      partial = complete.dup
      partial.delete(field)
      assert_raises(ArgumentError, "accepted missing #{field}") { MB.replay_report(partial) }
      assert_raises(ArgumentError, "accepted missing #{field}") { MB.replay_json(JSON.generate(partial)) }
    end
    profile_only = report.to_h.merge("profile" => "release")
    assert_raises(ArgumentError) { MB.replay_report(profile_only) }
    assert_raises(ArgumentError) { MB.replay_json(JSON.generate(profile_only)) }

    %w[revision runtime os arch profile].each do |field|
      [nil, "", "bad\0value"].each do |invalid|
        malformed = complete.merge("profile" => "release", field => invalid)
        assert_raises(ArgumentError, "accepted #{field}=#{invalid.inspect}") { MB.replay_report(malformed) }
        assert_raises(ArgumentError, "accepted #{field}=#{invalid.inspect}") { MB.replay_json(JSON.generate(malformed)) }
      end
      whitespace = complete.merge("profile" => "release", field => " ")
      assert_equal report, MB.replay_report(whitespace)
      assert_equal report, MB.replay_json(JSON.generate(whitespace))
    end
    [nil, "", " \t\n\r\f\v", "\0", "x\0y"].each do |invalid|
      malformed = complete.merge("workload" => invalid)
      assert_raises(ArgumentError) { MB.replay_report(malformed) }
      assert_raises(ArgumentError) { MB.replay_json(JSON.generate(malformed)) }
    end
    [nil, [1], ["bad\0arg"]].each do |invalid|
      malformed = complete.merge("invocation" => invalid)
      assert_raises(ArgumentError) { MB.replay_report(malformed) }
      assert_raises(ArgumentError) { MB.replay_json(JSON.generate(malformed)) }
    end
    ["\u2003", "😀", "\uFFFD", "\\uD800"].each do |accepted|
      generic = complete.merge("workload" => accepted)
      assert_equal report, MB.replay_report(generic)
      assert_equal report, MB.replay_json(JSON.generate(generic))
    end
  end

  def test_json_ingress_rejects_duplicate_decoded_members_and_unknown_root_keys
    report = simple_report
    complete = report.to_h.merge(
      "workload" => "generic-workload", "invocation" => [], "revision" => "shared",
      "runtime" => "shared", "os" => "test", "arch" => "test"
    )
    text = JSON.generate(complete)
    assert_equal report, MB.replay_json(text)
    # The same member name appears in the root and config objects, not one scope.

    invalid = [
      text.sub('"pairs":5', '"pairs":5,"pairs":5'),
      text.sub('"baseline_ns":1', '"baseline_ns":1,"baseline_ns":1'),
      text.sub('"verdict":"inconclusive"', '"verdict":"inconclusive","verdict":"inconclusive"'),
      text.sub('"revision":"shared"', '"revision":"shared","revision":"other"'),
      text.sub('"pairs":5', '"pairs":5,"pa\\u0069rs":5')
    ]
    invalid.each { |duplicate| assert_raises(ArgumentError) { MB.replay_json(duplicate) } }
    assert_raises(ArgumentError) { MB.replay_json(text.sub(/\}\z/, ",}")) }

    assert_raises(ArgumentError) { MB.replay_report(complete.merge("extra" => true)) }
    assert_raises(ArgumentError) { MB.replay_json(JSON.generate(complete.merge("extra" => true))) }
  end

  def test_json_ingress_accepts_maximum_balanced_report_with_reused_pair_member_names
    pairs = Array.new(MB::MAX_PAIRS) do |index|
      MB::Pair.new(
        baseline_ns: 1,
        candidate_ns: 1,
        order: index.even? ? "baseline-first" : "candidate-first"
      )
    end
    config = MB::Config.defaults.to_h.merge("pairs" => MB::MAX_PAIRS)
    analysis = MB.analyze(pairs)
    raw = {
      "config" => config,
      "batch_size" => 1,
      "pairs" => pairs.map(&:to_h),
      "analysis" => analysis.to_h
    }

    replayed = MB.replay_json(JSON.generate(raw))
    assert_equal MB::MAX_PAIRS, replayed.pairs.length
    assert_equal analysis, replayed.analysis
  end

  def test_json_ingress_rejects_duplicate_members_after_escape_decoding
    raw = simple_report.to_h.merge(
      "workload" => "generic-workload",
      "invocation" => [],
      "revision" => "shared",
      "runtime" => "shared",
      "os" => "test",
      "arch" => "test"
    )
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
