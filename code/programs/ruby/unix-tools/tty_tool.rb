#!/usr/bin/env ruby
# frozen_string_literal: true

# tty_tool.rb -- Print the terminal name connected to standard input
# ===================================================================
#
# === What This Program Does ===
#
# This is a reimplementation of the POSIX `tty` utility. It prints the
# file name of the terminal connected to standard input.
#
# === How tty Works ===
#
# If standard input is a terminal:
#
#     $ tty
#     /dev/ttys003
#
# If standard input is NOT a terminal (e.g., piped):
#
#     $ echo | tty
#     not a tty
#
# The exit status tells you the answer:
#   - 0: standard input is a terminal
#   - 1: standard input is NOT a terminal
#
# === The -s (silent) Flag ===
#
# With `-s`, tty prints nothing at all. It only communicates via exit
# status. This is useful in shell scripts:
#
#     if tty -s; then
#       echo "Interactive session"
#     else
#       echo "Non-interactive (piped or redirected)"
#     fi
#
# === Implementation ===
#
# We use `$stdin.tty?` to check if stdin is connected to a terminal.
# To get the terminal's file path, we use `$stdin.path` if available,
# or fall back to `/dev/tty`.
#
# === How CLI Builder Powers This ===
#
# The JSON spec defines a single boolean flag `-s`/`--silent`. CLI
# Builder handles parsing and provides --help and --version.

require "coding_adventures_cli_builder"

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

TTY_SPEC_FILE = File.join(File.dirname(__FILE__), "tty.json")

# ---------------------------------------------------------------------------
# Business Logic: get_tty_name
# ---------------------------------------------------------------------------
# Determine the terminal name for the given input stream.
#
# Parameters:
#   input - An IO object to check (default: $stdin)
#
# Returns: [String, Boolean]
#   - The terminal path (or "not a tty"), and whether it IS a tty.
#
# If the input is a terminal, we try to determine its path. The method
# `IO#path` is available on File objects but not on all IO streams.
# For $stdin specifically, on Unix systems, we can use the `ttyname`
# system call via IO#inspect or the /dev/fd symlink.

def get_tty_name(input = $stdin)
  if input.tty?
    # Try to get the tty path. On most Unix systems, IO#ttyname is not
    # directly available in Ruby, but we can use the POSIX ttyname(3)
    # function through IO#inspect or by reading /dev/stdin.
    begin
      path = File.readlink("/dev/fd/#{input.fileno}")
      [path, true]
    rescue Errno::EINVAL, Errno::ENOENT, NotImplementedError
      # If readlink fails, fall back to /dev/tty as a generic name.
      ["/dev/tty", true]
    end
  else
    ["not a tty", false]
  end
end

# ---------------------------------------------------------------------------
# Main entry point
# ---------------------------------------------------------------------------

def tty_main
  # --- Step 1: Parse arguments ---------------------------------------------
  begin
    result = CodingAdventures::CliBuilder::Parser.new(TTY_SPEC_FILE, ["tty"] + ARGV).parse
  rescue CodingAdventures::CliBuilder::ParseErrors => e
    e.errors.each { |err| warn "tty: #{err.message}" }
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
    tty_name, is_tty = get_tty_name

    # In silent mode, print nothing -- just exit with the status.
    unless result.flags["silent"]
      puts tty_name
    end

    exit(is_tty ? 0 : 1)
  end
end

# Only run main when this file is executed directly.
tty_main if __FILE__ == $PROGRAM_NAME
