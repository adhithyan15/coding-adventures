#!/usr/bin/env ruby
# frozen_string_literal: true

# paste_tool.rb -- Merge lines of files
# ========================================
#
# === What This Program Does ===
#
# This is a reimplementation of the GNU `paste` utility. It merges
# corresponding lines from multiple files, joining them with a
# delimiter (tab by default).
#
# === Parallel Mode (default) ===
#
# In parallel mode, paste reads one line from each file and joins
# them with the delimiter. If files have different lengths, missing
# lines are treated as empty strings.
#
#     $ paste file1 file2
#     a    1
#     b    2
#     c    3
#
# This is equivalent to a "zip" operation on the files' lines.
#
# === Serial Mode (-s) ===
#
# In serial mode, paste processes one file at a time. All lines from
# a single file are joined into one output line.
#
#     $ paste -s file1 file2
#     a    b    c
#     1    2    3
#
# === Delimiter List (-d) ===
#
# The -d flag specifies a list of delimiters that are used cyclically.
# For example, -d ",:" alternates between comma and colon:
#
#     $ paste -d ",:" f1 f2 f3
#     a,1:x
#     b,2:y
#
# Special escape sequences in the delimiter list:
#   \n  → newline
#   \t  → tab
#   \\  → literal backslash
#   \0  → empty string (no delimiter)

require "coding_adventures_cli_builder"

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

PASTE_SPEC_FILE = File.join(File.dirname(__FILE__), "paste.json")

# ---------------------------------------------------------------------------
# Business Logic: parse_delimiters
# ---------------------------------------------------------------------------
# Parse the delimiter list string, handling escape sequences.
# The delimiter list is used cyclically when pasting multiple files.
#
# Parameters:
#   delim_str - The delimiter string (e.g., ",:" or "\\t\\n")
#
# Returns: An array of delimiter characters.

def parse_delimiters(delim_str)
  return ["\t"] if delim_str.nil? || delim_str.empty?

  delimiters = []
  i = 0
  while i < delim_str.length
    if delim_str[i] == "\\"
      # Escape sequence
      i += 1
      case delim_str[i]
      when "n" then delimiters << "\n"
      when "t" then delimiters << "\t"
      when "\\" then delimiters << "\\"
      when "0" then delimiters << ""
      else
        # Unknown escape: use the character literally
        delimiters << (delim_str[i] || "\\")
      end
    else
      delimiters << delim_str[i]
    end
    i += 1
  end

  delimiters
end

# ---------------------------------------------------------------------------
# Business Logic: paste_parallel
# ---------------------------------------------------------------------------
# Merge lines from multiple IO streams in parallel (zip mode).
# Each output line consists of one line from each input, joined by
# delimiters. The delimiter cycles through the delimiter list.
#
# Parameters:
#   ios        - Array of IO objects to read from
#   delimiters - Array of delimiter characters (cycled)
#   line_sep   - Line separator for output ("\n" or "\0")
#
# Returns: An array of output lines (without trailing separator).

def paste_parallel(ios, delimiters, line_sep)
  # Read all lines from each stream
  all_lines = ios.map do |io|
    lines = []
    io.each_line(line_sep) { |l| lines << l.chomp(line_sep) }
    lines
  end

  # Find the longest file
  max_len = all_lines.map(&:length).max || 0
  return [] if max_len == 0

  output = []
  (0...max_len).each do |line_idx|
    parts = all_lines.each_with_index.map do |file_lines, file_idx|
      line = file_lines[line_idx] || ""
      delim = if file_idx < all_lines.length - 1
                delimiters[file_idx % delimiters.length]
              else
                ""
              end
      [line, delim]
    end

    # Join: line1 + delim1 + line2 + delim2 + ... + lineN
    result = ""
    parts.each_with_index do |(line, delim), idx|
      result += line
      result += delim if idx < parts.length - 1
    end

    output << result
  end

  output
end

# ---------------------------------------------------------------------------
# Business Logic: paste_serial
# ---------------------------------------------------------------------------
# Merge lines from each file serially. Each file's lines are joined
# into a single output line, using the delimiter list cyclically.
#
# Parameters:
#   ios        - Array of IO objects to read from
#   delimiters - Array of delimiter characters (cycled)
#   line_sep   - Line separator for input ("\n" or "\0")
#
# Returns: An array of output lines.

def paste_serial(ios, delimiters, line_sep)
  output = []

  ios.each do |io|
    lines = []
    io.each_line(line_sep) { |l| lines << l.chomp(line_sep) }

    # Join all lines from this file with cycling delimiters
    if lines.empty?
      output << ""
    else
      result = lines[0]
      (1...lines.length).each do |i|
        delim = delimiters[(i - 1) % delimiters.length]
        result += delim + lines[i]
      end
      output << result
    end
  end

  output
end

# ---------------------------------------------------------------------------
# Main entry point
# ---------------------------------------------------------------------------

def paste_main
  # --- Step 1: Parse arguments ---------------------------------------------
  begin
    result = CodingAdventures::CliBuilder::Parser.new(PASTE_SPEC_FILE, ["paste"] + ARGV).parse
  rescue CodingAdventures::CliBuilder::ParseErrors => e
    e.errors.each { |err| warn "paste: #{err.message}" }
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
    delimiters = parse_delimiters(result.flags["delimiters"])
    serial = result.flags["serial"] || false
    line_sep = result.flags["zero_terminated"] ? "\0" : "\n"

    # Open all files
    ios = files.map do |filename|
      if filename == "-"
        $stdin
      else
        begin
          File.open(filename, "r")
        rescue Errno::ENOENT
          warn "paste: #{filename}: No such file or directory"
          exit 1
        end
      end
    end

    output = if serial
               paste_serial(ios, delimiters, line_sep)
             else
               paste_parallel(ios, delimiters, line_sep)
             end

    output.each { |line| print line + line_sep }

    # Close opened files
    ios.each { |io| io.close if io != $stdin }
  end
end

# Only run main when this file is executed directly.
paste_main if __FILE__ == $PROGRAM_NAME
