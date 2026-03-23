#!/usr/bin/env ruby
# frozen_string_literal: true

# unexpand_tool.rb -- Convert spaces to tabs
# ============================================
#
# === What This Program Does ===
#
# This is a reimplementation of the GNU `unexpand` utility. It is the
# inverse of `expand`: it replaces runs of spaces with tab characters
# where possible, based on tab stop positions.
#
# === Default Behavior ===
#
# By default, unexpand only converts spaces at the beginning of each
# line (before any non-blank character).
#
# === The -a Flag (All Blanks) ===
#
# With -a, unexpand converts spaces throughout the entire line.
#
# === Tab Stops ===
#
#   -t 4       Tab stops every 4 columns.
#   -t 4,8,12  Explicit tab stop positions.

require "coding_adventures_cli_builder"

UNEXPAND_SPEC_FILE = File.join(File.dirname(__FILE__), "unexpand.json")

# ---------------------------------------------------------------------------
# Business Logic: parse_tab_stops
# ---------------------------------------------------------------------------

def unexpand_parse_tab_stops(tab_str)
  return 8 if tab_str.nil?
  if tab_str.include?(",")
    tab_str.split(",").map { |s| s.strip.to_i }.sort
  else
    tab_str.to_i
  end
end

# ---------------------------------------------------------------------------
# Business Logic: is_tab_stop
# ---------------------------------------------------------------------------

def unexpand_is_tab_stop(column, tab_stops)
  if tab_stops.is_a?(Integer)
    (column % tab_stops) == 0
  else
    tab_stops.include?(column)
  end
end

# ---------------------------------------------------------------------------
# Business Logic: unexpand_line
# ---------------------------------------------------------------------------

def unexpand_unexpand_line(line, tab_stops, convert_all:)
  result = []
  column = 0
  space_count = 0
  seen_non_blank = false

  line.each_char do |ch|
    if ch == " " && (convert_all || !seen_non_blank)
      space_count += 1
      column += 1

      if unexpand_is_tab_stop(column, tab_stops) && space_count > 1
        result << "\t"
        space_count = 0
      end
    elsif ch == "\t"
      result << (" " * space_count) if space_count > 0
      space_count = 0
      result << "\t"
      if tab_stops.is_a?(Integer)
        column += (tab_stops - column % tab_stops)
      else
        next_stop = tab_stops.find { |s| s > column } || (column + 1)
        column = next_stop
      end
    else
      if space_count > 0
        result << (" " * space_count)
        space_count = 0
      end
      seen_non_blank = true if ch != " "
      result << ch
      if ch == "\n"
        column = 0
        seen_non_blank = false
      else
        column += 1
      end
    end
  end

  result << (" " * space_count) if space_count > 0
  result.join
end

# ---------------------------------------------------------------------------
# Main entry point
# ---------------------------------------------------------------------------

def unexpand_main
  begin
    result = CodingAdventures::CliBuilder::Parser.new(UNEXPAND_SPEC_FILE, ["unexpand"] + ARGV).parse
  rescue CodingAdventures::CliBuilder::ParseErrors => e
    e.errors.each { |err| warn "unexpand: #{err.message}" }
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
    convert_all = result.flags["all"] || false

    tab_stops = unexpand_parse_tab_stops(tab_str)

    files = result.arguments.fetch("files", [])
    files = [files] if files.is_a?(String)
    files = ["-"] if files.empty?

    files.each do |filename|
      begin
        io = (filename == "-") ? $stdin : File.open(filename, "r")
        io.each_line do |line|
          $stdout.write(unexpand_unexpand_line(line, tab_stops, convert_all: convert_all))
        end
        io.close unless filename == "-"
      rescue Errno::ENOENT
        warn "unexpand: #{filename}: No such file or directory"
      end
    end
  end
end

unexpand_main if __FILE__ == $PROGRAM_NAME
