#!/usr/bin/env ruby
# frozen_string_literal: true

# sort_tool.rb -- Sort lines of text files
# ==========================================
#
# === What This Program Does ===
#
# This is a reimplementation of the GNU `sort` utility. It reads lines
# from files (or standard input), sorts them, and writes the result to
# standard output. By default, lines are sorted lexicographically
# (dictionary order by byte value).
#
# === Sorting Modes ===
#
# sort supports several comparison modes:
#
#   (default)    Lexicographic comparison using Ruby's String#<=>
#   -n           Numeric sort: compare leading numeric values
#   -g           General numeric sort: parse as floating point
#   -h           Human-readable sort: parse suffixes like K, M, G
#   -M           Month sort: JAN < FEB < ... < DEC
#   -V           Version sort: natural comparison of embedded numbers
#
# === Modifiers ===
#
#   -r           Reverse the comparison (descending order)
#   -f           Fold case: treat lowercase as uppercase for comparison
#   -u           Unique: suppress duplicate lines
#   -d           Dictionary order: consider only blanks and alphanumeric
#   -i           Ignore non-printing characters
#   -b           Ignore leading blanks in comparison keys
#   -s           Stable sort: preserve input order for equal elements
#   -k KEYDEF    Sort by key field(s)
#   -t SEP       Use SEP as the field separator
#   -z           Use NUL as line delimiter instead of newline
#   -c           Check if input is already sorted
#   -m           Merge already-sorted files
#   -o FILE      Write output to FILE
#
# === Algorithm ===
#
# Ruby's Array#sort is a stable merge sort variant (Timsort in modern
# implementations). We build a comparison function from the flags and
# pass it to sort. The stable flag (-s) is technically always satisfied
# by Ruby's sort, but we honor it for compatibility.
#
# === Key Fields (-k) ===
#
# The -k flag specifies sort keys in the format:
#   -k F1[.C1][OPTS][,F2[.C2][OPTS]]
#
# Where F1 is the start field, C1 is the start character position,
# F2 is the end field, and C2 is the end character position. Fields
# are 1-indexed. OPTS can include sort-type modifiers (n, g, h, M, r,
# f, d, i, b) that apply only to this key.
#
# For this implementation, we support the basic -k syntax without
# per-key modifiers.

require "coding_adventures_cli_builder"

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

SORT_SPEC_FILE = File.join(File.dirname(__FILE__), "sort.json")

# ---------------------------------------------------------------------------
# Constants: month ordering for -M
# ---------------------------------------------------------------------------
# Month sort compares three-letter month abbreviations. Unknown strings
# sort before JAN.

MONTH_ORDER = {
  "JAN" => 1, "FEB" => 2, "MAR" => 3, "APR" => 4,
  "MAY" => 5, "JUN" => 6, "JUL" => 7, "AUG" => 8,
  "SEP" => 9, "OCT" => 10, "NOV" => 11, "DEC" => 12
}.freeze

# ---------------------------------------------------------------------------
# Business Logic: parse_human_size
# ---------------------------------------------------------------------------
# Parse a human-readable size string like "1K", "2.5M", "3G" into a
# numeric value for comparison.
#
# Suffixes and their multipliers:
#   K = 1024, M = 1024^2, G = 1024^3, T = 1024^4, P = 1024^5, E = 1024^6
#
# If the string doesn't match, returns 0.0.

HUMAN_SUFFIXES = {
  "K" => 1024.0,
  "M" => 1024.0**2,
  "G" => 1024.0**3,
  "T" => 1024.0**4,
  "P" => 1024.0**5,
  "E" => 1024.0**6
}.freeze

def parse_human_size(str)
  stripped = str.strip
  return 0.0 if stripped.empty?

  match = stripped.match(/\A([+-]?\d+\.?\d*)\s*([KMGTPE])?\z/i)
  return 0.0 unless match

  number = match[1].to_f
  suffix = match[2]&.upcase
  multiplier = suffix ? (HUMAN_SUFFIXES[suffix] || 1.0) : 1.0
  number * multiplier
end

# ---------------------------------------------------------------------------
# Business Logic: extract_sort_key
# ---------------------------------------------------------------------------
# Given a line and sort options, extract the portion of the line used
# for comparison. When -k is specified, extract the key field range.
# When -t is specified, use that as the field separator (default: runs
# of whitespace).
#
# Parameters:
#   line      - The full text line
#   key_spec  - A key specification string (e.g., "2,2" or "1.3,1.5"),
#               or nil for the whole line
#   separator - The field separator character, or nil for whitespace
#
# Returns: The extracted key string.

def extract_sort_key(line, key_spec, separator)
  return line unless key_spec

  # Parse key spec: "F1[.C1][,F2[.C2]]"
  parts = key_spec.split(",", 2)
  start_spec = parts[0]
  end_spec = parts[1]

  # Parse start field and character
  start_parts = start_spec.split(".", 2)
  start_field = start_parts[0].to_i
  start_char = start_parts[1]&.to_i

  # Split the line into fields
  fields = if separator
             line.split(separator, -1)
           else
             line.split
           end

  # Fields are 1-indexed; convert to 0-indexed
  start_idx = [start_field - 1, 0].max

  if end_spec
    end_parts = end_spec.split(".", 2)
    end_field = end_parts[0].to_i
    end_idx = [end_field - 1, start_idx].max
  else
    end_idx = fields.length - 1
  end

  # Extract the range of fields
  selected = fields[start_idx..end_idx] || []

  result = if separator
             selected.join(separator)
           else
             selected.join(" ")
           end

  # Apply character-level slicing if specified
  if start_char && start_char > 0
    result = result[(start_char - 1)..] || ""
  end

  result
end

# ---------------------------------------------------------------------------
# Business Logic: build_comparator
# ---------------------------------------------------------------------------
# Build a comparison lambda based on sort flags. This lambda takes two
# strings (sort keys) and returns -1, 0, or 1.
#
# The comparator is the heart of the sort tool. It implements:
#   - Lexicographic comparison (default)
#   - Numeric comparison (-n)
#   - General numeric comparison (-g)
#   - Human-readable comparison (-h)
#   - Month comparison (-M)
#   - Version sort (-V)
#   - Case folding (-f)
#   - Dictionary order (-d)
#   - Ignore non-printing (-i)
#   - Ignore leading blanks (-b)
#   - Reverse (-r)

def build_comparator(flags)
  lambda do |a, b|
    # --- Pre-processing transforms ---
    ka = a.dup
    kb = b.dup

    # -b: strip leading blanks from the key
    if flags["ignore_leading_blanks"]
      ka = ka.lstrip
      kb = kb.lstrip
    end

    # -d: consider only blanks and alphanumeric characters
    if flags["dictionary_order"]
      ka = ka.gsub(/[^[:alnum:][:space:]]/, "")
      kb = kb.gsub(/[^[:alnum:][:space:]]/, "")
    end

    # -i: consider only printable characters
    if flags["ignore_nonprinting"]
      ka = ka.gsub(/[^[:print:]]/, "")
      kb = kb.gsub(/[^[:print:]]/, "")
    end

    # -f: fold case
    if flags["ignore_case"]
      ka = ka.downcase
      kb = kb.downcase
    end

    # --- Comparison ---
    cmp = if flags["numeric_sort"]
            # -n: Compare leading numeric portions
            ka.to_f <=> kb.to_f
          elsif flags["general_numeric_sort"]
            # -g: General numeric sort (handles scientific notation)
            va = Float(ka.strip, exception: false) || 0.0
            vb = Float(kb.strip, exception: false) || 0.0
            va <=> vb
          elsif flags["human_numeric_sort"]
            # -h: Human-readable sort (K, M, G suffixes)
            parse_human_size(ka) <=> parse_human_size(kb)
          elsif flags["month_sort"]
            # -M: Month sort
            ma = MONTH_ORDER[ka.strip.upcase[0, 3]] || 0
            mb = MONTH_ORDER[kb.strip.upcase[0, 3]] || 0
            ma <=> mb
          elsif flags["version_sort"]
            # -V: Version sort -- split into numeric and non-numeric chunks
            # and compare them piecewise
            version_compare(ka, kb)
          else
            # Default: lexicographic comparison
            ka <=> kb
          end

    cmp || 0
  end
end

# ---------------------------------------------------------------------------
# Business Logic: version_compare
# ---------------------------------------------------------------------------
# Compare two strings using "version sort" semantics. This splits each
# string into alternating runs of digits and non-digits, then compares
# each chunk: digit chunks are compared numerically, text chunks
# lexicographically.
#
#   "file2" < "file10"    (because 2 < 10 numerically)
#   "1.2.3" < "1.10.1"   (because 2 < 10 in the second component)

def version_compare(str_a, str_b)
  # Split into chunks of digits and non-digits
  chunks_a = str_a.scan(/\d+|[^\d]+/)
  chunks_b = str_b.scan(/\d+|[^\d]+/)

  [chunks_a.length, chunks_b.length].max.times do |i|
    ca = chunks_a[i]
    cb = chunks_b[i]

    # If one string ran out of chunks, it sorts first
    return -1 if ca.nil?
    return 1 if cb.nil?

    # Compare: numeric chunks by value, text chunks lexicographically
    if ca.match?(/\A\d+\z/) && cb.match?(/\A\d+\z/)
      cmp = ca.to_i <=> cb.to_i
      return cmp unless cmp == 0
      # If numerically equal (e.g., "01" vs "1"), compare by length
      cmp = ca.length <=> cb.length
      return cmp unless cmp == 0
    else
      cmp = ca <=> cb
      return cmp unless cmp == 0
    end
  end

  0
end

# ---------------------------------------------------------------------------
# Business Logic: sort_lines
# ---------------------------------------------------------------------------
# Sort an array of lines according to the given flags.
#
# Parameters:
#   lines - Array of strings (lines to sort)
#   flags - Hash of flag values from CLI Builder
#
# Returns: A new sorted array of strings.

def sort_lines(lines, flags)
  comparator = build_comparator(flags)
  separator = flags["field_separator"]
  key_specs = flags["key"]  # may be an array if repeatable

  # Build the sort-key extractor
  sorted = lines.sort do |a, b|
    if key_specs && !key_specs.empty?
      # Compare by each key in order; first non-zero comparison wins
      specs = key_specs.is_a?(Array) ? key_specs : [key_specs]
      cmp = 0
      specs.each do |spec|
        ka = extract_sort_key(a, spec, separator)
        kb = extract_sort_key(b, spec, separator)
        cmp = comparator.call(ka, kb)
        break unless cmp == 0
      end
      cmp
    else
      comparator.call(a, b)
    end
  end

  # -r: reverse the result
  sorted.reverse! if flags["reverse"]

  # -u: remove duplicates (keeping first occurrence)
  if flags["unique"]
    seen = {}
    sorted = sorted.select do |line|
      key = if key_specs && !key_specs.empty?
              specs = key_specs.is_a?(Array) ? key_specs : [key_specs]
              specs.map { |s| extract_sort_key(line, s, separator) }.join("\0")
            else
              line
            end
      # Apply same transforms for uniqueness
      key = key.downcase if flags["ignore_case"]
      if seen[key]
        false
      else
        seen[key] = true
        true
      end
    end
  end

  sorted
end

# ---------------------------------------------------------------------------
# Business Logic: check_sorted
# ---------------------------------------------------------------------------
# Check whether lines are sorted. Returns true if sorted, false otherwise.
# Prints a diagnostic message to stderr for the first out-of-order line.

def check_sorted(lines, flags)
  comparator = build_comparator(flags)
  separator = flags["field_separator"]
  key_specs = flags["key"]

  (1...lines.length).each do |i|
    a = lines[i - 1]
    b = lines[i]

    cmp = if key_specs && !key_specs.empty?
            specs = key_specs.is_a?(Array) ? key_specs : [key_specs]
            result = 0
            specs.each do |spec|
              ka = extract_sort_key(a, spec, separator)
              kb = extract_sort_key(b, spec, separator)
              result = comparator.call(ka, kb)
              break unless result == 0
            end
            result
          else
            comparator.call(a, b)
          end

    cmp = -cmp if flags["reverse"]

    if cmp > 0
      warn "sort: disorder: #{b}"
      return false
    end

    if flags["unique"] && cmp == 0
      warn "sort: disorder: #{b}"
      return false
    end
  end

  true
end

# ---------------------------------------------------------------------------
# Business Logic: read_lines
# ---------------------------------------------------------------------------
# Read all lines from an IO, stripping the line terminator.

def sort_read_lines(io, delimiter)
  lines = []
  io.each_line(delimiter) do |line|
    lines << line.chomp(delimiter)
  end
  lines
end

# ---------------------------------------------------------------------------
# Main entry point
# ---------------------------------------------------------------------------

def sort_main
  # --- Step 1: Parse arguments ---------------------------------------------
  begin
    result = CodingAdventures::CliBuilder::Parser.new(SORT_SPEC_FILE, ["sort"] + ARGV).parse
  rescue CodingAdventures::CliBuilder::ParseErrors => e
    e.errors.each { |err| warn "sort: #{err.message}" }
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
    # --- Step 3: Read input ------------------------------------------------
    files = result.arguments.fetch("files", ["-"])
    delimiter = result.flags["zero_terminated"] ? "\0" : "\n"
    all_lines = []

    files.each do |filename|
      if filename == "-"
        all_lines.concat(sort_read_lines($stdin, delimiter))
      else
        begin
          File.open(filename, "r") do |f|
            all_lines.concat(sort_read_lines(f, delimiter))
          end
        rescue Errno::ENOENT
          warn "sort: cannot read: #{filename}: No such file or directory"
          exit 2
        end
      end
    end

    # --- Step 4: Sort or check ---------------------------------------------
    if result.flags["check"]
      exit(check_sorted(all_lines, result.flags) ? 0 : 1)
    end

    sorted = sort_lines(all_lines, result.flags)

    # --- Step 5: Output ----------------------------------------------------
    output_io = if result.flags["output"]
                  File.open(result.flags["output"], "w")
                else
                  $stdout
                end

    sorted.each { |line| output_io.print(line + delimiter) }
    output_io.close if output_io != $stdout
  end
end

# Only run main when this file is executed directly.
sort_main if __FILE__ == $PROGRAM_NAME
