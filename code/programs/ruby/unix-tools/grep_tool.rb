#!/usr/bin/env ruby
# frozen_string_literal: true

# grep_tool.rb -- Print lines that match patterns
# ==================================================
#
# === What This Program Does ===
#
# This is a reimplementation of the GNU `grep` utility. It searches
# input files for lines matching a given pattern, and prints those
# lines to standard output. The name comes from the ed editor command
# "g/re/p" (globally search for a regular expression and print).
#
# === How grep Works ===
#
#     $ grep "error" logfile.txt         # find lines containing "error"
#     $ grep -i "warning" *.log          # case-insensitive search
#     $ grep -rn "TODO" src/             # recursive search with line numbers
#     $ grep -c "pattern" file           # count matching lines
#     $ grep -v "comment" file           # print non-matching lines
#
# === Regular Expressions ===
#
# By default, grep uses basic regular expressions (BRE). Special
# characters like +, ?, |, (, ) must be escaped. With -E (extended),
# these work without escaping. With -F (fixed strings), no regex
# interpretation happens at all.
#
# Examples of patterns:
#   "hello"        - literal string
#   "^start"       - lines starting with "start"
#   "end$"         - lines ending with "end"
#   "a.b"          - "a", any character, "b"
#   "[0-9]+"       - one or more digits (with -E)
#
# === Matching Modes ===
#
#   -w  Whole word matching (pattern must be bounded by word boundaries)
#   -x  Whole line matching (pattern must match the entire line)
#   -v  Invert match (print lines that do NOT match)
#
# === Output Modes ===
#
#   -c  Count: print only the count of matching lines
#   -l  Files-with-matches: print only filenames containing matches
#   -L  Files-without-match: print only filenames with no matches
#   -o  Only-matching: print only the matched portion of each line
#   -n  Line numbers: prefix each line with its line number
#   -H  With filename: prefix each line with the filename
#   -h  No filename: suppress the filename prefix
#
# === Implementation ===
#
# We use Ruby's Regexp class for pattern matching. The grep_match
# function handles a single line, while grep_file processes an entire
# file. This separation makes each piece independently testable.

require "coding_adventures_cli_builder"

GREP_SPEC_FILE = File.join(File.dirname(__FILE__), "grep.json")

# ---------------------------------------------------------------------------
# Business Logic: grep_build_pattern
# ---------------------------------------------------------------------------
# Build a Regexp from the user's pattern string and flags.
#
# This handles the different regex modes:
#   - Basic regexp (default): Ruby's Regexp is already extended-compatible,
#     so we use it directly.
#   - Extended regexp (-E): same as default in Ruby.
#   - Fixed strings (-F): escape all regex metacharacters.
#   - Perl regexp (-P): Ruby's Regexp is Perl-compatible enough.
#
# Additional modifiers:
#   -w  Wrap pattern in \b...\b for word boundary matching.
#   -x  Wrap pattern in ^...\$ for full-line matching.
#   -i  Case-insensitive matching.

def grep_build_pattern(pattern_str, opts = {})
  # Fixed strings: escape all regex metacharacters.
  pat = if opts[:fixed_strings]
    Regexp.escape(pattern_str)
  else
    pattern_str
  end

  # Word boundary matching.
  pat = "\\b(?:#{pat})\\b" if opts[:word_regexp]

  # Full-line matching.
  pat = "\\A(?:#{pat})\\z" if opts[:line_regexp]

  # Build the Regexp with optional case-insensitivity.
  flags = opts[:ignore_case] ? Regexp::IGNORECASE : 0
  Regexp.new(pat, flags)
end

# ---------------------------------------------------------------------------
# Business Logic: grep_match
# ---------------------------------------------------------------------------
# Test whether a single line matches the given pattern.
#
# Parameters:
#   line    - The text line to test (without trailing newline).
#   pattern - A compiled Regexp object.
#   opts    - Options hash:
#     :invert_match - Return true when the line does NOT match.
#
# Returns true if the line matches (or doesn't match, if inverted).

def grep_match(line, pattern, opts = {})
  matched = pattern.match?(line)
  opts[:invert_match] ? !matched : matched
end

# ---------------------------------------------------------------------------
# Business Logic: grep_file
# ---------------------------------------------------------------------------
# Search a file (or array of lines) for matches and return results.
#
# Parameters:
#   source  - A file path (String) or an Array of lines.
#   pattern - A compiled Regexp object.
#   opts    - Options hash:
#     :invert_match      - Invert matching sense
#     :count             - Return only the match count
#     :files_with_matches - Return filename if any match found
#     :files_without_match - Return filename if no match found
#     :only_matching     - Return only the matched portions
#     :line_number       - Include line numbers
#     :max_count         - Stop after this many matches
#     :with_filename     - Prefix output with filename
#     :filename          - The filename to display (for stdin or override)
#
# Returns an array of formatted result strings, or a count, etc.

def grep_file(source, pattern, opts = {})
  # --- Read lines from source ---------------------------------------------------
  lines = if source.is_a?(Array)
    source
  else
    begin
      File.readlines(source, chomp: true)
    rescue Errno::ENOENT
      warn "grep: #{source}: No such file or directory"
      return nil
    rescue Errno::EISDIR
      warn "grep: #{source}: Is a directory"
      return nil
    rescue Errno::EACCES
      warn "grep: #{source}: Permission denied"
      return nil
    end
  end

  filename = opts[:filename] || (source.is_a?(String) ? source : nil)
  results = []
  match_count = 0

  lines.each_with_index do |line, idx|
    # Check if we've hit the max count.
    break if opts[:max_count] && match_count >= opts[:max_count]

    if grep_match(line, pattern, opts)
      match_count += 1

      # For count-only mode, just count.
      next if opts[:count]
      # For files-with-matches mode, we can stop at the first match.
      if opts[:files_with_matches]
        return [filename] if filename
        return []
      end

      # Build the output line.
      if opts[:only_matching] && !opts[:invert_match]
        # Print only the matched portions.
        line.scan(pattern).each do |m|
          match_str = m.is_a?(Array) ? m.first : m
          parts = []
          parts << "#{filename}:" if opts[:with_filename] && filename
          parts << "#{idx + 1}:" if opts[:line_number]
          parts << match_str
          results << parts.join
        end
      else
        parts = []
        parts << "#{filename}:" if opts[:with_filename] && filename
        parts << "#{idx + 1}:" if opts[:line_number]
        parts << line
        results << parts.join
      end
    end
  end

  # --- Handle count mode -------------------------------------------------------
  if opts[:count]
    prefix = opts[:with_filename] && filename ? "#{filename}:" : ""
    return ["#{prefix}#{match_count}"]
  end

  # --- Handle files-with-matches (no matches found) ----------------------------
  return [] if opts[:files_with_matches]

  # --- Handle files-without-match ----------------------------------------------
  if opts[:files_without_match]
    return match_count == 0 && filename ? [filename] : []
  end

  results
end

# ---------------------------------------------------------------------------
# Main entry point
# ---------------------------------------------------------------------------

def grep_main
  begin
    result = CodingAdventures::CliBuilder::Parser.new(GREP_SPEC_FILE, ["grep"] + ARGV).parse
  rescue CodingAdventures::CliBuilder::ParseErrors => e
    e.errors.each { |err| warn "grep: #{err.message}" }
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
    pattern_str = result.arguments["pattern"] || ""
    # Support -e flag for multiple patterns.
    regexp_patterns = result.flags["regexp"]
    if regexp_patterns
      regexp_patterns = [regexp_patterns] if regexp_patterns.is_a?(String)
      pattern_str = regexp_patterns.join("|")
    end

    build_opts = {
      fixed_strings: result.flags["fixed_strings"] || false,
      ignore_case: result.flags["ignore_case"] || false,
      word_regexp: result.flags["word_regexp"] || false,
      line_regexp: result.flags["line_regexp"] || false,
    }

    pattern = grep_build_pattern(pattern_str, build_opts)

    files = result.arguments.fetch("files", [])
    files = [files] if files.is_a?(String)

    # Determine filename display mode.
    multi_file = files.length > 1
    with_filename = if result.flags["with_filename"]
      true
    elsif result.flags["no_filename"]
      false
    else
      multi_file
    end

    search_opts = {
      invert_match: result.flags["invert_match"] || false,
      count: result.flags["count"] || false,
      files_with_matches: result.flags["files_with_matches"] || false,
      files_without_match: result.flags["files_without_match"] || false,
      only_matching: result.flags["only_matching"] || false,
      line_number: result.flags["line_number"] || false,
      max_count: result.flags["max_count"],
      with_filename: with_filename,
    }

    found_any = false

    if files.empty?
      # Read from stdin.
      lines = $stdin.readlines(chomp: true)
      results = grep_file(lines, pattern, search_opts)
      if results && !results.empty?
        found_any = true
        results.each { |r| puts r }
      end
    else
      files.each do |file|
        results = grep_file(file, pattern, search_opts)
        next unless results
        unless results.empty?
          found_any = true
          results.each { |r| puts r }
        end
      end
    end

    exit(found_any ? 0 : 1)
  end
end

grep_main if __FILE__ == $PROGRAM_NAME
