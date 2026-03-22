#!/usr/bin/env ruby
# frozen_string_literal: true

# tee_tool.rb -- Read from standard input and write to standard output and files
# ===============================================================================
#
# === What This Program Does ===
#
# This is a reimplementation of the GNU `tee` utility. It reads from
# standard input and writes to both standard output AND one or more
# files simultaneously. Think of it as a T-junction for data: the
# input flows through to stdout AND gets copied to each file.
#
# === Why tee Is Useful ===
#
# In a pipeline, you sometimes want to save intermediate results
# while still passing data downstream:
#
#     $ generate_data | tee raw_data.log | process_data > result.txt
#
# Here, `tee` saves a copy of the raw data to `raw_data.log` while
# simultaneously passing it to `process_data`.
#
# === Flags ===
#
#   -a   Append to files instead of overwriting them. Without this flag,
#        tee truncates each output file before writing.
#
#   -i   Ignore the SIGINT signal. This is useful in pipelines where
#        you want tee to keep running even if the user presses Ctrl+C.
#
# === How tee Reads ===
#
# tee reads from stdin in chunks (not line by line) for efficiency.
# Each chunk is written to stdout and to every output file. If a
# write to a file fails, tee prints an error but continues processing.

require "coding_adventures_cli_builder"

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

TEE_SPEC_FILE = File.join(File.dirname(__FILE__), "tee.json")

# ---------------------------------------------------------------------------
# Business Logic: tee_stream
# ---------------------------------------------------------------------------
# Read from an input stream and write to stdout plus a list of file IO objects.
#
# We read in chunks for efficiency. Each chunk is written to all outputs.
# If writing to a file fails, we log the error but continue.
#
# Parameters:
#   input   - The input IO stream (typically $stdin)
#   outputs - Array of IO objects to write to (files opened by the caller)

def tee_stream(input, outputs)
  while (chunk = input.read(8192))
    # Always write to stdout.
    $stdout.write(chunk)
    $stdout.flush

    # Write to each file.
    outputs.each do |out|
      begin
        out.write(chunk)
        out.flush
      rescue IOError, Errno::ENOSPC => e
        warn "tee: #{e.message}"
      end
    end
  end
end

# ---------------------------------------------------------------------------
# Main entry point
# ---------------------------------------------------------------------------

def tee_main
  # --- Step 1: Parse arguments ---------------------------------------------
  begin
    result = CodingAdventures::CliBuilder::Parser.new(TEE_SPEC_FILE, ["tee"] + ARGV).parse
  rescue CodingAdventures::CliBuilder::ParseErrors => e
    e.errors.each { |err| warn "tee: #{err.message}" }
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
    files = result.arguments.fetch("files", [])
    append_mode = result.flags["append"]
    ignore_interrupts = result.flags["ignore_interrupts"]

    # If -i is set, ignore SIGINT (Ctrl+C).
    Signal.trap("INT", "IGNORE") if ignore_interrupts

    # Open all output files.
    mode = append_mode ? "a" : "w"
    file_ios = []

    files.each do |filename|
      begin
        file_ios << File.open(filename, mode)
      rescue Errno::ENOENT
        warn "tee: #{filename}: No such file or directory"
      rescue Errno::EACCES
        warn "tee: #{filename}: Permission denied"
      end
    end

    # Read from stdin and write to stdout + all files.
    begin
      tee_stream($stdin, file_ios)
    ensure
      # Always close the files, even if an error occurs.
      file_ios.each(&:close)
    end
  end
end

# Only run main when this file is executed directly.
tee_main if __FILE__ == $PROGRAM_NAME
