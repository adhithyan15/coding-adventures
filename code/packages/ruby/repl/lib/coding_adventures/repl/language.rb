# frozen_string_literal: true

# ============================================================================
# Language — the pluggable evaluator interface
# ============================================================================
#
# Every REPL needs a "language" — something that takes a string of user input
# and evaluates it, returning either a result, an error, or a signal to quit.
#
# This module documents the interface that any language implementation must
# satisfy. Ruby uses duck typing, so there is no enforcement: any object that
# responds to `eval` with the correct return contract can be used. The module
# is here for documentation and as an optional mixin.
#
# ## The contract
#
# `eval(input)` receives the raw string the user typed and must return one of:
#
#   [:ok, String]   — evaluation succeeded; the String is the output to print
#                     (may be nil if there is nothing to show, e.g. an assignment)
#   [:error, String] — evaluation failed; the String is the error message
#   :quit            — the user asked to exit; the loop should terminate cleanly
#
# ## Design note: why a tagged union return?
#
# Ruby doesn't have sum types, but we can simulate them with tagged arrays.
# Returning `[:ok, value]` vs `[:error, message]` vs `:quit` gives callers a
# clear, exhaustive set of cases to pattern-match (via `case/in`). This is far
# safer than raising exceptions across thread boundaries or relying on nil.
#
# Compare with Elixir's `{:ok, value} | {:error, reason}` — same idea, just
# in Ruby's syntax.

module CodingAdventures
  module Repl
    # Language is the evaluator interface.
    #
    # Any object that implements `eval(input)` according to the contract above
    # can serve as a language backend for the REPL loop.
    #
    # Implementors may include this module to signal intent, but it is not
    # required — duck typing suffices.
    module Language
      # Evaluate a single line of user input.
      #
      # @param input [String] the raw text entered by the user
      # @return [Array(:ok, String|nil)]   evaluation succeeded
      # @return [Array(:error, String)]    evaluation failed with a message
      # @return [:quit]                    the session should end
      def eval(input)
        raise NotImplementedError, "#{self.class}#eval must be implemented"
      end
    end
  end
end
