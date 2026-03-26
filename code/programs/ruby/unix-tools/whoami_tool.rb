#!/usr/bin/env ruby
# frozen_string_literal: true

# whoami_tool.rb -- Print the effective username
# ===============================================
#
# === What This Program Does ===
#
# This is a reimplementation of the `whoami` utility. It prints the
# user name associated with the current effective user ID to standard
# output, followed by a newline.
#
# === How whoami Works ===
#
#     $ whoami
#     alice
#
# This is roughly equivalent to `id -un`, but simpler. It answers
# the question: "What user am I running as right now?"
#
# === Effective vs Real User ID ===
#
# When you run a program with `sudo`, your *real* user ID stays the
# same (e.g., alice), but your *effective* user ID changes (e.g., root).
# `whoami` reports the *effective* user -- the identity the system is
# actually using for permission checks.
#
# This is different from `logname`, which reports the *login* name --
# the user who originally logged into the terminal session.
#
# === Implementation ===
#
# We use `Etc.getpwuid(Process.euid).name` to look up the effective
# user's name from the system password database. This is the most
# reliable approach because it uses the actual effective user ID
# rather than trusting environment variables (which can be spoofed).
#
# As a fallback, we check `ENV["USER"]`, which works on most Unix
# systems but can be overridden.
#
# === How CLI Builder Powers This ===
#
# The whoami spec has no flags and no arguments. CLI Builder provides
# --help and --version automatically.

require "etc"
require "coding_adventures_cli_builder"

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

WHOAMI_SPEC_FILE = File.join(File.dirname(__FILE__), "whoami.json")

# ---------------------------------------------------------------------------
# Business Logic: get_effective_username
# ---------------------------------------------------------------------------
# Return the effective username of the current process.
#
# Strategy:
#   1. Try Etc.getpwuid(Process.euid) -- the most reliable method.
#      This looks up the effective user ID in the password database.
#   2. Fall back to ENV["USER"] if the password database lookup fails.
#   3. Return nil if neither method works.

def get_effective_username
  Etc.getpwuid(Process.euid).name
rescue ArgumentError
  # The effective UID might not have a corresponding entry in /etc/passwd
  # (e.g., in some container environments). Fall back to $USER.
  ENV["USER"]
end

# ---------------------------------------------------------------------------
# Main entry point
# ---------------------------------------------------------------------------

def whoami_main
  # --- Step 1: Parse arguments ---------------------------------------------
  begin
    result = CodingAdventures::CliBuilder::Parser.new(WHOAMI_SPEC_FILE, ["whoami"] + ARGV).parse
  rescue CodingAdventures::CliBuilder::ParseErrors => e
    e.errors.each { |err| warn "whoami: #{err.message}" }
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
    username = get_effective_username
    if username
      puts username
    else
      warn "whoami: cannot find name for user ID #{Process.euid}"
      exit 1
    end
  end
end

# Only run main when this file is executed directly.
whoami_main if __FILE__ == $PROGRAM_NAME
