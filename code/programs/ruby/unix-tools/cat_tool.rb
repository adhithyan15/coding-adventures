#!/usr/bin/env ruby
# frozen_string_literal: true

# cat_tool.rb -- Concatenate files and print on standard output
# ==============================================================
#
# === What This Program Does ===
#
# This is a reimplementation of the GNU `cat` utility. It reads files
# sequentially and writes their contents to standard output. When no
# file is given, or when the filename is `-`, it reads from standard
# input.
#
# === Why "cat"? ===
#
# The name `cat` is short for "concatenate". The original purpose was
# to concatenate multiple files together:
#
#     $ cat header.txt body.txt footer.txt > document.txt
#
# But in practice, `cat` is used far more often to display a single
# file's contents or to pipe stdin through a processing pipeline.
#
# === Flags ===
#
#   -n    Number all output lines. Lines are right-justified in a 6-wide
#         field, followed by a tab and the line content.
#
#   -b    Number only non-blank lines. This overrides -n if both are given.
#         The line counter still increments only for non-blank lines.
#
#   -s    Squeeze repeated blank lines. Multiple consecutive blank lines
#         are collapsed into a single blank line.
#
#   -T    Show tabs as ^I. This makes invisible tab characters visible
#         without affecting other whitespace.
#
#   -E    Show ends. Append a $ character at the end of each line, making
#         trailing whitespace visible.
#
#   -v    Show non-printing characters using ^ and M- notation. Control
#         characters (0x00-0x1F, except tab and newline) are shown as
#         ^@ through ^_. DEL (0x7F) is shown as ^?. High bytes (0x80-0xFF)
#         are shown as M- followed by the equivalent low-byte notation.
#
#   -A    Show all. Equivalent to -vET (show non-printing, show ends,
#         show tabs). This is the "debugging" mode for inspecting files.
#
# === Line Numbering Format ===
#
# Line numbers are formatted as "%6d\t" -- a 6-character right-justified
# integer followed by a tab. This matches GNU cat exactly:
#
#     $ cat -n file.txt
#          1	first line
#          2	second line
#          3	third line

require "coding_adventures_cli_builder"

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

CAT_SPEC_FILE = File.join(File.dirname(__FILE__), "cat.json")

# ---------------------------------------------------------------------------
# Business Logic: process_nonprinting
# ---------------------------------------------------------------------------
# Convert non-printing characters to ^ and M- notation.
#
# This function processes a string byte by byte and replaces non-printing
# characters with their visible representations:
#
#   - 0x00-0x08, 0x0A-0x1F: ^@ through ^_ (control characters)
#   - 0x09 (tab): left as-is (handled separately by -T flag)
#   - 0x7F (DEL): ^?
#   - 0x80-0xFF: M- prefix followed by the equivalent 0x00-0x7F notation
#
# Note: newlines (0x0A) are NOT converted here because they are line
# delimiters, not content. The caller has already split on newlines.

def process_nonprinting(char)
  byte = char.b.ord

  if byte < 32
    # Control characters: ^@ through ^_ (except tab, which we leave)
    if byte == 9 # tab
      char
    else
      "^" + (byte + 64).chr
    end
  elsif byte == 127
    # DEL character
    "^?"
  elsif byte >= 128
    # High bytes: M- notation
    if byte < 128 + 32
      "M-^" + (byte - 128 + 64).chr
    elsif byte == 128 + 127
      "M-^?"
    else
      "M-" + (byte - 128).chr
    end
  else
    char
  end
end

# ---------------------------------------------------------------------------
# Business Logic: cat_stream
# ---------------------------------------------------------------------------
# Process a single IO stream (file or stdin) and write to stdout.
#
# This function implements all the line-processing logic for cat:
# numbering, squeezing, tab display, end-of-line markers, and
# non-printing character visualization.
#
# Parameters:
#   io      - An IO object to read from
#   flags   - Hash of flag values from CLI Builder
#   line_no - Current line number (for -n/-b numbering across files)
#
# Returns: The updated line number (so numbering continues across files).

def cat_stream(io, flags, line_no)
  # Resolve the -A shorthand: -A is equivalent to -vET.
  show_all      = flags["show_all"]
  show_nonprint = flags["show_nonprinting"] || show_all
  show_ends     = flags["show_ends"] || show_all
  show_tabs     = flags["show_tabs"] || show_all
  number        = flags["number"]
  number_nb     = flags["number_nonblank"]
  squeeze       = flags["squeeze_blank"]

  # Track whether the previous line was blank, for -s (squeeze) support.
  prev_blank = false

  # Read the stream line by line. We use `each_line` which preserves
  # the trailing newline on each line (except possibly the last line
  # of the file if it doesn't end with a newline).
  io.each_line do |raw_line|
    # Strip the trailing newline/carriage-return for processing.
    # We'll add appropriate line endings back when we output.
    line = raw_line.chomp

    # --- Squeeze blank lines (-s) ----------------------------------------
    # If this line is blank and the previous line was also blank, skip it.
    is_blank = line.empty?
    if squeeze && is_blank && prev_blank
      next
    end
    prev_blank = is_blank

    # --- Process non-printing characters (-v) ----------------------------
    if show_nonprint
      line = line.chars.map { |c| process_nonprinting(c) }.join
    end

    # --- Show tabs (-T) --------------------------------------------------
    if show_tabs
      line = line.gsub("\t", "^I")
    end

    # --- Show ends (-E) --------------------------------------------------
    if show_ends
      line = line + "$"
    end

    # --- Line numbering (-n / -b) ----------------------------------------
    # -b overrides -n: with -b, only non-blank lines get numbers.
    if number_nb
      if !is_blank
        print format("%6d\t", line_no)
        line_no += 1
      end
    elsif number
      print format("%6d\t", line_no)
      line_no += 1
    end

    # --- Output the processed line ---------------------------------------
    puts line
  end

  line_no
end

# ---------------------------------------------------------------------------
# Main entry point
# ---------------------------------------------------------------------------

def cat_main
  # --- Step 1: Parse arguments ---------------------------------------------
  begin
    result = CodingAdventures::CliBuilder::Parser.new(CAT_SPEC_FILE, ["cat"] + ARGV).parse
  rescue CodingAdventures::CliBuilder::ParseErrors => e
    e.errors.each { |err| warn "cat: #{err.message}" }
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
    line_no = 1

    files.each do |filename|
      if filename == "-"
        # Read from standard input.
        line_no = cat_stream($stdin, result.flags, line_no)
      else
        # Read from a file. If the file doesn't exist or can't be read,
        # print an error to stderr and continue with the next file
        # (matching GNU cat behavior).
        begin
          File.open(filename, "r") do |f|
            line_no = cat_stream(f, result.flags, line_no)
          end
        rescue Errno::ENOENT
          warn "cat: #{filename}: No such file or directory"
        rescue Errno::EACCES
          warn "cat: #{filename}: Permission denied"
        rescue Errno::EISDIR
          warn "cat: #{filename}: Is a directory"
        end
      end
    end
  end
end

# Only run main when this file is executed directly.
cat_main if __FILE__ == $PROGRAM_NAME
