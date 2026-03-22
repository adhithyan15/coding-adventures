#!/usr/bin/env ruby
# frozen_string_literal: true

# tr_tool.rb -- Translate or delete characters
# ==============================================
#
# === What This Program Does ===
#
# This is a reimplementation of the GNU `tr` utility. It reads from
# standard input, transforms characters, and writes to standard output.
#
# === Operations ===
#
# 1. Translate: Replace SET1 characters with SET2 characters
# 2. Delete (-d): Remove all SET1 characters
# 3. Squeeze (-s): Collapse runs of repeated characters
#
# === Character Sets ===
#
# Sets support ranges (a-z), character classes ([:upper:]), and
# escape sequences (\n, \t, etc.).
#
# === The -c Flag (Complement) ===
#
# With -c, SET1 is replaced by its complement -- all characters NOT in SET1.

require "coding_adventures_cli_builder"

TR_SPEC_FILE = File.join(File.dirname(__FILE__), "tr.json")

# ---------------------------------------------------------------------------
# Business Logic: expand_escapes
# ---------------------------------------------------------------------------

def tr_expand_escapes(s)
  escape_map = {"n" => "\n", "t" => "\t", "r" => "\r", "a" => "\a",
                "b" => "\b", "f" => "\f", "v" => "\v", "\\" => "\\"}
  result = []
  i = 0
  while i < s.length
    if s[i] == "\\" && i + 1 < s.length && escape_map.key?(s[i + 1])
      result << escape_map[s[i + 1]]
      i += 2
    else
      result << s[i]
      i += 1
    end
  end
  result.join
end

# ---------------------------------------------------------------------------
# Business Logic: expand_set
# ---------------------------------------------------------------------------

def tr_expand_set(set_str)
  set_str = tr_expand_escapes(set_str)

  class_map = {
    "[:upper:]" => ("A".."Z").to_a.join,
    "[:lower:]" => ("a".."z").to_a.join,
    "[:digit:]" => ("0".."9").to_a.join,
    "[:alpha:]" => ("A".."Z").to_a.join + ("a".."z").to_a.join,
    "[:alnum:]" => ("A".."Z").to_a.join + ("a".."z").to_a.join + ("0".."9").to_a.join,
    "[:space:]" => " \t\n\r\f\v",
    "[:blank:]" => " \t",
    "[:punct:]" => '!"#$%&\'()*+,-./:;<=>?@[\\]^_`{|}~',
    "[:xdigit:]" => "0123456789ABCDEFabcdef"
  }

  class_map.each { |name, expansion| set_str = set_str.gsub(name, expansion) }

  result = []
  i = 0
  while i < set_str.length
    if i + 2 < set_str.length && set_str[i + 1] == "-" && set_str[i].ord <= set_str[i + 2].ord
      (set_str[i].ord..set_str[i + 2].ord).each { |c| result << c.chr }
      i += 3
    else
      result << set_str[i]
      i += 1
    end
  end

  result.join
end

# ---------------------------------------------------------------------------
# Business Logic: tr_translate
# ---------------------------------------------------------------------------

def tr_translate(text, set1_chars, set2_chars, squeeze:)
  padded_set2 = if set2_chars.empty?
                  set1_chars
                else
                  set2_chars.ljust(set1_chars.length, set2_chars[-1])
                end

  trans_map = {}
  set1_chars.each_char.with_index { |ch, i| trans_map[ch] = padded_set2[i] if i < padded_set2.length }

  squeeze_set = squeeze ? set2_chars.chars.to_set : Set.new

  result = []
  prev_char = ""

  text.each_char do |ch|
    translated = trans_map.fetch(ch, ch)
    next if squeeze && squeeze_set.include?(translated) && translated == prev_char
    result << translated
    prev_char = translated
  end

  result.join
end

# ---------------------------------------------------------------------------
# Business Logic: tr_delete
# ---------------------------------------------------------------------------

def tr_delete(text, set1_chars, squeeze:, squeeze_set_chars:)
  delete_chars = set1_chars.chars.to_set
  squeeze_chars = squeeze ? squeeze_set_chars.chars.to_set : Set.new

  result = []
  prev_char = ""

  text.each_char do |ch|
    next if delete_chars.include?(ch)
    next if squeeze && squeeze_chars.include?(ch) && ch == prev_char
    result << ch
    prev_char = ch
  end

  result.join
end

# ---------------------------------------------------------------------------
# Business Logic: tr_squeeze_only
# ---------------------------------------------------------------------------

def tr_squeeze_only(text, set1_chars)
  squeeze_chars = set1_chars.chars.to_set
  result = []
  prev_char = ""

  text.each_char do |ch|
    next if squeeze_chars.include?(ch) && ch == prev_char
    result << ch
    prev_char = ch
  end

  result.join
end

# ---------------------------------------------------------------------------
# Business Logic: complement_set
# ---------------------------------------------------------------------------

def tr_complement(set_chars)
  char_set = set_chars.chars.to_set
  (0..255).map(&:chr).reject { |c| char_set.include?(c) }.join
end

# ---------------------------------------------------------------------------
# Main entry point
# ---------------------------------------------------------------------------

def tr_main
  begin
    result = CodingAdventures::CliBuilder::Parser.new(TR_SPEC_FILE, ["tr"] + ARGV).parse
  rescue CodingAdventures::CliBuilder::ParseErrors => e
    e.errors.each { |err| warn "tr: #{err.message}" }
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
    complement = result.flags["complement"] || false
    delete = result.flags["delete"] || false
    squeeze = result.flags["squeeze_repeats"] || false

    set1_raw = result.arguments.fetch("set1", "")
    set2_raw = result.arguments["set2"] || ""

    set1_chars = tr_expand_set(set1_raw)
    set2_chars = set2_raw.empty? ? "" : tr_expand_set(set2_raw)

    set1_chars = tr_complement(set1_chars) if complement

    text = $stdin.read || ""

    output = if delete && squeeze
               tr_delete(text, set1_chars, squeeze: true, squeeze_set_chars: set2_chars)
             elsif delete
               tr_delete(text, set1_chars, squeeze: false, squeeze_set_chars: "")
             elsif squeeze && set2_raw.empty?
               tr_squeeze_only(text, set1_chars)
             else
               if set2_raw.empty?
                 warn "tr: missing operand after SET1"
                 exit 1
               end
               tr_translate(text, set1_chars, set2_chars, squeeze: squeeze)
             end

    $stdout.write(output)
  end
end

tr_main if __FILE__ == $PROGRAM_NAME
