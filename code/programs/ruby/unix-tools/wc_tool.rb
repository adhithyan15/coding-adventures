#!/usr/bin/env ruby
# frozen_string_literal: true

# wc_tool.rb -- Print newline, word, and byte counts for each file
# =================================================================
#
# === What This Program Does ===
#
# This is a reimplementation of the GNU `wc` (word count) utility. It
# reads files (or standard input) and prints counts of lines, words,
# and bytes for each file. When multiple files are given, it also
# prints a "total" line at the end.
#
# === Default Behavior ===
#
# With no flags, `wc` prints three counts for each file:
#
#     $ wc file.txt
#       10   42  280 file.txt
#
# That's lines, words, bytes -- in that order. Each count is
# right-justified so that columns align when multiple files are shown.
#
# === Flags ===
#
#   -l    Print only the line count. A "line" is defined by the number
#         of newline characters (\n) in the file. A file with no
#         trailing newline counts one fewer line than you might expect.
#
#   -w    Print only the word count. A "word" is any sequence of
#         non-whitespace characters. This matches the POSIX definition.
#
#   -c    Print only the byte count. This counts raw bytes, not
#         characters. For ASCII files, bytes == characters; for UTF-8,
#         a single character may be multiple bytes.
#
#   -m    Print only the character count. This counts Unicode characters
#         (code points), which may differ from the byte count for
#         multi-byte encodings like UTF-8.
#
#   -L    Print the length of the longest line. This measures display
#         width: the number of characters in the longest line, not
#         counting the newline character.
#
# === Output Format ===
#
# Counts are right-justified. The field width is determined by the
# largest count across all files, with a minimum width of 1. This
# ensures columns align properly. A single space separates each field,
# and the filename (if any) appears after a space at the end.
#
# When reading from stdin (no filename), the filename field is omitted
# or shows empty depending on context.
#
# === Multiple Files ===
#
# When two or more files are counted, a "total" line is appended:
#
#     $ wc a.txt b.txt
#       10   42  280 a.txt
#       20   84  560 b.txt
#       30  126  840 total

require "coding_adventures_cli_builder"

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

WC_SPEC_FILE = File.join(File.dirname(__FILE__), "wc.json")

# ---------------------------------------------------------------------------
# Data class: FileCounts
# ---------------------------------------------------------------------------
# A simple struct to hold the counts for a single file/stream.
# Using a Struct keeps the code clean and self-documenting.

FileCounts = Struct.new(:lines, :words, :bytes, :chars, :max_line_length, :name)

# ---------------------------------------------------------------------------
# Business Logic: count_stream
# ---------------------------------------------------------------------------
# Read an IO stream and return a FileCounts with all metrics.
#
# We read the entire content into memory. For a production `wc`, you'd
# want to stream line by line to handle huge files, but for this
# educational implementation, simplicity wins.
#
# Parameters:
#   io   - An IO object to read from
#   name - A display name for this stream (filename or nil for stdin)
#
# Returns: A FileCounts struct with all five metrics populated.

def count_stream(io, name)
  content = io.read || ""

  # --- Line count ----------------------------------------------------------
  # Count the number of newline characters. This matches POSIX `wc -l`:
  # a file with "hello\nworld" has 1 line, not 2, because there's only
  # one newline. A file ending with a newline ("hello\nworld\n") has 2.
  line_count = content.count("\n")

  # --- Word count ----------------------------------------------------------
  # Split on whitespace to count words. Ruby's `split` with no arguments
  # splits on any whitespace (spaces, tabs, newlines) and ignores leading
  # and trailing whitespace. This matches POSIX word counting.
  word_count = content.split.length

  # --- Byte count ----------------------------------------------------------
  # The byte count is simply the length of the raw byte string.
  byte_count = content.bytesize

  # --- Character count -----------------------------------------------------
  # The character count uses Ruby's `length`, which counts Unicode
  # code points in the string's encoding. For UTF-8 content, this may
  # differ from bytesize (e.g., "é" is 1 character but 2 bytes).
  char_count = content.length

  # --- Maximum line length -------------------------------------------------
  # Split on newlines and find the longest line. We measure character
  # count (display width), not byte count. The newline itself is not
  # counted as part of the line length.
  max_len = 0
  content.each_line do |raw_line|
    line = raw_line.chomp
    max_len = line.length if line.length > max_len
  end

  FileCounts.new(line_count, word_count, byte_count, char_count, max_len, name)
end

# ---------------------------------------------------------------------------
# Business Logic: format_counts
# ---------------------------------------------------------------------------
# Format one FileCounts as a string for output.
#
# The flags hash determines which counts to display. If no count flags
# are set, the default is lines + words + bytes (matching GNU wc).
#
# Each count is right-justified to the given width, and counts are
# separated by a single space. The filename (if present) appears at
# the end.
#
# Parameters:
#   counts - A FileCounts struct
#   flags  - Hash of flag values from CLI Builder
#   width  - Field width for right-justification
#
# Returns: A formatted string (without trailing newline).

def format_counts(counts, flags, width)
  # Determine which columns to show. If no specific flag is set,
  # show the default triple: lines, words, bytes.
  show_lines    = flags["lines"]
  show_words    = flags["words"]
  show_bytes    = flags["bytes"]
  show_chars    = flags["chars"]
  show_max_len  = flags["max_line_length"]

  default_mode = !show_lines && !show_words && !show_bytes && !show_chars && !show_max_len

  parts = []

  # Build the output columns in the canonical order:
  # lines, words, bytes/chars, max-line-length
  if show_lines || default_mode
    parts << format("%#{width}d", counts.lines)
  end
  if show_words || default_mode
    parts << format("%#{width}d", counts.words)
  end
  if show_bytes || default_mode
    parts << format("%#{width}d", counts.bytes)
  end
  if show_chars
    parts << format("%#{width}d", counts.chars)
  end
  if show_max_len
    parts << format("%#{width}d", counts.max_line_length)
  end

  # Append the filename if present.
  line = parts.join(" ")
  line = "#{line} #{counts.name}" if counts.name
  line
end

# ---------------------------------------------------------------------------
# Business Logic: compute_width
# ---------------------------------------------------------------------------
# Determine the field width for right-justified output.
#
# We look at all counts across all files and find the maximum value.
# The field width is the number of digits in that maximum, with a
# minimum of 1 (so single-digit counts still display properly).

def compute_width(all_counts, flags)
  show_lines    = flags["lines"]
  show_words    = flags["words"]
  show_bytes    = flags["bytes"]
  show_chars    = flags["chars"]
  show_max_len  = flags["max_line_length"]

  default_mode = !show_lines && !show_words && !show_bytes && !show_chars && !show_max_len

  max_val = 0

  all_counts.each do |counts|
    max_val = [max_val, counts.lines].max if show_lines || default_mode
    max_val = [max_val, counts.words].max if show_words || default_mode
    max_val = [max_val, counts.bytes].max if show_bytes || default_mode
    max_val = [max_val, counts.chars].max if show_chars
    max_val = [max_val, counts.max_line_length].max if show_max_len
  end

  [max_val.to_s.length, 1].max
end

# ---------------------------------------------------------------------------
# Main entry point
# ---------------------------------------------------------------------------

def wc_main
  # --- Step 1: Parse arguments ---------------------------------------------
  begin
    result = CodingAdventures::CliBuilder::Parser.new(WC_SPEC_FILE, ["wc"] + ARGV).parse
  rescue CodingAdventures::CliBuilder::ParseErrors => e
    e.errors.each { |err| warn "wc: #{err.message}" }
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
    all_counts = []

    # Count each file/stream.
    files.each do |filename|
      if filename == "-"
        counts = count_stream($stdin, nil)
        all_counts << counts
      else
        begin
          File.open(filename, "r") do |f|
            counts = count_stream(f, filename)
            all_counts << counts
          end
        rescue Errno::ENOENT
          warn "wc: #{filename}: No such file or directory"
        rescue Errno::EACCES
          warn "wc: #{filename}: Permission denied"
        rescue Errno::EISDIR
          warn "wc: #{filename}: Is a directory"
        end
      end
    end

    # If multiple files were counted, compute a total row.
    if all_counts.length > 1
      total = FileCounts.new(
        all_counts.sum(&:lines),
        all_counts.sum(&:words),
        all_counts.sum(&:bytes),
        all_counts.sum(&:chars),
        all_counts.map(&:max_line_length).max || 0,
        "total"
      )
      all_counts << total
    end

    # Compute the field width based on all counts (including total).
    width = compute_width(all_counts, result.flags)

    # Print each line.
    all_counts.each do |counts|
      puts format_counts(counts, result.flags, width)
    end
  end
end

# Only run main when this file is executed directly.
wc_main if __FILE__ == $PROGRAM_NAME
