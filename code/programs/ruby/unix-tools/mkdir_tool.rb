#!/usr/bin/env ruby
# frozen_string_literal: true

# mkdir_tool.rb -- Create directories
# =====================================
#
# === What This Program Does ===
#
# This is a reimplementation of the GNU `mkdir` utility. It creates one
# or more directories. By default, it creates a single level; with `-p`,
# it creates the full chain of parent directories as needed.
#
# === The -p Flag (Parents) ===
#
# Without `-p`, `mkdir a/b/c` fails if `a/b` doesn't exist.
# With `-p`, mkdir creates `a`, then `a/b`, then `a/b/c` -- the entire
# chain of missing directories. It also silently succeeds if the directory
# already exists, which makes it safe to use in scripts.
#
# === The -m Flag (Mode) ===
#
# The `-m` flag sets the permission bits of the new directory. It accepts
# an octal string like `755` or `0700`.
#
# === The -v Flag (Verbose) ===
#
# With `-v`, mkdir prints a message for each directory it creates:
#
#     $ mkdir -pv a/b/c
#     mkdir: created directory 'a'
#     mkdir: created directory 'a/b'
#     mkdir: created directory 'a/b/c'

require "fileutils"
require "coding_adventures_cli_builder"

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

MKDIR_SPEC_FILE = File.join(File.dirname(__FILE__), "mkdir.json")

# ---------------------------------------------------------------------------
# Business Logic: create_directory
# ---------------------------------------------------------------------------
# Create a single directory, optionally with parents.
#
# Parameters:
#   path    - The directory path to create
#   parents - If true, create parent directories as needed
#   mode    - Octal permission mode (Integer), or nil for default
#   verbose - If true, print a message for each created directory
#
# Returns true on success, false on failure.

def mkdir_create_directory(path, parents:, mode:, verbose:)
  if parents
    FileUtils.mkdir_p(path)
  else
    Dir.mkdir(path)
  end

  # Set mode if specified (mkdir_p doesn't support mode directly).
  FileUtils.chmod(mode, path) if mode

  puts "mkdir: created directory '#{path}'" if verbose
  true
rescue Errno::EEXIST
  warn "mkdir: cannot create directory '#{path}': File exists"
  false
rescue Errno::ENOENT
  warn "mkdir: cannot create directory '#{path}': No such file or directory"
  false
rescue Errno::EACCES
  warn "mkdir: cannot create directory '#{path}': Permission denied"
  false
end

# ---------------------------------------------------------------------------
# Business Logic: parse_mode
# ---------------------------------------------------------------------------
# Parse an octal mode string like '755' into an Integer.
#
# Returns the integer value, or nil if invalid.

def mkdir_parse_mode(mode_str)
  Integer(mode_str, 8)
rescue ArgumentError
  nil
end

# ---------------------------------------------------------------------------
# Main entry point
# ---------------------------------------------------------------------------

def mkdir_main
  # --- Step 1: Parse arguments ---------------------------------------------
  begin
    result = CodingAdventures::CliBuilder::Parser.new(MKDIR_SPEC_FILE, ["mkdir"] + ARGV).parse
  rescue CodingAdventures::CliBuilder::ParseErrors => e
    e.errors.each { |err| warn "mkdir: #{err.message}" }
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
    parents = result.flags["parents"] || false
    mode_str = result.flags["mode"]
    verbose = result.flags["verbose"] || false

    mode = nil
    if mode_str
      mode = mkdir_parse_mode(mode_str)
      if mode.nil?
        warn "mkdir: invalid mode '#{mode_str}'"
        exit 1
      end
    end

    directories = result.arguments.fetch("directories", [])
    directories = [directories] if directories.is_a?(String)

    exit_code = 0
    directories.each do |dir|
      exit_code = 1 unless mkdir_create_directory(dir, parents: parents, mode: mode, verbose: verbose)
    end

    exit exit_code
  end
end

mkdir_main if __FILE__ == $PROGRAM_NAME
