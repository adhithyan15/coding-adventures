#!/usr/bin/env ruby
# frozen_string_literal: true

# rmdir_tool.rb -- Remove empty directories
# ===========================================
#
# === What This Program Does ===
#
# This is a reimplementation of the GNU `rmdir` utility. It removes
# directories, but only if they are empty. This is a safety feature:
# unlike `rm -r`, rmdir will never accidentally delete files.
#
# === The -p Flag (Parents) ===
#
# With `-p`, rmdir removes the directory and then tries to remove each
# parent component of the path. For example:
#
#     $ rmdir -p a/b/c
#
# This first removes `a/b/c`, then `a/b`, then `a`.
#
# === The --ignore-fail-on-non-empty Flag ===
#
# Suppresses error messages when a directory is not empty.

require "coding_adventures_cli_builder"

RMDIR_SPEC_FILE = File.join(File.dirname(__FILE__), "rmdir.json")

# ---------------------------------------------------------------------------
# Business Logic: remove_directory
# ---------------------------------------------------------------------------
# Remove a single empty directory.
#
# Returns true on success, false on failure.

def rmdir_remove_directory(path, verbose:, ignore_non_empty:)
  Dir.rmdir(path)
  puts "rmdir: removing directory, '#{path}'" if verbose
  true
rescue Errno::ENOENT
  warn "rmdir: failed to remove '#{path}': No such file or directory"
  false
rescue Errno::ENOTEMPTY, Errno::EEXIST
  unless ignore_non_empty
    warn "rmdir: failed to remove '#{path}': Directory not empty"
  end
  false
rescue Errno::EACCES
  warn "rmdir: failed to remove '#{path}': Permission denied"
  false
end

# ---------------------------------------------------------------------------
# Business Logic: remove_with_parents
# ---------------------------------------------------------------------------
# Remove a directory and then each parent in turn.

def rmdir_remove_with_parents(path, verbose:, ignore_non_empty:)
  current = path
  success = true

  while current && current != "/" && !current.empty?
    unless rmdir_remove_directory(current, verbose: verbose, ignore_non_empty: ignore_non_empty)
      success = false
      break
    end

    parent = File.dirname(current)
    break if parent == current
    current = parent
  end

  success
end

# ---------------------------------------------------------------------------
# Main entry point
# ---------------------------------------------------------------------------

def rmdir_main
  begin
    result = CodingAdventures::CliBuilder::Parser.new(RMDIR_SPEC_FILE, ["rmdir"] + ARGV).parse
  rescue CodingAdventures::CliBuilder::ParseErrors => e
    e.errors.each { |err| warn "rmdir: #{err.message}" }
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
    parents = result.flags["parents"] || false
    verbose = result.flags["verbose"] || false
    ignore_non_empty = result.flags["ignore_fail_on_non_empty"] || false

    directories = result.arguments.fetch("directories", [])
    directories = [directories] if directories.is_a?(String)

    exit_code = 0
    directories.each do |dir|
      if parents
        exit_code = 1 unless rmdir_remove_with_parents(dir, verbose: verbose, ignore_non_empty: ignore_non_empty)
      else
        exit_code = 1 unless rmdir_remove_directory(dir, verbose: verbose, ignore_non_empty: ignore_non_empty)
      end
    end

    exit exit_code
  end
end

rmdir_main if __FILE__ == $PROGRAM_NAME
