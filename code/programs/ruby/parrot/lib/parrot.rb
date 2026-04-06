# frozen_string_literal: true

# ============================================================================
# Parrot — top-level module and entry point for the Parrot REPL
# ============================================================================
#
# This file is the public API for the Parrot program. It provides:
#
#   Parrot.run  — start the REPL reading from $stdin and writing to $stdout.
#
# The implementation delegates entirely to the `coding_adventures_repl` gem.
# This file's job is just to wire the pieces together:
#
#   EchoLanguage   — evaluates input by echoing it back (or quitting on :quit)
#   Parrot::Prompt — produces parrot-themed banner and inline prompts
#   SilentWaiting  — no-op animation (echo is instant; no spinner needed)
#   Repl.run_with_io — the REPL loop from the framework
#
# ## Require ordering
#
# Ruby requires files from top to bottom. Dependencies must be required
# before the code that uses them:
#
#   1. parrot/prompt  → requires coding_adventures/repl/prompt (the interface)
#   2. coding_adventures_repl → requires the full framework (EchoLanguage, etc.)
#
# If we required coding_adventures_repl AFTER parrot/prompt, the Prompt
# module would already be loaded (from the path require inside prompt.rb), but
# EchoLanguage and SilentWaiting would not yet be defined. The order here is
# safe because parrot/prompt requires only the Prompt module, and then
# coding_adventures_repl loads the rest.

require_relative "parrot/prompt"
require "coding_adventures_repl"

# Parrot is the top-level namespace for the Parrot REPL program.
#
# It consists of a single class method `run` that starts an interactive
# session. The session runs until the user types `:quit` or EOF (Ctrl-D).
module Parrot
  # run — start the Parrot REPL with real stdin/stdout.
  #
  # Wires together all the REPL components and delegates to
  # CodingAdventures::Repl.run_with_io with real I/O functions:
  #
  #   input_fn  — reads one line from $stdin (gets returns nil on EOF)
  #   output_fn — writes a string to $stdout (no trailing newline added)
  #
  # The `&.chomp` call strips the trailing newline that `gets` appends.
  # If `gets` returns nil (EOF), `&.` short-circuits and returns nil,
  # which the loop treats as quit.
  #
  # @return [nil]
  def self.run
    CodingAdventures::Repl.run_with_io(
      language:  CodingAdventures::Repl::EchoLanguage.new,
      prompt:    Parrot::Prompt.new,
      waiting:   CodingAdventures::Repl::SilentWaiting.new,
      input_fn:  -> { $stdin.gets&.chomp },
      output_fn: ->(text) { $stdout.write(text) },
      mode:      :async
    )
  end
end
