#!/usr/bin/env ruby
# frozen_string_literal: true

# pwd_tool.rb -- Print the absolute pathname of the current working directory
# ==========================================================================
#
# === What This Program Does ===
#
# This is a reimplementation of the POSIX `pwd` utility. It prints the
# absolute path of the current working directory to standard output.
#
# === How CLI Builder Powers This ===
#
# The entire command-line interface -- flags, help text, version output,
# error messages -- is defined in `pwd.json`. This program never parses
# a single argument by hand. Instead:
#
# 1. We hand `pwd.json` and ARGV to CLI Builder's Parser.
# 2. The parser validates the input, enforces mutual exclusivity of
#    `-L` and `-P`, generates help text, and returns a typed result.
# 3. We case/when on the result type and run the business logic.
#
# The result is that *this file contains only business logic*. All parsing,
# validation, and help generation happen inside CLI Builder, driven by the
# JSON spec.
#
# === Logical vs Physical Paths ===
#
# When you `cd` through a symbolic link, the shell updates the `$PWD`
# environment variable to reflect the path *as you typed it* -- including
# the symlink. This is the "logical" path.
#
# The "physical" path resolves all symlinks. For example, if `/home` is
# a symlink to `/usr/home`:
#
#     Logical:  /home/user       (what $PWD says)
#     Physical: /usr/home/user   (what the filesystem says)
#
# By default (`-L`), we print the logical path. With `-P`, we resolve
# symlinks and print the physical path.
#
# === POSIX Compliance Note ===
#
# If `$PWD` is not set, or if it doesn't match the actual current
# directory, even `-L` mode falls back to the physical path. This
# matches POSIX behavior.

require "pathname"
require "coding_adventures_cli_builder"

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------
# The spec file lives alongside this script. We resolve the path relative
# to this file's location so that the program works regardless of the
# user's current directory.

SPEC_FILE = File.join(File.dirname(__FILE__), "pwd.json")

# ---------------------------------------------------------------------------
# Business Logic: get_logical_pwd
# ---------------------------------------------------------------------------
# Return the logical working directory.
#
# The logical path comes from the `$PWD` environment variable, which
# the shell maintains as the user navigates -- including through symlinks.
#
# If `$PWD` is not set or is stale (doesn't match the real cwd), we
# fall back to the physical path. This matches POSIX behavior: the
# logical path is best-effort, never wrong.

def get_logical_pwd
  env_pwd = ENV["PWD"]

  if env_pwd
    # Verify that $PWD actually points to the current directory.
    # It could be stale if the directory was moved/deleted, or if
    # the process changed directories without updating $PWD.
    begin
      env_real = File.realpath(env_pwd)
      cwd_real = File.realpath(".")
      return env_pwd if env_real == cwd_real
    rescue SystemCallError
      # If realpath fails (e.g., path doesn't exist), fall through
      # to the physical path below.
    end
  end

  # Fallback: resolve the physical path.
  Pathname.new(".").realpath.to_s
end

# ---------------------------------------------------------------------------
# Business Logic: get_physical_pwd
# ---------------------------------------------------------------------------
# Return the physical working directory with all symlinks resolved.
#
# This calls Pathname.new('.').realpath, which follows every symlink in
# the path to produce the canonical filesystem path.

def get_physical_pwd
  Pathname.new(".").realpath.to_s
end

# ---------------------------------------------------------------------------
# Main entry point
# ---------------------------------------------------------------------------
# Parse arguments via CLI Builder, then print the current working directory.

def main
  # --- Step 1: Parse arguments ---------------------------------------------
  # Hand the spec file and ARGV to CLI Builder. We prepend "pwd" to ARGV
  # because CLI Builder expects argv[0] to be the program name (just like
  # sys.argv in Python includes the script name).
  begin
    result = CodingAdventures::CliBuilder::Parser.new(SPEC_FILE, ["pwd"] + ARGV).parse
  rescue CodingAdventures::CliBuilder::ParseErrors => e
    e.errors.each { |err| warn "pwd: #{err.message}" }
    exit 1
  end

  # --- Step 2: Dispatch on result type -------------------------------------
  # CLI Builder returns one of:
  #   - HelpResult:    user passed --help
  #   - VersionResult: user passed --version
  #   - ParseResult:   normal invocation; flags and arguments are populated

  case result
  when CodingAdventures::CliBuilder::HelpResult
    puts result.text
    exit 0
  when CodingAdventures::CliBuilder::VersionResult
    puts result.version
    exit 0
  when CodingAdventures::CliBuilder::ParseResult
    # --- Step 3: Business logic --------------------------------------------
    # This is the *only* part that is specific to the pwd tool.
    # CLI Builder has already validated the flags, so we just check
    # whether the "physical" flag is set.
    if result.flags["physical"]
      puts get_physical_pwd
    else
      puts get_logical_pwd
    end
  end
end

# Only run main when this file is executed directly (not when required
# from tests). This is the Ruby equivalent of Python's
# `if __name__ == "__main__"`.
main if __FILE__ == $PROGRAM_NAME
