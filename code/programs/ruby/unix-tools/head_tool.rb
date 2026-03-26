#!/usr/bin/env ruby
# frozen_string_literal: true

# head_tool.rb -- Output the first part of files
# ================================================
#
# === What This Program Does ===
#
# This is a reimplementation of the GNU `head` utility. It prints the
# first 10 lines (by default) of each file to standard output. When
# multiple files are given, each is preceded by a header giving the
# file name.
#
# === How head Selects Output ===
#
# By default, head prints the first 10 lines. Two flags change this:
#
#   -n NUM   Print the first NUM lines instead of 10.
#   -c NUM   Print the first NUM bytes instead of lines.
#
# These two flags are mutually exclusive -- you can't ask for both
# a line count and a byte count at the same time.
#
# === Headers ===
#
# When multiple files are given, head prints a header before each:
#
#     ==> filename <==
#
# The -q flag suppresses headers entirely. The -v flag forces headers
# even for a single file. These are mutually exclusive.
#
# === Zero-Terminated Mode ===
#
# With -z, the line delimiter changes from newline (\n) to NUL (\0).
# This is useful when processing filenames or other data that may
# contain newlines.

require "coding_adventures_cli_builder"

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

HEAD_SPEC_FILE = File.join(File.dirname(__FILE__), "head.json")

# ---------------------------------------------------------------------------
# Business Logic: head_lines
# ---------------------------------------------------------------------------
# Print the first `count` lines from an IO stream.
#
# We read the stream line by line and print until we've output the
# requested number of lines. The delimiter is either newline or NUL,
# controlled by the zero_terminated flag.
#
# Parameters:
#   io        - An IO object to read from
#   count     - Number of lines to output
#   delimiter - The line delimiter character ("\n" or "\0")

def head_lines(io, count, delimiter)
  lines_printed = 0
  separator = delimiter == "\0" ? "\0" : "\n"

  io.each_line(separator) do |line|
    break if lines_printed >= count
    print line
    lines_printed += 1
  end
end

# ---------------------------------------------------------------------------
# Business Logic: head_bytes
# ---------------------------------------------------------------------------
# Print the first `count` bytes from an IO stream.
#
# We read in chunks for efficiency, but cap the total output at the
# requested byte count.
#
# Parameters:
#   io    - An IO object to read from
#   count - Number of bytes to output

def head_bytes(io, count)
  remaining = count
  while remaining > 0
    chunk = io.read([remaining, 8192].min)
    break unless chunk
    print chunk
    remaining -= chunk.bytesize
  end
end

# ---------------------------------------------------------------------------
# Business Logic: print_header
# ---------------------------------------------------------------------------
# Print the "==> filename <==" header that head uses for multi-file output.
#
# Parameters:
#   filename - The name to display in the header
#   first    - Whether this is the first file (no leading blank line)

def head_print_header(filename, first)
  puts unless first
  puts "==> #{filename} <=="
end

# ---------------------------------------------------------------------------
# Main entry point
# ---------------------------------------------------------------------------

def head_main
  # --- Step 1: Parse arguments ---------------------------------------------
  begin
    result = CodingAdventures::CliBuilder::Parser.new(HEAD_SPEC_FILE, ["head"] + ARGV).parse
  rescue CodingAdventures::CliBuilder::ParseErrors => e
    e.errors.each { |err| warn "head: #{err.message}" }
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
    byte_mode = !result.flags["bytes"].nil? && result.flags["bytes"] != 0
    count = byte_mode ? result.flags["bytes"] : (result.flags["lines"] || 10)
    quiet = result.flags["quiet"]
    verbose = result.flags["verbose"]
    zero_terminated = result.flags["zero_terminated"]
    delimiter = zero_terminated ? "\0" : "\n"

    # Determine whether to show headers. By default, headers are shown
    # only when there are multiple files. -v forces them on, -q forces
    # them off.
    show_headers = if quiet
                     false
                   elsif verbose
                     true
                   else
                     files.length > 1
                   end

    files.each_with_index do |filename, idx|
      if filename == "-"
        head_print_header("standard input", idx == 0) if show_headers
        if byte_mode
          head_bytes($stdin, count)
        else
          head_lines($stdin, count, delimiter)
        end
      else
        begin
          File.open(filename, "rb") do |f|
            head_print_header(filename, idx == 0) if show_headers
            if byte_mode
              head_bytes(f, count)
            else
              head_lines(f, count, delimiter)
            end
          end
        rescue Errno::ENOENT
          warn "head: cannot open '#{filename}' for reading: No such file or directory"
        rescue Errno::EACCES
          warn "head: cannot open '#{filename}' for reading: Permission denied"
        rescue Errno::EISDIR
          warn "head: error reading '#{filename}': Is a directory"
        end
      end
    end
  end
end

# Only run main when this file is executed directly.
head_main if __FILE__ == $PROGRAM_NAME
