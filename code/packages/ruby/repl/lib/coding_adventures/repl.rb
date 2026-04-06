# frozen_string_literal: true

# ============================================================================
# CodingAdventures::Repl — the main module and convenience API
# ============================================================================
#
# This file is the public surface of the REPL framework. It provides two
# class-level methods:
#
#   Repl.run(...)          — runs with real stdin/stdout
#   Repl.run_with_io(...)  — runs with injected I/O (for testing / embedding)
#
# All submodules and classes are required at the bottom of this file (via the
# top-level `coding_adventures_repl.rb`), so by the time user code calls
# `Repl.run`, everything is available.
#
# ## Design: module with class methods vs. a plain class
#
# We use a module with `module_function`-style class methods rather than
# instantiating a `Repl` object. This is idiomatic Ruby for framework entry
# points (compare `JSON.parse`, `YAML.load`, `CSV.parse`). The framework
# internals (Loop, Language, Prompt, Waiting) are all objects; the module is
# just a thin convenience wrapper.
#
# ## Defaults
#
#   language: EchoLanguage.new  — echoes input, quits on ":quit"
#   prompt:   DefaultPrompt.new — "> " and "... "
#   waiting:  SilentWaiting.new — no animation, polls every 100ms
#
# These defaults make `Repl.run` work out of the box with zero configuration.

require_relative "repl/version"
require_relative "repl/language"
require_relative "repl/prompt"
require_relative "repl/waiting"
require_relative "repl/echo_language"
require_relative "repl/default_prompt"
require_relative "repl/silent_waiting"
require_relative "repl/loop"

module CodingAdventures
  # Repl is the entry-point module for the REPL framework.
  #
  # It wires together the three pluggable interfaces (Language, Prompt,
  # Waiting) and the I/O injection layer into a runnable loop.
  module Repl
    # Run an interactive REPL session using real stdin/stdout.
    #
    # This is the method you call in a command-line application:
    #
    #   CodingAdventures::Repl.run(language: MyLanguage.new)
    #
    # @param language [#eval]          optional language backend (default: EchoLanguage)
    # @param prompt   [#global_prompt] optional prompt generator (default: DefaultPrompt)
    # @param waiting  [#start,...]     optional waiting strategy (default: SilentWaiting)
    # @return [nil]
    def self.run(language: EchoLanguage.new, prompt: DefaultPrompt.new, waiting: SilentWaiting.new)
      # Real-stdin input function: `$stdin.gets` returns a String (with trailing
      # newline) or nil on EOF (Ctrl-D). The loop handles both cases.
      input_fn  = -> { $stdin.gets }

      # Real-stdout output function: print without a trailing newline so the
      # prompt and output end up on separate lines only when we explicitly add
      # them. The language backend is responsible for any trailing newlines in
      # its output; the loop adds a newline after non-nil output.
      output_fn = ->(s) { $stdout.print(s) }

      run_with_io(
        language:  language,
        prompt:    prompt,
        waiting:   waiting,
        input_fn:  input_fn,
        output_fn: output_fn
      )
    end

    # Run a REPL session with injected I/O — the primary API for testing.
    #
    # @param language  [#eval]                language backend
    # @param prompt    [#global_prompt]        prompt generator
    # @param waiting   [#start,#tick,...]      waiting strategy (may be nil
    #                                          when mode: :sync)
    # @param input_fn  [Proc] → String|nil     returns next input line or nil
    # @param output_fn [Proc(String) → nil]    receives each output string
    # @param mode      [:async, :sync]         evaluation strategy (default: :async)
    # @return [nil]
    def self.run_with_io(
      language:,
      prompt:,
      waiting:,
      input_fn:,
      output_fn:,
      mode: :async
    )
      Loop.new(
        language:  language,
        prompt:    prompt,
        waiting:   waiting,
        input_fn:  input_fn,
        output_fn: output_fn,
        mode:      mode
      ).run
    end
  end
end
