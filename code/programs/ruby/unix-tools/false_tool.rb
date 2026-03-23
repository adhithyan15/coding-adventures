#!/usr/bin/env ruby
# frozen_string_literal: true

# false_tool.rb -- Do nothing, unsuccessfully
# =============================================
#
# === What This Program Does ===
#
# This is a reimplementation of the POSIX `false` utility. It does nothing
# and exits with a status code of 1 (failure). It is the mirror image of
# `true` -- where `true` always succeeds, `false` always fails.
#
# === Why Does This Exist? ===
#
# `false` is the complement to `true`. Shell scripts need a command that
# always fails. Common uses:
#
#   - Disabling commands: `alias rm=false` (prevents accidental deletion)
#   - Test scaffolding:   `if false; then ...; fi` (dead code / skip block)
#   - Error propagation:  `false || echo "something went wrong"`
#
# The exit code 1 is the conventional "general error" code in Unix.
# Unlike `true`, which always exits 0 regardless of arguments, GNU
# `false` exits 1 even when given `--help` or `--version` (though it
# still prints the help/version text). Our implementation follows this
# behavior.
#
# === How CLI Builder Powers This ===
#
# Like `true`, the spec declares no flags and no arguments. CLI Builder
# provides `--help` and `--version` automatically. The key difference
# from `true` is the exit code: we exit 1 after everything.

require "coding_adventures_cli_builder"

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------
# The spec file lives alongside this script. We resolve the path relative
# to this file's location so that the program works regardless of the
# user's current directory.

FALSE_SPEC_FILE = File.join(File.dirname(__FILE__), "false.json")

# ---------------------------------------------------------------------------
# Main entry point
# ---------------------------------------------------------------------------
# Parse arguments via CLI Builder, then exit unsuccessfully.
#
# GNU `false` is interesting: even `--help` and `--version` produce their
# output but still exit with status 1. We follow this convention.

def false_main
  # --- Step 1: Parse arguments ---------------------------------------------
  # Hand the spec file and ARGV to CLI Builder. We prepend "false" to ARGV
  # because CLI Builder expects argv[0] to be the program name.
  begin
    result = CodingAdventures::CliBuilder::Parser.new(FALSE_SPEC_FILE, ["false"] + ARGV).parse
  rescue CodingAdventures::CliBuilder::ParseErrors => e
    # Like GNU `false`, we always exit 1, even on parse errors.
    e.errors.each { |err| warn "false: #{err.message}" }
    exit 1
  end

  # --- Step 2: Dispatch on result type -------------------------------------
  # Note: GNU `false` exits 1 even for --help and --version. We print the
  # text but always return failure. This matches coreutils behavior.
  case result
  when CodingAdventures::CliBuilder::HelpResult
    puts result.text
    exit 1
  when CodingAdventures::CliBuilder::VersionResult
    puts result.version
    exit 1
  when CodingAdventures::CliBuilder::ParseResult
    # --- Step 3: Business logic --------------------------------------------
    # The business logic of `false` is the opposite of `true`: do nothing,
    # but signal failure. Exit with status 1.
    exit 1
  end
end

# Only run main when this file is executed directly (not when required
# from tests). This is the Ruby equivalent of Python's
# `if __name__ == "__main__"`.
false_main if __FILE__ == $PROGRAM_NAME
