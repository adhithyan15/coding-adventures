#!/usr/bin/env ruby
# frozen_string_literal: true

# true_tool.rb -- Do nothing, successfully
# =========================================
#
# === What This Program Does ===
#
# This is a reimplementation of the POSIX `true` utility. It does nothing
# and exits with a status code of 0 (success). That's it. No output, no
# side effects -- just a successful exit.
#
# === Why Does This Exist? ===
#
# `true` is one of the simplest Unix programs. It exists because shell
# scripts often need a command that always succeeds. Common uses:
#
#   - Infinite loops:    `while true; do ...; done`
#   - Default commands:  `PAGER="${PAGER:-true}"` (no-op if unset)
#   - Conditional chains: `true && echo "this always runs"`
#
# The POSIX standard requires that `true` accept (and ignore) any
# operands or options, but GNU coreutils adds `--help` and `--version`.
# Our implementation follows the GNU convention via CLI Builder.
#
# === How CLI Builder Powers This ===
#
# Even though `true` has no flags or arguments of its own, CLI Builder
# still provides `--help` and `--version` for free via the JSON spec.
# The spec declares no flags and no arguments -- CLI Builder handles
# the rest.

require "coding_adventures_cli_builder"

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------
# The spec file lives alongside this script. We resolve the path relative
# to this file's location so that the program works regardless of the
# user's current directory.

TRUE_SPEC_FILE = File.join(File.dirname(__FILE__), "true.json")

# ---------------------------------------------------------------------------
# Main entry point
# ---------------------------------------------------------------------------
# Parse arguments via CLI Builder, then exit successfully.
#
# The only interesting cases are --help and --version, which CLI Builder
# handles. For a normal invocation (no flags), we simply exit 0.

def true_main
  # --- Step 1: Parse arguments ---------------------------------------------
  # Hand the spec file and ARGV to CLI Builder. We prepend "true" to ARGV
  # because CLI Builder expects argv[0] to be the program name.
  begin
    result = CodingAdventures::CliBuilder::Parser.new(TRUE_SPEC_FILE, ["true"] + ARGV).parse
  rescue CodingAdventures::CliBuilder::ParseErrors => e
    # GNU `true` ignores all errors and always exits 0. However, we still
    # handle --help and --version via CLI Builder, so we parse first. If
    # parsing fails (e.g., truly malformed input), we exit 0 anyway --
    # that's what `true` does.
    e.errors.each { |err| warn "true: #{err.message}" }
    exit 0
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
    # The business logic of `true` is beautifully simple: do nothing.
    # The program exits with status 0 (success) by falling through.
    exit 0
  end
end

# Only run main when this file is executed directly (not when required
# from tests). This is the Ruby equivalent of Python's
# `if __name__ == "__main__"`.
true_main if __FILE__ == $PROGRAM_NAME
