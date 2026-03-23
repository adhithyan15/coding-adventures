#!/usr/bin/env ruby
# frozen_string_literal: true

# diff_tool.rb -- Compare files line by line
# ============================================
#
# === What This Program Does ===
#
# This is a reimplementation of the GNU `diff` utility. It compares two
# files (or directories) line by line and reports the differences between
# them. The output can be in several formats: normal, unified (-u),
# context (-c), or brief (-q).
#
# === How diff Works ===
#
#     $ diff file1.txt file2.txt         # normal diff output
#     $ diff -u file1.txt file2.txt      # unified format (like git diff)
#     $ diff -r dir1/ dir2/              # compare directories recursively
#     $ diff -i file1.txt file2.txt      # ignore case differences
#
# === The LCS Algorithm ===
#
# At its heart, diff uses the Longest Common Subsequence (LCS) algorithm
# to find which lines are shared between the two files. A "subsequence"
# is a sequence of elements that appear in the same order but not
# necessarily consecutively.
#
# Example:
#   File A: ["a", "b", "c", "d", "e"]
#   File B: ["a", "c", "d", "f"]
#   LCS:    ["a", "c", "d"]
#
# Lines in A but not in LCS are "deletions."
# Lines in B but not in LCS are "additions."
#
# We use a dynamic programming table to compute LCS efficiently.
# The table has dimensions (m+1) x (n+1) where m and n are the
# lengths of the two files. Each cell (i,j) stores the length of
# the LCS of the first i lines of A and the first j lines of B.
#
# === Output Formats ===
#
# Normal format (default):
#   2,3c2,3       <-- change command: lines 2-3 in file1, lines 2-3 in file2
#   < old line    <-- line from file1
#   ---
#   > new line    <-- line from file2
#
# Unified format (-u):
#   --- file1.txt
#   +++ file2.txt
#   @@ -1,5 +1,4 @@
#    context line
#   -deleted line
#   +added line
#    context line
#
# Context format (-c):
#   *** file1.txt
#   --- file2.txt
#   ***************
#   *** 1,5 ****
#   ...

require "coding_adventures_cli_builder"

DIFF_SPEC_FILE = File.join(File.dirname(__FILE__), "diff.json")

# ---------------------------------------------------------------------------
# Business Logic: normalize_line
# ---------------------------------------------------------------------------
# Apply normalization transforms to a line for comparison purposes.
# The original line is preserved for output; this normalized version is
# used only for comparison.
#
# Options:
#   :ignore_case       - Downcase the line before comparing
#   :ignore_all_space  - Remove all whitespace
#   :ignore_space_change - Collapse runs of whitespace to single space

def diff_normalize_line(line, opts = {})
  result = line.dup
  if opts[:ignore_all_space]
    result = result.gsub(/\s+/, "")
  elsif opts[:ignore_space_change]
    result = result.gsub(/\s+/, " ").strip
  end
  result = result.downcase if opts[:ignore_case]
  result
end

# ---------------------------------------------------------------------------
# Business Logic: compute_lcs_table
# ---------------------------------------------------------------------------
# Build the LCS dynamic programming table. This is the core of the diff
# algorithm.
#
# The table is a 2D array where table[i][j] = length of the LCS of
# lines_a[0..i-1] and lines_b[0..j-1].
#
# Time complexity: O(m * n) where m = lines_a.length, n = lines_b.length.
# Space complexity: O(m * n) for the table.
#
# We use this table in backtrack_lcs to reconstruct the actual edit script.

def compute_lcs_table(lines_a, lines_b, opts = {})
  m = lines_a.length
  n = lines_b.length

  # Initialize (m+1) x (n+1) table with zeros.
  # Row 0 and column 0 represent the empty prefix.
  table = Array.new(m + 1) { Array.new(n + 1, 0) }

  # Fill the table bottom-up.
  # If lines match, extend the LCS from the diagonal.
  # Otherwise, take the better of the two sub-problems.
  (1..m).each do |i|
    (1..n).each do |j|
      norm_a = diff_normalize_line(lines_a[i - 1], opts)
      norm_b = diff_normalize_line(lines_b[j - 1], opts)
      table[i][j] = if norm_a == norm_b
                       table[i - 1][j - 1] + 1
                     else
                       [table[i - 1][j], table[i][j - 1]].max
                     end
    end
  end

  table
end

# ---------------------------------------------------------------------------
# Business Logic: backtrack_edits
# ---------------------------------------------------------------------------
# Walk the LCS table backwards to produce an edit script.
#
# Starting from table[m][n], we trace back to table[0][0]:
#   - If lines match: record an "equal" edit and move diagonally.
#   - If table[i-1][j] >= table[i][j-1]: record a "delete" (line from A).
#   - Otherwise: record an "insert" (line from B).
#
# Returns an array of hashes with keys:
#   :type  - :equal, :delete, or :insert
#   :line  - the text of the line
#   :pos_a - 1-based line number in file A (for :equal and :delete)
#   :pos_b - 1-based line number in file B (for :equal and :insert)

def backtrack_edits(table, lines_a, lines_b, opts = {})
  edits = []
  i = lines_a.length
  j = lines_b.length

  while i > 0 || j > 0
    if i > 0 && j > 0
      norm_a = diff_normalize_line(lines_a[i - 1], opts)
      norm_b = diff_normalize_line(lines_b[j - 1], opts)
      if norm_a == norm_b
        edits.unshift({ type: :equal, line: lines_a[i - 1], pos_a: i, pos_b: j })
        i -= 1
        j -= 1
      elsif table[i - 1][j] >= table[i][j - 1]
        edits.unshift({ type: :delete, line: lines_a[i - 1], pos_a: i })
        i -= 1
      else
        edits.unshift({ type: :insert, line: lines_b[j - 1], pos_b: j })
        j -= 1
      end
    elsif i > 0
      edits.unshift({ type: :delete, line: lines_a[i - 1], pos_a: i })
      i -= 1
    else
      edits.unshift({ type: :insert, line: lines_b[j - 1], pos_b: j })
      j -= 1
    end
  end

  edits
end

# ---------------------------------------------------------------------------
# Business Logic: compute_edits
# ---------------------------------------------------------------------------
# Convenience method: compute LCS table and extract edit script in one call.

def compute_edits(lines_a, lines_b, opts = {})
  table = compute_lcs_table(lines_a, lines_b, opts)
  backtrack_edits(table, lines_a, lines_b, opts)
end

# ---------------------------------------------------------------------------
# Business Logic: group_edits_into_hunks
# ---------------------------------------------------------------------------
# Group consecutive edits into "hunks" -- regions of change surrounded
# by context lines. Each hunk contains a contiguous stretch of changes
# plus surrounding context lines.
#
# Parameters:
#   edits        - Array of edit hashes from backtrack_edits
#   context_size - Number of context lines to include before/after changes
#
# Returns an array of hunk arrays, where each hunk is a sub-array of edits.

def group_edits_into_hunks(edits, context_size = 3)
  return [] if edits.empty?

  # Find indices of change edits (non-equal)
  change_indices = []
  edits.each_with_index do |edit, idx|
    change_indices << idx if edit[:type] != :equal
  end

  return [] if change_indices.empty?

  # Build hunk ranges: each change expands to include context lines.
  # Overlapping ranges are merged.
  ranges = []
  change_indices.each do |ci|
    range_start = [ci - context_size, 0].max
    range_end = [ci + context_size, edits.length - 1].min

    if ranges.empty? || range_start > ranges.last[1] + 1
      ranges << [range_start, range_end]
    else
      ranges.last[1] = range_end
    end
  end

  # Convert ranges to hunk arrays
  ranges.map { |s, e| edits[s..e] }
end

# ---------------------------------------------------------------------------
# Business Logic: format_normal
# ---------------------------------------------------------------------------
# Format edits as a "normal" diff. This is the traditional diff output
# format with commands like "2,3c4,5" or "7a8" or "10d9".
#
# The command letters are:
#   a = add (lines were added in file B)
#   d = delete (lines were deleted from file A)
#   c = change (lines were changed between files)

def format_normal(edits)
  output = []
  i = 0

  while i < edits.length
    edit = edits[i]

    if edit[:type] == :equal
      i += 1
      next
    end

    # Collect consecutive non-equal edits
    deletes = []
    inserts = []

    while i < edits.length && edits[i][:type] != :equal
      if edits[i][:type] == :delete
        deletes << edits[i]
      else
        inserts << edits[i]
      end
      i += 1
    end

    # Build the command line
    if !deletes.empty? && !inserts.empty?
      # Change
      a_range = format_line_range(deletes.map { |d| d[:pos_a] })
      b_range = format_line_range(inserts.map { |ins| ins[:pos_b] })
      output << "#{a_range}c#{b_range}"
      deletes.each { |d| output << "< #{d[:line]}" }
      output << "---"
      inserts.each { |ins| output << "> #{ins[:line]}" }
    elsif !deletes.empty?
      # Delete
      a_range = format_line_range(deletes.map { |d| d[:pos_a] })
      # The position in B is the line after which the deletion occurs
      b_pos = deletes.first[:pos_a] - 1
      # Find the corresponding B position
      b_pos = find_b_position_for_delete(edits, deletes.first)
      output << "#{a_range}d#{b_pos}"
      deletes.each { |d| output << "< #{d[:line]}" }
    else
      # Add
      b_range = format_line_range(inserts.map { |ins| ins[:pos_b] })
      a_pos = find_a_position_for_insert(edits, inserts.first)
      output << "#{a_pos}a#{b_range}"
      inserts.each { |ins| output << "> #{ins[:line]}" }
    end
  end

  output.join("\n")
end

# ---------------------------------------------------------------------------
# Helper: format_line_range
# ---------------------------------------------------------------------------
# Format a range of line numbers. Single line: "5". Range: "5,8".

def format_line_range(positions)
  if positions.length == 1
    positions.first.to_s
  else
    "#{positions.first},#{positions.last}"
  end
end

# ---------------------------------------------------------------------------
# Helper: find_b_position_for_delete
# ---------------------------------------------------------------------------
# Find the B-side line number corresponding to a deletion point.
# We look backwards through the edits for the nearest equal or insert
# line to find the B position.

def find_b_position_for_delete(edits, delete_edit)
  idx = edits.index(delete_edit)
  # Walk backwards to find a reference point
  (idx - 1).downto(0) do |k|
    return edits[k][:pos_b] if edits[k][:pos_b]
  end
  0
end

# ---------------------------------------------------------------------------
# Helper: find_a_position_for_insert
# ---------------------------------------------------------------------------
# Find the A-side line number corresponding to an insertion point.

def find_a_position_for_insert(edits, insert_edit)
  idx = edits.index(insert_edit)
  (idx - 1).downto(0) do |k|
    return edits[k][:pos_a] if edits[k][:pos_a]
  end
  0
end

# ---------------------------------------------------------------------------
# Business Logic: format_unified
# ---------------------------------------------------------------------------
# Format edits as unified diff output. This is the format used by
# `diff -u` and by git. It shows changes with +/- prefixes and
# @@ hunk headers.
#
# Example:
#   --- a/file.txt
#   +++ b/file.txt
#   @@ -1,5 +1,4 @@
#    context line
#   -removed line
#   +added line
#    context line

def format_unified(edits, file_a, file_b, context_size = 3)
  hunks = group_edits_into_hunks(edits, context_size)
  return "" if hunks.empty?

  output = []
  output << "--- #{file_a}"
  output << "+++ #{file_b}"

  hunks.each do |hunk|
    # Calculate hunk header: line numbers and counts for each side
    a_lines = hunk.select { |e| e[:type] == :equal || e[:type] == :delete }
    b_lines = hunk.select { |e| e[:type] == :equal || e[:type] == :insert }

    a_start = a_lines.empty? ? 0 : a_lines.first[:pos_a]
    b_start = b_lines.empty? ? 0 : b_lines.first[:pos_b]
    a_count = a_lines.length
    b_count = b_lines.length

    output << "@@ -#{a_start},#{a_count} +#{b_start},#{b_count} @@"

    hunk.each do |edit|
      case edit[:type]
      when :equal  then output << " #{edit[:line]}"
      when :delete then output << "-#{edit[:line]}"
      when :insert then output << "+#{edit[:line]}"
      end
    end
  end

  output.join("\n")
end

# ---------------------------------------------------------------------------
# Business Logic: format_context
# ---------------------------------------------------------------------------
# Format edits as context diff output (diff -c). Shows the old and new
# versions of each hunk separately, with *** and --- markers.

def format_context(edits, file_a, file_b, context_size = 3)
  hunks = group_edits_into_hunks(edits, context_size)
  return "" if hunks.empty?

  output = []
  output << "*** #{file_a}"
  output << "--- #{file_b}"

  hunks.each do |hunk|
    output << "***************"

    # Old file section
    a_edits = hunk.select { |e| e[:type] == :equal || e[:type] == :delete }
    unless a_edits.empty?
      a_start = a_edits.first[:pos_a]
      a_end = a_edits.last[:pos_a]
      output << "*** #{a_start},#{a_end} ****"
      hunk.each do |edit|
        case edit[:type]
        when :equal  then output << "  #{edit[:line]}"
        when :delete then output << "- #{edit[:line]}"
        when :insert then next  # skip inserts in old section
        end
      end
    end

    # New file section
    b_edits = hunk.select { |e| e[:type] == :equal || e[:type] == :insert }
    unless b_edits.empty?
      b_start = b_edits.first[:pos_b]
      b_end = b_edits.last[:pos_b]
      output << "--- #{b_start},#{b_end} ----"
      hunk.each do |edit|
        case edit[:type]
        when :equal  then output << "  #{edit[:line]}"
        when :insert then output << "+ #{edit[:line]}"
        when :delete then next  # skip deletes in new section
        end
      end
    end
  end

  output.join("\n")
end

# ---------------------------------------------------------------------------
# Business Logic: diff_files
# ---------------------------------------------------------------------------
# Compare two files and return the formatted diff output.
#
# Parameters:
#   file_a - Path to the first file
#   file_b - Path to the second file
#   opts   - Hash of options:
#     :format            - :normal, :unified, or :context
#     :context_size      - Number of context lines (default 3)
#     :ignore_case       - Ignore case differences
#     :ignore_all_space  - Ignore all whitespace
#     :ignore_space_change - Ignore changes in whitespace amount
#     :ignore_blank_lines  - Ignore blank line changes
#     :brief             - Only report whether files differ
#
# Returns: [output_string, exit_code]
#   exit_code 0 = files identical, 1 = files differ, 2 = error

def diff_files(file_a, file_b, opts = {})
  begin
    content_a = File.read(file_a)
    content_b = File.read(file_b)
  rescue Errno::ENOENT => e
    return ["diff: #{e.message}", 2]
  end

  lines_a = content_a.lines.map(&:chomp)
  lines_b = content_b.lines.map(&:chomp)

  # Filter blank lines if requested
  if opts[:ignore_blank_lines]
    lines_a = lines_a.reject { |l| l.strip.empty? }
    lines_b = lines_b.reject { |l| l.strip.empty? }
  end

  edits = compute_edits(lines_a, lines_b, opts)

  # Check if files are identical
  has_changes = edits.any? { |e| e[:type] != :equal }

  unless has_changes
    return ["", 0]
  end

  if opts[:brief]
    return ["Files #{file_a} and #{file_b} differ", 1]
  end

  context_size = opts[:context_size] || 3

  output = case opts[:format]
           when :unified
             format_unified(edits, file_a, file_b, context_size)
           when :context
             format_context(edits, file_a, file_b, context_size)
           else
             format_normal(edits)
           end

  [output, 1]
end

# ---------------------------------------------------------------------------
# Business Logic: diff_directories
# ---------------------------------------------------------------------------
# Recursively compare two directories. Lists files that exist in only
# one directory and diffs files that exist in both.

def diff_directories(dir_a, dir_b, opts = {})
  output_parts = []
  exit_code = 0

  entries_a = Dir.entries(dir_a).sort - %w[. ..]
  entries_b = Dir.entries(dir_b).sort - %w[. ..]

  # Apply exclusion patterns
  if opts[:exclude]
    excludes = opts[:exclude].is_a?(Array) ? opts[:exclude] : [opts[:exclude]]
    excludes.each do |pattern|
      entries_a = entries_a.reject { |e| File.fnmatch?(pattern, e) }
      entries_b = entries_b.reject { |e| File.fnmatch?(pattern, e) }
    end
  end

  all_entries = (entries_a + entries_b).uniq.sort

  all_entries.each do |entry|
    path_a = File.join(dir_a, entry)
    path_b = File.join(dir_b, entry)

    if !entries_a.include?(entry)
      output_parts << "Only in #{dir_b}: #{entry}"
      exit_code = 1
    elsif !entries_b.include?(entry)
      output_parts << "Only in #{dir_a}: #{entry}"
      exit_code = 1
    elsif File.directory?(path_a) && File.directory?(path_b)
      sub_output, sub_code = diff_directories(path_a, path_b, opts)
      output_parts << sub_output unless sub_output.empty?
      exit_code = sub_code if sub_code > exit_code
    elsif File.file?(path_a) && File.file?(path_b)
      sub_output, sub_code = diff_files(path_a, path_b, opts)
      output_parts << sub_output unless sub_output.empty?
      exit_code = sub_code if sub_code > exit_code
    end
  end

  [output_parts.join("\n"), exit_code]
end

# ---------------------------------------------------------------------------
# Main entry point
# ---------------------------------------------------------------------------

def diff_main
  begin
    result = CodingAdventures::CliBuilder::Parser.new(DIFF_SPEC_FILE, ["diff"] + ARGV).parse
  rescue CodingAdventures::CliBuilder::ParseErrors => e
    e.errors.each { |err| warn "diff: #{err.message}" }
    exit 2
  end

  case result
  when CodingAdventures::CliBuilder::HelpResult
    puts result.text
    exit 0
  when CodingAdventures::CliBuilder::VersionResult
    puts result.version
    exit 0
  when CodingAdventures::CliBuilder::ParseResult
    file_a = result.arguments["file1"]
    file_b = result.arguments["file2"]

    opts = {
      ignore_case: result.flags["ignore_case"] || false,
      ignore_all_space: result.flags["ignore_all_space"] || false,
      ignore_space_change: result.flags["ignore_space_change"] || false,
      ignore_blank_lines: result.flags["ignore_blank_lines"] || false,
      brief: result.flags["brief"] || false,
      exclude: result.flags["exclude"],
    }

    # Determine output format
    if result.flags["unified"] && result.flags["unified"] != 3
      opts[:format] = :unified
      opts[:context_size] = result.flags["unified"]
    elsif result.flags["context_format"] && result.flags["context_format"] != 3
      opts[:format] = :context
      opts[:context_size] = result.flags["context_format"]
    elsif result.flags["normal"]
      opts[:format] = :normal
    else
      # Default: check if -u or -c were specified (even with default value)
      opts[:format] = :normal
    end

    if result.flags["recursive"] && File.directory?(file_a) && File.directory?(file_b)
      output, code = diff_directories(file_a, file_b, opts)
    else
      output, code = diff_files(file_a, file_b, opts)
    end

    puts output unless output.empty?
    exit code
  end
end

diff_main if __FILE__ == $PROGRAM_NAME
