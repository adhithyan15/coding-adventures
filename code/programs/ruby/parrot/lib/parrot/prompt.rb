# frozen_string_literal: true

# ============================================================================
# Parrot::Prompt — the personality layer for the Parrot REPL
# ============================================================================
#
# The `CodingAdventures::Repl::Prompt` module defines the contract for prompt
# objects in the REPL framework. Any object that includes it and implements
# `global_prompt` and `line_prompt` can be used as a prompt.
#
# `Parrot::Prompt` fills in those two methods with parrot-themed strings.
#
# ## Why include a module rather than inherit from a class?
#
# `CodingAdventures::Repl::Prompt` is defined as a Ruby `module`, not a
# `class`. Modules in Ruby define a "mixin" — a set of methods you opt into
# by writing `include ModuleName`. You cannot inherit from a module with `<`.
#
# Modules serve two purposes in this codebase:
#
#   1. Interface declaration (like Go interfaces or Java interfaces)
#   2. Shared default behaviour via methods that call `super`
#
# Here the module provides stub implementations that raise `NotImplementedError`
# if not overridden — a classic Ruby "abstract method" pattern. By including
# the module and overriding both methods, Parrot::Prompt fully satisfies the
# contract without any raise paths being reachable.
#
# ## Design: why a separate file?
#
# Keeping the prompt in `lib/parrot/prompt.rb` (separate from `lib/parrot.rb`)
# lets us test it independently. The tests can require just this file and
# inspect the prompt strings without running the REPL loop.

require "coding_adventures/repl/prompt"

module Parrot
  # Prompt satisfies the CodingAdventures::Repl::Prompt interface with
  # parrot-themed text.
  #
  # A parrot repeats everything you say. The prompts use parrot emoji (🦜)
  # and friendly text to reinforce the theme.
  #
  # ## Two prompts
  #
  # global_prompt — shown before each new line of input. Contains a banner
  #                 with the program name and a usage hint.
  #
  # line_prompt   — shown when a multi-line expression is being continued.
  #                 EchoLanguage never produces multi-line sessions, but the
  #                 interface requires this method. It returns a short inline
  #                 prompt matching the parrot theme.
  class Prompt
    include CodingAdventures::Repl::Prompt

    # global_prompt — the banner shown before each new input.
    #
    # Returns a two-line banner followed by a blank line:
    #
    #   🦜 Parrot REPL
    #   I repeat everything you say! Type :quit to exit.
    #
    # The trailing "\n\n" separates the banner from any output printed
    # during the previous evaluation. The newlines are part of the prompt
    # string because the loop writes prompt strings verbatim without adding
    # whitespace.
    #
    # @return [String] the parrot banner text
    def global_prompt
      "🦜 Parrot REPL\nI repeat everything you say! Type :quit to exit.\n\n"
    end

    # line_prompt — the continuation prompt shown during multi-line input.
    #
    # EchoLanguage evaluates every line immediately, so this prompt is
    # never used in the Parrot REPL. It is provided for completeness.
    #
    # @return [String] a short inline prompt with parrot emoji
    def line_prompt
      "🦜 > "
    end
  end
end
