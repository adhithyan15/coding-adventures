#!/usr/bin/env ruby
# frozen_string_literal: true

# yes_tool.rb -- Repeatedly output a line with 'y' or a specified string
# ========================================================================
#
# === What This Program Does ===
#
# This is a reimplementation of the GNU `yes` utility. It repeatedly
# outputs a string to standard output until it is killed or the output
# pipe is closed.
#
# === How yes Works ===
#
# At its simplest, yes prints "y" over and over:
#
#     $ yes | head -3
#     y
#     y
#     y
#
# If you give it arguments, it joins them with spaces and repeats that
# line instead:
#
#     $ yes hello world | head -3
#     hello world
#     hello world
#     hello world
#
# === Why Does This Exist? ===
#
# `yes` exists to feed automatic "yes" responses to interactive programs
# that ask for confirmation:
#
#     $ yes | rm -i *.tmp    # Answers "y" to every "remove?" prompt
#     $ yes n | some_program # Answers "n" to every prompt
#
# It is also used for quick stress tests and to generate large amounts
# of output for benchmarking pipes.
#
# === Testability ===
#
# An infinite loop is hard to test! We solve this by extracting a
# `yes_output` method that takes a max_lines parameter. In production,
# max_lines is nil (infinite). In tests, we pass a small number.
# The method also accepts an IO object so we can capture output
# without touching $stdout.
#
# === How CLI Builder Powers This ===
#
# The JSON spec defines a variadic "string" argument with a default
# of "y". CLI Builder handles --help, --version, and argument collection.

require "coding_adventures_cli_builder"

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

YES_SPEC_FILE = File.join(File.dirname(__FILE__), "yes.json")

# ---------------------------------------------------------------------------
# Business Logic: yes_output
# ---------------------------------------------------------------------------
# Write `line` repeatedly to `io`.
#
# Parameters:
#   line      - The string to print on each line.
#   io        - The IO object to write to (default: $stdout).
#   max_lines - Maximum number of lines to print (nil = infinite).
#
# In production, max_lines is nil and the loop runs until the process
# is killed or the output pipe breaks (Errno::EPIPE). In tests, we
# pass a finite max_lines so the method returns.
#
# We rescue Errno::EPIPE because that is the normal exit path: the
# downstream process (e.g., `head`) closes its stdin, and we get a
# broken pipe signal. This is not an error -- it is how Unix pipes
# are designed to work.

def yes_output(line, io = $stdout, max_lines = nil)
  count = 0
  loop do
    break if max_lines && count >= max_lines
    io.puts(line)
    count += 1
  end
rescue Errno::EPIPE
  # Broken pipe is the normal exit when piped to another command.
  # Nothing to do -- just stop writing.
end

# ---------------------------------------------------------------------------
# Main entry point
# ---------------------------------------------------------------------------

def yes_main
  # --- Step 1: Parse arguments ---------------------------------------------
  begin
    result = CodingAdventures::CliBuilder::Parser.new(YES_SPEC_FILE, ["yes"] + ARGV).parse
  rescue CodingAdventures::CliBuilder::ParseErrors => e
    e.errors.each { |err| warn "yes: #{err.message}" }
    exit 1
  end

  # --- Step 2: Dispatch on result type -------------------------------------
  case result
  when CodingAdventures::CliBuilder::HelpResult
    puts result.text
    exit 0
  when CodingAdventures::CliBuilder::VersionResult
    puts result.version
    exit 0
  when CodingAdventures::CliBuilder::ParseResult
    # --- Step 3: Business logic --------------------------------------------
    # Collect the variadic string arguments. If none were given, CLI Builder
    # provides the default "y" from the spec. We join them with spaces,
    # just like GNU yes does.
    strings = result.arguments.fetch("string", [])
    line = strings.empty? ? "y" : strings.join(" ")

    # Print forever (until killed or pipe breaks).
    yes_output(line)
  end
end

# Only run main when this file is executed directly.
yes_main if __FILE__ == $PROGRAM_NAME
