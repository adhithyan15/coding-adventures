#!/usr/bin/env ruby
# frozen_string_literal: true

# comm_tool.rb -- Compare two sorted files line by line
# =======================================================
#
# === What This Program Does ===
#
# This is a reimplementation of the GNU `comm` utility. It compares
# two sorted files and produces three-column output:
#
#   Column 1: Lines unique to FILE1
#   Column 2: Lines unique to FILE2
#   Column 3: Lines common to both files
#
# === Three-Column Output ===
#
# The default output uses tabs to separate columns:
#
#     $ comm file1 file2
#     alpha
#             beta
#                     common_line
#     delta
#
# Lines unique to file1 appear with no leading tabs. Lines unique to
# file2 appear with one leading tab. Common lines appear with two
# leading tabs. This creates a visual three-column layout.
#
# === Suppressing Columns ===
#
#   -1    Suppress column 1 (lines unique to FILE1)
#   -2    Suppress column 2 (lines unique to FILE2)
#   -3    Suppress column 3 (common lines)
#
# These can be combined: -12 shows only common lines, -23 shows only
# lines unique to FILE1, etc.
#
# === Algorithm ===
#
# Since both files must be sorted, we use a merge-like algorithm:
# advance through both files simultaneously, comparing current lines.
# If file1's line is smaller, it's unique to file1. If file2's line
# is smaller, it's unique to file2. If they're equal, it's common.
#
# This is the same merge step used in merge sort, running in O(n+m)
# time where n and m are the lengths of the two files.

require "coding_adventures_cli_builder"

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

COMM_SPEC_FILE = File.join(File.dirname(__FILE__), "comm.json")

# ---------------------------------------------------------------------------
# Business Logic: compare_sorted
# ---------------------------------------------------------------------------
# Compare two sorted arrays of lines and produce three-column output.
#
# The algorithm uses two pointers (i, j) advancing through lines1 and
# lines2. At each step:
#   - If lines1[i] < lines2[j]: line is unique to file1 (column 1)
#   - If lines1[i] > lines2[j]: line is unique to file2 (column 2)
#   - If lines1[i] == lines2[j]: line is common (column 3)
#
# Parameters:
#   lines1     - Array of lines from file1 (sorted)
#   lines2     - Array of lines from file2 (sorted)
#   suppress   - Array of booleans [sup1, sup2, sup3] for column suppression
#   output_sep - The column separator (default: "\t")
#
# Returns: An array of formatted output lines.

def compare_sorted(lines1, lines2, suppress, output_sep)
  sup1, sup2, sup3 = suppress
  output = []
  i = 0
  j = 0

  # --- Merge pass ---------------------------------------------------------
  # Walk both arrays simultaneously, comparing current elements.
  # This is the core merge step from merge sort.
  while i < lines1.length && j < lines2.length
    cmp = lines1[i] <=> lines2[j]

    if cmp < 0
      # Line is unique to file1 (column 1)
      unless sup1
        prefix = build_comm_prefix(1, suppress, output_sep)
        output << "#{prefix}#{lines1[i]}"
      end
      i += 1
    elsif cmp > 0
      # Line is unique to file2 (column 2)
      unless sup2
        prefix = build_comm_prefix(2, suppress, output_sep)
        output << "#{prefix}#{lines2[j]}"
      end
      j += 1
    else
      # Line is common to both (column 3)
      unless sup3
        prefix = build_comm_prefix(3, suppress, output_sep)
        output << "#{prefix}#{lines1[i]}"
      end
      i += 1
      j += 1
    end
  end

  # --- Remaining lines from file1 -----------------------------------------
  while i < lines1.length
    unless sup1
      prefix = build_comm_prefix(1, suppress, output_sep)
      output << "#{prefix}#{lines1[i]}"
    end
    i += 1
  end

  # --- Remaining lines from file2 -----------------------------------------
  while j < lines2.length
    unless sup2
      prefix = build_comm_prefix(2, suppress, output_sep)
      output << "#{prefix}#{lines2[j]}"
    end
    j += 1
  end

  output
end

# ---------------------------------------------------------------------------
# Business Logic: build_comm_prefix
# ---------------------------------------------------------------------------
# Build the tab/separator prefix for a given column.
#
# The prefix consists of separator characters for each non-suppressed
# column that comes before the target column. For example, if no columns
# are suppressed and we're printing column 3, the prefix is "\t\t"
# (one tab for column 1, one for column 2).
#
# Parameters:
#   column   - Which column (1, 2, or 3) this line belongs to
#   suppress - Array of booleans [sup1, sup2, sup3]
#   sep      - The separator string
#
# Returns: The prefix string.

def build_comm_prefix(column, suppress, sep)
  sup1, sup2, _sup3 = suppress
  prefix = ""

  case column
  when 1
    # Column 1: no prefix needed
  when 2
    # Column 2: one separator for each non-suppressed column before it
    prefix += sep unless sup1
  when 3
    # Column 3: separators for each non-suppressed column before it
    prefix += sep unless sup1
    prefix += sep unless sup2
  end

  prefix
end

# ---------------------------------------------------------------------------
# Main entry point
# ---------------------------------------------------------------------------

def comm_main
  # --- Step 1: Parse arguments ---------------------------------------------
  begin
    result = CodingAdventures::CliBuilder::Parser.new(COMM_SPEC_FILE, ["comm"] + ARGV).parse
  rescue CodingAdventures::CliBuilder::ParseErrors => e
    e.errors.each { |err| warn "comm: #{err.message}" }
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
    # --- Step 3: Read files ------------------------------------------------
    file1 = result.arguments["file1"]
    file2 = result.arguments["file2"]
    delimiter = result.flags["zero_terminated"] ? "\0" : "\n"
    output_sep = result.flags["output_delimiter"] || "\t"

    lines1 = read_comm_file(file1, delimiter)
    lines2 = read_comm_file(file2, delimiter)

    # --- Step 4: Compare and output ----------------------------------------
    suppress = [
      result.flags["suppress_col1"] || false,
      result.flags["suppress_col2"] || false,
      result.flags["suppress_col3"] || false
    ]

    output = compare_sorted(lines1, lines2, suppress, output_sep)
    output.each { |line| print line + delimiter }
  end
end

# ---------------------------------------------------------------------------
# Helper: read a file into lines
# ---------------------------------------------------------------------------

def read_comm_file(filename, delimiter)
  if filename == "-"
    lines = []
    $stdin.each_line(delimiter) { |l| lines << l.chomp(delimiter) }
    lines
  else
    begin
      content = File.read(filename)
      lines = []
      content.each_line(delimiter) { |l| lines << l.chomp(delimiter) }
      lines
    rescue Errno::ENOENT
      warn "comm: #{filename}: No such file or directory"
      exit 1
    end
  end
end

# Only run main when this file is executed directly.
comm_main if __FILE__ == $PROGRAM_NAME
