#!/usr/bin/env ruby
# frozen_string_literal: true

# uniq_tool.rb -- Report or omit repeated lines
# ===============================================
#
# === What This Program Does ===
#
# This is a reimplementation of the GNU `uniq` utility. It filters
# ADJACENT duplicate lines from its input. Non-adjacent duplicates
# are not affected -- sort the input first if needed.
#
# === Output Modes ===
#
#   Default: Print one copy of each group of adjacent identical lines.
#   -d:      Print only lines that appear more than once.
#   -u:      Print only lines that appear exactly once.
#   -c:      Prefix each output line with its occurrence count.
#
# === Comparison Modifiers ===
#
#   -i:  Ignore case differences.
#   -f N: Skip the first N fields before comparing.
#   -s N: Skip the first N characters before comparing.
#   -w N: Compare at most N characters.

require "coding_adventures_cli_builder"

UNIQ_SPEC_FILE = File.join(File.dirname(__FILE__), "uniq.json")

# ---------------------------------------------------------------------------
# Business Logic: get_comparison_key
# ---------------------------------------------------------------------------

def uniq_comparison_key(line, skip_fields:, skip_chars:, check_chars:, ignore_case:)
  s = line

  # Skip fields (whitespace-delimited).
  if skip_fields > 0
    remaining = s
    skip_fields.times do
      remaining = remaining.lstrip
      idx = remaining.index(/[ \t]/) || remaining.length
      remaining = remaining[idx..]
    end
    s = remaining || ""
  end

  # Skip characters.
  s = s[skip_chars..] || "" if skip_chars > 0

  # Limit comparison width.
  s = s[0, check_chars] if check_chars

  # Fold case.
  s = s.downcase if ignore_case

  s
end

# ---------------------------------------------------------------------------
# Business Logic: uniq_lines
# ---------------------------------------------------------------------------

def uniq_filter_lines(lines, count:, repeated:, unique:, ignore_case:,
                      skip_fields:, skip_chars:, check_chars:)
  return [] if lines.empty?

  result = []

  current_line = lines[0]
  current_key = uniq_comparison_key(current_line, skip_fields: skip_fields,
                                    skip_chars: skip_chars, check_chars: check_chars,
                                    ignore_case: ignore_case)
  current_count = 1

  emit = lambda do |line, group_count|
    return if repeated && group_count < 2
    return if unique && group_count > 1
    if count
      result << format("%7d %s", group_count, line)
    else
      result << line
    end
  end

  lines[1..].each do |line|
    key = uniq_comparison_key(line, skip_fields: skip_fields,
                              skip_chars: skip_chars, check_chars: check_chars,
                              ignore_case: ignore_case)

    if key == current_key
      current_count += 1
    else
      emit.call(current_line, current_count)
      current_line = line
      current_key = key
      current_count = 1
    end
  end

  emit.call(current_line, current_count)
  result
end

# ---------------------------------------------------------------------------
# Main entry point
# ---------------------------------------------------------------------------

def uniq_main
  begin
    result = CodingAdventures::CliBuilder::Parser.new(UNIQ_SPEC_FILE, ["uniq"] + ARGV).parse
  rescue CodingAdventures::CliBuilder::ParseErrors => e
    e.errors.each { |err| warn "uniq: #{err.message}" }
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
    count_flag = result.flags["count"] || false
    repeated = result.flags["repeated"] || false
    unique_flag = result.flags["unique"] || false
    ignore_case = result.flags["ignore_case"] || false
    skip_fields = result.flags["skip_fields"] || 0
    skip_chars = result.flags["skip_chars"] || 0
    check_chars = result.flags["check_chars"]

    input_file = result.arguments["input_file"]
    output_file = result.arguments["output_file"]

    # Read input.
    raw_lines = if input_file && input_file != "-"
                  begin
                    File.readlines(input_file)
                  rescue Errno::ENOENT
                    warn "uniq: #{input_file}: No such file or directory"
                    exit 1
                  end
                else
                  $stdin.readlines
                end

    lines = raw_lines.map { |l| l.chomp("\n") }

    output_lines = uniq_filter_lines(lines, count: count_flag, repeated: repeated,
                                     unique: unique_flag, ignore_case: ignore_case,
                                     skip_fields: skip_fields, skip_chars: skip_chars,
                                     check_chars: check_chars)

    if output_file
      File.open(output_file, "w") { |f| output_lines.each { |l| f.puts l } }
    else
      output_lines.each { |l| puts l }
    end
  end
end

uniq_main if __FILE__ == $PROGRAM_NAME
