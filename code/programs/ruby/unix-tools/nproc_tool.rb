#!/usr/bin/env ruby
# frozen_string_literal: true

# nproc_tool.rb -- Print the number of processing units available
# ================================================================
#
# === What This Program Does ===
#
# This is a reimplementation of the GNU `nproc` utility. It prints the
# number of processing units (CPU cores) available to the current process.
#
# === How nproc Works ===
#
#     $ nproc
#     8
#
# On a system with 8 CPU cores, nproc prints 8. Simple!
#
# === Available vs Installed ===
#
# Modern systems can restrict which CPUs a process can use (via cgroups,
# taskset, or similar mechanisms). By default, nproc reports only the
# CPUs *available* to the current process. The `--all` flag overrides
# this and reports the total number of *installed* processors.
#
# In practice, on most desktop systems, these numbers are the same.
# The distinction matters on servers where processes may be confined
# to a subset of CPUs for resource management.
#
# === The --ignore Flag ===
#
# The `--ignore N` flag subtracts N from the count, with a minimum
# result of 1. This is useful for leaving some CPUs free:
#
#     $ nproc --ignore 2   # On an 8-core system, prints 6
#
# Use case: a build system might use `nproc --ignore 1` to leave one
# core free for the rest of the system, keeping the machine responsive
# during long builds.
#
# === Implementation ===
#
# Ruby's `Etc.nprocessors` returns the number of online processors.
# This is the same value that the C library's `sysconf(_SC_NPROCESSORS_ONLN)`
# returns on Linux, or `sysctl hw.logicalcpu` on macOS.
#
# Note: Ruby's Etc.nprocessors does not distinguish between "available"
# and "installed" in the way GNU nproc does (via sched_getaffinity).
# On most systems, they are the same. We use Etc.nprocessors for both
# modes in this educational implementation.
#
# === How CLI Builder Powers This ===
#
# The JSON spec defines two flags: `--all` (boolean) and `--ignore N`
# (integer). CLI Builder handles parsing and validation.

require "etc"
require "coding_adventures_cli_builder"

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

NPROC_SPEC_FILE = File.join(File.dirname(__FILE__), "nproc.json")

# ---------------------------------------------------------------------------
# Business Logic: get_nproc
# ---------------------------------------------------------------------------
# Return the number of processors, optionally subtracting `ignore`.
#
# Parameters:
#   all    - If true, report all installed processors (vs available).
#            In this implementation, both use Etc.nprocessors.
#   ignore - Number of processors to subtract (default: 0).
#
# Returns: Integer >= 1.
#
# The result is always at least 1 because a system with zero usable
# processors makes no sense -- you need at least one to run the program
# that is calling nproc!

def get_nproc(all: false, ignore: 0)
  # Get the processor count. In this implementation, --all and the
  # default both use Etc.nprocessors. A production implementation
  # would use sched_getaffinity for the default and sysconf for --all.
  count = Etc.nprocessors

  # Subtract the ignore count, but never go below 1.
  result = count - ignore
  [result, 1].max
end

# ---------------------------------------------------------------------------
# Main entry point
# ---------------------------------------------------------------------------

def nproc_main
  # --- Step 1: Parse arguments ---------------------------------------------
  begin
    result = CodingAdventures::CliBuilder::Parser.new(NPROC_SPEC_FILE, ["nproc"] + ARGV).parse
  rescue CodingAdventures::CliBuilder::ParseErrors => e
    e.errors.each { |err| warn "nproc: #{err.message}" }
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
    all_flag = result.flags["all"] || false
    ignore_count = result.flags["ignore"] || 0

    puts get_nproc(all: all_flag, ignore: ignore_count)
  end
end

# Only run main when this file is executed directly.
nproc_main if __FILE__ == $PROGRAM_NAME
