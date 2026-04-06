# frozen_string_literal: true

# ============================================================================
# EchoLanguage — the simplest possible Language implementation
# ============================================================================
#
# EchoLanguage is the "hello world" of language backends. It does no parsing
# or evaluation: it simply reflects the user's input back as output.
#
# This is useful for:
#   1. Testing the REPL loop without a real language backend
#   2. Verifying I/O injection plumbing works end-to-end
#   3. Demonstrating the minimum interface needed for a Language
#
# ## Behavior
#
#   Input ":quit"  → :quit       — terminates the session
#   Any other input → [:ok, input] — echoes the input back
#
# ## Why ":quit" as the quit signal?
#
# The colon prefix follows the convention used by many REPLs (IRB uses
# `:exit`, psql uses `\q`, etc.). A unique prefix prevents accidental
# quitting when a program legitimately uses the word "quit" as data.
# We use ":quit" as a simple, memorable default. Real language backends
# might support `:quit`, `exit`, `Ctrl-D`, etc.

module CodingAdventures
  module Repl
    # EchoLanguage satisfies the Language interface by echoing input back.
    #
    # It is the canonical minimal Language implementation and the default
    # used by `Repl.run` when no language is specified.
    class EchoLanguage
      include Language

      # Evaluate user input by echoing it, or quit on ":quit".
      #
      # @param input [String] raw user input
      # @return [:quit] if input == ":quit"
      # @return [Array(:ok, String)] otherwise, with the input as the value
      def eval(input)
        # The quit signal is checked first so the user can always exit.
        return :quit if input == ":quit"

        # Echo the input back unchanged. The :ok tag signals success.
        # A real language backend might return [:ok, result.inspect] here.
        [:ok, input]
      end
    end
  end
end
