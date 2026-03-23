#!/usr/bin/env ruby
# frozen_string_literal: true

# dirname_tool.rb -- Strip last component from file name
# =======================================================
#
# === What This Program Does ===
#
# This is a reimplementation of the GNU `dirname` utility. Given a
# pathname, it strips the last component (the filename), leaving just
# the directory path.
#
# === Examples ===
#
#     $ dirname /usr/bin/sort
#     /usr/bin
#
#     $ dirname stdio.h
#     .
#
#     $ dirname /usr/
#     /
#
# === How Dirname Works ===
#
# The algorithm mirrors POSIX:
#
# 1. If the path has no slashes, the directory is "." (current dir).
# 2. Strip trailing slashes.
# 3. Strip everything after the last slash.
# 4. Strip trailing slashes again.
# 5. If the result is empty, return "/".
#
# The key insight: dirname and basename are complementary. For any
# path P, `dirname P` / `basename P` reconstructs P (modulo trailing
# slashes and "." for bare filenames).
#
# === Why "dirname_tool.rb"? ===
#
# Ruby has a built-in File.dirname method. We name this file
# dirname_tool.rb to avoid confusion.

require "coding_adventures_cli_builder"

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

DIRNAME_SPEC_FILE = File.join(File.dirname(__FILE__), "dirname.json")

# ---------------------------------------------------------------------------
# Business Logic: compute_dirname
# ---------------------------------------------------------------------------
# Extract the directory component from a path.
#
# This implements the POSIX dirname algorithm:
#
# 1. If path contains no '/', return ".".
# 2. Strip trailing '/' characters.
# 3. Remove everything after the last '/'.
# 4. Strip trailing '/' characters again.
# 5. If the result is empty, return "/".
#
# Parameters:
#   path - The pathname to process
#
# Returns: The directory component as a string.

def compute_dirname(path)
  # Handle empty string.
  return "." if path.empty?

  # If the path is all slashes, return "/".
  return "/" if path.match?(%r{\A/+\z})

  # If there are no slashes at all, the directory is ".".
  return "." unless path.include?("/")

  # Strip trailing slashes.
  work = path.sub(%r{/+\z}, "")

  # If stripping slashes emptied the string (e.g., "/"), return "/".
  return "/" if work.empty?

  # If there are no slashes left after stripping, return ".".
  return "." unless work.include?("/")

  # Remove the last component (everything after the last slash).
  work = work.sub(%r{/[^/]*\z}, "")

  # Strip trailing slashes from the result.
  work = work.sub(%r{/+\z}, "")

  # If the result is empty, it was the root directory.
  work.empty? ? "/" : work
end

# ---------------------------------------------------------------------------
# Main entry point
# ---------------------------------------------------------------------------

def dirname_main
  # --- Step 1: Parse arguments ---------------------------------------------
  begin
    result = CodingAdventures::CliBuilder::Parser.new(DIRNAME_SPEC_FILE, ["dirname"] + ARGV).parse
  rescue CodingAdventures::CliBuilder::ParseErrors => e
    e.errors.each { |err| warn "dirname: #{err.message}" }
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
    names = result.arguments.fetch("names", [])
    zero = result.flags["zero"]
    terminator = zero ? "\0" : "\n"

    names.each do |name|
      print compute_dirname(name)
      print terminator
    end
  end
end

# Only run main when this file is executed directly.
dirname_main if __FILE__ == $PROGRAM_NAME
