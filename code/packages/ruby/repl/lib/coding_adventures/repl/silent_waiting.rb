# frozen_string_literal: true

# ============================================================================
# SilentWaiting — a no-op Waiting implementation
# ============================================================================
#
# SilentWaiting satisfies the Waiting interface without producing any output
# or side effects. It is the default waiting strategy for the REPL loop.
#
# ## When to use SilentWaiting
#
# For most language backends (e.g., EchoLanguage), evaluation is near-instant.
# There's no need for a spinner or progress indicator. SilentWaiting lets the
# loop poll efficiently without any UI clutter.
#
# For truly long-running evaluations (e.g., a language that compiles before
# running), you might replace SilentWaiting with a spinner implementation.
#
# ## The null object pattern
#
# SilentWaiting is an instance of the Null Object pattern: instead of
# handling the absence of a waiting strategy with `if waiting`, we always
# have a waiting object — it just doesn't do anything. This keeps the loop
# code simple and uniform.
#
#   # Without null object:
#   waiting.start if waiting
#   waiting.tick(state) if waiting
#
#   # With null object (SilentWaiting):
#   waiting.start      # always safe to call
#   waiting.tick(state) # always safe to call
#
# ## tick_ms = 100
#
# 100 milliseconds is chosen as the default polling interval. This means:
#   - The loop checks for thread completion up to 10 times per second
#   - If evaluation takes 50ms, we'll notice within ~100ms — imperceptible lag
#   - CPU overhead is minimal: `thread.join(0.1)` is essentially free when the
#     thread is still running

module CodingAdventures
  module Repl
    # SilentWaiting is a no-op Waiting implementation.
    #
    # All methods are no-ops except `tick_ms`, which returns 100ms as the
    # default polling interval. Use this when no waiting animation is needed.
    class SilentWaiting
      include Waiting

      # Begin a waiting period. No-op.
      #
      # @return [nil] state is always nil for SilentWaiting
      def start
        nil
      end

      # Advance the waiting animation by one tick. No-op.
      #
      # @param _state [nil] ignored
      # @return [nil] state remains nil
      def tick(_state)
        nil
      end

      # Polling interval in milliseconds.
      #
      # @return [Integer] 100 — check for eval completion 10 times per second
      def tick_ms
        100
      end

      # End the waiting period. No-op.
      #
      # @param _state [nil] ignored
      # @return [nil]
      def stop(_state)
        nil
      end
    end
  end
end
