# frozen_string_literal: true

module CodingAdventures
  module TestBootstrap
    module Fuzz
      # What a target sees: coverage reporting and two deterministic budgets
      # (VER06i section 5).
      #
      # A campaign allocates a FRESH Handle for every execution and retires it the
      # moment the target returns.  Every mutation takes the handle's Mutex, so a
      # thread, fiber or Enumerator the target spawned can only ever touch its own
      # retired handle: after retirement `edge` does nothing and `tick`/`reserve`
      # raise a retired sentinel.  It never reaches a later run's counters or map.
      class Handle
        # Raised by #tick and #reserve.  It is an Exception, not a StandardError, so
        # a plain `rescue => e` misses it; but Ruby has no throwable a target cannot
        # catch with `rescue Exception`, so the guarantee comes from STICKINESS (every
        # later call rethrows) plus the executor's post-run check, not from the type.
        # It carries the campaign token and the epoch (execution number) that raised
        # it, so a sentinel stored and re-raised in a later run is recognisably stale.
        class BudgetExceeded < Exception # rubocop:disable Lint/InheritException
          attr_reader :campaign, :epoch, :which

          def initialize(campaign, epoch, which)
            super("fuzz budget exhausted")
            @campaign = campaign
            @epoch = epoch
            @which = which # :steps, :memory or :retired
          end
        end

        # A stable edge id derived from a name (FNV-1a): deterministic across runs
        # and machines (no object ids, no hash randomisation).
        def self.edge_id(label)
          Prng.fnv1a64(label)
        end

        attr_reader :campaign, :epoch

        # @api private: only the executor constructs handles.
        def initialize(step_budget, mem_budget, campaign, epoch, cov)
          @mutex = Mutex.new
          @step_budget = step_budget
          @mem_budget = mem_budget
          @campaign = campaign
          @epoch = epoch
          @cov = cov
          @steps = 0
          @reserved = 0
          @edge_calls = 0
          @retired = false
          @exhausted = nil
        end

        # Reports that the target executed the program point with this id
        # (reduced mod 65,536).  A no-op on a retired handle.
        def edge(id)
          @mutex.synchronize do
            return nil if @retired

            @cov&.record(id)
            @edge_calls += 1
          end
          nil
        end

        # Counts one unit of work.  After `step_budget` ticks the execution is a
        # timeout: tick raises a sentinel and keeps raising it on every later call.
        # A loop that never ticks cannot be stopped (the engine starts no thread),
        # so CI needs an external wall-clock limit.
        def tick
          @mutex.synchronize do
            raise BudgetExceeded.new(@campaign, @epoch, :retired) if @retired
            raise @exhausted if @exhausted

            if @steps >= @step_budget
              @exhausted = BudgetExceeded.new(@campaign, @epoch, :steps)
              raise @exhausted
            end
            @steps += 1
          end
          nil
        end

        # Declares an allocation of `n` bytes about to be made on behalf of the
        # input.  When the running total of one execution passes `mem_budget` the
        # execution is an out-of-memory outcome.  Non-positive `n` reserves nothing.
        # Exhaustion is shared with #tick: the first breach of either wins.
        def reserve(n)
          Limits.check_int!(n, "n")
          @mutex.synchronize do
            raise BudgetExceeded.new(@campaign, @epoch, :retired) if @retired
            raise @exhausted if @exhausted
            return nil unless n.positive?

            if n > @mem_budget - @reserved
              @exhausted = BudgetExceeded.new(@campaign, @epoch, :memory)
              raise @exhausted
            end
            @reserved += n
          end
          nil
        end

        # @api private: the executor ends the handle's life (after the target).
        def retire
          @mutex.synchronize do
            @retired = true
            @cov = nil
          end
          nil
        end

        # @api private
        def retired? = @mutex.synchronize { @retired }

        # @api private: the sentinel of the FIRST breach of either budget, or nil.
        def exhausted = @mutex.synchronize { @exhausted }

        # @api private: how many times the target called #edge.
        def edge_calls = @mutex.synchronize { @edge_calls }

        def inspect = "#<Fuzz::Handle>"

        alias_method :to_s, :inspect
      end
    end
  end
end
