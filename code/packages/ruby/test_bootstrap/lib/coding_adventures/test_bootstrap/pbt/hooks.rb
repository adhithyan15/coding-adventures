# frozen_string_literal: true

module CodingAdventures
  module TestBootstrap
    module Pbt
      # Sabotage hooks: the only production indirections added for the negative
      # controls (VER05h section 6).  A test swaps one, runs the corpus, and the
      # runner must report problems.  Production code never reassigns them.
      # Ruby has no gem-private visibility, so this is a documented test seam.
      module Hooks
        DEFAULT_BISECT = ->(current, diff) { current - (diff >> 1) } # >> floors on negatives
        DEFAULT_UNIT = ->(current, diff) { current - (diff <=> 0) }
        DEFAULT_RNG = ->(seed) { Prng::SplitMix64.new(seed) }

        class << self
          attr_accessor :bisect_candidate, :unit_candidate, :skip_deletion_phase, :new_rng

          def reset
            @bisect_candidate = DEFAULT_BISECT
            @unit_candidate = DEFAULT_UNIT
            @skip_deletion_phase = false
            @new_rng = DEFAULT_RNG
            ShrinkBudget.default_max_work = Limits::MAX_SHRINK_WORK if defined?(ShrinkBudget)
          end
        end

        reset
      end
    end
  end
end
