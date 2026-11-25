# frozen_string_literal: true

module CodingAdventures
  module TestBootstrap
    module Fuzz
      # CWE-400 ceilings (VER06i section 6).  A fuzz campaign is attacker-reachable
      # work (a CI job fed a hostile configuration or fixture), so every count and
      # length that drives a loop or an allocation has a hard ceiling.
      module Limits
        MAX_ITERATIONS = 1_000_000
        MAX_INPUT_BYTES = 1 << 20
        MAX_CRASHES = 1000
        MAX_INITIAL_CORPUS = 10_000
        MAX_STEP_BUDGET = 100_000_000
        MAX_MEM_BUDGET = 1 << 30
        MAX_MINIMIZE_BUDGET = 100_000
        DEFAULT_MINIMIZE_BUDGET = 2000
        DEFAULT_STEP_BUDGET = 200_000
        DEFAULT_MEM_BUDGET = 64 << 20
        # Bytes of corpus seeds plus crash inputs one campaign keeps.
        MAX_RETAINED_BYTES = 256 << 20
        # Bytes one mutate_sequence call may copy (initial copy + one buffer per step).
        MAX_SEQUENCE_WORK = 1 << 28
        # Stack locations inspected per crash, however deep the recursion.
        MAX_WALK = 4096
        # Frames kept in a signature (the innermost survive).
        MAX_FRAMES = 64
        MAX_KIND_LEN = 64
        MAX_TARGET_LEN = 128
        # VER06 section 4.1: 8,192 bytes = 65,536 bits = 65,536 edges.
        COVERAGE_MAP_SIZE = 65_536
        COVERAGE_BYTES = COVERAGE_MAP_SIZE / 8

        def self.check_int!(value, name)
          raise TypeError, "#{name} must be an Integer, not #{value.class}" unless value.is_a?(Integer)
        end
      end
    end
  end
end
