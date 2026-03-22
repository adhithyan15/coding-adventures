#!/usr/bin/env ruby
# frozen_string_literal: true

# rev_tool.rb -- Reverse lines characterwise
# =============================================
#
# === What This Program Does ===
#
# This is a reimplementation of the `rev` utility. It reads each line
# from the input (files or stdin) and prints it with the characters
# in reverse order.
#
# === Examples ===
#
#     $ echo "hello" | rev
#     olleh
#
#     $ echo "12345" | rev
#     54321
#
#     $ printf "abc\ndef\n" | rev
#     cba
#     fed
#
# === How rev Works ===
#
# The algorithm is trivially simple:
#
# 1. Read each line from input.
# 2. Strip the trailing newline (if any).
# 3. Reverse the characters.
# 4. Print the reversed line followed by a newline.
#
# The simplicity of rev makes it a great introductory Unix tool.
# Despite its simplicity, it's genuinely useful -- for example,
# to reverse the order of fields in a delimited string when
# combined with `cut`.
#
# === Unicode Considerations ===
#
# Ruby's String#reverse operates on characters (Unicode code points),
# not bytes. So reversing a string with multi-byte UTF-8 characters
# works correctly:
#
#     "cafe\u0301".reverse  => "\u0301efac"  (combining marks may shift)
#
# Note: combining characters (like accent marks) don't reverse
# "correctly" in a visual sense, but this matches the behavior of
# GNU rev.

require "coding_adventures_cli_builder"

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

REV_SPEC_FILE = File.join(File.dirname(__FILE__), "rev.json")

# ---------------------------------------------------------------------------
# Business Logic: rev_stream
# ---------------------------------------------------------------------------
# Reverse each line in an IO stream and print to stdout.
#
# Parameters:
#   io - An IO object to read from

def rev_stream(io)
  io.each_line do |line|
    # Strip the trailing newline, reverse, then add it back.
    # This ensures the output always ends with a newline, matching
    # the behavior of GNU rev.
    puts line.chomp.reverse
  end
end

# ---------------------------------------------------------------------------
# Main entry point
# ---------------------------------------------------------------------------

def rev_main
  # --- Step 1: Parse arguments ---------------------------------------------
  begin
    result = CodingAdventures::CliBuilder::Parser.new(REV_SPEC_FILE, ["rev"] + ARGV).parse
  rescue CodingAdventures::CliBuilder::ParseErrors => e
    e.errors.each { |err| warn "rev: #{err.message}" }
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

    files.each do |filename|
      if filename == "-"
        rev_stream($stdin)
      else
        begin
          File.open(filename, "r") do |f|
            rev_stream(f)
          end
        rescue Errno::ENOENT
          warn "rev: cannot open '#{filename}': No such file or directory"
        rescue Errno::EACCES
          warn "rev: cannot open '#{filename}': Permission denied"
        rescue Errno::EISDIR
          warn "rev: '#{filename}': Is a directory"
        end
      end
    end
  end
end

# Only run main when this file is executed directly.
rev_main if __FILE__ == $PROGRAM_NAME
