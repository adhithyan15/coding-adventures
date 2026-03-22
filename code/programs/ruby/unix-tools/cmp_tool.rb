#!/usr/bin/env ruby
# frozen_string_literal: true

# cmp_tool.rb -- Compare two files byte by byte
# ================================================
#
# === What This Program Does ===
#
# This is a reimplementation of the POSIX `cmp` utility. It compares
# two files byte by byte and reports the first difference found.
# Unlike `diff`, which works on lines, `cmp` works on raw bytes --
# making it suitable for comparing binary files.
#
# === How cmp Works ===
#
#     $ cmp file1.bin file2.bin
#     file1.bin file2.bin differ: byte 42, line 3
#
#     $ cmp -l file1.bin file2.bin    # list all differing bytes
#     42 101 102                      # byte_number octal_byte1 octal_byte2
#
#     $ cmp -s file1.bin file2.bin    # silent mode -- exit code only
#     $ echo $?
#     1
#
# === Exit Codes ===
#
#   0 - Files are identical
#   1 - Files differ
#   2 - An error occurred (e.g., file not found)
#
# === The -i and -n Flags ===
#
#   -i SKIP   Skip the first SKIP bytes. Can be SKIP1:SKIP2 to skip
#             different amounts in each file.
#   -n LIMIT  Compare at most LIMIT bytes.

require "coding_adventures_cli_builder"

CMP_SPEC_FILE = File.join(File.dirname(__FILE__), "cmp.json")

# ---------------------------------------------------------------------------
# Business Logic: parse_skip_value
# ---------------------------------------------------------------------------
# Parse the -i (ignore-initial) value. It can be a single number
# (same skip for both files) or SKIP1:SKIP2 (different skip per file).
#
# Returns: [skip1, skip2] as integers.

def cmp_parse_skip(skip_str)
  return [0, 0] unless skip_str

  parts = skip_str.to_s.split(":", 2)
  skip1 = parts[0].to_i
  skip2 = parts.length > 1 ? parts[1].to_i : skip1
  [skip1, skip2]
end

# ---------------------------------------------------------------------------
# Business Logic: cmp_compare
# ---------------------------------------------------------------------------
# Compare two IO streams byte by byte.
#
# Parameters:
#   io_a     - First IO stream (or file path string)
#   io_b     - Second IO stream (or file path string)
#   name_a   - Display name for the first file
#   name_b   - Display name for the second file
#   opts     - Hash of options:
#     :silent      - Produce no output; return exit code only
#     :list        - List all differing bytes (not just the first)
#     :print_bytes - Show the differing bytes as characters
#     :skip        - Skip string (e.g., "10" or "10:20")
#     :max_bytes   - Maximum number of bytes to compare
#
# Returns: [output_lines, exit_code]
#   exit_code: 0 = identical, 1 = differ, 2 = error

def cmp_compare(io_a, io_b, name_a, name_b, opts = {})
  skip1, skip2 = cmp_parse_skip(opts[:skip])
  max_bytes = opts[:max_bytes]

  # Skip initial bytes
  io_a.read(skip1) if skip1 > 0
  io_b.read(skip2) if skip2 > 0

  byte_number = 1  # 1-based byte position (after skip)
  line_number = 1  # Track line numbers for the default message
  output_lines = []
  files_differ = false
  bytes_compared = 0

  loop do
    break if max_bytes && bytes_compared >= max_bytes

    byte_a = io_a.read(1)
    byte_b = io_b.read(1)

    # Both streams exhausted: files are identical (up to this point)
    if byte_a.nil? && byte_b.nil?
      break
    end

    # One stream exhausted before the other: EOF mismatch
    if byte_a.nil? || byte_b.nil?
      eof_file = byte_a.nil? ? name_a : name_b
      unless opts[:silent]
        output_lines << "cmp: EOF on #{eof_file} after byte #{byte_number - 1}, line #{line_number}"
      end
      return [output_lines, 1]
    end

    if byte_a != byte_b
      files_differ = true

      unless opts[:silent]
        if opts[:list]
          # -l mode: list every differing byte
          # Format: byte_number octal_a octal_b
          line = "#{byte_number} #{byte_a.ord.to_s(8)} #{byte_b.ord.to_s(8)}"
          if opts[:print_bytes]
            line = "#{byte_number} #{byte_a.ord.to_s(8)} #{printable_char(byte_a)} #{byte_b.ord.to_s(8)} #{printable_char(byte_b)}"
          end
          output_lines << line
        else
          # Default mode: report the first difference and stop
          output_lines << "#{name_a} #{name_b} differ: byte #{byte_number}, line #{line_number}"
          return [output_lines, 1]
        end
      end
    end

    # Track line numbers (newline = 0x0A)
    line_number += 1 if byte_a == "\n"
    byte_number += 1
    bytes_compared += 1
  end

  exit_code = files_differ ? 1 : 0
  [output_lines, exit_code]
end

# ---------------------------------------------------------------------------
# Helper: printable_char
# ---------------------------------------------------------------------------
# Return a printable representation of a byte for -b output.

def printable_char(byte)
  c = byte.ord
  if c >= 32 && c < 127
    byte
  else
    "\\#{c.to_s(8).rjust(3, "0")}"
  end
end

# ---------------------------------------------------------------------------
# Main entry point
# ---------------------------------------------------------------------------

def cmp_main
  begin
    result = CodingAdventures::CliBuilder::Parser.new(CMP_SPEC_FILE, ["cmp"] + ARGV).parse
  rescue CodingAdventures::CliBuilder::ParseErrors => e
    e.errors.each { |err| warn "cmp: #{err.message}" }
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
    file1 = result.arguments["file1"]
    file2 = result.arguments.fetch("file2", "-")

    opts = {
      silent: result.flags["silent"] || false,
      list: result.flags["list"] || false,
      print_bytes: result.flags["print_bytes"] || false,
      skip: result.flags["ignore_initial"],
      max_bytes: result.flags["max_bytes"],
    }

    begin
      io_a = (file1 == "-") ? $stdin : File.open(file1, "rb")
      io_b = (file2 == "-") ? $stdin : File.open(file2, "rb")

      output, code = cmp_compare(io_a, io_b, file1, file2, opts)
      output.each { |line| puts line }

      io_a.close if file1 != "-"
      io_b.close if file2 != "-"

      exit code
    rescue Errno::ENOENT => e
      warn "cmp: #{e.message}"
      exit 2
    end
  end
end

cmp_main if __FILE__ == $PROGRAM_NAME
