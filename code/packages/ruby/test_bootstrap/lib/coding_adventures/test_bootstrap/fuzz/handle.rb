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
        #
        # It is UNFORGEABLE by identity, not by fields: it carries no data at all.  Who
        # issued it, in which execution and for which budget lives in the issuing
        # CampaignToken's private table, keyed by the sentinel object itself.  A target
        # that builds its own `BudgetExceeded.new` has an object no table knows: it is
        # an ordinary crash, never a timeout, and never aborts the campaign.
        class BudgetExceeded < Exception # rubocop:disable Lint/InheritException
          def initialize
            super("fuzz budget exhausted")
          end
        end

        # One per campaign.  It records every sentinel its handles issue
        # (sentinel -> [epoch, which]) and a process-wide weak map says which token
        # owns a sentinel, so a sentinel of an OUTER campaign can be told from a
        # forged one and re-raised to its owner.
        class CampaignToken
          OWNERS = ObjectSpace::WeakMap.new
          private_constant :OWNERS

          def self.owner_of(sentinel) = OWNERS[sentinel]

          def initialize
            @issued = {}.compare_by_identity
          end

          # @api private
          def issue(epoch, which)
            sentinel = BudgetExceeded.new
            @issued[sentinel] = [epoch, which].freeze
            OWNERS[sentinel] = self
            sentinel
          end

          # [epoch, which] for a sentinel this token issued, else nil.
          def provenance(sentinel) = @issued[sentinel]

          def inspect = "#<Fuzz::Handle::CampaignToken>"

          alias_method :to_s, :inspect
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
          @exhausted_which = nil
          @retired_sentinel = nil
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
            raise retired_sentinel if @retired
            raise @exhausted if @exhausted

            if @steps >= @step_budget
              breach(:steps)
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
            raise retired_sentinel if @retired
            raise @exhausted if @exhausted
            return nil unless n.positive?

            if n > @mem_budget - @reserved
              breach(:memory)
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

        # @api private: :steps or :memory, whichever budget broke first, or nil.
        def exhausted_which = @mutex.synchronize { @exhausted_which }

        # @api private: how many times the target called #edge.
        def edge_calls = @mutex.synchronize { @edge_calls }

        # Called with the mutex held.
        def breach(which)
          @exhausted_which = which
          @exhausted = @campaign.issue(@epoch, which)
        end
        private :breach

        # One memoised sentinel per handle: a stale thread spinning on a retired
        # handle cannot mint unbounded objects.  Called with the mutex held.
        def retired_sentinel
          @retired_sentinel ||= @campaign.issue(@epoch, :retired)
        end
        private :retired_sentinel

        def inspect = "#<Fuzz::Handle>"

        alias_method :to_s, :inspect
      end
    end
  end
end
