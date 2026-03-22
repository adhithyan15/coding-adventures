#!/usr/bin/env ruby
# frozen_string_literal: true

# logname_tool.rb -- Print the user's login name
# ================================================
#
# === What This Program Does ===
#
# This is a reimplementation of the POSIX `logname` utility. It prints
# the login name of the current user -- the name used to log into the
# system originally.
#
# === How logname Works ===
#
#     $ logname
#     alice
#
# === logname vs whoami ===
#
# These two commands answer different questions:
#
#   - `logname`:  "Who logged into this terminal session?"
#   - `whoami`:   "What user am I running as right now?"
#
# They differ when you switch users:
#
#     $ logname        # -> alice (who logged in)
#     $ sudo whoami    # -> root  (effective user after sudo)
#     $ sudo logname   # -> alice (still the original login user)
#
# This distinction matters for auditing: logname tells you the human
# who is responsible, regardless of what user they have switched to.
#
# === Implementation ===
#
# We try `Etc.getlogin` first, which queries the system's utmp/utmpx
# database to find who is logged into the controlling terminal. If that
# fails, we fall back to the LOGNAME environment variable, which POSIX
# requires login(1) to set.
#
# If neither is available, we print an error and exit with status 1,
# matching POSIX behavior.
#
# === How CLI Builder Powers This ===
#
# The logname spec has no flags and no arguments. CLI Builder provides
# --help and --version automatically.

require "etc"
require "coding_adventures_cli_builder"

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

LOGNAME_SPEC_FILE = File.join(File.dirname(__FILE__), "logname.json")

# ---------------------------------------------------------------------------
# Business Logic: get_login_name
# ---------------------------------------------------------------------------
# Return the login name of the current user.
#
# Strategy:
#   1. Try Etc.getlogin -- queries the utmp database for the controlling
#      terminal's login record.
#   2. Fall back to ENV["LOGNAME"], which POSIX requires login(1) to set.
#   3. Return nil if neither is available.

def get_login_name
  name = Etc.getlogin
  return name if name && !name.empty?

  logname = ENV["LOGNAME"]
  return logname if logname && !logname.empty?

  nil
end

# ---------------------------------------------------------------------------
# Main entry point
# ---------------------------------------------------------------------------

def logname_main
  # --- Step 1: Parse arguments ---------------------------------------------
  begin
    result = CodingAdventures::CliBuilder::Parser.new(LOGNAME_SPEC_FILE, ["logname"] + ARGV).parse
  rescue CodingAdventures::CliBuilder::ParseErrors => e
    e.errors.each { |err| warn "logname: #{err.message}" }
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
    name = get_login_name
    if name
      puts name
    else
      warn "logname: no login name"
      exit 1
    end
  end
end

# Only run main when this file is executed directly.
logname_main if __FILE__ == $PROGRAM_NAME
