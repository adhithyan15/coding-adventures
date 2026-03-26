#!/usr/bin/env ruby
# frozen_string_literal: true

# sleep_tool.rb -- Delay for a specified amount of time
# ======================================================
#
# === What This Program Does ===
#
# This is a reimplementation of the GNU `sleep` utility. It pauses
# execution for a specified duration. Unlike POSIX sleep (which only
# accepts whole seconds), the GNU version supports fractional seconds
# and duration suffixes.
#
# === How sleep Works ===
#
#     $ sleep 5        # Sleep for 5 seconds
#     $ sleep 0.5      # Sleep for half a second
#     $ sleep 2m       # Sleep for 2 minutes
#     $ sleep 1h 30m   # Sleep for 1 hour and 30 minutes
#
# === Duration Suffixes ===
#
# Each duration argument can have an optional suffix:
#
#   Suffix  Meaning         Multiplier
#   ------  -------         ----------
#   s       seconds         1
#   m       minutes         60
#   h       hours           3600
#   d       days            86400
#
# No suffix means seconds (the default).
#
# === Multiple Arguments ===
#
# When multiple duration arguments are given, they are *summed*:
#
#     $ sleep 1m 30s    # = 60 + 30 = 90 seconds
#     $ sleep 1h 2m 3s  # = 3600 + 120 + 3 = 3723 seconds
#
# === Implementation ===
#
# We parse each duration string into seconds, sum them, and call
# `Kernel.sleep`. The parsing is done in `parse_duration` which
# handles the suffix multiplier. For testability, we accept a
# `sleep_func` parameter so tests can substitute a no-op.
#
# === How CLI Builder Powers This ===
#
# The JSON spec defines a required variadic "duration" argument.
# CLI Builder ensures at least one duration is provided.

require "coding_adventures_cli_builder"

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

SLEEP_SPEC_FILE = File.join(File.dirname(__FILE__), "sleep.json")

# ---------------------------------------------------------------------------
# Constants: suffix multipliers
# ---------------------------------------------------------------------------
# A hash mapping each supported suffix character to its multiplier
# (in seconds). This makes the parsing code clean and extensible.

SUFFIX_MULTIPLIERS = {
  "s" => 1,        # seconds
  "m" => 60,       # minutes
  "h" => 3600,     # hours
  "d" => 86_400    # days
}.freeze

# ---------------------------------------------------------------------------
# Business Logic: parse_duration
# ---------------------------------------------------------------------------
# Parse a single duration string into seconds.
#
# A duration string is a number (integer or float) optionally followed
# by a suffix character (s, m, h, d). If no suffix is given, the
# number is treated as seconds.
#
# Examples:
#   "5"    -> 5.0      (5 seconds)
#   "5s"   -> 5.0      (5 seconds, explicit)
#   "2m"   -> 120.0    (2 minutes)
#   "1.5h" -> 5400.0   (1.5 hours)
#   "1d"   -> 86400.0  (1 day)
#
# Raises ArgumentError if the string cannot be parsed.

def parse_duration(str)
  # Check if the string ends with a known suffix.
  if str =~ /\A([0-9]*\.?[0-9]+)([smhd])?\z/
    number = Float($1)
    suffix = $2 || "s"
    multiplier = SUFFIX_MULTIPLIERS[suffix]
    number * multiplier
  else
    raise ArgumentError, "sleep: invalid time interval '#{str}'"
  end
end

# ---------------------------------------------------------------------------
# Business Logic: total_sleep_seconds
# ---------------------------------------------------------------------------
# Parse multiple duration strings and return their sum in seconds.
#
# Parameters:
#   durations - An array of duration strings (e.g., ["1m", "30s"]).
#
# Returns: Float -- the total duration in seconds.

def total_sleep_seconds(durations)
  durations.sum { |d| parse_duration(d) }
end

# ---------------------------------------------------------------------------
# Business Logic: perform_sleep
# ---------------------------------------------------------------------------
# Sleep for the given number of seconds.
#
# Parameters:
#   seconds    - The number of seconds to sleep.
#   sleep_func - A callable that performs the sleep (default: Kernel.method(:sleep)).
#                This parameter exists for testability: tests can pass a
#                no-op lambda to avoid actually waiting.

def perform_sleep(seconds, sleep_func = Kernel.method(:sleep))
  sleep_func.call(seconds)
end

# ---------------------------------------------------------------------------
# Main entry point
# ---------------------------------------------------------------------------

def sleep_main
  # --- Step 1: Parse arguments ---------------------------------------------
  begin
    result = CodingAdventures::CliBuilder::Parser.new(SLEEP_SPEC_FILE, ["sleep"] + ARGV).parse
  rescue CodingAdventures::CliBuilder::ParseErrors => e
    e.errors.each { |err| warn "sleep: #{err.message}" }
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
    durations = result.arguments.fetch("duration", [])

    begin
      seconds = total_sleep_seconds(durations)
    rescue ArgumentError => e
      warn e.message
      exit 1
    end

    perform_sleep(seconds)
  end
end

# Only run main when this file is executed directly.
sleep_main if __FILE__ == $PROGRAM_NAME
