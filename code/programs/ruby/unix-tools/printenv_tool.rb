#!/usr/bin/env ruby
# frozen_string_literal: true

# printenv_tool.rb -- Print all or part of environment
# =====================================================
#
# === What This Program Does ===
#
# This is a reimplementation of the GNU `printenv` utility. It prints
# the values of the specified environment variables. If no variables
# are specified, it prints all environment variables.
#
# === Examples ===
#
#     $ printenv HOME
#     /home/user
#
#     $ printenv HOME PATH
#     /home/user
#     /usr/bin:/bin:/usr/sbin
#
#     $ printenv
#     HOME=/home/user
#     PATH=/usr/bin:/bin
#     ...
#
# === Exit Status ===
#
# printenv has a meaningful exit status:
#
#   0 - All specified variables were found.
#   1 - At least one specified variable was not found.
#   2 - An error occurred.
#
# When no variables are specified (print all), the exit status is 0.
#
# === Difference from env ===
#
# `printenv` and `env` both print environment variables, but they
# differ in usage:
#
# - `printenv VAR` prints just the value of VAR (no "VAR=" prefix).
# - `env` always prints in "KEY=VALUE" format.
# - `env` can also modify the environment before running a command.
#
# === The -0 Flag ===
#
# The -0 (null) flag terminates each output line with a NUL character
# instead of a newline. This is useful when piping to `xargs -0`,
# since environment variable values can contain newlines.

require "coding_adventures_cli_builder"

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

PRINTENV_SPEC_FILE = File.join(File.dirname(__FILE__), "printenv.json")

# ---------------------------------------------------------------------------
# Main entry point
# ---------------------------------------------------------------------------

def printenv_main
  # --- Step 1: Parse arguments ---------------------------------------------
  begin
    result = CodingAdventures::CliBuilder::Parser.new(PRINTENV_SPEC_FILE, ["printenv"] + ARGV).parse
  rescue CodingAdventures::CliBuilder::ParseErrors => e
    e.errors.each { |err| warn "printenv: #{err.message}" }
    exit 2
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
    variables = result.arguments.fetch("variables", [])
    null_terminated = result.flags["null"]
    terminator = null_terminated ? "\0" : "\n"

    if variables.empty?
      # No variables specified: print all environment variables in
      # KEY=VALUE format, sorted by key for consistency.
      ENV.sort.each do |key, value|
        print "#{key}=#{value}"
        print terminator
      end
    else
      # Print the value of each specified variable.
      # Track whether any variable was not found for the exit status.
      all_found = true

      variables.each do |var|
        value = ENV[var]
        if value
          print value
          print terminator
        else
          # Variable not found. Print nothing for it, but remember
          # so we can set exit status to 1.
          all_found = false
        end
      end

      exit 1 unless all_found
    end
  end
end

# Only run main when this file is executed directly.
printenv_main if __FILE__ == $PROGRAM_NAME
