#!/usr/bin/env ruby
# frozen_string_literal: true

# split_tool.rb -- Split a file into pieces
# ============================================
#
# === What This Program Does ===
#
# This is a reimplementation of the GNU `split` utility. It splits a
# file into fixed-size pieces. By default, it creates output files
# with 1000 lines each, named xaa, xab, xac, ... .
#
# === How split Works ===
#
#     $ split largefile.txt              # 1000-line chunks: xaa, xab, ...
#     $ split -l 100 data.csv chunk_     # 100-line chunks: chunk_aa, chunk_ab
#     $ split -b 1M bigfile archive_     # 1MB chunks
#     $ split -d -a 3 file prefix_       # numeric suffixes: prefix_000, prefix_001
#
# === Suffix Generation ===
#
# By default, split uses alphabetic suffixes: aa, ab, ..., az, ba, bb, ...
# With -d, it uses numeric suffixes: 00, 01, 02, ...
# With -x, it uses hexadecimal suffixes: 00, 01, ..., 0f, 10, ...
#
# The suffix length (-a) determines how many characters to use. With
# -a 2 (default), you get up to 676 files (26^2) with alphabetic,
# or 100 files (10^2) with numeric suffixes.
#
# === Split Modes ===
#
#   -l N   Split by lines (default: 1000 lines per file)
#   -b N   Split by bytes (supports K, M, G suffixes)
#   -n N   Split into N equal-sized chunks
#
# === Implementation ===
#
# The core logic is split into two functions:
#   - split_by_lines: reads content and divides into line-based chunks
#   - split_by_bytes: reads content and divides into byte-based chunks
#
# Both use a shared suffix-generation function to produce filenames.

require "coding_adventures_cli_builder"

SPLIT_SPEC_FILE = File.join(File.dirname(__FILE__), "split.json")

# ---------------------------------------------------------------------------
# Business Logic: split_generate_suffix
# ---------------------------------------------------------------------------
# Generate the Nth suffix string for the given mode and length.
#
# Parameters:
#   n             - The 0-based index of the output file.
#   suffix_length - Number of characters in the suffix (default: 2).
#   numeric       - Use numeric suffixes (0-9) instead of alphabetic (a-z).
#   hex           - Use hexadecimal suffixes (0-f).
#
# Examples:
#   split_generate_suffix(0, 2, false, false)  => "aa"
#   split_generate_suffix(1, 2, false, false)  => "ab"
#   split_generate_suffix(26, 2, false, false) => "ba"
#   split_generate_suffix(0, 2, true, false)   => "00"
#   split_generate_suffix(15, 2, false, true)  => "0f"
#
# Raises an error if n exceeds the maximum representable suffix.

def split_generate_suffix(n, suffix_length: 2, numeric: false, hex: false)
  if numeric
    # Numeric suffixes: base 10.
    max = 10**suffix_length
    raise "split: output file suffixes exhausted" if n >= max
    format("%0#{suffix_length}d", n)
  elsif hex
    # Hexadecimal suffixes: base 16.
    max = 16**suffix_length
    raise "split: output file suffixes exhausted" if n >= max
    format("%0#{suffix_length}x", n)
  else
    # Alphabetic suffixes: base 26 (a-z).
    max = 26**suffix_length
    raise "split: output file suffixes exhausted" if n >= max

    suffix = ""
    remaining = n
    suffix_length.times do
      suffix = ("a".ord + (remaining % 26)).chr + suffix
      remaining /= 26
    end
    suffix
  end
end

# ---------------------------------------------------------------------------
# Business Logic: split_parse_size
# ---------------------------------------------------------------------------
# Parse a size string like "1K", "10M", "1G" into bytes.
#
# Supported suffixes (case-insensitive):
#   K or KB = 1024
#   M or MB = 1048576
#   G or GB = 1073741824
#
# No suffix means bytes.

def split_parse_size(size_str)
  case size_str.to_s.strip
  when /\A(\d+)\s*[Kk][Bb]?\z/
    ::Regexp.last_match(1).to_i * 1024
  when /\A(\d+)\s*[Mm][Bb]?\z/
    ::Regexp.last_match(1).to_i * 1024 * 1024
  when /\A(\d+)\s*[Gg][Bb]?\z/
    ::Regexp.last_match(1).to_i * 1024 * 1024 * 1024
  when /\A(\d+)\z/
    ::Regexp.last_match(1).to_i
  else
    raise "split: invalid size '#{size_str}'"
  end
end

# ---------------------------------------------------------------------------
# Business Logic: split_by_lines
# ---------------------------------------------------------------------------
# Split content into chunks of N lines each.
#
# Parameters:
#   content - The full file content as a string.
#   n       - Number of lines per chunk.
#   prefix  - Output filename prefix (default: "x").
#   opts:
#     :suffix_length - Suffix length (default: 2).
#     :numeric       - Use numeric suffixes.
#     :hex           - Use hex suffixes.
#     :additional_suffix - Extra suffix to append (e.g., ".txt").
#
# Returns an array of [filename, chunk_content] pairs.

def split_by_lines(content, n, prefix = "x", opts = {})
  lines = content.lines
  chunks = []
  file_index = 0

  lines.each_slice(n) do |slice|
    suffix = split_generate_suffix(file_index,
                                    suffix_length: opts[:suffix_length] || 2,
                                    numeric: opts[:numeric] || false,
                                    hex: opts[:hex] || false)
    extra = opts[:additional_suffix] || ""
    filename = "#{prefix}#{suffix}#{extra}"
    chunks << [filename, slice.join]
    file_index += 1
  end

  chunks
end

# ---------------------------------------------------------------------------
# Business Logic: split_by_bytes
# ---------------------------------------------------------------------------
# Split content into chunks of N bytes each.
#
# Parameters:
#   content - The full file content as a string.
#   n       - Number of bytes per chunk.
#   prefix  - Output filename prefix (default: "x").
#   opts:   - Same as split_by_lines.
#
# Returns an array of [filename, chunk_content] pairs.

def split_by_bytes(content, n, prefix = "x", opts = {})
  chunks = []
  file_index = 0
  offset = 0
  bytes = content.b  # Force binary encoding for byte-accurate splitting.

  while offset < bytes.length
    chunk = bytes[offset, n]
    suffix = split_generate_suffix(file_index,
                                    suffix_length: opts[:suffix_length] || 2,
                                    numeric: opts[:numeric] || false,
                                    hex: opts[:hex] || false)
    extra = opts[:additional_suffix] || ""
    filename = "#{prefix}#{suffix}#{extra}"
    chunks << [filename, chunk]
    offset += n
    file_index += 1
  end

  chunks
end

# ---------------------------------------------------------------------------
# Business Logic: split_by_number
# ---------------------------------------------------------------------------
# Split content into exactly N chunks of roughly equal size.
#
# Parameters:
#   content - The full file content as a string.
#   n       - Number of chunks to produce.
#   prefix  - Output filename prefix.
#   opts:   - Same as split_by_lines.
#
# Returns an array of [filename, chunk_content] pairs.

def split_by_number(content, n, prefix = "x", opts = {})
  bytes = content.b
  total = bytes.length
  chunk_size = (total.to_f / n).ceil
  chunk_size = 1 if chunk_size < 1

  chunks = []
  file_index = 0
  offset = 0

  n.times do |idx|
    # Last chunk gets whatever remains.
    remaining = total - offset
    break if remaining <= 0

    size = if idx == n - 1
      remaining
    else
      [chunk_size, remaining].min
    end

    chunk = bytes[offset, size]
    suffix = split_generate_suffix(file_index,
                                    suffix_length: opts[:suffix_length] || 2,
                                    numeric: opts[:numeric] || false,
                                    hex: opts[:hex] || false)
    extra = opts[:additional_suffix] || ""
    filename = "#{prefix}#{suffix}#{extra}"
    chunks << [filename, chunk]
    offset += size
    file_index += 1
  end

  chunks
end

# ---------------------------------------------------------------------------
# Main entry point
# ---------------------------------------------------------------------------

def split_main
  begin
    result = CodingAdventures::CliBuilder::Parser.new(SPLIT_SPEC_FILE, ["split"] + ARGV).parse
  rescue CodingAdventures::CliBuilder::ParseErrors => e
    e.errors.each { |err| warn "split: #{err.message}" }
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
    file_path = result.arguments.fetch("file", "-")
    prefix = result.arguments.fetch("prefix", "x")

    # Read input.
    content = if file_path == "-"
      $stdin.read
    else
      File.read(file_path)
    end

    opts = {
      suffix_length: result.flags.fetch("suffix_length", 2).to_i,
      numeric: result.flags["numeric_suffixes"] || false,
      hex: result.flags["hex_suffixes"] || false,
      additional_suffix: result.flags["additional_suffix"],
    }

    verbose = result.flags["verbose"] || false

    # Determine split mode.
    chunks = if result.flags["bytes"]
      byte_count = split_parse_size(result.flags["bytes"])
      split_by_bytes(content, byte_count, prefix, opts)
    elsif result.flags["number"]
      num = result.flags["number"].to_i
      split_by_number(content, num, prefix, opts)
    else
      line_count = result.flags.fetch("lines", 1000).to_i
      split_by_lines(content, line_count, prefix, opts)
    end

    # Write output files.
    chunks.each do |filename, chunk_content|
      warn "creating file '#{filename}'" if verbose
      File.write(filename, chunk_content)
    end
  end
end

split_main if __FILE__ == $PROGRAM_NAME
