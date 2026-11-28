# frozen_string_literal: true

module CodingAdventures
  module Microbenchmark
    module ArrayDriver
      WORKLOAD_ID = "indexed-array-sum-4096"
      ARRAY_LENGTH = 4_096

      module_function

      # Build the fixed input before any timing starts; the indexed and
      # enumerator variants then traverse the same immutable array.
      def make_array
        Array.new(ARRAY_LENGTH) { |index| (17 * index + 3) % 251 }
      end

      def expected_sum(values)
        values.sum
      end

      # Each operation computes the same checksum. The branch is outside the
      # inner value access; each baseline/candidate block specializes its path.
      def indexed_batch(values, operations)
        total = 0
        operations.times do
          index = 0
          while index < values.length
            total += values[index]
            index += 1
          end
        end
        total
      end

      def iterator_batch(values, operations)
        total = 0
        operations.times do
          values.each { |value| total += value }
        end
        total
      end

      # The CLI report adds experiment context to the reusable collection
      # record, preserving all integer fields as Ruby Integers for JSON output.
      def run(config, invocation:, revision:, runtime:, os:, machine:)
        values = make_array
        per_operation = expected_sum(values)
        report = Microbenchmark.collect(
          baseline: ->(count) { indexed_batch(values, count) },
          candidate: ->(count) { iterator_batch(values, count) },
          expected_checksum: per_operation,
          config: config
        )
        report.to_h.merge(
          "workload" => WORKLOAD_ID,
          "invocation" => invocation.dup,
          "revision" => revision,
          "runtime" => runtime,
          "os" => os,
          "arch" => machine
        )
      end
    end

    module CLI
      INTEGER_FLAGS = {
        "--pairs" => :pairs,
        "--confidence" => :confidence_percent,
        "--seed" => :seed,
        "--target-ns" => :target_ns,
        "--max-batch" => :max_batch
      }.freeze
      FLOAT_FLAGS = {"--factor" => :minimum_factor}.freeze

      module_function

      # Parse only known options and decimal integers so alternate Ruby Integer
      # syntaxes cannot make a copied invocation ambiguous.
      def parse_arguments(arguments)
        options = Config.defaults.to_h.transform_keys(&:to_sym)
        revision = nil
        index = 0
        while index < arguments.length
          flag = arguments[index]
          unless INTEGER_FLAGS.key?(flag) || FLOAT_FLAGS.key?(flag) || flag == "--revision"
            raise ArgumentError, "invalid argument: #{flag}"
          end
          value = arguments[index + 1]
          raise ArgumentError, "missing value for #{flag}" unless value

          index += 2
          if flag == "--revision"
            revision = value
          elsif INTEGER_FLAGS.key?(flag)
            unless /\A-?(?:0|[1-9][0-9]*)\z/.match?(value)
              raise ArgumentError, "invalid integer for #{flag}"
            end
            options[INTEGER_FLAGS.fetch(flag)] = Integer(value, 10)
          else
            begin
              options[FLOAT_FLAGS.fetch(flag)] = Float(value)
            rescue ArgumentError, TypeError
              raise ArgumentError, "invalid number for #{flag}"
            end
          end
        end

        [Config.new(**options).validate!, revision]
      end

      def run(arguments = ARGV, output: $stdout, error_output: $stderr, environment: ENV, program: $PROGRAM_NAME)
        config, selected_revision = parse_arguments(arguments)
        revision = selected_revision
        if revision.nil?
          revision = [environment["GIT_REVISION"], environment["GITHUB_SHA"]].find do |candidate|
            candidate.is_a?(String) && !candidate.empty?
          end || "unknown"
        end
        result = ArrayDriver.run(
          config,
          invocation: [File.basename(program), *arguments],
          revision: revision,
          runtime: "#{RUBY_ENGINE} #{RUBY_VERSION}",
          os: RbConfig::CONFIG.fetch("host_os"),
          machine: RbConfig::CONFIG.fetch("host_cpu")
        )
        CodingAdventures::Microbenchmark.replay_report(result)
        output.write(JSON.generate(result))
        output.write("\n")
        0
      rescue => exception
        error_output.puts("array-microbenchmark: #{exception.message}")
        1
      end
    end
  end
end
