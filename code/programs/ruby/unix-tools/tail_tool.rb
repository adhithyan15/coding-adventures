#!/usr/bin/env ruby
# frozen_string_literal: true

# tail_tool.rb -- Output the last part of files
# ===============================================
#
# === What This Program Does ===
#
# This is a reimplementation of the GNU `tail` utility. It prints the
# last 10 lines (by default) of each file to standard output.
#
# === The "+NUM" Convention ===
#
# Unlike most tools where a number means "how many from the end",
# tail supports a leading `+` to mean "start from line/byte NUM":
#
#     tail -n 5 file.txt     # last 5 lines
#     tail -n +3 file.txt    # everything from line 3 onward
#
# This is why the -n and -c flags are strings, not integers: we need
# to detect the leading `+` sign.
#
# === Headers ===
#
# When multiple files are given, tail prints a header before each:
#
#     ==> filename <==
#
# The -q flag suppresses headers. The -v flag forces headers even
# for a single file.
#
# === Follow Mode (-f) ===
#
# The -f flag causes tail to keep watching the file after reaching
# the end, printing new data as it's appended. This implementation
# does NOT implement -f (it's an interactive feature that doesn't
# make sense in a batch context), but we accept the flag for
# compatibility.

require "coding_adventures_cli_builder"

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

TAIL_SPEC_FILE = File.join(File.dirname(__FILE__), "tail.json")

# ---------------------------------------------------------------------------
# Business Logic: parse_count
# ---------------------------------------------------------------------------
# Parse a count string that may have a leading `+` sign.
#
# Returns: [Integer, Boolean] -- the count and whether it's from-start mode.
#
# Examples:
#   "10"  -> [10, false]   (last 10)
#   "+3"  -> [3, true]     (from line/byte 3)
#   "-5"  -> [5, false]    (last 5)

def parse_tail_count(str)
  if str.start_with?("+")
    [str[1..].to_i, true]
  elsif str.start_with?("-")
    [str[1..].to_i, false]
  else
    [str.to_i, false]
  end
end

# ---------------------------------------------------------------------------
# Business Logic: tail_lines
# ---------------------------------------------------------------------------
# Print lines from an IO stream according to tail semantics.
#
# If from_start is true, print everything from line `count` onward
# (1-indexed). Otherwise, print the last `count` lines.
#
# Parameters:
#   io         - An IO object to read from
#   count      - Number of lines
#   from_start - If true, start from line `count`; if false, last `count`
#   delimiter  - Line delimiter character

def tail_lines(io, count, from_start, delimiter)
  separator = delimiter == "\0" ? "\0" : "\n"

  if from_start
    # Print everything starting from line `count` (1-indexed).
    # So +1 means "from the first line" (all lines), +2 means "skip first line".
    line_num = 0
    io.each_line(separator) do |line|
      line_num += 1
      print line if line_num >= count
    end
  else
    # Collect all lines, then print the last `count`.
    # For a production tool, you'd use a circular buffer, but for
    # educational purposes, reading all lines is clearer.
    all_lines = io.each_line(separator).to_a
    start_idx = [all_lines.length - count, 0].max
    all_lines[start_idx..].each { |line| print line }
  end
end

# ---------------------------------------------------------------------------
# Business Logic: tail_bytes
# ---------------------------------------------------------------------------
# Print bytes from an IO stream according to tail semantics.
#
# If from_start is true, print everything from byte `count` onward
# (1-indexed). Otherwise, print the last `count` bytes.

def tail_bytes(io, count, from_start)
  content = io.read || ""

  if from_start
    # +1 means "from the first byte" (all content).
    start_idx = [count - 1, 0].max
    print content.byteslice(start_idx..)
  else
    start_idx = [content.bytesize - count, 0].max
    print content.byteslice(start_idx..)
  end
end

# ---------------------------------------------------------------------------
# Business Logic: print_header
# ---------------------------------------------------------------------------

def tail_print_header(filename, first)
  puts unless first
  puts "==> #{filename} <=="
end

# ---------------------------------------------------------------------------
# Main entry point
# ---------------------------------------------------------------------------

def tail_main
  # --- Step 1: Parse arguments ---------------------------------------------
  begin
    result = CodingAdventures::CliBuilder::Parser.new(TAIL_SPEC_FILE, ["tail"] + ARGV).parse
  rescue CodingAdventures::CliBuilder::ParseErrors => e
    e.errors.each { |err| warn "tail: #{err.message}" }
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
    files = result.arguments.fetch("files", ["-"])
    byte_mode = !result.flags["bytes"].nil? && result.flags["bytes"] != ""
    count_str = byte_mode ? result.flags["bytes"] : (result.flags["lines"] || "10")
    count, from_start = parse_tail_count(count_str.to_s)
    quiet = result.flags["quiet"]
    verbose = result.flags["verbose"]
    zero_terminated = result.flags["zero_terminated"]
    delimiter = zero_terminated ? "\0" : "\n"

    show_headers = if quiet
                     false
                   elsif verbose
                     true
                   else
                     files.length > 1
                   end

    files.each_with_index do |filename, idx|
      if filename == "-"
        tail_print_header("standard input", idx == 0) if show_headers
        if byte_mode
          tail_bytes($stdin, count, from_start)
        else
          tail_lines($stdin, count, from_start, delimiter)
        end
      else
        begin
          File.open(filename, "rb") do |f|
            tail_print_header(filename, idx == 0) if show_headers
            if byte_mode
              tail_bytes(f, count, from_start)
            else
              tail_lines(f, count, from_start, delimiter)
            end
          end
        rescue Errno::ENOENT
          warn "tail: cannot open '#{filename}' for reading: No such file or directory"
        rescue Errno::EACCES
          warn "tail: cannot open '#{filename}' for reading: Permission denied"
        rescue Errno::EISDIR
          warn "tail: error reading '#{filename}': Is a directory"
        end
      end
    end
  end
end

# Only run main when this file is executed directly.
tail_main if __FILE__ == $PROGRAM_NAME
