# frozen_string_literal: true

# ============================================================================
# Waiting — the interface for async eval feedback
# ============================================================================
#
# When the REPL hands input to the language backend, evaluation happens on a
# separate thread. The main thread sits in a poll loop waiting for the
# evaluator to finish. During that wait, the Waiting interface controls what
# feedback (if any) the user receives.
#
# Classic examples of waiting behavior:
#   - A spinner:     /-\|/-\|  (cycles on each tick)
#   - A progress bar that grows: [===>   ]
#   - A simple dot:  . . . .  (prints a dot every second)
#   - Silent:        nothing (SilentWaiting — the default)
#
# ## The state machine
#
# The Waiting interface is a tiny stateful machine:
#
#   start() → initial_state
#      |
#      v
#   tick(state) → new_state   ← called every tick_ms milliseconds
#      |
#      v  (repeated until eval finishes)
#   stop(state) → nil         ← called once when eval completes
#
# State is an arbitrary value — the Waiting implementation decides what it
# stores. SilentWaiting uses `nil`. A spinner might store an integer index
# into the frames array.
#
# By making state explicit (rather than stored in instance variables), the
# interface is functional-ish: easy to test and reason about without setting
# up complex object state.
#
# ## Why not just use callbacks?
#
# The poll-based approach with `tick` is simpler to implement in pure Ruby
# (no extra threads, no concurrency issues). The main thread already has a
# tight loop calling `thread.join(tick_ms / 1000.0)` — it's natural to call
# `tick` in that same loop.

module CodingAdventures
  module Repl
    # Waiting is the async-eval feedback interface.
    #
    # Implement `start`, `tick`, `tick_ms`, and `stop` to provide custom
    # waiting animations or progress indicators while the language backend
    # evaluates user input on a background thread.
    module Waiting
      # Called once when async evaluation begins.
      #
      # @return [Object] initial state value (passed to `tick` and `stop`)
      def start
        raise NotImplementedError, "#{self.class}#start must be implemented"
      end

      # Called once per tick while waiting for evaluation to complete.
      #
      # @param state [Object] the state returned by the previous `tick` (or `start`)
      # @return [Object] the new state for the next tick
      def tick(state)
        raise NotImplementedError, "#{self.class}#tick must be implemented"
      end

      # How many milliseconds to wait between ticks.
      #
      # Smaller values give more responsive animations but consume more CPU.
      # 100ms is a good default — imperceptible latency, minimal overhead.
      #
      # @return [Integer] milliseconds between ticks
      def tick_ms
        raise NotImplementedError, "#{self.class}#tick_ms must be implemented"
      end

      # Called once when evaluation completes (success, error, or quit).
      #
      # Use this to clear any animation artifacts (e.g., erase a spinner line).
      #
      # @param state [Object] the final state from the last `tick` (or `start`)
      # @return [nil]
      def stop(state)
        raise NotImplementedError, "#{self.class}#stop must be implemented"
      end
    end
  end
end
