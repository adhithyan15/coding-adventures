#!/usr/bin/env ruby
# frozen_string_literal: true

# cut_tool.rb -- Remove sections from each line of files
# ========================================================
#
# === What This Program Does ===
#
# This is a reimplementation of the GNU `cut` utility. It selects
# portions of each line from files and writes them to standard output.
# You can select by byte position (-b), character position (-c), or
# field (-f).
#
# === Selection Modes ===
#
# Exactly one of these must be specified:
#
#   -b LIST   Select only these bytes
#   -c LIST   Select only these characters
#   -f LIST   Select only these fields (separated by delimiter)
#
# === Range Notation ===
#
# The LIST argument uses a flexible range notation:
#
#   N       Select only the Nth element (1-indexed)
#   N-M     Select elements N through M (inclusive)
#   N-      Select element N through the end of the line
#   -M      Select from the beginning through element M
#   N,M,P   Comma-separated list of any of the above
#
# Examples:
#   "1-3,5,7-"  → Select positions 1, 2, 3, 5, and 7 onwards
#   "1,3"       → Select positions 1 and 3 only
#   "-5"        → Select positions 1 through 5
#
# === Fields Mode (-f) ===
#
# In field mode, lines are split by a delimiter (tab by default).
# The -d flag changes the delimiter. The -s flag suppresses lines
# that don't contain the delimiter character.
#
# === Complement Mode ===
#
# The --complement flag inverts the selection: it outputs everything
# EXCEPT the selected positions.

require "coding_adventures_cli_builder"

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

CUT_SPEC_FILE = File.join(File.dirname(__FILE__), "cut.json")

# ---------------------------------------------------------------------------
# Business Logic: parse_range_list
# ---------------------------------------------------------------------------
# Parse a range specification string into a set of 1-indexed positions.
# The max_len parameter caps open-ended ranges like "5-" or "-3".
#
# Range notation:
#   "1"     → [1]
#   "1-3"   → [1, 2, 3]
#   "5-"    → [5, 6, ..., max_len]
#   "-3"    → [1, 2, 3]
#   "1,3,5" → [1, 3, 5]
#
# Parameters:
#   list_str - The range specification string (e.g., "1-3,5,7-")
#   max_len  - The maximum position (length of the line/field array)
#
# Returns: A sorted array of unique 1-indexed positions.

def parse_range_list(list_str, max_len)
  positions = []

  list_str.split(",").each do |part|
    part = part.strip

    if part.include?("-")
      # It's a range: "N-M", "N-", or "-M"
      sides = part.split("-", -1)
      start_pos = sides[0].empty? ? 1 : sides[0].to_i
      end_pos = sides[1].empty? ? max_len : sides[1].to_i

      # Clamp to valid range
      start_pos = [start_pos, 1].max
      end_pos = [end_pos, max_len].min

      (start_pos..end_pos).each { |p| positions << p }
    else
      # Single position
      pos = part.to_i
      positions << pos if pos >= 1 && pos <= max_len
    end
  end

  positions.uniq.sort
end

# ---------------------------------------------------------------------------
# Business Logic: cut_line_by_positions
# ---------------------------------------------------------------------------
# Cut a line by byte or character positions. In Ruby, bytes and
# characters differ for multi-byte encodings (UTF-8). For -b mode,
# we work with raw bytes; for -c mode, with characters.
#
# Parameters:
#   line       - The input line (without newline)
#   list_str   - The range specification (e.g., "1-3,5")
#   mode       - :bytes or :chars
#   complement - Whether to complement the selection
#   output_delim - Delimiter between selected ranges (nil = use original)
#
# Returns: The cut portion of the line.

def cut_line_by_positions(line, list_str, mode, complement, output_delim)
  elements = if mode == :bytes
               line.bytes.to_a
             else
               line.chars.to_a
             end

  max_len = elements.length
  selected_positions = parse_range_list(list_str, max_len)

  if complement
    all_positions = (1..max_len).to_a
    selected_positions = all_positions - selected_positions
  end

  selected = selected_positions.map { |p| elements[p - 1] }.compact

  if mode == :bytes
    selected.pack("C*").force_encoding(line.encoding)
  else
    if output_delim
      selected.join(output_delim)
    else
      selected.join
    end
  end
end

# ---------------------------------------------------------------------------
# Business Logic: cut_line_by_fields
# ---------------------------------------------------------------------------
# Cut a line by field positions. Fields are separated by a delimiter
# character (default: tab).
#
# Parameters:
#   line          - The input line (without newline)
#   list_str      - The range specification (e.g., "1,3")
#   delimiter     - The field delimiter character
#   complement    - Whether to complement the selection
#   only_delimited - If true, skip lines without the delimiter
#   output_delim  - Output delimiter (default: same as input)
#
# Returns: The cut portion, or nil if the line should be suppressed.

def cut_line_by_fields(line, list_str, delimiter, complement, only_delimited, output_delim)
  # If the line doesn't contain the delimiter and -s is set, suppress it
  unless line.include?(delimiter)
    return nil if only_delimited
    return line
  end

  fields = line.split(delimiter, -1)
  max_len = fields.length
  selected_positions = parse_range_list(list_str, max_len)

  if complement
    all_positions = (1..max_len).to_a
    selected_positions = all_positions - selected_positions
  end

  selected = selected_positions.map { |p| fields[p - 1] }.compact
  out_delim = output_delim || delimiter
  selected.join(out_delim)
end

# ---------------------------------------------------------------------------
# Business Logic: cut_line
# ---------------------------------------------------------------------------
# Dispatch to the appropriate cut function based on the mode.
#
# Parameters:
#   line  - The input line (without newline)
#   flags - Hash of flag values from CLI Builder
#
# Returns: The cut result, or nil to suppress the line.

def cut_line(line, flags)
  complement = flags["complement"] || false
  output_delim = flags["output_delimiter"]

  if flags["bytes"]
    cut_line_by_positions(line, flags["bytes"], :bytes, complement, output_delim)
  elsif flags["characters"]
    cut_line_by_positions(line, flags["characters"], :chars, complement, output_delim)
  elsif flags["fields"]
    delimiter = flags["delimiter"] || "\t"
    only_delimited = flags["only_delimited"] || false
    cut_line_by_fields(line, flags["fields"], delimiter, complement, only_delimited, output_delim)
  else
    # Should not happen (mutually exclusive group requires one)
    line
  end
end

# ---------------------------------------------------------------------------
# Main entry point
# ---------------------------------------------------------------------------

def cut_main
  # --- Step 1: Parse arguments ---------------------------------------------
  begin
    result = CodingAdventures::CliBuilder::Parser.new(CUT_SPEC_FILE, ["cut"] + ARGV).parse
  rescue CodingAdventures::CliBuilder::ParseErrors => e
    e.errors.each { |err| warn "cut: #{err.message}" }
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
    delimiter = result.flags["zero_terminated"] ? "\0" : "\n"

    files.each do |filename|
      io = if filename == "-"
             $stdin
           else
             begin
               File.open(filename, "r")
             rescue Errno::ENOENT
               warn "cut: #{filename}: No such file or directory"
               next
             end
           end

      io.each_line(delimiter) do |raw_line|
        line = raw_line.chomp(delimiter)
        output = cut_line(line, result.flags)
        puts output unless output.nil?
      end

      io.close if io != $stdin
    end
  end
end

# Only run main when this file is executed directly.
cut_main if __FILE__ == $PROGRAM_NAME
