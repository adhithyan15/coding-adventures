# frozen_string_literal: true

module CodingAdventures
  module Microbenchmark
    # This tiny generator is local to the collector. Keeping it here makes a
    # fixed seed reproduce the same balanced order within the Ruby lane.
    class SplitMix64
      MASK64 = (1 << 64) - 1
      GOLDEN = 0x9E3779B97F4A7C15

      def initialize(seed)
        @state = seed & MASK64
      end

      def next_u64
        @state = (@state + GOLDEN) & MASK64
        value = @state
        value = ((value ^ (value >> 30)) * 0xBF58476D1CE4E5B9) & MASK64
        value = ((value ^ (value >> 27)) * 0x94D049BB133111EB) & MASK64
        (value ^ (value >> 31)) & MASK64
      end

      # Rejection sampling avoids modulo bias when shuffling the order table.
      def bounded(bound)
        threshold = ((-bound) & MASK64) % bound
        loop do
          value = next_u64
          return value % bound if value >= threshold
        end
      end
    end

    module_function

    MONOTONIC_CLOCK = lambda do
      Process.clock_gettime(Process::CLOCK_MONOTONIC, :nanosecond)
    end

    # Verify, calibrate and sample paired batches. A failure escapes before a
    # Report is built, so callers never receive a partial result.
    def collect(baseline:, candidate:, expected_checksum:, config: Config.defaults, clock: MONOTONIC_CLOCK)
      normalized = (config.is_a?(Config) ? config : raise(TypeError, "config must be a Config")).validate!
      unless baseline.respond_to?(:call) && candidate.respond_to?(:call) && clock.respond_to?(:call)
        raise TypeError, "baseline, candidate, and clock must be callable"
      end

      checksum = signed_i64(expected_checksum, "expected_checksum")
      verify = lambda do |function|
        actual = signed_i64(function.call(1), "checksum")
        raise ArgumentError, "checksum mismatch" unless actual == checksum
      end
      verify.call(baseline)
      verify.call(candidate)

      previous_tick = nil
      read_clock = lambda do
        tick = signed_i64(clock.call, "clock", minimum: 0)
        raise ArgumentError, "clock must not regress" if previous_tick && tick < previous_tick

        previous_tick = tick
        tick
      end

      timed = lambda do |function, size, calibration|
        expected_batch = checked_checksum(checksum, size)
        started = read_clock.call
        actual = signed_i64(function.call(size), "checksum")
        finished = read_clock.call
        raise ArgumentError, "checksum mismatch" unless actual == expected_batch

        duration = finished - started
        if duration.negative? || (!calibration && duration.zero?)
          raise ArgumentError, "measured duration must be positive and clock must not regress"
        end
        duration
      end

      batch_size = 1
      loop do
        baseline_duration = timed.call(baseline, batch_size, true)
        candidate_duration = timed.call(candidate, batch_size, true)
        break if baseline_duration >= normalized.target_ns && candidate_duration >= normalized.target_ns
        raise ArgumentError, "calibration limit exhausted" if batch_size >= normalized.max_batch

        batch_size = [batch_size * 2, normalized.max_batch].min
      end

      observations = schedule(normalized.pairs, normalized.seed).map do |order|
        if order == "baseline-first"
          baseline_ns = timed.call(baseline, batch_size, false)
          candidate_ns = timed.call(candidate, batch_size, false)
        else
          candidate_ns = timed.call(candidate, batch_size, false)
          baseline_ns = timed.call(baseline, batch_size, false)
        end
        Pair.new(baseline_ns: baseline_ns, candidate_ns: candidate_ns, order: order).freeze
      end

      Report.new(
        config: normalized,
        batch_size: batch_size,
        pairs: observations.freeze,
        analysis: analyze(
          observations,
          confidence_percent: normalized.confidence_percent,
          minimum_factor: normalized.minimum_factor
        )
      ).freeze
    end

    def schedule(pair_count, seed)
      orders = Array.new((pair_count + 1) / 2, "baseline-first") + Array.new(pair_count / 2, "candidate-first")
      random = SplitMix64.new(seed)
      (orders.length - 1).downto(1) do |index|
        other = random.bounded(index + 1)
        orders[index], orders[other] = orders[other], orders[index]
      end
      orders
    end
    private_class_method :schedule

    def checked_checksum(checksum, operations)
      expected = checksum * operations
      signed_i64(expected, "batch checksum")
    end
    private_class_method :checked_checksum
  end
end
