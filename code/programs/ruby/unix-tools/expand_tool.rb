#!/usr/bin/env ruby
# frozen_string_literal: true

# expand_tool.rb -- Convert tabs to spaces
# ==========================================
#
# === What This Program Does ===
#
# This is a reimplementation of the GNU `expand` utility. It replaces
# tab characters with spaces to reach the next tab stop.
#
# === How Tab Stops Work ===
#
# A tab doesn't represent a fixed number of spaces. It advances the
# cursor to the next tab stop position. With 8-column tab stops:
# positions 0, 8, 16, 24, etc.
#
# Formula: spaces_needed = tab_size - (column % tab_size)
#
# === Custom Tab Stops ===
#
#   -t 4      Tab stops every 4 columns.
#   -t 4,8,12 Explicit tab stop positions.
#
# === The -i Flag (Initial Only) ===
#
# Only expand tabs at the beginning of each line (before non-blank chars).

require "coding_adventures_cli_builder"

EXPAND_SPEC_FILE = File.join(File.dirname(__FILE__), "expand.json")

# ---------------------------------------------------------------------------
# Business Logic: parse_tab_stops
# ---------------------------------------------------------------------------

def expand_parse_tab_stops(tab_str)
  return 8 if tab_str.nil?

  if tab_str.include?(",")
    tab_str.split(",").map { |s| s.strip.to_i }.sort
  else
    tab_str.to_i
  end
end

# ---------------------------------------------------------------------------
# Business Logic: spaces_to_next_stop
# ---------------------------------------------------------------------------

def expand_spaces_to_next_stop(column, tab_stops)
  if tab_stops.is_a?(Integer)
    tab_stops - (column % tab_stops)
  else
    tab_stops.each { |stop| return stop - column if stop > column }
    1 # Past the last explicit stop.
  end
end

# ---------------------------------------------------------------------------
# Business Logic: expand_line
# ---------------------------------------------------------------------------

def expand_expand_line(line, tab_stops, initial_only:)
  result = []
  column = 0
  seen_non_blank = false

  line.each_char do |ch|
    if ch == "\t"
      if initial_only && seen_non_blank
        result << "\t"
        column += 1
      else
        num_spaces = expand_spaces_to_next_stop(column, tab_stops)
        result << (" " * num_spaces)
        column += num_spaces
      end
    elsif ch == "\n"
      result << ch
      column = 0
      seen_non_blank = false
    else
      seen_non_blank = true if ch != " "
      result << ch
      column += 1
    end
  end

  result.join
end

# ---------------------------------------------------------------------------
# Main entry point
# ---------------------------------------------------------------------------

def expand_main
  begin
    result = CodingAdventures::CliBuilder::Parser.new(EXPAND_SPEC_FILE, ["expand"] + ARGV).parse
  rescue CodingAdventures::CliBuilder::ParseErrors => e
    e.errors.each { |err| warn "expand: #{err.message}" }
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
    tab_str = result.flags["tabs"]
    initial_only = result.flags["initial"] || false

    tab_stops = expand_parse_tab_stops(tab_str)

    files = result.arguments.fetch("files", [])
    files = [files] if files.is_a?(String)
    files = ["-"] if files.empty?

    files.each do |filename|
      begin
        io = (filename == "-") ? $stdin : File.open(filename, "r")
        io.each_line do |line|
          $stdout.write(expand_expand_line(line, tab_stops, initial_only: initial_only))
        end
        io.close unless filename == "-"
      rescue Errno::ENOENT
        warn "expand: #{filename}: No such file or directory"
      end
    end
  end
end

expand_main if __FILE__ == $PROGRAM_NAME
