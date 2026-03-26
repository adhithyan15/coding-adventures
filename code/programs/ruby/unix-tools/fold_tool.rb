#!/usr/bin/env ruby
# frozen_string_literal: true

# fold_tool.rb -- Wrap each input line to fit in specified width
# ===============================================================
#
# === What This Program Does ===
#
# This is a reimplementation of the GNU `fold` utility. It wraps long
# lines by inserting newlines so that no output line exceeds a given
# width (default 80 columns).
#
# === Flags ===
#
#   -w N  Use N columns as the width (default 80).
#   -s    Break at spaces when possible.
#   -b    Count bytes rather than columns.
#
# === Break at Spaces (-s) ===
#
# Without -s, fold breaks at exactly the width limit, even mid-word.
# With -s, fold prefers to break at the last space before the limit.

require "coding_adventures_cli_builder"

FOLD_SPEC_FILE = File.join(File.dirname(__FILE__), "fold.json")

# ---------------------------------------------------------------------------
# Business Logic: fold_line
# ---------------------------------------------------------------------------

def fold_fold_line(line, width, break_at_spaces:, count_bytes:)
  return line if width <= 0

  result = []
  column = 0
  last_space_idx = -1
  segment_start = 0

  line.each_char do |ch|
    if ch == "\n"
      result << ch
      column = 0
      last_space_idx = -1
      segment_start = result.length
      next
    end

    advance = if count_bytes
                1
              elsif ch == "\t"
                8 - (column % 8)
              elsif ch == "\b"
                column > 0 ? -1 : 0
              else
                1
              end

    if column + advance > width
      if break_at_spaces && last_space_idx >= segment_start
        result.insert(last_space_idx + 1, "\n")
        after_break = result[(last_space_idx + 2)..].join
        column = 0
        after_break.each_char do |c|
          if count_bytes
            column += 1
          elsif c == "\t"
            column += 8 - (column % 8)
          elsif c == "\b"
            column = [0, column - 1].max
          else
            column += 1
          end
        end
        last_space_idx = -1
        segment_start = last_space_idx + 2
      else
        result << "\n"
        column = 0
        last_space_idx = -1
        segment_start = result.length
      end
    end

    result << ch
    column += advance
    last_space_idx = result.length - 1 if ch == " "
  end

  result.join
end

# ---------------------------------------------------------------------------
# Main entry point
# ---------------------------------------------------------------------------

def fold_main
  begin
    result = CodingAdventures::CliBuilder::Parser.new(FOLD_SPEC_FILE, ["fold"] + ARGV).parse
  rescue CodingAdventures::CliBuilder::ParseErrors => e
    e.errors.each { |err| warn "fold: #{err.message}" }
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
    width = result.flags["width"] || 80
    break_at_spaces = result.flags["spaces"] || false
    count_bytes = result.flags["bytes"] || false

    files = result.arguments.fetch("files", [])
    files = [files] if files.is_a?(String)
    files = ["-"] if files.empty?

    files.each do |filename|
      begin
        io = (filename == "-") ? $stdin : File.open(filename, "r")
        io.each_line do |line|
          stripped = line.chomp("\n")
          folded = fold_fold_line(stripped, width, break_at_spaces: break_at_spaces,
                                  count_bytes: count_bytes)
          $stdout.write(folded)
          $stdout.write("\n") if line.end_with?("\n")
        end
        io.close unless filename == "-"
      rescue Errno::ENOENT
        warn "fold: #{filename}: No such file or directory"
      end
    end
  end
end

fold_main if __FILE__ == $PROGRAM_NAME
