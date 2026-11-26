# frozen_string_literal: true

module CodingAdventures
  module TestBootstrap
    module Pbt
      # The outcome of Runner.check.  In redacted (CI) mode `seed`, `counterexample`
      # and `minimal` are nil; only the message, iteration, step count and digest
      # remain.
      Verdict = Struct.new(
        :passed, :failure_id, :message, :cases_run, :iteration, :shrink_steps,
        :seed, :counterexample, :minimal, :digest
      ) do
        def failed? = !passed
      end

      # Runner options.  `ci` is :on, :off or :unset.  :unset is treated as CI:
      # the engine fails closed and redacts.  The gem never reads the
      # environment; `Options.ci_from_lookup` is a pure helper (pass `ENV`).
      class Options
        CI_MODES = %i[on off unset].freeze

        attr_reader :num_tests, :max_shrink_steps, :ci

        def initialize(num_tests: 100, max_shrink_steps: 1000, ci: :unset)
          raise ArgumentError, "ci must be one of #{CI_MODES.inspect}" unless CI_MODES.include?(ci)

          Limits.check_range!(num_tests, 0, Limits::MAX_CASES, "num_tests")
          Limits.check_range!(max_shrink_steps, 0, Limits::MAX_SHRINK_STEPS, "max_shrink_steps")
          @num_tests = num_tests
          @max_shrink_steps = max_shrink_steps
          @ci = ci
        end

        # `CI` set to anything except empty, "0" or "false" (case-insensitive,
        # trimmed) means CI.  `lookup` is anything with `[]` (ENV, a Hash).
        def self.ci_from_lookup(lookup)
          value = lookup["CI"]
          return :off if value.nil?

          (%w[0 false].include?(value.strip.downcase) || value.strip.empty?) ? :off : :on
        end
      end

      # The honest runner (VER05 7.1, VER05e 5, VER05h 3.1).
      #
      #   * zero cases evaluated is `property-exhausted`, never a pass;
      #   * a generator error is `property-exhausted`;
      #   * a property that raises is `property-panic`;
      #   * the reported minimal witness is RE-VERIFIED: if the shrinker raises,
      #     returns something that no longer fails, a value the property raises
      #     on, or a nonsensical step count, the original candidate is reported
      #     with 0 steps;
      #   * redacted mode never emits the witness or the seed;
      #   * the call always returns a verdict (invalid options raise instead).
      #
      # There is no wall-clock timeout: `property-timeout` is registered and
      # never produced, so a property that loops forever hangs the run.
      module Runner
        module_function

        # Most elements the work estimate of one sampled value will visit.
        WALK_CAP = 10_000

        # Runs the block over `options.num_tests` candidates from `seed`.  The block
        # returns truthy when the property holds.  `shrink:` overrides the
        # generator's shrinker (so a defective shrinker can be tested).
        def check(seed, gen, options = nil, shrink: nil, &prop)
          raise ArgumentError, "a property block is required" unless prop
          raise TypeError, "gen must be a Gen" unless gen.is_a?(Gen)
          raise TypeError, "seed must be an Integer, not #{seed.class}" unless seed.is_a?(Integer)
          raise ArgumentError, "shrink must be callable" unless shrink.nil? || shrink.respond_to?(:call)

          opts = options || Options.new
          raise TypeError, "options must be Options" unless opts.is_a?(Options)

          seed &= Prng::MASK64
          redact = opts.ci != :off
          if opts.num_tests == 0
            return verdict(false, FailureIds::EXHAUSTED,
              "no cases were evaluated (num_tests is 0); a property that never ran is not a pass", 0, 0)
          end

          # A check nested in a property (an ambient scope exists) pays one attempt
          # at entry, one per case after the first and the size of every sampled
          # collection as work, all charged up the chain: neither nesting depth,
          # branching nor num_tests multiplies the outermost budget.  A starved
          # nested check reports property-exhausted without evaluating anything.
          outer = ShrinkBudget.current
          nested = !outer.nil?
          return spent(0) if nested && !outer.charge(1)

          # The WHOLE check runs in one scope (case loop included).
          ShrinkBudget.enter_check(opts.num_tests, opts.max_shrink_steps) do |scope|
            rng = Hooks.new_rng.call(seed)
            1.upto(opts.num_tests) do |iteration|
              return spent(iteration) if nested && iteration > 1 && !scope.charge(1)

              begin
                candidate = gen.sample(rng)
              rescue Exception => e # rubocop:disable Lint/RescueException
                raise unless Safe.recoverable?(e)

                return exhausted(e, iteration, redact)
              end

              return spent(iteration) if nested && !scope.charge_copy(size_of(candidate))

              begin
                holds = prop.call(candidate)
              rescue Exception => e # rubocop:disable Lint/RescueException
                raise unless Safe.recoverable?(e)

                return panic(seed, iteration, e, redact)
              end

              return falsified(seed, iteration, candidate, gen, prop, shrink, opts, redact, scope) unless holds
            end

            if !nested && scope.refused?
              # A nested check or generator inside this run was starved, so the
              # property was not fully evaluated: never report that as a pass.
              return verdict(false, FailureIds::EXHAUSTED,
                "a nested check was starved by the shrink budget; the property was not fully evaluated",
                opts.num_tests, opts.num_tests)
            end

            verdict(true, nil, nil, opts.num_tests, 0)
          end
        end

        def verdict(passed, failure_id, message, cases_run, iteration, **extra)
          Verdict.new(passed: passed, failure_id: failure_id, message: message, cases_run: cases_run,
            iteration: iteration, shrink_steps: extra.fetch(:shrink_steps, 0), seed: extra[:seed],
            counterexample: extra[:counterexample], minimal: extra[:minimal], digest: extra[:digest])
        end

        # The enclosing shrink budget was spent: exhausted, never a pass.
        def spent(iteration)
          verdict(false, FailureIds::EXHAUSTED, "the enclosing shrink budget was spent at iteration #{iteration}",
            [0, iteration - 1].max, iteration)
        end

        # An approximate size of a sampled value in elements (work): a String, Array
        # or Hash counts its length plus, one level deeper, the length of each
        # element that is itself one of those; any other Enumerable is counted by
        # enumerating.  ONE visit allowance of WALK_CAP is shared by the whole walk
        # (each element visited and each element enumerated inside a nested
        # Enumerable spends from it), so 10,000 elements that are themselves
        # Enumerables cannot cost 10,000 x WALK_CAP block calls.
        # Approximate by design; a hostile collection counts 0.
        def size_of(value)
          allowance = [WALK_CAP]
          n = shallow(value, allowance)
          return 0 if n == 0

          total = n
          if value.is_a?(Array) || value.is_a?(Hash)
            each = value.is_a?(Hash) ? :each_value : :each
            value.public_send(each) do |e|
              break if allowance[0] <= 0

              allowance[0] -= 1
              total += shallow(e, allowance)
            end
          end
          total
        rescue Exception => e # rubocop:disable Lint/RescueException
          raise unless Safe.recoverable?(e)

          0
        end

        # `allowance` is a one-element Array holding the visits left; counting an
        # arbitrary Enumerable spends from it.
        def shallow(value, allowance)
          case value
          when String then value.bytesize
          when Array, Hash, Struct then value.size
          when Enumerable
            n = 0
            value.each do
              break if allowance[0] <= 0

              allowance[0] -= 1
              n += 1
            end
            n
          else 0
          end
        end

        def exhausted(error, iteration, redact)
          text =
            if error.is_a?(ExhaustedError)
              "filter precondition rejected all candidates at iteration #{iteration}"
            elsif redact
              "generator error at iteration #{iteration} (#{Safe.class_name(error)})"
            else
              "generator error at iteration #{iteration}: #{Safe.safe_string(error)}"
            end
          verdict(false, FailureIds::EXHAUSTED, text, iteration - 1, iteration)
        end

        def panic(seed, iteration, error, redact)
          if redact
            return verdict(false, FailureIds::PANIC,
              "property evaluation raised at iteration #{iteration} (#{Safe.class_name(error)})", iteration, iteration)
          end

          verdict(false, FailureIds::PANIC,
            "property evaluation raised at iteration #{iteration}: #{Safe.safe_string(error)}",
            iteration, iteration, seed: seed)
        end

        def falsified(seed, iteration, candidate, gen, prop, shrink, opts, redact, check_scope)
          # A candidate the property raises on is not a counterexample to the claim;
          # it is a different failure, so it is never accepted as "smaller".
          falsifies = lambda do |v|
            !prop.call(v)
          rescue Exception => e # rubocop:disable Lint/RescueException
            raise unless Safe.recoverable?(e)

            false
          end

          minimal = candidate
          steps = 0

          # Untrusted shrinker: fall back to the original.  The shrink scope is a
          # child of the check scope (itself a child of any enclosing shrink): every
          # evaluation is charged to the whole chain.
          begin
            shrunk =
              ShrinkBudget.enter_child(opts.max_shrink_steps) do |scope|
                budgeted = scope.charged(falsifies)
                if shrink
                  shrink.call(candidate, budgeted, opts.max_shrink_steps)
                else
                  gen.shrink_nested(candidate, budgeted, opts.max_shrink_steps)
                end
              end
            value, used = shrunk
            # Re-verify: the witness we report must really fail the property.  One
            # attempt is charged to the check scope; if the chain refuses it the
            # shrunk value is not adopted.
            if used.is_a?(Integer) && used >= 0 && used <= opts.max_shrink_steps &&
                check_scope.charge(1) && falsifies.call(value)
              minimal = value
              steps = used
            end
          rescue Exception => e # rubocop:disable Lint/RescueException
            raise unless Safe.recoverable?(e)
            # keep the original
          end

          if redact
            preimage = "iter:#{iteration};steps:#{steps};shrunk:#{CorpusJson::Canon.render(minimal)}"
            digest = format("%016x", Prng.fnv1a64(preimage))
            return verdict(false, FailureIds::COUNTEREXAMPLE,
              "property falsified after #{iteration} iterations (shrunk in #{steps} steps, repro-fnv: #{digest})",
              iteration, iteration, shrink_steps: steps, digest: digest)
          end

          verdict(false, FailureIds::COUNTEREXAMPLE,
            "property falsified at iteration #{iteration} (seed=#{seed}): initial=#{CorpusJson::Canon.render(candidate)}, " \
            "minimal=#{CorpusJson::Canon.render(minimal)} after #{steps} shrink steps",
            iteration, iteration, shrink_steps: steps, seed: seed, counterexample: candidate, minimal: minimal)
        end
      end
    end
  end
end
