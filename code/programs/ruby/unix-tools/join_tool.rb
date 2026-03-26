#!/usr/bin/env ruby
# frozen_string_literal: true

# join_tool.rb -- Join lines of two files on a common field
# ===========================================================
#
# === What This Program Does ===
#
# This is a reimplementation of the GNU `join` utility. It performs a
# relational join operation on two sorted files, combining lines that
# share a common field value. Think of it as a simplified SQL JOIN
# for text files.
#
# === How join Works ===
#
# Given two files sorted by a common key field:
#
#   File 1 (students.txt):     File 2 (grades.txt):
#     1 Alice                    1 A
#     2 Bob                      2 B
#     3 Charlie                  3 A
#
#     $ join students.txt grades.txt
#     1 Alice A
#     2 Bob B
#     3 Charlie A
#
# The default behavior:
#   - Join on field 1 of both files
#   - Fields are separated by whitespace
#   - Both files must be sorted on the join field
#   - Output: join field, then remaining fields from file 1, then file 2
#
# === Merge-Join Algorithm ===
#
# Because both files are sorted on the join field, we can use a
# merge-join algorithm (O(n+m) time complexity) instead of a nested
# loop join (O(n*m)). The algorithm:
#
#   1. Read the next line from each file.
#   2. Compare their join fields.
#   3. If they match: output the joined line, advance both.
#   4. If file1's key < file2's key: advance file1.
#   5. If file1's key > file2's key: advance file2.
#
# This is the same algorithm databases use for merge joins.
#
# === Flags ===
#
#   -1 FIELD  Join on field FIELD of file 1 (default: 1)
#   -2 FIELD  Join on field FIELD of file 2 (default: 1)
#   -j FIELD  Equivalent to -1 FIELD -2 FIELD
#   -a FILENUM Also print unpairable lines from file FILENUM
#   -v FILENUM Like -a, but suppress joined output
#   -e EMPTY  Replace missing fields with EMPTY
#   -o FORMAT Specify output format
#   -t CHAR   Use CHAR as field separator
#   -i        Ignore case when comparing join fields

require "coding_adventures_cli_builder"

JOIN_SPEC_FILE = File.join(File.dirname(__FILE__), "join.json")

# ---------------------------------------------------------------------------
# Business Logic: join_parse_line
# ---------------------------------------------------------------------------
# Split a line into fields using the given separator.
#
# If separator is nil, split on whitespace (like awk's default).
# Returns an array of field strings.

def join_parse_line(line, separator: nil)
  if separator
    line.split(separator, -1)
  else
    line.split
  end
end

# ---------------------------------------------------------------------------
# Business Logic: join_format_output
# ---------------------------------------------------------------------------
# Format a joined output line from two input lines.
#
# Parameters:
#   key    - The shared join field value.
#   fields1 - All fields from file 1 (including the join field).
#   fields2 - All fields from file 2 (including the join field).
#   opts:
#     :field1    - Join field index for file 1 (0-based).
#     :field2    - Join field index for file 2 (0-based).
#     :separator - Output field separator (default: space).
#     :format    - Output format string (e.g., "1.2,2.2").
#     :empty     - Replacement for missing fields.
#
# Returns the formatted output string.

def join_format_output(key, fields1, fields2, opts = {})
  sep = opts[:separator] || " "
  empty = opts[:empty] || ""
  field1_idx = opts[:field1] || 0
  field2_idx = opts[:field2] || 0

  if opts[:format]
    # Parse format string like "1.1,2.1,1.2" or "0 1.1 2.1".
    # Format spec: FILENUM.FIELDNUM (1-based), or 0 for join field.
    specs = opts[:format].split(/[,\s]+/)
    parts = specs.map do |spec|
      if spec == "0"
        key
      elsif spec =~ /\A(\d+)\.(\d+)\z/
        filenum = ::Regexp.last_match(1).to_i
        fieldnum = ::Regexp.last_match(2).to_i - 1
        source = filenum == 1 ? fields1 : fields2
        if source && fieldnum >= 0 && fieldnum < source.length
          source[fieldnum]
        else
          empty
        end
      else
        spec
      end
    end
    return parts.join(sep)
  end

  # Default format: join field, then remaining fields from file 1,
  # then remaining fields from file 2.
  parts = [key]

  if fields1
    fields1.each_with_index do |f, i|
      parts << f unless i == field1_idx
    end
  end

  if fields2
    fields2.each_with_index do |f, i|
      parts << f unless i == field2_idx
    end
  end

  parts.join(sep)
end

# ---------------------------------------------------------------------------
# Business Logic: join_files
# ---------------------------------------------------------------------------
# Perform a merge-join on two arrays of lines.
#
# Parameters:
#   lines1 - Array of strings (lines from file 1).
#   lines2 - Array of strings (lines from file 2).
#   opts:
#     :field1       - Join field for file 1 (1-based, default: 1).
#     :field2       - Join field for file 2 (1-based, default: 1).
#     :separator    - Field separator (nil = whitespace).
#     :ignore_case  - Case-insensitive comparison.
#     :unpaired     - Array of file numbers to print unpairable lines from.
#     :only_unpaired - File number for which to only print unpaired lines.
#     :empty        - Replacement for missing fields.
#     :format       - Output format string.
#     :header       - Treat first line as header.
#
# Returns an array of output strings.

def join_files(lines1, lines2, opts = {})
  field1_idx = (opts[:field1] || 1) - 1
  field2_idx = (opts[:field2] || 1) - 1
  separator = opts[:separator]
  ignore_case = opts[:ignore_case] || false
  unpaired = opts[:unpaired] || []
  only_unpaired = opts[:only_unpaired]
  output = []

  # Handle header line: join them unconditionally.
  if opts[:header] && !lines1.empty? && !lines2.empty?
    h1_fields = join_parse_line(lines1[0], separator: separator)
    h2_fields = join_parse_line(lines2[0], separator: separator)
    h1_key = h1_fields[field1_idx] || ""
    output << join_format_output(h1_key, h1_fields, h2_fields,
                                  field1: field1_idx, field2: field2_idx,
                                  separator: separator || " ", empty: opts[:empty],
                                  format: opts[:format])
    lines1 = lines1[1..]
    lines2 = lines2[1..]
  end

  i = 0
  j = 0

  while i < lines1.length || j < lines2.length
    if i >= lines1.length
      # File 1 exhausted -- print remaining unpaired from file 2.
      if unpaired.include?(2) || only_unpaired == 2
        while j < lines2.length
          f2 = join_parse_line(lines2[j], separator: separator)
          key2 = f2[field2_idx] || ""
          output << join_format_output(key2, nil, f2,
                                        field1: field1_idx, field2: field2_idx,
                                        separator: separator || " ", empty: opts[:empty],
                                        format: opts[:format])
          j += 1
        end
      end
      break
    end

    if j >= lines2.length
      # File 2 exhausted -- print remaining unpaired from file 1.
      if unpaired.include?(1) || only_unpaired == 1
        while i < lines1.length
          f1 = join_parse_line(lines1[i], separator: separator)
          key1 = f1[field1_idx] || ""
          output << join_format_output(key1, f1, nil,
                                        field1: field1_idx, field2: field2_idx,
                                        separator: separator || " ", empty: opts[:empty],
                                        format: opts[:format])
          i += 1
        end
      end
      break
    end

    f1 = join_parse_line(lines1[i], separator: separator)
    f2 = join_parse_line(lines2[j], separator: separator)

    key1 = f1[field1_idx] || ""
    key2 = f2[field2_idx] || ""

    cmp_key1 = ignore_case ? key1.downcase : key1
    cmp_key2 = ignore_case ? key2.downcase : key2

    if cmp_key1 == cmp_key2
      # --- Keys match: produce joined output -----------------------------------
      # Handle duplicate keys in file 2: pair the current file 1 line with
      # all matching file 2 lines.
      unless only_unpaired
        # Collect all file 2 lines with the same key.
        j_start = j
        while j < lines2.length
          f2_check = join_parse_line(lines2[j], separator: separator)
          k2 = f2_check[field2_idx] || ""
          break if (ignore_case ? k2.downcase : k2) != cmp_key1
          j += 1
        end
        j_end = j

        # Also collect all file 1 lines with the same key.
        i_start = i
        while i < lines1.length
          f1_check = join_parse_line(lines1[i], separator: separator)
          k1 = f1_check[field1_idx] || ""
          break if (ignore_case ? k1.downcase : k1) != cmp_key1
          i += 1
        end

        # Cross-product of matching lines.
        (i_start...i).each do |ii|
          ff1 = join_parse_line(lines1[ii], separator: separator)
          (j_start...j_end).each do |jj|
            ff2 = join_parse_line(lines2[jj], separator: separator)
            output << join_format_output(key1, ff1, ff2,
                                          field1: field1_idx, field2: field2_idx,
                                          separator: separator || " ", empty: opts[:empty],
                                          format: opts[:format])
          end
        end
      else
        # Skip matched lines when only_unpaired is set.
        i += 1
        j += 1
      end
    elsif cmp_key1 < cmp_key2
      # File 1's key is smaller -- it's unpaired in file 1.
      if unpaired.include?(1) || only_unpaired == 1
        output << join_format_output(key1, f1, nil,
                                      field1: field1_idx, field2: field2_idx,
                                      separator: separator || " ", empty: opts[:empty],
                                      format: opts[:format])
      end
      i += 1
    else
      # File 2's key is smaller -- it's unpaired in file 2.
      if unpaired.include?(2) || only_unpaired == 2
        output << join_format_output(key2, nil, f2,
                                      field1: field1_idx, field2: field2_idx,
                                      separator: separator || " ", empty: opts[:empty],
                                      format: opts[:format])
      end
      j += 1
    end
  end

  output
end

# ---------------------------------------------------------------------------
# Main entry point
# ---------------------------------------------------------------------------

def join_main
  begin
    result = CodingAdventures::CliBuilder::Parser.new(JOIN_SPEC_FILE, ["join"] + ARGV).parse
  rescue CodingAdventures::CliBuilder::ParseErrors => e
    e.errors.each { |err| warn "join: #{err.message}" }
    exit 1
  end

  case result
  when CodingAdventures::CliBuilder::HelpResult
    puts result.text
    exit 0
  when CodingAdventures::CliBuilder::VersionResult
    puts result.version
    exit 0
  when CodingAdventures::CliBuilder::ParseResult
    file1_path = result.arguments["file1"]
    file2_path = result.arguments["file2"]

    # Read files (- means stdin).
    lines1 = file1_path == "-" ? $stdin.readlines(chomp: true) : File.readlines(file1_path, chomp: true)
    lines2 = file2_path == "-" ? $stdin.readlines(chomp: true) : File.readlines(file2_path, chomp: true)

    # Determine join fields.
    field1 = result.flags["field1"] || result.flags["join_field"] || 1
    field2 = result.flags["field2"] || result.flags["join_field"] || 1

    # Parse unpaired flags.
    unpaired_raw = result.flags["unpaired"] || []
    unpaired_raw = [unpaired_raw] if unpaired_raw.is_a?(String)
    unpaired = unpaired_raw.map(&:to_i)

    only_unpaired = result.flags["only_unpaired"]
    only_unpaired = only_unpaired.to_i if only_unpaired

    opts = {
      field1: field1.to_i,
      field2: field2.to_i,
      separator: result.flags["separator"],
      ignore_case: result.flags["ignore_case"] || false,
      unpaired: unpaired,
      only_unpaired: only_unpaired,
      empty: result.flags["empty"],
      format: result.flags["format"],
      header: result.flags["header"] || false,
    }

    output = join_files(lines1, lines2, opts)
    output.each { |line| puts line }
  end
end

join_main if __FILE__ == $PROGRAM_NAME
