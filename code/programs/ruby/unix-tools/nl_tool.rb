#!/usr/bin/env ruby
# frozen_string_literal: true

# nl_tool.rb -- Number lines of files
# =====================================
#
# === What This Program Does ===
#
# This is a reimplementation of the GNU `nl` utility. It reads text
# and writes it to standard output with line numbers added.
#
# === Numbering Styles ===
#
#   a        Number all lines.
#   t        Number only non-empty lines (default for body).
#   n        Don't number any lines (default for header/footer).
#   pREGEX   Number only lines matching the regex.
#
# === Number Format ===
#
#   ln  Left justified, no leading zeros.
#   rn  Right justified, no leading zeros (default).
#   rz  Right justified, with leading zeros.
#
# === Logical Pages ===
#
# Section delimiters: \:\:\: (header), \:\: (body), \: (footer).
# The delimiter characters can be changed with -d.

require "coding_adventures_cli_builder"

NL_SPEC_FILE = File.join(File.dirname(__FILE__), "nl.json")

# ---------------------------------------------------------------------------
# Business Logic: should_number_line
# ---------------------------------------------------------------------------

def nl_should_number(line, style)
  case style
  when "a" then true
  when "t" then !line.strip.empty?
  when "n" then false
  else
    if style.start_with?("p")
      pattern = style[1..]
      !!(line =~ /#{pattern}/)
    else
      false
    end
  end
end

# ---------------------------------------------------------------------------
# Business Logic: format_number
# ---------------------------------------------------------------------------

def nl_format_number(num, fmt, width)
  case fmt
  when "ln" then num.to_s.ljust(width)
  when "rz" then num.to_s.rjust(width, "0")
  else num.to_s.rjust(width) # rn
  end
end

# ---------------------------------------------------------------------------
# Business Logic: detect_section
# ---------------------------------------------------------------------------

def nl_detect_section(line, delim)
  stripped = line.chomp("\n")
  return "header" if stripped == delim * 3
  return "body" if stripped == delim * 2
  return "footer" if stripped == delim
  nil
end

# ---------------------------------------------------------------------------
# Business Logic: number_lines
# ---------------------------------------------------------------------------

def nl_number_lines(lines, body_style:, header_style:, footer_style:,
                    start_number:, increment:, number_format:, number_width:,
                    separator:, section_delimiter:)
  result = []
  current_number = start_number
  current_section = "body"

  style_map = {"header" => header_style, "body" => body_style, "footer" => footer_style}

  lines.each do |line|
    section = nl_detect_section(line, section_delimiter)
    if section
      current_section = section
      current_number = start_number if section == "header"
      result << ""
      next
    end

    style = style_map[current_section]

    if nl_should_number(line, style)
      num_str = nl_format_number(current_number, number_format, number_width)
      result << "#{num_str}#{separator}#{line}"
      current_number += increment
    else
      blank = " " * number_width
      result << "#{blank}#{separator}#{line}"
    end
  end

  result
end

# ---------------------------------------------------------------------------
# Main entry point
# ---------------------------------------------------------------------------

def nl_main
  begin
    result = CodingAdventures::CliBuilder::Parser.new(NL_SPEC_FILE, ["nl"] + ARGV).parse
  rescue CodingAdventures::CliBuilder::ParseErrors => e
    e.errors.each { |err| warn "nl: #{err.message}" }
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
    body_style = result.flags["body_numbering"] || "t"
    header_style = result.flags["header_numbering"] || "n"
    footer_style = result.flags["footer_numbering"] || "n"
    start_number = result.flags["starting_line_number"] || 1
    line_increment = result.flags["line_increment"] || 1
    number_format = result.flags["number_format"] || "rn"
    number_width = result.flags["number_width"] || 6
    separator = result.flags["number_separator"] || "\t"
    section_delimiter = result.flags["section_delimiter"] || "\\:"

    input_file = result.arguments["file"]

    raw_lines = if input_file && input_file != "-"
                  begin
                    File.readlines(input_file)
                  rescue Errno::ENOENT
                    warn "nl: #{input_file}: No such file or directory"
                    exit 1
                  end
                else
                  $stdin.readlines
                end

    lines = raw_lines.map { |l| l.chomp("\n") }

    output = nl_number_lines(lines, body_style: body_style, header_style: header_style,
                             footer_style: footer_style, start_number: start_number,
                             increment: line_increment, number_format: number_format,
                             number_width: number_width, separator: separator,
                             section_delimiter: section_delimiter)

    output.each { |l| puts l }
  end
end

nl_main if __FILE__ == $PROGRAM_NAME
