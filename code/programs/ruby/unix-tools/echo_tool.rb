#!/usr/bin/env ruby
# frozen_string_literal: true

# echo_tool.rb -- Display a line of text
# ========================================
#
# === What This Program Does ===
#
# This is a reimplementation of the `echo` utility. It writes its arguments
# to standard output, separated by single spaces, followed by a newline.
#
# === How echo Works ===
#
# At its simplest, echo joins its arguments with spaces and prints them:
#
#     $ echo hello world
#     hello world
#
# Three flags modify this behavior:
#
#   -n   Suppress the trailing newline. Useful when building prompts or
#        appending to partial lines.
#
#   -e   Enable interpretation of backslash escape sequences. Without this
#        flag, a literal `\n` in the input is printed as two characters
#        (`\` and `n`). With `-e`, it becomes an actual newline character.
#
#   -E   Disable interpretation of backslash escapes (the default). This
#        flag exists so you can explicitly override a previous `-e` in a
#        script.
#
# === Supported Escape Sequences (with -e) ===
#
#   \\    backslash
#   \a    alert (bell, BEL)
#   \b    backspace
#   \c    produce no further output (stops immediately)
#   \f    form feed
#   \n    newline
#   \r    carriage return
#   \t    horizontal tab
#   \v    vertical tab
#   \0nnn octal value (1-3 digits)
#   \xHH  hexadecimal value (1-2 digits)
#
# === POSIX vs GNU ===
#
# POSIX echo has notoriously underspecified behavior around flags and
# escapes. Different systems handle `-n` and `-e` differently. Our
# implementation follows the GNU coreutils convention: escapes are OFF
# by default, and `-e` enables them.

require "coding_adventures_cli_builder"

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

ECHO_SPEC_FILE = File.join(File.dirname(__FILE__), "echo.json")

# ---------------------------------------------------------------------------
# Business Logic: interpret_escapes
# ---------------------------------------------------------------------------
# Process backslash escape sequences in a string.
#
# This function walks through the string character by character. When it
# encounters a backslash, it looks at the next character to determine
# which escape sequence is being used. If the next character is not a
# recognized escape, the backslash is preserved literally.
#
# The special `\c` escape causes the function to stop processing
# immediately -- no further output is produced, not even a trailing
# newline. We signal this to the caller by returning [processed_string, true].
#
# Returns: [String, Boolean] -- the processed string and whether \c was hit.

def interpret_escapes(text)
  result = +""
  i = 0

  while i < text.length
    if text[i] == "\\"
      i += 1

      # If the string ends with a lone backslash, preserve it literally.
      if i >= text.length
        result << "\\"
        break
      end

      case text[i]
      when "\\"  then result << "\\"
      when "a"   then result << "\a"        # Alert (bell)
      when "b"   then result << "\b"        # Backspace
      when "c"   then return [result, true] # Stop all output
      when "f"   then result << "\f"        # Form feed
      when "n"   then result << "\n"        # Newline
      when "r"   then result << "\r"        # Carriage return
      when "t"   then result << "\t"        # Horizontal tab
      when "v"   then result << "\v"        # Vertical tab
      when "0"
        # --- Octal escape: \0nnn (1-3 octal digits after the 0) -----------
        # Examples: \0101 = 'A' (65 decimal), \012 = newline (10 decimal)
        octal = +""
        j = i + 1
        while j < text.length && j < i + 4 && text[j] >= "0" && text[j] <= "7"
          octal << text[j]
          j += 1
        end
        result << (octal.empty? ? "\0" : octal.to_i(8).chr)
        i = j - 1
      when "x"
        # --- Hex escape: \xHH (1-2 hex digits) ----------------------------
        hex = +""
        j = i + 1
        while j < text.length && j < i + 3 && text[j] =~ /[0-9a-fA-F]/
          hex << text[j]
          j += 1
        end
        if hex.empty?
          result << "\\x"
        else
          result << hex.to_i(16).chr
          i = j - 1
        end
      else
        # Unrecognized escape -- preserve the backslash and character.
        result << "\\" << text[i]
      end
    else
      result << text[i]
    end

    i += 1
  end

  [result, false]
end

# ---------------------------------------------------------------------------
# Main entry point
# ---------------------------------------------------------------------------

def echo_main
  # --- Step 1: Parse arguments ---------------------------------------------
  begin
    result = CodingAdventures::CliBuilder::Parser.new(ECHO_SPEC_FILE, ["echo"] + ARGV).parse
  rescue CodingAdventures::CliBuilder::ParseErrors => e
    e.errors.each { |err| warn "echo: #{err.message}" }
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
    # Join all positional arguments with spaces (just like real echo).
    strings = result.arguments.fetch("strings", [])
    output = strings.join(" ")

    # If -e is set, process escape sequences in the output.
    stop_output = false
    if result.flags["enable_escapes"]
      output, stop_output = interpret_escapes(output)
    end

    # Print the output. If -n is set, suppress the trailing newline.
    # If \c was encountered during escape processing, also suppress
    # the trailing newline (and any text after \c was already removed).
    if result.flags["no_newline"] || stop_output
      print output
    else
      puts output
    end
  end
end

# Only run main when this file is executed directly.
echo_main if __FILE__ == $PROGRAM_NAME
