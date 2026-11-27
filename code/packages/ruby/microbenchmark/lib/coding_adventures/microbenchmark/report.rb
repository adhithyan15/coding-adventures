# frozen_string_literal: true

module CodingAdventures
  module Microbenchmark
    # JSON's object_class hook sees every nested object before repeated decoded
    # member names can overwrite an earlier value.
    class UniqueJsonObject < Hash
      def []=(key, value)
        raise ArgumentError, "duplicate JSON object member: #{key.inspect}" if key?(key)

        super
      end
    end

    module_function

    # Parse a standard JSON document. Ruby JSON maps integer tokens to Integer
    # directly, so values above 2**53 are not rounded before i64 validation.
    def replay_json(text)
      raise TypeError, "report JSON must be a String" unless text.is_a?(String)

      parsed = JSON.parse(text, allow_nan: false, allow_duplicate_key: false,
        object_class: UniqueJsonObject)
      replay_report(parsed)
    rescue JSON::ParserError => error
      raise ArgumentError, "invalid report JSON: #{error.message}"
    end

    # Validate the raw observation schema, enforce the collected-order balance,
    # and recompute derived statistics. Metadata fields added by the CLI are
    # intentionally preserved by the caller and do not change the analysis.
    def replay_report(value)
      raise ArgumentError, "report must be an object" unless value.is_a?(Hash)
      validate_root_members(value)
      validate_metadata(value)

      config = Config.from_h(value.fetch("config"))
      batch_size = positive_i64(value.fetch("batch_size"), "batch_size")
      raise ArgumentError, "batch_size exceeds max_batch" if batch_size > config.max_batch

      rows = value.fetch("pairs")
      unless rows.is_a?(Array) && rows.length == config.pairs
        raise ArgumentError, "report pair count does not match config"
      end

      pairs = rows.map.with_index do |row, index|
        raise ArgumentError, "pair #{index} must be an object" unless row.is_a?(Hash)

        Pair.new(
          baseline_ns: positive_i64(row.fetch("baseline_ns"), "pair #{index} baseline_ns"),
          candidate_ns: positive_i64(row.fetch("candidate_ns"), "pair #{index} candidate_ns"),
          order: row.fetch("order")
        ).freeze
      end
      unless pairs.all? { |pair| %w[baseline-first candidate-first].include?(pair.order) }
        raise ArgumentError, "report has an invalid order value"
      end

      baseline_first = pairs.count { |pair| pair.order == "baseline-first" }
      required_baseline_first = (config.pairs + 1) / 2
      unless baseline_first == required_baseline_first
        raise ArgumentError, "report order is not balanced"
      end

      analysis = analyze(
        pairs,
        confidence_percent: config.confidence_percent,
        minimum_factor: config.minimum_factor
      )
      validate_claimed_analysis(value.fetch("analysis"), analysis)

      Report.new(config: config, batch_size: batch_size, pairs: pairs.freeze, analysis: analysis).freeze
    rescue KeyError => error
      raise ArgumentError, "report is missing #{error.key}"
    end

    def validate_metadata(value)
      group = %w[workload invocation revision runtime os arch]
      present = group.count { |field| value.key?(field) }
      has_profile = value.key?("profile")
      if present.zero?
        raise ArgumentError, "profile metadata requires the complete metadata group" if has_profile

        return
      end
      unless present == group.length
        raise ArgumentError, "report metadata group must contain all six fields"
      end

      workload = value.fetch("workload")
      unless valid_nonempty_metadata_string?(workload) && !workload.match?(/\A[ \t\n\r\f\v]*\z/)
        raise ArgumentError, "invalid workload metadata"
      end
      %w[revision runtime os arch].each do |field|
        unless valid_nonempty_metadata_string?(value.fetch(field))
          raise ArgumentError, "invalid #{field} metadata"
        end
      end

      invocation = value.fetch("invocation")
      valid_invocation = invocation.is_a?(Array) && invocation.all? { |item| valid_metadata_string?(item) }
      unless valid_invocation
        raise ArgumentError, "invocation metadata must be an array of UTF-8 strings without NUL"
      end
      if has_profile && !valid_nonempty_metadata_string?(value.fetch("profile"))
        raise ArgumentError, "invalid profile metadata"
      end
    end
    private_class_method :validate_metadata

    def validate_root_members(value)
      allowed = %w[config batch_size pairs analysis workload invocation revision runtime os arch profile]
      unknown = value.keys - allowed
      raise ArgumentError, "unknown report member: #{unknown.first}" unless unknown.empty?
    end
    private_class_method :validate_root_members

    def valid_metadata_string?(value)
      value.is_a?(String) && value.dup.force_encoding(Encoding::UTF_8).valid_encoding? && !value.include?("\0")
    end
    private_class_method :valid_metadata_string?

    def valid_nonempty_metadata_string?(value)
      valid_metadata_string?(value) && !value.empty?
    end
    private_class_method :valid_nonempty_metadata_string?

    # Only compare the three public analysis claims. Extra top-level CLI
    # metadata remains outside the reusable report object.
    def validate_claimed_analysis(claimed, expected)
      raise ArgumentError, "analysis must be an object" unless claimed.is_a?(Hash)

      ratio = send(:finite_float, claimed.fetch("ratio"), "analysis.ratio")
      interval = claimed.fetch("interval")
      if expected.interval.nil?
        raise ArgumentError, "reported analysis does not match raw pairs" unless interval.nil?
      else
        unless interval.is_a?(Array) && interval.length == 2
          raise ArgumentError, "analysis.interval must contain two finite numbers"
        end
        bounds = interval.map { |bound| send(:finite_float, bound, "analysis.interval") }
        raise ArgumentError, "reported analysis does not match raw pairs" unless bounds == expected.interval
      end
      unless ratio == expected.ratio && claimed.fetch("verdict") == expected.verdict
        raise ArgumentError, "reported analysis does not match raw pairs"
      end

      expected
    rescue KeyError => error
      raise ArgumentError, "analysis is missing #{error.key}"
    end
    private_class_method :validate_claimed_analysis
  end
end
