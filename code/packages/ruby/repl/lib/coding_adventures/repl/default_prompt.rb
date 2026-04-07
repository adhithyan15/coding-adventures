# frozen_string_literal: true

# ============================================================================
# DefaultPrompt — the standard minimal prompt
# ============================================================================
#
# A prompt tells the user "I'm listening." The two most universal prompt
# strings in the Unix world are:
#
#   "$ "  — the shell (the OS is listening)
#   "> "  — the REPL  (a language interpreter is listening)
#   "... " — continuation (more input expected)
#
# DefaultPrompt uses "> " and "... " — the same strings used by Python,
# Node.js, and many other interactive interpreters. They're short, readable,
# and universally understood.
#
# ## Example session with DefaultPrompt
#
#   > hello
#   hello
#   > :quit
#   (session ends)
#
# The trailing space after ">" and "..." is important — without it, the user's
# cursor would be immediately adjacent to the prompt character, making input
# harder to read. Compare:
#
#   >hello    ← hard to read (no space)
#   > hello   ← easy to read (one space)

module CodingAdventures
  module Repl
    # DefaultPrompt provides minimal "> " and "... " prompts.
    #
    # It is the canonical Prompt implementation used by the REPL when no
    # custom prompt is provided.
    class DefaultPrompt
      include Prompt

      # The primary prompt, shown at the start of a fresh statement.
      #
      # @return [String] "> "
      def global_prompt
        "> "
      end

      # The continuation prompt, shown when more input is expected.
      #
      # @return [String] "... "
      def line_prompt
        "... "
      end
    end
  end
end
