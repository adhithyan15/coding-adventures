#!/usr/bin/env ruby
# frozen_string_literal: true

# basename_tool.rb -- Strip directory and suffix from filenames
# ==============================================================
#
# === What This Program Does ===
#
# This is a reimplementation of the GNU `basename` utility. Given a
# pathname, it strips the leading directory components, leaving just
# the final component (the "base name").
#
# === Examples ===
#
#     $ basename /usr/bin/sort
#     sort
#
#     $ basename /usr/include/stdio.h .h
#     stdio
#
#     $ basename -s .h /usr/include/stdio.h /usr/include/stdlib.h
#     stdio
#     stdlib
#
# === How Basename Works ===
#
# The algorithm is simple:
#
# 1. Remove any trailing slashes.
# 2. Remove everything up to and including the last slash.
# 3. If a suffix is specified and the name ends with it (and the name
#    is not equal to the suffix), remove the suffix.
#
# === Traditional vs Multiple Mode ===
#
# Traditional basename takes one NAME and an optional SUFFIX:
#
#     basename NAME [SUFFIX]
#
# With -a or -s, multiple names can be processed:
#
#     basename -a name1 name2 name3
#     basename -s .txt name1.txt name2.txt
#
# The -s flag implies -a (multiple mode).
#
# === Why "basename_tool.rb"? ===
#
# Ruby has a built-in File.basename method. We name this file
# basename_tool.rb to avoid any confusion or shadowing.

require "coding_adventures_cli_builder"

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

BASENAME_SPEC_FILE = File.join(File.dirname(__FILE__), "basename.json")

# ---------------------------------------------------------------------------
# Business Logic: strip_basename
# ---------------------------------------------------------------------------
# Extract the base name from a path, optionally removing a suffix.
#
# This implements the POSIX basename algorithm:
#
# 1. If the entire string is slashes, return "/".
# 2. Strip trailing slashes.
# 3. Strip everything up to and including the last slash.
# 4. If suffix is given and name ends with it (and name != suffix),
#    remove the suffix.
#
# Parameters:
#   path   - The pathname to process
#   suffix - Optional suffix to remove (nil or empty string for none)
#
# Returns: The base name as a string.

def strip_basename(path, suffix = nil)
  # Handle empty string edge case.
  return "" if path.empty?

  # If the path is all slashes, return "/".
  return "/" if path.match?(%r{\A/+\z})

  # Strip trailing slashes.
  name = path.sub(%r{/+\z}, "")

  # Strip leading directory components.
  name = name.split("/").last || ""

  # Remove the suffix if specified and applicable.
  # The suffix is only removed if:
  #   - It's non-empty
  #   - The name ends with it
  #   - The name is not equal to the suffix (so basename(".h", ".h") => ".h")
  if suffix && !suffix.empty? && name.end_with?(suffix) && name != suffix
    name = name[0...(name.length - suffix.length)]
  end

  name
end

# ---------------------------------------------------------------------------
# Main entry point
# ---------------------------------------------------------------------------

def basename_main
  # --- Step 1: Parse arguments ---------------------------------------------
  begin
    result = CodingAdventures::CliBuilder::Parser.new(BASENAME_SPEC_FILE, ["basename"] + ARGV).parse
  rescue CodingAdventures::CliBuilder::ParseErrors => e
    e.errors.each { |err| warn "basename: #{err.message}" }
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
    names = result.arguments.fetch("name", [])
    suffix = result.flags["suffix"]
    multiple = result.flags["multiple"] || (suffix && !suffix.empty?)
    zero = result.flags["zero"]
    terminator = zero ? "\0" : "\n"

    if multiple || names.length > 1
      # Multiple mode: process each name with the -s suffix.
      names.each do |name|
        print strip_basename(name, suffix)
        print terminator
      end
    else
      # Traditional mode: basename NAME [SUFFIX]
      # In traditional mode, if there are exactly 2 args and -a is not
      # set, the second arg is treated as a suffix.
      # But since our spec takes variadic names, and we're not in
      # multiple mode, just process the first name.
      name = names[0] || ""
      print strip_basename(name, suffix)
      print terminator
    end
  end
end

# Only run main when this file is executed directly.
basename_main if __FILE__ == $PROGRAM_NAME
