# frozen_string_literal: true

module CodingAdventures
  module TestBootstrap
    module Fuzz
      # Bounds of a campaign; every field is validated (CWE-400).
      #
      # `minimize_budget` is the most target executions spent minimising ONE novel
      # crash; on top of it the whole campaign spends at most `max_iterations`
      # minimisation executions in total.  `step_budget` and `mem_budget` are the
      # per-execution deterministic budgets of Handle#tick and Handle#reserve.
      Options = Data.define(:max_iterations, :seed, :max_input_len, :base_energy,
        :minimize_budget, :step_budget, :mem_budget) do
        def initialize(max_iterations: 1000, seed: 42, max_input_len: 4096, base_energy: 10,
          minimize_budget: Limits::DEFAULT_MINIMIZE_BUDGET,
          step_budget: Limits::DEFAULT_STEP_BUDGET, mem_budget: Limits::DEFAULT_MEM_BUDGET)
          super
          validate
        end

        def validate
          to_h.each { |name, value| Limits.check_int!(value, name) }
          check(:max_iterations, 0, Limits::MAX_ITERATIONS)
          check(:max_input_len, 0, Limits::MAX_INPUT_BYTES)
          check(:base_energy, 0, Limits::MAX_ITERATIONS)
          check(:minimize_budget, 0, Limits::MAX_MINIMIZE_BUDGET)
          check(:step_budget, 0, Limits::MAX_STEP_BUDGET)
          check(:mem_budget, 0, Limits::MAX_MEM_BUDGET)
        end

        def check(name, lo, hi)
          value = public_send(name)
          raise ArgumentError, "#{name} must be in [#{lo}, #{hi}]" unless value.between?(lo, hi)
        end
        private :check

        # The seed is private (VER06 section 6): never printed.
        def inspect = "#<Fuzz::Options max_iterations=#{max_iterations}>"

        alias_method :to_s, :inspect
      end
    end
  end
end
