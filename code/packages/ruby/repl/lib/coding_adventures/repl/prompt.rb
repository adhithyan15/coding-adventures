# frozen_string_literal: true

# ============================================================================
# Prompt — the interface for generating input prompts
# ============================================================================
#
# A REPL prompt is the little text printed before the cursor to tell the user
# "I'm ready for input." Classic examples:
#
#   irb(main):001:0>       # IRB's prompt — encodes context
#   >>>                    # Python's continuation prompt
#   $                      # a shell prompt
#   >                      # a minimal prompt
#
# Two prompts are distinguished:
#
#   global_prompt — shown at the start of a fresh statement. The user has
#                   finished (or never started) some expression.
#   line_prompt   — shown when the user has typed an incomplete expression and
#                   we are waiting for more. Like Python's `...` continuation
#                   prompt. Many simple REPLs never use this (they only accept
#                   single-line input), but the interface supports it for
#                   multi-line languages.
#
# ## Design note: why separate prompt and language?
#
# Separating the prompt from the language backend keeps each concern narrow.
# The language impl shouldn't need to know how the prompt is displayed; the
# prompt impl shouldn't need to know anything about language semantics. This
# lets you mix any language with any prompt style.

module CodingAdventures
  module Repl
    # Prompt is the interface for user-facing prompt strings.
    #
    # Any object that implements `global_prompt` and `line_prompt` satisfies
    # this interface. Include this module to signal intent.
    module Prompt
      # Prompt shown at the start of a new expression.
      #
      # @return [String] the prompt text (e.g., "> ")
      def global_prompt
        raise NotImplementedError, "#{self.class}#global_prompt must be implemented"
      end

      # Prompt shown when continuing a multi-line expression.
      #
      # @return [String] the continuation prompt (e.g., "... ")
      def line_prompt
        raise NotImplementedError, "#{self.class}#line_prompt must be implemented"
      end
    end
  end
end
