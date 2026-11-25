# frozen_string_literal: true

module CodingAdventures
  module TestBootstrap
    module Pbt
      # A generator could not produce a value (a filter that never accepts, a
      # spent budget).  The runner reports it as `property-exhausted`.
      class ExhaustedError < StandardError; end

      # A shrink was asked to start from a value that does not falsify the
      # property: a caller bug, loud by design.
      class NotFalsifyingError < ArgumentError
        def initialize(message = "the initial value must falsify the property")
          super
        end
      end

      # A generator of values: it draws from a PRNG stream and knows how to shrink
      # what it drew (VER05 3, 4).
      #
      # `sampler` is `(rng) -> value`.  `shrinker` is nil (values are reported
      # unshrunk) or a callable `(value, falsifies, max_steps) -> [value, steps]`
      # where `falsifies` is a callable returning truthy while a candidate still
      # falsifies the property.
      class Gen
        def initialize(sampler, shrinker = nil)
          raise ArgumentError, "a sampler is required" unless sampler.respond_to?(:call)
          raise ArgumentError, "a shrinker must be callable" unless shrinker.nil? || shrinker.respond_to?(:call)

          @sampler = sampler
          @shrinker = shrinker
        end

        def self.of(sampler = nil, shrinker = nil, &block)
          new(sampler || block, shrinker)
        end

        # Draws one value.
        def sample(rng) = @sampler.call(rng)

        # Shrinks `value` while `falsifies` holds; [value, 0] if there is no
        # shrinker.  A PUBLIC ENTRY POINT (VER05h 3.1): it opens a child of the
        # ambient budget (a fresh root outside any shrink) with its own local cap,
        # and charges every evaluation of `falsifies`, so however a property or a
        # custom shrinker nests calls, the total work stays inside the outermost
        # budget.
        def shrink(value, max_steps, &falsifies)
          raise ArgumentError, "a falsifies block is required" unless falsifies

          Limits.check_budget!(max_steps)
          ShrinkBudget.enter_child(max_steps) do |budget|
            run(value, budget.charged(falsifies), max_steps)
          end
        end

        # For combinators shrinking their components: joins the enclosing budget
        # instead of opening a new one (a fresh root only when none is open).
        # Public only because Ruby has no gem-private visibility; as defence in
        # depth it still charges every evaluation of `falsifies` to the budget it
        # joined, so even a leaf shrinker that charges nothing is bounded.
        # :nodoc:
        def shrink_nested(value, falsifies, max_steps)
          Limits.check_budget!(max_steps)
          ShrinkBudget.enter(max_steps) do |budget|
            run(value, budget.charged(falsifies), max_steps)
          end
        end

        private

        def run(value, falsifies, max_steps)
          return [value, 0] if @shrinker.nil?

          @shrinker.call(value, falsifies, max_steps)
        end
      end
    end
  end
end
