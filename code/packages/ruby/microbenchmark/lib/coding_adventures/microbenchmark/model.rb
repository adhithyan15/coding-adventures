# frozen_string_literal: true

module CodingAdventures
  module Microbenchmark
    MIN_I64 = -(1 << 63)
    MAX_I64 = (1 << 63) - 1
    MAX_PAIRS = 10_000
    DEFAULT_PAIRS = 40
    DEFAULT_CONFIDENCE = 95
    DEFAULT_FACTOR = 1.05
    DEFAULT_SEED = 0
    DEFAULT_TARGET_NS = 100_000
    DEFAULT_MAX_BATCH = 65_536

    # Convert integer-shaped fields with strict type/range checks. Ruby's
    # arbitrary-precision Integer is useful here, but reports still obey i64.
    def self.signed_i64(value, name, minimum: MIN_I64, maximum: MAX_I64)
      unless value.instance_of?(Integer) && value >= minimum && value <= maximum
        raise ArgumentError, "#{name} must be a signed 64-bit integer in range"
      end

      value
    end

    def self.positive_i64(value, name)
      signed_i64(value, name, minimum: 1)
    end

    # Factors and samples are binary64, so retain them as finite Float values.
    def self.finite_float(value, name)
      unless value.is_a?(Numeric) && !value.is_a?(Complex)
        raise ArgumentError, "#{name} must be a finite number"
      end

      number = Float(value)
      raise ArgumentError, "#{name} must be a finite number" unless number.finite?

      number
    rescue RangeError, TypeError, ArgumentError
      raise ArgumentError, "#{name} must be a finite number"
    end
    private_class_method :finite_float

    # Immutable values make accidental changes between collect and replay less
    # likely; hashes are emitted explicitly so JSON keys remain contractual.
    Config = Struct.new(
      :pairs, :confidence_percent, :minimum_factor, :seed, :target_ns, :max_batch
    ) do
      def self.defaults
        new(
          pairs: DEFAULT_PAIRS,
          confidence_percent: DEFAULT_CONFIDENCE,
          minimum_factor: DEFAULT_FACTOR,
          seed: DEFAULT_SEED,
          target_ns: DEFAULT_TARGET_NS,
          max_batch: DEFAULT_MAX_BATCH
        ).validate!
      end

      def validate!
        unless pairs.instance_of?(Integer) && (1..MAX_PAIRS).cover?(pairs)
          raise ArgumentError, "pairs must be an integer from 1 through #{MAX_PAIRS}"
        end
        unless confidence_percent.instance_of?(Integer) && (1..99).cover?(confidence_percent)
          raise ArgumentError, "confidence_percent must be an integer from 1 through 99"
        end

        factor = Microbenchmark.send(:finite_float, minimum_factor, "minimum_factor")
        raise ArgumentError, "minimum_factor must be greater than 1" unless factor > 1.0

        Microbenchmark.signed_i64(seed, "seed")
        Microbenchmark.positive_i64(target_ns, "target_ns")
        Microbenchmark.positive_i64(max_batch, "max_batch")
        self.class.new(
          pairs: pairs,
          confidence_percent: confidence_percent,
          minimum_factor: factor,
          seed: seed,
          target_ns: target_ns,
          max_batch: max_batch
        ).freeze
      end

      def to_h
        {
          "pairs" => pairs,
          "confidence_percent" => confidence_percent,
          "minimum_factor" => minimum_factor,
          "seed" => seed,
          "target_ns" => target_ns,
          "max_batch" => max_batch
        }
      end

      def self.from_h(hash)
        raise ArgumentError, "config must be an object" unless hash.is_a?(Hash)

        config = new(
          pairs: hash.fetch("pairs"),
          confidence_percent: hash.fetch("confidence_percent"),
          minimum_factor: hash.fetch("minimum_factor"),
          seed: hash.fetch("seed"),
          target_ns: hash.fetch("target_ns"),
          max_batch: hash.fetch("max_batch")
        )
        config.validate!
      rescue KeyError => error
        raise ArgumentError, "config is missing #{error.key}"
      end
    end

    Pair = Struct.new(:baseline_ns, :candidate_ns, :order) do
      def to_h
        {"baseline_ns" => baseline_ns, "candidate_ns" => candidate_ns, "order" => order}
      end
    end

    Analysis = Struct.new(:ratio, :interval, :verdict) do
      def to_h
        {"ratio" => ratio, "interval" => interval, "verdict" => verdict}
      end
    end

    Report = Struct.new(:config, :batch_size, :pairs, :analysis) do
      def to_h
        {
          "config" => config.to_h,
          "batch_size" => batch_size,
          "pairs" => pairs.map(&:to_h),
          "analysis" => analysis.to_h
        }
      end
    end
  end
end
