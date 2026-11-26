# frozen_string_literal: true

require "open3"
require "stringio"
require_relative "test_helper"

class MicrobenchmarkArrayDriverTest < Minitest::Test
  MB = CodingAdventures::Microbenchmark
  Driver = MB::ArrayDriver

  def test_workload_matches_the_published_indexed_array
    values = Driver.make_array
    assert_equal 4_096, values.length
    assert_equal [3, 20, 37, 54], values.first(4)
    assert_equal 3 * Driver.expected_sum(values), Driver.indexed_batch(values, 3)
    assert_equal 3 * Driver.expected_sum(values), Driver.iterator_batch(values, 3)
    assert_equal "indexed-array-sum-4096", Driver::WORKLOAD_ID
  end

  def test_argument_parser_accepts_full_signed_seed_and_rejects_malformed_values
    config, revision = MB::CLI.parse_arguments([
      "--pairs", "3", "--confidence", "90", "--factor", "1.1", "--seed", "-9223372036854775808",
      "--target-ns", "10", "--max-batch", "4", "--revision", "test-revision"
    ])
    assert_equal 3, config.pairs
    assert_equal 90, config.confidence_percent
    assert_equal MB::MIN_I64, config.seed
    assert_equal 4, config.max_batch
    assert_equal "test-revision", revision
    assert_raises(ArgumentError) { MB::CLI.parse_arguments(["--unknown", "1"]) }
    assert_raises(ArgumentError) { MB::CLI.parse_arguments(["--pairs"]) }
    assert_raises(ArgumentError) { MB::CLI.parse_arguments(["--seed", "1_000"]) }
    assert_raises(ArgumentError) { MB::CLI.parse_arguments(["--max-batch", (MB::MAX_I64 + 1).to_s]) }
  end

  def test_native_cli_emits_positive_raw_pairs_and_replays_without_speed_threshold
    executable = File.expand_path("../bin/array-microbenchmark", __dir__)
    stdout, stderr, status = Open3.capture3(
      RbConfig.ruby,
      executable,
      "--pairs", "6", "--target-ns", "1", "--max-batch", "8", "--revision", "test-revision"
    )
    assert status.success?, stderr
    raw = JSON.parse(stdout, allow_nan: false)
    replayed = MB.replay_report(raw)
    assert_equal Driver::WORKLOAD_ID, raw.fetch("workload")
    assert_equal "test-revision", raw.fetch("revision")
    assert_equal RbConfig::CONFIG.fetch("host_os"), raw.fetch("os")
    assert_equal 6, replayed.pairs.length
    assert replayed.pairs.all? { |pair| pair.baseline_ns.positive? && pair.candidate_ns.positive? }
    assert_equal RbConfig::CONFIG.fetch("host_cpu"), raw.fetch("arch")
    assert_equal %w[analysis arch batch_size config invocation os pairs revision runtime workload], raw.keys.sort
    assert_equal 1, stdout.lines.length
  end

  def test_cli_returns_failure_without_partial_json_for_bad_arguments
    output = StringIO.new
    errors = StringIO.new
    status = MB::CLI.run(["--pairs", "0"], output: output, error_output: errors, environment: {}, program: "array-microbenchmark")
    assert_equal 1, status
    assert_empty output.string
    assert_match(/pairs must be an integer/, errors.string)
  end
end
