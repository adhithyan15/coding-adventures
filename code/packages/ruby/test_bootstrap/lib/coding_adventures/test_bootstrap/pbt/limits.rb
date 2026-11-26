# frozen_string_literal: true

module CodingAdventures
  module TestBootstrap
    module Pbt
      # Bounds that keep every loop and allocation finite (CWE-400).  A caller or
      # a fixture can ask for more only by being refused.
      module Limits
        MAX_CASES = 100_000
        MAX_SHRINK_STEPS = 10_000
        MAX_INPUT_BYTES = 1 << 20
        MAX_LIST_LEN = 100_000
        # Most values any corpus entry point will generate.
        MAX_GEN_COUNT = 1_000_000
        # Attempt budget of one shrink = ATTEMPT_FACTOR * max_steps + ATTEMPT_SLACK:
        # a rejected attempt copies its candidate but is not an accepted step.
        ATTEMPT_FACTOR = 16
        ATTEMPT_SLACK = 64
        # count x width bound for gen_bytes / gen_bounded.
        MAX_GEN_BYTES_TOTAL = 1 << 20
        MAX_FREQUENCY_TOTAL = 1 << 32
        # Most elements one whole shrink may copy, summed over every attempt of
        # every nested shrinker (an attempt copies its candidate).
        MAX_SHRINK_WORK = 64 * 1024 * 1024
        MAX_RETRIES = 10_000
        DEFAULT_MAX_RETRIES = 100
        MAX_TUPLE_ARITY = 64

        def self.attempt_budget(max_steps)
          (ATTEMPT_FACTOR * max_steps) + ATTEMPT_SLACK
        end

        # An Integer within [lo, hi]; `name` is for the message.
        def self.check_range!(value, lo, hi, name)
          raise TypeError, "#{name} must be an Integer, not #{value.class}" unless value.is_a?(Integer)
          return if value.between?(lo, hi)

          raise ArgumentError, "#{name} #{value} outside [#{lo}, #{hi}]"
        end

        def self.check_budget!(max_steps)
          check_range!(max_steps, 0, MAX_SHRINK_STEPS, "max_steps")
        end

        def self.check_count!(count)
          check_range!(count, 0, MAX_GEN_COUNT, "count")
        end
      end
    end
  end
end
