# frozen_string_literal: true

module CodingAdventures
  module Microbenchmark
    module_function

    # Analyze arbitrary valid paired observations. Collection reports have a
    # separate balance check because analysis alone must accept fixtures whose
    # order fields are arbitrary but still valid observations.
    def analyze(pairs, confidence_percent: DEFAULT_CONFIDENCE, minimum_factor: DEFAULT_FACTOR)
      raise ArgumentError, "pairs must be a non-empty Array" unless pairs.is_a?(Array) && !pairs.empty?
      unless confidence_percent.instance_of?(Integer) && (1..99).cover?(confidence_percent)
        raise ArgumentError, "confidence_percent must be an integer from 1 through 99"
      end

      factor = send(:finite_float, minimum_factor, "minimum_factor")
      raise ArgumentError, "minimum_factor must be greater than 1" unless factor > 1.0

      ratios = pairs.map.with_index do |pair, index|
        unless pair.is_a?(Pair) && %w[baseline-first candidate-first].include?(pair.order)
          raise ArgumentError, "pair #{index} has an invalid order"
        end

        baseline = positive_i64(pair.baseline_ns, "pair #{index} baseline_ns")
        candidate = positive_i64(pair.candidate_ns, "pair #{index} candidate_ns")
        # VER07d rounds operands to binary64 before division, matching the Go
        # and Python reference analyzers for full-width nanosecond values.
        candidate.to_f / baseline.to_f
      end

      ratio = CodingAdventures::RankStatistics.median(ratios)
      interval = CodingAdventures::RankStatistics.median_interval(ratios, confidence_percent)
      verdict = "inconclusive"
      if interval
        fast_boundary = 1.0 / factor
        verdict = if interval[1] < fast_boundary
          "faster"
        elsif interval[0] > factor
          "slower"
        elsif interval[0] >= fast_boundary && interval[1] <= factor
          "equivalent"
        else
          "inconclusive"
        end
      end

      Analysis.new(ratio: ratio, interval: interval, verdict: verdict).freeze
    end
  end
end
